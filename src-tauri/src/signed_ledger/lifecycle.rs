//! WP-75: canonical lifecycle event definitions for wiring signed events into
//! specimen mutation commands — the WP-67 "extend automatic signing to all
//! mutation commands" follow-up.
//!
//! WP-67 shipped the full signing/verification engine (`super`) and wired a
//! single demonstrating call (`specimen_created`), disclosing that extending it
//! to the rest of the lifecycle was incremental follow-up. This module supplies
//! that: it centralizes the event-type vocabulary and builds each signed event's
//! payload, so every call site emits a consistent, auditable JSON body instead of
//! ad-hoc inline objects — and so the payload shape is unit-tested here once,
//! rather than at each command call site (which can't be exercised without the
//! GTK/WebKit `tauri-commands` build).
//!
//! Call sites use `super::try_append_signed_event(conn, user_id, EVENT_TYPE,
//! "specimen", Some(id), &payload)` — best-effort, so a ledger hiccup never fails
//! the primary mutation (mirrors `log_audit(...).ok()`).

use serde_json::json;

// ── Canonical lifecycle event types (the `event_type` column) ────────────────
pub const SPECIMEN_CREATED: &str = "specimen_created";
pub const SPECIMEN_PASSAGED: &str = "specimen_passaged";
pub const SPECIMEN_DIED: &str = "specimen_died";
pub const SPECIMEN_SPLIT: &str = "specimen_split";
pub const SPECIMEN_STATUS_CHANGED: &str = "specimen_status_changed";
pub const SPECIMEN_ARCHIVED: &str = "specimen_archived";

/// Every lifecycle event type, for validation and the ledger-filter UI.
pub const ALL: &[&str] = &[
    SPECIMEN_CREATED,
    SPECIMEN_PASSAGED,
    SPECIMEN_DIED,
    SPECIMEN_SPLIT,
    SPECIMEN_STATUS_CHANGED,
    SPECIMEN_ARCHIVED,
];

/// A passage/subculture event. `event_type` distinguishes a `death` subculture
/// from a normal passage at the caller; this returns the matching signed
/// event-type alongside the payload so the two never drift apart.
pub fn passage(specimen_id: &str, passage_number: i64, subculture_event_type: &str) -> (&'static str, String) {
    let is_death = subculture_event_type == "death";
    let event_type = if is_death { SPECIMEN_DIED } else { SPECIMEN_PASSAGED };
    let payload = json!({
        "event": event_type,
        "specimen_id": specimen_id,
        "passage_number": passage_number,
        "subculture_event_type": subculture_event_type,
    })
    .to_string();
    (event_type, payload)
}

/// A split event: one parent forks into N child accessions.
pub fn split(parent_id: &str, child_accessions: &[String]) -> String {
    json!({
        "event": SPECIMEN_SPLIT,
        "specimen_id": parent_id,
        "children": child_accessions,
        "child_count": child_accessions.len(),
    })
    .to_string()
}

/// A status/health-status change on a specimen.
pub fn status_change(specimen_id: &str, field: &str, from: Option<&str>, to: &str) -> String {
    json!({
        "event": SPECIMEN_STATUS_CHANGED,
        "specimen_id": specimen_id,
        "field": field,
        "from": from,
        "to": to,
    })
    .to_string()
}

/// A specimen archived (retired) event.
pub fn archived(specimen_id: &str) -> String {
    json!({
        "event": SPECIMEN_ARCHIVED,
        "specimen_id": specimen_id,
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_types_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for t in ALL {
            assert!(seen.insert(*t), "duplicate lifecycle event type: {t}");
        }
    }

    #[test]
    fn passage_maps_death_to_its_own_event_type() {
        let (et, _) = passage("s1", 3, "passage");
        assert_eq!(et, SPECIMEN_PASSAGED);
        let (et_d, payload) = passage("s1", 4, "death");
        assert_eq!(et_d, SPECIMEN_DIED);
        // Payload is valid JSON carrying the specimen and passage number.
        let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(v["specimen_id"], "s1");
        assert_eq!(v["passage_number"], 4);
        assert_eq!(v["event"], SPECIMEN_DIED);
    }

    #[test]
    fn split_payload_lists_children() {
        let payload = split("parent1", &["001A".to_string(), "001B".to_string()]);
        let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(v["event"], SPECIMEN_SPLIT);
        assert_eq!(v["specimen_id"], "parent1");
        assert_eq!(v["child_count"], 2);
        assert_eq!(v["children"][1], "001B");
    }

    #[test]
    fn status_change_records_from_and_to() {
        let payload = status_change("s1", "health_status", Some("good"), "dead");
        let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert_eq!(v["field"], "health_status");
        assert_eq!(v["from"], "good");
        assert_eq!(v["to"], "dead");
    }

    #[test]
    fn status_change_allows_absent_from() {
        let payload = status_change("s1", "stage", None, "callus");
        let v: serde_json::Value = serde_json::from_str(&payload).unwrap();
        assert!(v["from"].is_null());
        assert_eq!(v["to"], "callus");
    }

    #[test]
    fn archived_payload_is_valid() {
        let v: serde_json::Value = serde_json::from_str(&archived("s9")).unwrap();
        assert_eq!(v["event"], SPECIMEN_ARCHIVED);
        assert_eq!(v["specimen_id"], "s9");
    }
}
