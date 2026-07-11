// WP-60: Regulatory compliance export modules (FDA 21 CFR Part 11, USDA
// APHIS, CITES). Strictly additive and read-only against the database —
// this module reads existing records and writes nothing except the
// generated export bundle file and (once, on first use) the lab's signing
// keypair.
pub mod bundle;
pub mod signing;
pub mod zip_writer;

use rusqlite::{params, Connection};

/// Load the single lab-wide Ed25519 signing keypair (WP-60), generating and
/// persisting one on first use. Returns `(public_key_b64, private_key_b64)`.
///
/// This lives here — not in the `tauri-commands`-gated command layer — so the DB
/// helper is available to non-command code (e.g. `passport::store`) and to the
/// `--no-default-features` unit-test build. The command wrapper delegates here.
pub fn load_or_create_lab_signing_key(conn: &Connection) -> Result<(String, String), String> {
    let existing: Option<(String, String)> = conn
        .query_row(
            "SELECT public_key_b64, private_key_b64 FROM signing_keys WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    if let Some(pair) = existing {
        return Ok(pair);
    }
    let keypair = signing::generate_keypair();
    conn.execute(
        "INSERT INTO signing_keys (id, public_key_b64, private_key_b64) VALUES (1, ?1, ?2)",
        params![keypair.public_key_b64, keypair.private_key_b64],
    )
    .map_err(|e| e.to_string())?;
    Ok((keypair.public_key_b64, keypair.private_key_b64))
}
