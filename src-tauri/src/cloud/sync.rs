// WP-59: multi-device delta-journal sync — WAL segment naming/ordering.
//
// Conflict *detection* itself is not reimplemented here: it already exists
// in `db::sync::detect_sync_conflicts` (built for WP-51's LAN sync
// foundation) and is reused as-is, since both features share the same
// underlying model — the audit chain's `(lineage_id, chain_seq)` is the
// authoritative ordering for every device. This module only adds what
// WP-51 didn't need: naming/ordering the `{device_id}/{chain_seq_range}.wal`
// segment files that carry changes between devices via a shared cloud
// target, since WP-51 assumed a live LAN connection rather than
// asynchronous file-based exchange.
use crate::models::sync::ChangeRecord;

#[derive(Debug, Clone, PartialEq)]
pub struct WalSegmentInfo {
    pub device_id: String,
    pub chain_seq_start: i64,
    pub chain_seq_end: i64,
}

/// Builds the canonical `{device_id}/{start}-{end}.wal` segment filename for
/// a batch of changes this device is publishing to a shared cloud target.
pub fn segment_file_name(device_id: &str, chain_seq_start: i64, chain_seq_end: i64) -> String {
    format!("{device_id}/{chain_seq_start}-{chain_seq_end}.wal")
}

/// Parses a segment filename back into its components. Returns `None` for
/// anything that doesn't match the canonical shape — malformed or foreign
/// files in the shared folder are skipped, never treated as a fatal error.
pub fn parse_segment_file_name(path: &str) -> Option<WalSegmentInfo> {
    let (device_id, rest) = path.rsplit_once('/')?;
    let file_stem = rest.strip_suffix(".wal")?;
    let (start_str, end_str) = file_stem.split_once('-')?;
    let chain_seq_start: i64 = start_str.parse().ok()?;
    let chain_seq_end: i64 = end_str.parse().ok()?;
    if chain_seq_end < chain_seq_start || device_id.is_empty() {
        return None;
    }
    Some(WalSegmentInfo { device_id: device_id.to_string(), chain_seq_start, chain_seq_end })
}

/// Orders parsed segments for deterministic apply: `chain_seq_start`
/// ascending first (the audit chain's own ordering is authoritative per
/// ROADMAP.md WP-59), then `device_id` as a stable tiebreaker for segments
/// that start at the same position from different devices.
pub fn order_segments(mut segments: Vec<WalSegmentInfo>) -> Vec<WalSegmentInfo> {
    segments.sort_by(|a, b| a.chain_seq_start.cmp(&b.chain_seq_start).then(a.device_id.cmp(&b.device_id)));
    segments
}

/// Serializes a batch of changes for this device into the segment file body
/// (plain JSON — the file itself lives inside the already-encrypted cloud
/// backup target's storage area, so a second layer of encryption here would
/// be redundant; see ROADMAP.md WP-59 "As built").
pub fn serialize_segment(changes: &[ChangeRecord]) -> Result<String, String> {
    serde_json::to_string(changes).map_err(|e| e.to_string())
}

pub fn deserialize_segment(body: &str) -> Result<Vec<ChangeRecord>, String> {
    serde_json::from_str(body).map_err(|e| format!("Malformed WAL segment: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_file_name_round_trips_through_parse() {
        let name = segment_file_name("device-a", 10, 25);
        assert_eq!(name, "device-a/10-25.wal");
        let parsed = parse_segment_file_name(&name).unwrap();
        assert_eq!(parsed, WalSegmentInfo { device_id: "device-a".to_string(), chain_seq_start: 10, chain_seq_end: 25 });
    }

    #[test]
    fn parse_segment_file_name_rejects_malformed_input() {
        assert!(parse_segment_file_name("not-a-segment").is_none());
        assert!(parse_segment_file_name("device-a/abc-def.wal").is_none());
        assert!(parse_segment_file_name("device-a/25-10.wal").is_none(), "end before start is invalid");
    }

    #[test]
    fn order_segments_sorts_by_chain_seq_then_device_id() {
        let segments = vec![
            WalSegmentInfo { device_id: "device-b".to_string(), chain_seq_start: 5, chain_seq_end: 10 },
            WalSegmentInfo { device_id: "device-a".to_string(), chain_seq_start: 5, chain_seq_end: 10 },
            WalSegmentInfo { device_id: "device-a".to_string(), chain_seq_start: 1, chain_seq_end: 4 },
        ];
        let ordered = order_segments(segments);
        assert_eq!(ordered[0].chain_seq_start, 1);
        assert_eq!(ordered[1].device_id, "device-a");
        assert_eq!(ordered[2].device_id, "device-b");
    }

    #[test]
    fn segment_serialize_deserialize_round_trip() {
        let changes = vec![ChangeRecord {
            lineage_id: "lin1".to_string(),
            chain_seq: 1,
            entity_type: "specimen".to_string(),
            entity_id: Some("spec1".to_string()),
            action: "create".to_string(),
            old_value: None,
            new_value: Some("ACC-001".to_string()),
            details: None,
            prev_hash: Some("0".repeat(64)),
            entry_hash: Some("a".repeat(64)),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }];
        let body = serialize_segment(&changes).unwrap();
        let recovered = deserialize_segment(&body).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].lineage_id, "lin1");
    }

    #[test]
    fn deserialize_segment_rejects_malformed_json() {
        assert!(deserialize_segment("not json").is_err());
    }
}
