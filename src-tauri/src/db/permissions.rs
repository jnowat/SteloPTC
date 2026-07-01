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
//! (add a seed row + one `mask_optional_field` call at the relevant read
//! site) and is intentionally left as future work rather than a sweeping
//! rewrite of every read command in the codebase.

use super::DbResult;
use crate::models::permissions::FieldPermission;
use rusqlite::{params, Connection};

/// Returned in place of a real value when a field is masked. Chosen instead
/// of `null` so the frontend can distinguish "no data" from "hidden data"
/// unambiguously, and so masking never has to guess whether an underlying
/// `Option<String>` was already `None` for an unrelated reason.
pub const RESTRICTED_MARKER: &str = "[RESTRICTED]";

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
}
