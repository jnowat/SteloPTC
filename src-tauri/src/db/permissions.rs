//! WP-55 — Field-level permissions foundation.
//!
//! Masking is applied only in the read path, per-entity, by the command
//! layer calling `mask_optional_field` after loading a row (Rust has no
//! runtime reflection, so per-entity masking calls are the idiomatic
//! approach here — matching how every other cross-cutting concern in this
//! codebase, e.g. audit logging, is applied explicitly at each call site
//! rather than through a generic interceptor).
//!
//! Currently wired into two representative, explicitly-sensitive fields —
//! `strains.genomic_fingerprint` and `breeding_programs.goal` /
//! `target_traits` — matching the packet's own acceptance-criteria examples.
//! Extending masking to additional fields/entities is a mechanical follow-up
//! (add a seed row + one call to `FieldPermissionSet::mask_optional_field` at
//! the relevant read site) and is intentionally left as future work rather
//! than a sweeping rewrite of every read command in the codebase.
//!
//! **Write-path guard, mandatory wherever a masked field is also writable:**
//! a read command can return the [`RESTRICTED_MARKER`] placeholder in place
//! of a real value. If a UI pre-fills an edit form from that read result and
//! the user submits without noticing, the literal marker string would
//! overwrite the real value — permanently destroying it. Every write command
//! that accepts a value for a masked field **must** call
//! [`reject_if_restricted_marker`] on that value before persisting it. This
//! was found and fixed as a real (not hypothetical) bug: `update_strain_status`
//! unconditionally wrote `genomic_fingerprint` on every call, and
//! `StrainManager.svelte` pre-filled its form from the (possibly masked)
//! current value — see the regression tests below and in `commands::strains`.

use super::DbResult;
use crate::models::permissions::FieldPermission;
use rusqlite::{params, Connection};
use std::collections::HashMap;

/// Returned in place of a real value when a field is masked. Chosen instead
/// of `null` so the frontend can distinguish "no data" from "hidden data"
/// unambiguously, and so masking never has to guess whether an underlying
/// `Option<String>` was already `None` for an unrelated reason.
pub const RESTRICTED_MARKER: &str = "[RESTRICTED]";

/// Canonical registry of every field the read path actually masks at a call
/// site (WP-55). This is the single source of truth that ties together three
/// things which would otherwise drift silently:
///   1. the migration seed (`field_permissions` default rows, migration 036),
///   2. the `apply_field_permissions` mask calls in the command layer
///      (`commands::strains`, `commands::breeding`), and
///   3. what the admin permissions editor is allowed to configure.
///
/// A pair listed here MUST have a corresponding `mask_optional_field` call on
/// **every** read path that returns it. A pair NOT listed here is never masked,
/// even if a `field_permissions` row somehow exists for it — so we refuse to
/// create such a row (see [`set_field_permission`]) rather than let an admin
/// believe they hid a field that the read path still returns in full. The test
/// `maskable_fields_registry_matches_migration_seed` fails the build if this
/// list and the migration seed ever disagree, catching "added a seed row but
/// forgot the call site" (and the reverse) before it ships.
///
/// This is the deliberate hardening of the call-site masking model (WP-55
/// strategy decision, v1.40.2): keep per-call-site masking — the idiomatic
/// choice given Rust has no runtime reflection and the masked surface is
/// intentionally tiny — but pin the set of masked fields to one auditable
/// constant so the model cannot drift out of sync unnoticed.
pub const MASKABLE_FIELDS: &[(&str, &str)] = &[
    ("strain", "genomic_fingerprint"),
    ("breeding_program", "goal"),
    ("breeding_program", "target_traits"),
];

/// Whether `(entity_type, field_name)` is a field the read path actually masks
/// (i.e. present in [`MASKABLE_FIELDS`]). Used to reject permission rules that
/// would otherwise be silent no-ops.
pub fn is_maskable_field(entity_type: &str, field_name: &str) -> bool {
    MASKABLE_FIELDS
        .iter()
        .any(|(e, f)| *e == entity_type && *f == field_name)
}

/// Looks up whether `role` may see `field_name` on `entity_type`. Falls back
/// to visible (permissive default) when no explicit row exists, so adding a
/// brand-new sensitive field never silently locks everyone out before an
/// admin has configured it.
pub fn is_field_visible(conn: &Connection, role: &str, entity_type: &str, field_name: &str) -> bool {
    conn.query_row(
        "SELECT visible FROM field_permissions WHERE role = ?1 AND entity_type = ?2 AND field_name = ?3",
        params![role, entity_type, field_name],
        |r| r.get::<_, i64>(0),
    )
    .map(|v| v != 0)
    .unwrap_or(true)
}

/// Masks a single `Option<String>` field for the given role/entity/field.
/// Never turns `None` into a restricted marker (there's nothing to hide) and
/// never removes the field itself — callers always get `Some(...)` back for
/// a masked, previously-populated value.
///
/// Issues one query per call — fine for a single-record read (`get_strain`),
/// wasteful for a list of N rows (N queries with identical parameters). Use
/// [`FieldPermissionSet`] instead for any list/loop call site.
pub fn mask_optional_field(
    conn: &Connection,
    role: &str,
    entity_type: &str,
    field_name: &str,
    value: Option<String>,
) -> Option<String> {
    if value.is_none() {
        return value;
    }
    if is_field_visible(conn, role, entity_type, field_name) {
        value
    } else {
        Some(RESTRICTED_MARKER.to_string())
    }
}

/// Rejects a write when `value` is literally the restricted-value placeholder.
///
/// Call this on every value a write command accepts for a field that is also
/// gated by masking on read, **before** persisting it. This is the hard
/// backend guarantee that a masked read result can never be round-tripped
/// back into the database as if it were real data — it holds regardless of
/// what any frontend does or fails to do.
pub fn reject_if_restricted_marker(value: Option<&str>, field_label: &str) -> Result<(), String> {
    if value == Some(RESTRICTED_MARKER) {
        Err(format!(
            "'{}' cannot be saved as \"{}\" — that placeholder is shown when a field is \
             restricted for your role; it is never real data. Leave the field blank to keep \
             the existing value unchanged, or ask an admin to grant visibility before editing it.",
            field_label, RESTRICTED_MARKER
        ))
    } else {
        Ok(())
    }
}

/// A role's full set of field-visibility rules, loaded once and then queried
/// in memory — the fix for the N+1 pattern `mask_optional_field` has when
/// called once per field per row across a list of N records. Load one of
/// these per request (not per row) and reuse it for every row.
pub struct FieldPermissionSet {
    /// `(entity_type, field_name) -> visible`. Absent entries default to
    /// visible, matching `is_field_visible`'s permissive-default behavior.
    visible: HashMap<(String, String), bool>,
}

impl FieldPermissionSet {
    pub fn load(conn: &Connection, role: &str) -> DbResult<Self> {
        let mut stmt = conn.prepare(
            "SELECT entity_type, field_name, visible FROM field_permissions WHERE role = ?1",
        )?;
        let rows = stmt.query_map(params![role], |row| {
            Ok((
                (row.get::<_, String>("entity_type")?, row.get::<_, String>("field_name")?),
                row.get::<_, i64>("visible")? != 0,
            ))
        })?;
        let mut visible = HashMap::new();
        for row in rows {
            let (key, is_visible) = row?;
            visible.insert(key, is_visible);
        }
        Ok(Self { visible })
    }

    pub fn is_visible(&self, entity_type: &str, field_name: &str) -> bool {
        self.visible
            .get(&(entity_type.to_string(), field_name.to_string()))
            .copied()
            .unwrap_or(true)
    }

    /// In-memory equivalent of [`mask_optional_field`] — no query per call.
    pub fn mask_optional_field(&self, entity_type: &str, field_name: &str, value: Option<String>) -> Option<String> {
        if value.is_none() {
            return value;
        }
        if self.is_visible(entity_type, field_name) {
            value
        } else {
            Some(RESTRICTED_MARKER.to_string())
        }
    }
}

pub fn list_field_permissions(conn: &Connection) -> DbResult<Vec<FieldPermission>> {
    let mut stmt = conn.prepare(
        "SELECT id, role, entity_type, field_name, visible \
         FROM field_permissions ORDER BY entity_type, field_name, role",
    )?;
    let rows = stmt.query_map([], row_to_field_permission)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn row_to_field_permission(row: &rusqlite::Row) -> rusqlite::Result<FieldPermission> {
    Ok(FieldPermission {
        id: row.get("id")?,
        role: row.get("role")?,
        entity_type: row.get("entity_type")?,
        field_name: row.get("field_name")?,
        visible: row.get::<_, i64>("visible")? != 0,
    })
}

/// Upserts a single (role, entity_type, field_name) visibility rule. Takes
/// effect immediately for every subsequent read — there is no cache to
/// invalidate, since `is_field_visible` always queries live.
pub fn set_field_permission(
    conn: &Connection,
    role: &str,
    entity_type: &str,
    field_name: &str,
    visible: bool,
) -> DbResult<()> {
    // Refuse to store a rule for a field the read path does not actually mask.
    // Without this, an admin could toggle "hide" on an arbitrary field and get
    // a persisted `field_permissions` row that the read path never consults —
    // a false sense of security (the field is still returned in full). Only the
    // fields wired into a call-site mask (see `MASKABLE_FIELDS`) may be
    // configured; everything else is rejected loudly instead of silently
    // no-op'd.
    if !is_maskable_field(entity_type, field_name) {
        return Err(crate::db::DbError::Constraint(format!(
            "Field '{}.{}' is not a maskable field — only {:?} can have visibility rules. \
             Configuring visibility here would have no effect on what the read path returns.",
            entity_type, field_name, MASKABLE_FIELDS
        )));
    }
    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM field_permissions WHERE role = ?1 AND entity_type = ?2 AND field_name = ?3",
            params![role, entity_type, field_name],
            |r| r.get(0),
        )
        .ok();

    if let Some(id) = existing {
        conn.execute(
            "UPDATE field_permissions SET visible = ?1 WHERE id = ?2",
            params![visible as i64, id],
        )?;
    } else {
        conn.execute(
            "INSERT INTO field_permissions (id, role, entity_type, field_name, visible) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![uuid::Uuid::new_v4().to_string(), role, entity_type, field_name, visible as i64],
        )?;
    }
    Ok(())
}

/// Pure, independently-testable admin gate for the permissions editor,
/// mirroring the `check_profile_change_allowed` pattern (WP-26) of keeping
/// authorization logic as a plain function the command layer calls into.
pub fn validate_admin_role(role: &str) -> Result<(), String> {
    if role == "admin" {
        Ok(())
    } else {
        Err("Only admins can manage field permissions".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations::run_all;

    fn migrated_db() -> Connection {
        let conn = Connection::open_in_memory().expect("in-memory DB");
        run_all(&conn).expect("all migrations must succeed on a fresh in-memory DB");
        conn
    }

    #[test]
    fn is_field_visible_defaults_true_for_unseeded_field() {
        let conn = migrated_db();
        assert!(is_field_visible(&conn, "tech", "strain", "some_new_field_no_row_yet"));
    }

    #[test]
    fn migration_seeds_permissive_defaults_for_genomic_fingerprint() {
        let conn = migrated_db();
        for role in ["admin", "supervisor", "tech", "guest"] {
            assert!(
                is_field_visible(&conn, role, "strain", "genomic_fingerprint"),
                "role '{}' must default to visible for genomic_fingerprint",
                role
            );
        }
    }

    #[test]
    fn migration_seeds_permissive_defaults_for_breeding_fields() {
        let conn = migrated_db();
        for field in ["goal", "target_traits"] {
            for role in ["admin", "supervisor", "tech", "guest"] {
                assert!(is_field_visible(&conn, role, "breeding_program", field));
            }
        }
    }

    #[test]
    fn set_field_permission_hides_field_for_role() {
        let conn = migrated_db();
        set_field_permission(&conn, "tech", "strain", "genomic_fingerprint", false).unwrap();
        assert!(!is_field_visible(&conn, "tech", "strain", "genomic_fingerprint"));
        // Other roles are untouched.
        assert!(is_field_visible(&conn, "supervisor", "strain", "genomic_fingerprint"));
    }

    #[test]
    fn maskable_fields_registry_matches_migration_seed() {
        // Tripwire for the call-site masking model (WP-55): the fields the
        // migration seeds into `field_permissions` must be exactly the fields
        // the code masks at a call site (`MASKABLE_FIELDS`). If someone seeds a
        // new field without wiring a mask call (or removes a mask without
        // updating the seed), this fails — the two can never drift silently.
        let conn = migrated_db();
        let mut seeded: Vec<(String, String)> = conn
            .prepare("SELECT DISTINCT entity_type, field_name FROM field_permissions")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        let mut registry: Vec<(String, String)> = MASKABLE_FIELDS
            .iter()
            .map(|(e, f)| (e.to_string(), f.to_string()))
            .collect();
        seeded.sort();
        registry.sort();
        assert_eq!(
            seeded, registry,
            "the field_permissions migration seed and the MASKABLE_FIELDS registry must stay in sync"
        );
    }

    #[test]
    fn set_field_permission_rejects_a_non_maskable_field() {
        // Guard: an admin must not be able to persist a visibility rule for a
        // field the read path never masks — that would be a silent no-op that
        // looks like protection but isn't. `strain.name` is a real column but
        // is not in MASKABLE_FIELDS.
        let conn = migrated_db();
        let result = set_field_permission(&conn, "tech", "strain", "name", false);
        assert!(result.is_err(), "setting a rule for a non-maskable field must be refused");
        let msg = format!("{:?}", result.unwrap_err());
        assert!(msg.contains("not a maskable field"), "error should explain why: {msg}");
        // And nothing was written.
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM field_permissions WHERE entity_type='strain' AND field_name='name'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "a rejected rule must not leave a row behind");
    }

    #[test]
    fn is_maskable_field_matches_registry() {
        assert!(is_maskable_field("strain", "genomic_fingerprint"));
        assert!(is_maskable_field("breeding_program", "goal"));
        assert!(is_maskable_field("breeding_program", "target_traits"));
        assert!(!is_maskable_field("strain", "name"));
        assert!(!is_maskable_field("specimen", "notes"));
    }

    #[test]
    fn set_field_permission_upserts_without_duplicating() {
        let conn = migrated_db();
        set_field_permission(&conn, "tech", "strain", "genomic_fingerprint", false).unwrap();
        set_field_permission(&conn, "tech", "strain", "genomic_fingerprint", true).unwrap();
        assert!(is_field_visible(&conn, "tech", "strain", "genomic_fingerprint"));

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM field_permissions WHERE role='tech' AND entity_type='strain' AND field_name='genomic_fingerprint'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1, "upsert must not create a duplicate row");
    }

    #[test]
    fn mask_optional_field_returns_real_value_when_visible() {
        let conn = migrated_db();
        let value = mask_optional_field(&conn, "admin", "strain", "genomic_fingerprint", Some("ATCG123".to_string()));
        assert_eq!(value.as_deref(), Some("ATCG123"));
    }

    #[test]
    fn mask_optional_field_returns_restricted_marker_when_hidden() {
        let conn = migrated_db();
        set_field_permission(&conn, "tech", "strain", "genomic_fingerprint", false).unwrap();
        let value = mask_optional_field(&conn, "tech", "strain", "genomic_fingerprint", Some("ATCG123".to_string()));
        assert_eq!(value.as_deref(), Some(RESTRICTED_MARKER));
    }

    #[test]
    fn mask_optional_field_never_masks_a_none_value() {
        let conn = migrated_db();
        set_field_permission(&conn, "tech", "strain", "genomic_fingerprint", false).unwrap();
        let value = mask_optional_field(&conn, "tech", "strain", "genomic_fingerprint", None);
        assert_eq!(value, None, "there is nothing to hide when the underlying value is already absent");
    }

    #[test]
    fn list_field_permissions_returns_seeded_rows() {
        let conn = migrated_db();
        let perms = list_field_permissions(&conn).unwrap();
        // 4 roles x 3 fields (genomic_fingerprint, goal, target_traits) seeded by migration 036.
        assert_eq!(perms.len(), 12);
        assert!(perms.iter().all(|p| p.visible));
    }

    #[test]
    fn validate_admin_role_accepts_admin_rejects_others() {
        assert!(validate_admin_role("admin").is_ok());
        assert!(validate_admin_role("supervisor").is_err());
        assert!(validate_admin_role("tech").is_err());
        assert!(validate_admin_role("guest").is_err());
    }

    #[test]
    fn masking_never_reaches_audit_log_writes() {
        // Architectural guarantee: log_audit always stores the raw value it is
        // given. Masking is applied only when a read command constructs its
        // response — it has no code path into the audit-write functions.
        let conn = migrated_db();
        conn.execute(
            "INSERT INTO users (id, username, password_hash, display_name, role) VALUES ('user-1', 'user-1', 'x', 'Test', 'tech')",
            [],
        ).unwrap();
        set_field_permission(&conn, "tech", "strain", "genomic_fingerprint", false).unwrap();

        crate::db::queries::log_audit(
            &conn,
            Some("user-1"),
            "update",
            "strain",
            Some("strain-1"),
            None,
            Some("ATCG-SENSITIVE-VALUE"),
            None,
        )
        .unwrap();

        let stored: String = conn
            .query_row(
                "SELECT new_value FROM audit_log WHERE entity_type = 'strain' AND entity_id = 'strain-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, "ATCG-SENSITIVE-VALUE", "audit log must always capture the full, unmasked value");
    }

    // ── reject_if_restricted_marker (write-path corruption guard) ────────────

    #[test]
    fn reject_if_restricted_marker_rejects_the_exact_marker() {
        let result = reject_if_restricted_marker(Some(RESTRICTED_MARKER), "Genomic fingerprint");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Genomic fingerprint"));
    }

    #[test]
    fn reject_if_restricted_marker_allows_real_values() {
        assert!(reject_if_restricted_marker(Some("ATCG123"), "Genomic fingerprint").is_ok());
    }

    #[test]
    fn reject_if_restricted_marker_allows_none() {
        assert!(reject_if_restricted_marker(None, "Genomic fingerprint").is_ok());
    }

    #[test]
    fn reject_if_restricted_marker_allows_empty_string() {
        // Empty string is a real (if unusual) value a caller might legitimately
        // send to clear a field — only the exact marker text is rejected.
        assert!(reject_if_restricted_marker(Some(""), "Genomic fingerprint").is_ok());
    }

    // ── FieldPermissionSet (N+1 fix) ──────────────────────────────────────────

    #[test]
    fn field_permission_set_matches_is_field_visible_for_seeded_defaults() {
        let conn = migrated_db();
        let set = FieldPermissionSet::load(&conn, "tech").unwrap();
        assert_eq!(set.is_visible("strain", "genomic_fingerprint"), is_field_visible(&conn, "tech", "strain", "genomic_fingerprint"));
        assert_eq!(set.is_visible("breeding_program", "goal"), is_field_visible(&conn, "tech", "breeding_program", "goal"));
    }

    #[test]
    fn field_permission_set_defaults_true_for_unseeded_field() {
        let conn = migrated_db();
        let set = FieldPermissionSet::load(&conn, "tech").unwrap();
        assert!(set.is_visible("strain", "some_new_field_no_row_yet"));
    }

    #[test]
    fn field_permission_set_reflects_restrictions_after_load() {
        let conn = migrated_db();
        set_field_permission(&conn, "tech", "strain", "genomic_fingerprint", false).unwrap();
        let set = FieldPermissionSet::load(&conn, "tech").unwrap();
        assert!(!set.is_visible("strain", "genomic_fingerprint"));

        // A different role's set, loaded separately, is unaffected.
        let admin_set = FieldPermissionSet::load(&conn, "admin").unwrap();
        assert!(admin_set.is_visible("strain", "genomic_fingerprint"));
    }

    #[test]
    fn field_permission_set_mask_optional_field_matches_free_function() {
        let conn = migrated_db();
        set_field_permission(&conn, "tech", "strain", "genomic_fingerprint", false).unwrap();
        let set = FieldPermissionSet::load(&conn, "tech").unwrap();

        let via_set = set.mask_optional_field("strain", "genomic_fingerprint", Some("ATCG123".to_string()));
        let via_free_fn = mask_optional_field(&conn, "tech", "strain", "genomic_fingerprint", Some("ATCG123".to_string()));
        assert_eq!(via_set, via_free_fn);
        assert_eq!(via_set.as_deref(), Some(RESTRICTED_MARKER));
    }

    #[test]
    fn field_permission_set_never_masks_none() {
        let conn = migrated_db();
        set_field_permission(&conn, "tech", "strain", "genomic_fingerprint", false).unwrap();
        let set = FieldPermissionSet::load(&conn, "tech").unwrap();
        assert_eq!(set.mask_optional_field("strain", "genomic_fingerprint", None), None);
    }

    #[test]
    fn field_permission_set_one_query_covers_a_full_list() {
        // Regression guard for the N+1 pattern: loading once and checking N
        // times must not touch the database again after `load`. We can't
        // directly count queries here without instrumentation, but we can
        // assert the set is fully self-contained (no `Connection` stored)
        // by checking its type has no lifetime tied to the connection —
        // this is enforced at compile time (FieldPermissionSet owns a
        // HashMap<(String,String), bool>, not a &Connection), so a
        // compile-only check is the meaningful assertion here.
        let conn = migrated_db();
        let set = FieldPermissionSet::load(&conn, "tech").unwrap();
        drop(conn); // if `set` borrowed the connection, this would fail to compile/drop safely
        assert!(set.is_visible("strain", "genomic_fingerprint"));
    }
}
