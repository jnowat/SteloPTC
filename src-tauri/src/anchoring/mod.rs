// WP-66: Trust Layer Phase 2 — on-chain anchoring (Dogecoin `OP_RETURN`).
//
// This module is the deterministic, dependency-free core of the anchoring
// feature. It builds the exact `OP_RETURN` output script that carries an
// audit-checkpoint's Merkle root onto the Dogecoin chain, and — crucially —
// it can verify, given only the raw script data an anyone-can-read block
// explorer shows for a transaction, that the on-chain commitment matches a
// local checkpoint's root **without trusting the lab's database**.
//
// Scope, disclosed honestly (matching the WP-50 PostgreSQL / WP-59 S3-SFTP /
// WP-61 WASM precedent): this module and its command layer prepare the exact
// bytes to broadcast and verify what came back, but they do NOT broadcast to
// the Dogecoin network themselves. Publishing an `OP_RETURN` requires a funded
// wallet and a node or third-party broadcast API; that transport is left to an
// external wallet the operator already controls. The trust guarantee does not
// depend on who broadcasts — the payload is a public commitment anyone can
// verify — so the verifiable core ships now and the (credential-bearing,
// value-moving) broadcast step stays out of the app.
//
// Wire format — a standard Bitcoin/Dogecoin `OP_RETURN` scriptPubKey:
//   0x6a                     OP_RETURN
//   0x25                     pushdata length = 37 bytes
//   "STEL"                   4-byte protocol marker
//   0x01                     1-byte format version
//   <32 bytes>               the checkpoint Merkle root (raw, from its 64 hex chars)
// Total script = 39 bytes; the 37-byte pushdata payload sits well under
// Dogecoin's 80-byte standard-relay `OP_RETURN` limit.

use serde::Serialize;

pub mod store;

/// `OP_RETURN` opcode — marks a provably-unspendable, data-carrying output.
pub const OP_RETURN: u8 = 0x6a;
/// 4-byte protocol marker identifying a SteloPTC checkpoint anchor. Lets a
/// verifier distinguish our commitments from any other `OP_RETURN` data in the
/// same transaction or chain.
pub const MARKER: &[u8; 4] = b"STEL";
/// Format version for the anchor payload. Bumping this is how a future,
/// differently-structured payload stays distinguishable from v1.
pub const ANCHOR_VERSION: u8 = 0x01;
/// marker(4) + version(1) + merkle root(32).
pub const PAYLOAD_LEN: usize = 4 + 1 + 32;
/// Dogecoin's standard-relay cap on `OP_RETURN` data. Enforced so a payload can
/// never be built that a node would refuse to relay.
pub const MAX_OP_RETURN_DATA: usize = 80;

/// Lowercase-hex encode.
pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

/// Decode a hex string (case-insensitive, no `0x` prefix, whitespace ignored)
/// into bytes. Returns `Err` on odd length or a non-hex character rather than
/// panicking, so attacker/paste-controlled input is always handled cleanly.
pub fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    let cleaned = cleaned.strip_prefix("0x").or_else(|| cleaned.strip_prefix("0X")).unwrap_or(&cleaned);
    if !cleaned.len().is_multiple_of(2) {
        return Err(format!("Hex string has an odd length ({} chars)", cleaned.len()));
    }
    let mut out = Vec::with_capacity(cleaned.len() / 2);
    let bytes = cleaned.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i])?;
        let lo = hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn hex_nibble(c: u8) -> Result<u8, String> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(format!("Invalid hex character: '{}'", c as char)),
    }
}

/// Build the 37-byte anchor payload (marker + version + 32-byte root) from a
/// checkpoint's 64-hex-char Merkle root. The all-zero root (`ZERO_HASH`, an
/// empty checkpoint) is rejected — there is nothing meaningful to anchor.
pub fn build_anchor_payload(merkle_root_hex: &str) -> Result<Vec<u8>, String> {
    let root = hex_decode(merkle_root_hex)?;
    if root.len() != 32 {
        return Err(format!(
            "Merkle root must be 32 bytes (64 hex chars); got {} bytes",
            root.len()
        ));
    }
    if root.iter().all(|&b| b == 0) {
        return Err("Refusing to anchor an all-zero Merkle root (empty checkpoint)".to_string());
    }
    let mut payload = Vec::with_capacity(PAYLOAD_LEN);
    payload.extend_from_slice(MARKER);
    payload.push(ANCHOR_VERSION);
    payload.extend_from_slice(&root);
    debug_assert_eq!(payload.len(), PAYLOAD_LEN);
    Ok(payload)
}

/// Build the full `OP_RETURN` scriptPubKey bytes: `OP_RETURN <len> <payload>`.
pub fn build_op_return_script(merkle_root_hex: &str) -> Result<Vec<u8>, String> {
    let payload = build_anchor_payload(merkle_root_hex)?;
    if payload.len() > MAX_OP_RETURN_DATA {
        return Err(format!(
            "Anchor payload ({} bytes) exceeds the {}-byte OP_RETURN standard-relay limit",
            payload.len(),
            MAX_OP_RETURN_DATA
        ));
    }
    // payload is 37 bytes (< 76), so a single-byte pushdata opcode encodes the length.
    let mut script = Vec::with_capacity(2 + payload.len());
    script.push(OP_RETURN);
    script.push(payload.len() as u8);
    script.extend_from_slice(&payload);
    Ok(script)
}

/// Hex of the full `OP_RETURN` scriptPubKey — the exact string an operator
/// pastes into their wallet's raw-data / `OP_RETURN` field to broadcast, and
/// the string a block explorer displays back for the resulting output.
pub fn build_op_return_script_hex(merkle_root_hex: &str) -> Result<String, String> {
    Ok(hex_encode(&build_op_return_script(merkle_root_hex)?))
}

/// Extract the Merkle root (as lowercase hex) from an anchor payload, validating
/// the marker, version, and length. Accepts the bare 37-byte payload.
pub fn parse_anchor_payload(payload: &[u8]) -> Result<String, String> {
    if payload.len() != PAYLOAD_LEN {
        return Err(format!(
            "Anchor payload must be {} bytes; got {}",
            PAYLOAD_LEN,
            payload.len()
        ));
    }
    if &payload[0..4] != MARKER {
        return Err("Payload is not a SteloPTC anchor (missing 'STEL' marker)".to_string());
    }
    if payload[4] != ANCHOR_VERSION {
        return Err(format!(
            "Unsupported anchor version 0x{:02x} (expected 0x{:02x})",
            payload[4], ANCHOR_VERSION
        ));
    }
    Ok(hex_encode(&payload[5..PAYLOAD_LEN]))
}

/// Extract the Merkle root from a full `OP_RETURN` scriptPubKey, validating the
/// opcode and pushdata length before delegating to `parse_anchor_payload`.
pub fn parse_op_return_script(script: &[u8]) -> Result<String, String> {
    if script.len() < 2 || script[0] != OP_RETURN {
        return Err("Not an OP_RETURN output (first byte is not 0x6a)".to_string());
    }
    let declared = script[1] as usize;
    let data = &script[2..];
    if declared != data.len() {
        return Err(format!(
            "OP_RETURN pushdata length byte ({}) does not match the {} data bytes present",
            declared,
            data.len()
        ));
    }
    parse_anchor_payload(data)
}

/// Extract the anchored Merkle root from an arbitrary hex string a user copied
/// from a block explorer. Tolerant of the two shapes an explorer surfaces: the
/// full scriptPubKey (`6a25...`) or the bare payload (`5354454c01...`).
pub fn extract_root_from_hex(op_return_hex: &str) -> Result<String, String> {
    let bytes = hex_decode(op_return_hex)?;
    if bytes.first() == Some(&OP_RETURN) {
        parse_op_return_script(&bytes)
    } else {
        parse_anchor_payload(&bytes)
    }
}

/// The trustless check: does the on-chain `OP_RETURN` data commit to exactly the
/// Merkle root we expect? Compares case-insensitively. Never trusts anything but
/// the two arguments — so a verifier can run it against a block explorer's raw
/// output and the checkpoint's published root alone.
pub fn op_return_matches_root(op_return_hex: &str, expected_root_hex: &str) -> Result<bool, String> {
    let found = extract_root_from_hex(op_return_hex)?;
    Ok(found.eq_ignore_ascii_case(expected_root_hex.trim()))
}

/// Serializable preview of what a checkpoint anchor would publish — returned by
/// the `prepare_checkpoint_anchor` command so the UI can show copyable bytes and
/// step-by-step broadcast instructions.
#[derive(Debug, Serialize)]
pub struct AnchorPayloadPreview {
    pub merkle_root: String,
    pub payload_hex: String,
    pub op_return_script_hex: String,
    pub chain_name: String,
    pub marker: String,
    pub version: u8,
    pub byte_size: usize,
}

/// Build the preview for a checkpoint's Merkle root.
pub fn build_payload_preview(merkle_root_hex: &str, chain_name: &str) -> Result<AnchorPayloadPreview, String> {
    let payload = build_anchor_payload(merkle_root_hex)?;
    let script = build_op_return_script(merkle_root_hex)?;
    Ok(AnchorPayloadPreview {
        merkle_root: merkle_root_hex.to_lowercase(),
        payload_hex: hex_encode(&payload),
        op_return_script_hex: hex_encode(&script),
        chain_name: chain_name.to_string(),
        marker: String::from_utf8_lossy(MARKER).to_string(),
        version: ANCHOR_VERSION,
        byte_size: script.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "a3f1c09b8e7d6a5b4c3d2e1f00112233445566778899aabbccddeeff00112233";

    #[test]
    fn hex_round_trips() {
        let bytes = vec![0x6a, 0x25, 0x00, 0xff, 0xab];
        assert_eq!(hex_encode(&bytes), "6a2500ffab");
        assert_eq!(hex_decode("6a2500ffab").unwrap(), bytes);
    }

    #[test]
    fn hex_decode_is_case_insensitive_and_tolerant() {
        assert_eq!(hex_decode("AaBb").unwrap(), vec![0xaa, 0xbb]);
        assert_eq!(hex_decode("0xAABB").unwrap(), vec![0xaa, 0xbb]);
        assert_eq!(hex_decode(" aa bb ").unwrap(), vec![0xaa, 0xbb]);
    }

    #[test]
    fn hex_decode_rejects_odd_length_and_bad_chars() {
        assert!(hex_decode("abc").is_err());
        assert!(hex_decode("zz").is_err());
    }

    #[test]
    fn payload_has_marker_version_and_root() {
        let payload = build_anchor_payload(ROOT).unwrap();
        assert_eq!(payload.len(), PAYLOAD_LEN);
        assert_eq!(&payload[0..4], MARKER);
        assert_eq!(payload[4], ANCHOR_VERSION);
        assert_eq!(hex_encode(&payload[5..]), ROOT);
    }

    #[test]
    fn script_is_op_return_pushdata_payload() {
        let script = build_op_return_script(ROOT).unwrap();
        assert_eq!(script.len(), 39);
        assert_eq!(script[0], OP_RETURN);
        assert_eq!(script[1] as usize, PAYLOAD_LEN);
        assert!(script.len() <= 2 + MAX_OP_RETURN_DATA);
    }

    #[test]
    fn script_round_trips_back_to_the_root() {
        let hex = build_op_return_script_hex(ROOT).unwrap();
        assert_eq!(parse_op_return_script(&hex_decode(&hex).unwrap()).unwrap(), ROOT);
        assert_eq!(extract_root_from_hex(&hex).unwrap(), ROOT);
    }

    #[test]
    fn payload_hex_also_extracts_the_root() {
        let payload_hex = hex_encode(&build_anchor_payload(ROOT).unwrap());
        // Bare payload (no leading OP_RETURN) must still verify.
        assert_eq!(extract_root_from_hex(&payload_hex).unwrap(), ROOT);
    }

    #[test]
    fn rejects_all_zero_root() {
        let zero = "0".repeat(64);
        assert!(build_anchor_payload(&zero).is_err());
    }

    #[test]
    fn rejects_wrong_root_length() {
        assert!(build_anchor_payload("abcd").is_err());
    }

    #[test]
    fn parse_rejects_wrong_marker() {
        let mut payload = build_anchor_payload(ROOT).unwrap();
        payload[0] = b'X';
        assert!(parse_anchor_payload(&payload).is_err());
    }

    #[test]
    fn parse_rejects_wrong_version() {
        let mut payload = build_anchor_payload(ROOT).unwrap();
        payload[4] = 0x02;
        assert!(parse_anchor_payload(&payload).is_err());
    }

    #[test]
    fn parse_rejects_non_op_return_script() {
        let mut script = build_op_return_script(ROOT).unwrap();
        script[0] = 0x76; // OP_DUP
        assert!(parse_op_return_script(&script).is_err());
    }

    #[test]
    fn parse_rejects_length_mismatch() {
        let mut script = build_op_return_script(ROOT).unwrap();
        script[1] = 0x10; // claim 16 bytes when 37 are present
        assert!(parse_op_return_script(&script).is_err());
    }

    #[test]
    fn matches_root_is_true_for_matching_and_false_for_mismatch() {
        let hex = build_op_return_script_hex(ROOT).unwrap();
        assert!(op_return_matches_root(&hex, ROOT).unwrap());
        assert!(op_return_matches_root(&hex, &ROOT.to_uppercase()).unwrap());
        let other = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
        assert!(!op_return_matches_root(&hex, other).unwrap());
    }

    #[test]
    fn matches_root_errors_on_garbage_hex() {
        assert!(op_return_matches_root("not-hex!!", ROOT).is_err());
    }

    #[test]
    fn preview_exposes_all_broadcast_fields() {
        let preview = build_payload_preview(ROOT, "dogecoin").unwrap();
        assert_eq!(preview.merkle_root, ROOT);
        assert_eq!(preview.marker, "STEL");
        assert_eq!(preview.version, ANCHOR_VERSION);
        assert_eq!(preview.byte_size, 39);
        assert!(preview.op_return_script_hex.starts_with("6a25"));
        // The preview's own script must verify back to the root.
        assert!(op_return_matches_root(&preview.op_return_script_hex, ROOT).unwrap());
    }
}
