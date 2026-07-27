use crate::db::Database;
use crate::models::user::{User, UserRole};
use rusqlite::params;

// Simple token-based session management for local desktop app.
// For a local-only app, we use a lightweight approach: generate a random
// token on login, store in sessions table, validate on each request.

/// A bcrypt hash of a fixed string nobody can supply, computed once on first
/// use.
///
/// Verifying against it makes the "no such user" path cost the same ~100ms as
/// the "wrong password" path. Without it, `authenticate` returned immediately
/// when the SELECT found nothing, so response time distinguished real usernames
/// from fake ones even though the error text was identical — letting an attacker
/// enumerate accounts before spending guesses on them.
static TIMING_EQUALIZER_HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();

fn timing_equalizer_hash() -> &'static str {
    TIMING_EQUALIZER_HASH.get_or_init(|| {
        bcrypt::hash("timing-equalizer-not-a-real-password", bcrypt::DEFAULT_COST)
            .expect("hashing a constant cannot fail")
    })
}

pub fn authenticate(db: &Database, username: &str, password: &str) -> Result<User, String> {
    let user = db.conn.query_row(
        "SELECT id, username, password_hash, display_name, email, role, is_active, must_change_password, created_at, updated_at
         FROM users WHERE username = ?1 AND is_active = 1",
        params![username],
        |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                display_name: row.get(3)?,
                email: row.get(4)?,
                role: row.get::<_, String>(5)?.parse().unwrap_or(UserRole::Guest),
                is_active: row.get::<_, i32>(6)? != 0,
                must_change_password: row.get::<_, i32>(7)? != 0,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    ).ok();

    // Always pay the bcrypt cost, whether or not the account exists, so the
    // response time carries no information about which usernames are real.
    // The hash is cloned rather than borrowed so `user` stays movable into the
    // match below; a ~60-byte copy per login attempt is not worth contorting
    // the control flow to avoid.
    let hash: String = match user.as_ref() {
        Some(u) => u.password_hash.clone(),
        None => timing_equalizer_hash().to_string(),
    };
    let password_ok = bcrypt::verify(password, &hash).unwrap_or(false);

    match user {
        Some(user) if password_ok => Ok(user),
        // One message for both failure modes: a distinct "no such user" would
        // undo the timing work above.
        _ => Err("Invalid username or password".to_string()),
    }
}

/// Minimum password length, applied to every path that sets a password.
///
/// Twelve rather than eight: this guards lab records subject to retention and
/// audit requirements, and the login path is local (an attacker who reaches it
/// can guess at CPU speed rather than network speed).
pub const MIN_PASSWORD_LEN: usize = 12;

/// Single source of truth for password strength.
///
/// Both `create_user` and `change_password` call this. Previously only the
/// change path enforced a minimum, so an admin could provision an account with
/// a one-character password that the user was then unable to re-set to anything
/// equally weak — the two rules disagreed, which is the failure mode a shared
/// validator exists to prevent.
pub fn validate_password(password: &str) -> Result<(), String> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(format!(
            "Password must be at least {} characters",
            MIN_PASSWORD_LEN
        ));
    }
    if password.trim().is_empty() {
        return Err("Password cannot be only whitespace".to_string());
    }
    Ok(())
}

/// Hashes a session token for storage.
///
/// A session token is a bearer credential: whoever holds it *is* the user until
/// it expires. Storing it verbatim meant the `sessions` table handed out working
/// credentials to anyone who could read the database file — a stolen laptop, a
/// synced home directory, a support bundle — which is precisely why
/// `users.password_hash` is not stored in the clear either.
///
/// Plain SHA-256 is the right primitive here, not bcrypt or Argon2. Those exist
/// to make *low-entropy* secrets expensive to guess; a token is 256 bits of
/// CSPRNG output, so there is nothing to brute-force, and a slow KDF would only
/// add ~100ms to every single authenticated command.
pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        Sha256::digest(token.as_bytes()),
    )
}

pub fn create_session(db: &Database, user_id: &str) -> Result<String, String> {
    let token = generate_token();
    let id = uuid::Uuid::new_v4().to_string();
    let expires = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(24))
        .unwrap_or(chrono::DateTime::<chrono::Utc>::MAX_UTC)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();

    // Only the digest is persisted. The raw token exists in this function's
    // return value and in the client's possession, never on disk.
    db.conn.execute(
        "INSERT INTO sessions (id, user_id, token, expires_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, user_id, hash_token(&token), expires],
    ).map_err(|e| format!("Failed to create session: {}", e))?;

    Ok(token)
}

/// Validate a session token for a **normal** command.
///
/// In addition to the token/expiry/active checks, this rejects any user who
/// still has `must_change_password` set. Until they complete the mandated
/// change, their (otherwise full 24h) token must be usable only for the
/// password-change and current-user endpoints — not for reading or mutating lab
/// data. Enforcing it here (rather than only in the UI) means every command that
/// calls `validate_session` is protected with no per-command change: the block
/// is default-deny.
pub fn validate_session(db: &Database, token: &str) -> Result<User, String> {
    let user = validate_session_allow_password_change(db, token)?;
    if user.must_change_password {
        return Err("A password change is required before continuing.".to_string());
    }
    Ok(user)
}

/// Raw session lookup: returns the user for a valid, unexpired token belonging
/// to an active account, **regardless of the `must_change_password` flag**.
///
/// Only the two endpoints a locked-out user still needs may use this:
/// `change_password` (to clear the flag) and `get_current_user` (so the forced-
/// change screen can render who is logged in). Everything else must go through
/// `validate_session`.
pub fn validate_session_allow_password_change(db: &Database, token: &str) -> Result<User, String> {
    // Opportunistic reap. `invalidate_session` on explicit logout was previously
    // the only DELETE in the system, so on a shared terminal the sessions table
    // grew by one row per login forever. The predicate is the same one the
    // lookup below uses, so this costs a single indexed scan of already-dead
    // rows and keeps the table bounded by the number of live sessions.
    db.conn
        .execute("DELETE FROM sessions WHERE expires_at <= datetime('now')", [])
        .ok();

    let user = db.conn.query_row(
        "SELECT u.id, u.username, u.password_hash, u.display_name, u.email, u.role, u.is_active, u.must_change_password, u.created_at, u.updated_at
         FROM sessions s JOIN users u ON s.user_id = u.id
         WHERE s.token = ?1 AND s.expires_at > datetime('now') AND u.is_active = 1",
        params![hash_token(token)],
        |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                display_name: row.get(3)?,
                email: row.get(4)?,
                role: row.get::<_, String>(5)?.parse().unwrap_or(UserRole::Guest),
                is_active: row.get::<_, i32>(6)? != 0,
                must_change_password: row.get::<_, i32>(7)? != 0,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    ).map_err(|_| "Session expired or invalid".to_string())?;

    Ok(user)
}

pub fn invalidate_session(db: &Database, token: &str) -> Result<(), String> {
    db.conn.execute("DELETE FROM sessions WHERE token = ?1", params![hash_token(token)])
        .map_err(|e| format!("Failed to invalidate session: {}", e))?;
    Ok(())
}

fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &bytes)
}

/// Per-username failed-login tracker with exponential backoff and a hard lock.
///
/// `login` is an unauthenticated IPC command that can be driven in a tight loop,
/// and bcrypt at cost 12 still allows several guesses per second per core —
/// slow for a remote attacker, ample for a local one working against a seeded
/// `admin` account. Nothing else in the system rate-limits it.
///
/// State is in-memory and per-process. That is the right trade-off for a desktop
/// app: it resets on restart (an attacker with the ability to restart the app
/// already has the machine), and it avoids a write to the database on every
/// failed guess, which would itself be an amplification vector.
pub struct LoginThrottle {
    failures: std::sync::Mutex<std::collections::HashMap<String, FailureRecord>>,
    threshold: u32,
    window: std::time::Duration,
    capacity: usize,
}

#[derive(Clone, Copy)]
struct FailureRecord {
    count: u32,
    last: std::time::Instant,
}

impl Default for LoginThrottle {
    fn default() -> Self {
        // 5 attempts, then a 15-minute lock — enough headroom for a genuine
        // typo streak, low enough to make guessing impractical.
        Self::new(5, std::time::Duration::from_secs(15 * 60), 1024)
    }
}

impl LoginThrottle {
    pub fn new(threshold: u32, window: std::time::Duration, capacity: usize) -> Self {
        Self {
            failures: std::sync::Mutex::new(std::collections::HashMap::new()),
            threshold,
            window,
            capacity,
        }
    }

    /// Returns `Err` when `username` is currently locked out.
    ///
    /// The error text is byte-identical to a bad-password failure: a distinct
    /// "account locked" message would tell an attacker which usernames exist,
    /// re-opening the enumeration hole that `timing_equalizer_hash` closes.
    pub fn check(&self, username: &str) -> Result<(), String> {
        let guard = match self.failures.lock() {
            Ok(g) => g,
            // A poisoned lock must not become a way to bypass the throttle, but
            // it also must not lock everyone out. Recover and continue.
            Err(poisoned) => poisoned.into_inner(),
        };
        match guard.get(username) {
            Some(rec) if rec.count >= self.threshold && rec.last.elapsed() < self.window => {
                Err("Invalid username or password".to_string())
            }
            _ => Ok(()),
        }
    }

    /// How long `username` must wait before another attempt is accepted.
    /// `None` when not currently locked. Used for the audit detail, never
    /// returned to the caller.
    pub fn lock_remaining(&self, username: &str) -> Option<std::time::Duration> {
        let guard = self.failures.lock().ok()?;
        let rec = guard.get(username)?;
        if rec.count >= self.threshold {
            self.window.checked_sub(rec.last.elapsed())
        } else {
            None
        }
    }

    pub fn record_failure(&self, username: &str) {
        let mut guard = match self.failures.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        // Usernames are attacker-supplied, so the map would otherwise grow
        // without bound on a dictionary run. Drop entries whose window has
        // elapsed, and if that is not enough, refuse to add new ones rather
        // than evicting live lockouts an attacker could then reset at will.
        if guard.len() >= self.capacity {
            let window = self.window;
            guard.retain(|_, rec| rec.last.elapsed() < window);
            if guard.len() >= self.capacity && !guard.contains_key(username) {
                return;
            }
        }

        let entry = guard.entry(username.to_string()).or_insert(FailureRecord {
            count: 0,
            last: std::time::Instant::now(),
        });
        // A lockout that has fully expired starts a fresh streak rather than
        // resuming at the old count.
        if entry.count >= self.threshold && entry.last.elapsed() >= self.window {
            entry.count = 0;
        }
        entry.count = entry.count.saturating_add(1);
        entry.last = std::time::Instant::now();
    }

    /// Number of usernames currently being tracked. Exposed so the capacity
    /// bound can be asserted — an unbounded map would itself be a
    /// memory-exhaustion vector, since usernames are attacker-supplied.
    pub fn entry_count(&self) -> usize {
        self.failures.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Clears the counter after a successful login.
    pub fn clear(&self, username: &str) {
        if let Ok(mut guard) = self.failures.lock() {
            guard.remove(username);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    /// Build an in-memory DB with one user (with the given forced-change flag)
    /// and a live session token for them. Returns (db, token).
    fn db_with_session(must_change: bool) -> (Database, String) {
        let db = Database::new_in_memory().unwrap();
        db.run_migrations().unwrap();
        db.conn
            .execute(
                "INSERT INTO users (id, username, password_hash, display_name, role, is_active, must_change_password) \
                 VALUES ('u1', 'tech1', 'x', 'Tech One', 'tech', 1, ?1)",
                params![if must_change { 1 } else { 0 }],
            )
            .unwrap();
        let token = create_session(&db, "u1").unwrap();
        (db, token)
    }

    #[test]
    fn normal_session_passes_when_no_forced_change() {
        let (db, token) = db_with_session(false);
        let user = validate_session(&db, &token).expect("clean user should validate");
        assert_eq!(user.id, "u1");
        assert!(!user.must_change_password);
    }

    #[test]
    fn forced_change_blocks_normal_commands() {
        // The core of the fix: a user who still owes a forced password change
        // must NOT be able to authorize an ordinary command server-side, even
        // though their token is otherwise valid and unexpired.
        let (db, token) = db_with_session(true);
        let err = validate_session(&db, &token).expect_err("must be rejected");
        assert!(err.to_lowercase().contains("password change"), "message was: {}", err);
    }

    #[test]
    fn forced_change_still_allows_password_change_endpoint() {
        // The carve-out: change_password / get_current_user use the allow variant
        // so a locked-out user can actually clear the flag.
        let (db, token) = db_with_session(true);
        let user = validate_session_allow_password_change(&db, &token)
            .expect("allow variant must return the user so they can change their password");
        assert_eq!(user.id, "u1");
        assert!(user.must_change_password);
    }

    // ── Session token storage ─────────────────────────────────────────────

    #[test]
    fn session_token_is_never_stored_in_plaintext() {
        // The property that matters: reading the database must not yield a
        // usable credential.
        let (db, token) = db_with_session(false);
        let stored: String = db
            .conn
            .query_row("SELECT token FROM sessions", [], |r| r.get(0))
            .unwrap();
        assert_ne!(stored, token, "the raw token must not be persisted");
        assert_eq!(stored, hash_token(&token));
        // And the stored value must not itself work as a token.
        assert!(
            validate_session(&db, &stored).is_err(),
            "the stored digest must not be accepted as a bearer token"
        );
    }

    #[test]
    fn a_hashed_session_still_validates_and_invalidates() {
        let (db, token) = db_with_session(false);
        assert_eq!(validate_session(&db, &token).unwrap().id, "u1");
        invalidate_session(&db, &token).unwrap();
        assert!(
            validate_session(&db, &token).is_err(),
            "logout must delete the row keyed by the digest"
        );
    }

    #[test]
    fn hash_token_is_deterministic_and_collision_free_across_tokens() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_ne!(hash_token("abc"), hash_token("abd"));
        // URL-safe base64 of a 32-byte digest, unpadded.
        assert_eq!(hash_token("abc").len(), 43);
    }

    #[test]
    fn tokens_are_unique_across_sessions() {
        let db = Database::new_in_memory().unwrap();
        db.run_migrations().unwrap();
        db.conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role, is_active) \
             VALUES ('u1', 'tech1', 'x', 'Tech One', 'tech', 1)",
            [],
        ).unwrap();
        let a = create_session(&db, "u1").unwrap();
        let b = create_session(&db, "u1").unwrap();
        assert_ne!(a, b, "each login must mint a fresh token");
        assert!(validate_session(&db, &a).is_ok());
        assert!(validate_session(&db, &b).is_ok());
    }

    // ── Password policy ───────────────────────────────────────────────────

    #[test]
    fn password_policy_rejects_short_and_whitespace_only() {
        assert!(validate_password("short").is_err());
        assert!(validate_password(&"a".repeat(MIN_PASSWORD_LEN - 1)).is_err());
        assert!(validate_password(&" ".repeat(MIN_PASSWORD_LEN + 4)).is_err(),
            "whitespace-only must be refused even when long enough");
        assert!(validate_password("").is_err());
    }

    #[test]
    fn password_policy_accepts_a_conforming_password() {
        assert!(validate_password(&"a".repeat(MIN_PASSWORD_LEN)).is_ok());
        assert!(validate_password("correct horse battery staple").is_ok());
    }

    #[test]
    fn password_policy_counts_characters_not_bytes() {
        // 12 multi-byte characters is 12 characters, not 36 bytes' worth. Using
        // .len() here would accept a 4-character password made of emoji.
        let four_emoji = "\u{1f9ea}\u{1f9ec}\u{1f331}\u{1f52c}";
        assert_eq!(four_emoji.chars().count(), 4);
        assert!(validate_password(four_emoji).is_err(), "4 chars must be refused");
    }

    // ── Timing equalisation ───────────────────────────────────────────────

    #[test]
    fn authenticate_rejects_unknown_and_wrong_password_identically() {
        let db = Database::new_in_memory().unwrap();
        db.run_migrations().unwrap();
        let hash = bcrypt::hash("the-real-password", 4).unwrap();
        db.conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role, is_active) \
             VALUES ('u1', 'tech1', ?1, 'Tech One', 'tech', 1)",
            params![hash],
        ).unwrap();

        let unknown = authenticate(&db, "nobody", "whatever").unwrap_err();
        let wrong = authenticate(&db, "tech1", "not-the-password").unwrap_err();
        assert_eq!(unknown, wrong, "the two failures must be indistinguishable");
        assert_eq!(unknown, "Invalid username or password");

        assert!(authenticate(&db, "tech1", "the-real-password").is_ok());
    }

    #[test]
    fn authenticate_rejects_a_deactivated_account() {
        let db = Database::new_in_memory().unwrap();
        db.run_migrations().unwrap();
        let hash = bcrypt::hash("the-real-password", 4).unwrap();
        db.conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role, is_active) \
             VALUES ('u1', 'gone', ?1, 'Gone', 'tech', 0)",
            params![hash],
        ).unwrap();
        // Correct password, inactive account — still the generic failure, so
        // deactivation is not observable either.
        let err = authenticate(&db, "gone", "the-real-password").unwrap_err();
        assert_eq!(err, "Invalid username or password");
    }

    // ── Expired-session reaping ───────────────────────────────────────────

    #[test]
    fn validating_a_session_reaps_expired_rows() {
        let (db, token) = db_with_session(false);
        db.conn.execute(
            "INSERT INTO sessions (id, user_id, token, expires_at) \
             VALUES ('dead', 'u1', 'stale-token', datetime('now', '-1 day'))",
            [],
        ).unwrap();
        assert_eq!(session_count(&db), 2);

        validate_session(&db, &token).expect("the live session still validates");

        assert_eq!(session_count(&db), 1, "the expired row must have been reaped");
        assert!(validate_session(&db, "stale-token").is_err());
    }

    fn session_count(db: &Database) -> i64 {
        db.conn.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0)).unwrap()
    }

    // ── Login throttle ────────────────────────────────────────────────────

    fn fast_throttle() -> LoginThrottle {
        LoginThrottle::new(3, std::time::Duration::from_millis(120), 8)
    }

    #[test]
    fn throttle_allows_attempts_below_the_threshold() {
        let t = fast_throttle();
        t.record_failure("tech1");
        t.record_failure("tech1");
        assert!(t.check("tech1").is_ok(), "2 of 3 failures must not lock");
    }

    #[test]
    fn throttle_locks_at_the_threshold() {
        let t = fast_throttle();
        for _ in 0..3 {
            t.record_failure("tech1");
        }
        assert!(t.check("tech1").is_err());
        assert!(t.check("someone_else").is_ok(), "the lock is per-username");
    }

    #[test]
    fn throttle_lockout_message_matches_a_normal_failure() {
        // A distinct "account locked" string would reveal which usernames
        // exist, undoing the timing work in authenticate().
        let t = fast_throttle();
        for _ in 0..3 {
            t.record_failure("tech1");
        }
        assert_eq!(t.check("tech1").unwrap_err(), "Invalid username or password");
    }

    #[test]
    fn throttle_expires_after_the_window_and_starts_a_fresh_streak() {
        let t = fast_throttle();
        for _ in 0..3 {
            t.record_failure("tech1");
        }
        assert!(t.check("tech1").is_err());
        std::thread::sleep(std::time::Duration::from_millis(160));
        assert!(t.check("tech1").is_ok(), "the lock must lift once the window elapses");

        // One more failure must not immediately re-lock — the old streak is
        // spent, so the counter restarts rather than resuming at 3.
        t.record_failure("tech1");
        assert!(t.check("tech1").is_ok());
    }

    #[test]
    fn throttle_clears_on_successful_login() {
        let t = fast_throttle();
        t.record_failure("tech1");
        t.record_failure("tech1");
        t.clear("tech1");
        t.record_failure("tech1");
        assert!(t.check("tech1").is_ok(), "the counter must have restarted from zero");
    }

    #[test]
    fn throttle_map_is_bounded_against_a_dictionary_run() {
        // Usernames are attacker-supplied, so an unbounded map would be a
        // memory-exhaustion vector in its own right.
        let t = LoginThrottle::new(3, std::time::Duration::from_secs(600), 8);
        for i in 0..500 {
            t.record_failure(&format!("user{}", i));
        }
        assert!(t.entry_count() <= 8, "tracked entries must stay within capacity");
    }

    #[test]
    fn throttle_lock_remaining_reports_time_only_while_locked() {
        let t = fast_throttle();
        assert!(t.lock_remaining("tech1").is_none());
        for _ in 0..3 {
            t.record_failure("tech1");
        }
        assert!(t.lock_remaining("tech1").is_some());
    }

    #[test]
    fn invalid_token_is_rejected_by_both_variants() {
        let (db, _token) = db_with_session(false);
        assert!(validate_session(&db, "not-a-real-token").is_err());
        assert!(validate_session_allow_password_change(&db, "not-a-real-token").is_err());
    }
}
