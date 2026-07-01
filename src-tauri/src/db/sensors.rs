//! WP-54 — Environmental sensor integration foundation.
//!
//! `parse_sensor_payload`/`validate_reading_value` are real, tested,
//! transport-agnostic logic: they turn a raw payload string (as would arrive
//! from a serial line, a BLE characteristic notification, or an MQTT message
//! body — the three protocols named in the packet) into validated readings.
//! Manual entry (`create_environmental_reading`) is fully functional today.
//!
//! **Not implemented in this packet:** opening a real USB/serial port,
//! scanning for and subscribing to a BLE peripheral, or connecting to an
//! MQTT broker. Each of those requires a platform-specific hardware crate
//! (`serialport`, `btleplug`, `rumqttc`) with system dependencies (libudev,
//! D-Bus/bluez, a running broker) that cannot be meaningfully exercised or
//! verified in a sandboxed CI-style environment with no attached hardware.
//! Wiring one of those transports in is a mechanical follow-up: have the
//! listener call `parse_sensor_payload` on each incoming message and then
//! `create_environmental_reading` with the appropriate `source` value — the
//! ingestion pipeline below already accepts exactly that shape of input.
//!
//! **Trust gap in `source` (documented after the v1.39.1 self-review):**
//! because no hardware transport is wired up yet, nothing in this codebase
//! has ever legitimately produced a reading with `source` set to
//! `usb_serial`, `bluetooth`, or `mqtt` — every reading that exists today was
//! created through manual entry. `source` is a **caller-supplied label**,
//! not a system-verified fact: both `create_environmental_reading` and
//! `commands::sensors::ingest_sensor_payload` are ordinary `#[tauri::command]`
//! entry points, and neither has any way to confirm a reading claiming
//! `usb_serial` actually came from a USB device rather than a human (or a
//! script) simply passing that string. `validate_source` below only checks
//! that the value is one of the four known labels — it is input validation,
//! not provenance verification, and should not be mistaken for the latter.
//! Anyone relying on `source` for a compliance or audit narrative (e.g. "this
//! reading was objectively machine-collected, not operator-entered") should
//! not do so until a real transport listener exists and, ideally, is the
//! only caller permitted to set a non-`manual` source.

use super::DbResult;
use crate::models::sensors::{CreateEnvironmentalReadingRequest, EnvironmentalAlert, EnvironmentalReading, ParsedReading};
use rusqlite::{params, Connection};

const KNOWN_READING_TYPES: &[&str] = &["temp_c", "humidity_pct", "co2_ppm", "light_lux", "ph", "custom"];
const KNOWN_SOURCES: &[&str] = &["manual", "usb_serial", "bluetooth", "mqtt"];

/// Checks `source` against the same enum the `environmental_readings.source`
/// CHECK constraint enforces at the database layer, so a bad value gets a
/// clear, specific error here rather than a raw SQLite constraint-violation
/// message. This is input validation only — see the module doc comment's
/// "Trust gap in `source`" note for what it does *not* verify.
pub fn validate_source(source: &str) -> Result<(), String> {
    if KNOWN_SOURCES.contains(&source) {
        Ok(())
    } else {
        Err(format!(
            "Unknown reading source '{}'. Must be one of: {}",
            source,
            KNOWN_SOURCES.join(", ")
        ))
    }
}

/// Parses a raw sensor payload into one or more readings. Accepts two common
/// shapes: a comma-separated `key=value` line (typical of a USB/serial
/// microcontroller sketch, e.g. `temp_c=24.5,humidity_pct=61.2`) or a flat
/// JSON object (typical of an MQTT message body, e.g.
/// `{"temp_c": 24.5, "humidity_pct": 61.2}`). Unknown keys are skipped rather
/// than rejecting the whole payload — a firmware update that adds a new field
/// should not break ingestion of the fields SteloPTC already understands.
pub fn parse_sensor_payload(raw: &str) -> Result<Vec<ParsedReading>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Sensor payload is empty".to_string());
    }

    if trimmed.starts_with('{') {
        parse_json_payload(trimmed)
    } else {
        parse_kv_payload(trimmed)
    }
}

fn parse_kv_payload(payload: &str) -> Result<Vec<ParsedReading>, String> {
    let mut readings = Vec::new();
    for pair in payload.split(',') {
        let pair = pair.trim();
        if pair.is_empty() {
            continue;
        }
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("").trim();
        let value_str = parts
            .next()
            .ok_or_else(|| format!("Malformed key=value pair: '{}'", pair))?
            .trim();
        if !KNOWN_READING_TYPES.contains(&key) {
            continue;
        }
        let value: f64 = value_str
            .parse()
            .map_err(|_| format!("Could not parse numeric value for '{}': '{}'", key, value_str))?;
        readings.push(ParsedReading { reading_type: key.to_string(), value });
    }
    if readings.is_empty() {
        return Err("No recognized reading_type keys found in payload".to_string());
    }
    Ok(readings)
}

fn parse_json_payload(payload: &str) -> Result<Vec<ParsedReading>, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(payload).map_err(|e| format!("Invalid JSON sensor payload: {}", e))?;
    let obj = parsed.as_object().ok_or("Sensor JSON payload must be a flat object")?;

    let mut readings = Vec::new();
    for key in KNOWN_READING_TYPES {
        if let Some(v) = obj.get(*key) {
            let value = v.as_f64().ok_or_else(|| format!("Field '{}' is not numeric", key))?;
            readings.push(ParsedReading { reading_type: key.to_string(), value });
        }
    }
    if readings.is_empty() {
        return Err("No recognized reading_type keys found in JSON payload".to_string());
    }
    Ok(readings)
}

/// Sanity-range validation per reading type. Deliberately generous — this
/// catches transport garbage (a misread byte producing e.g. `humidity_pct =
/// 6512.0`), not precise scientific bounds.
pub fn validate_reading_value(reading_type: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("Reading value for '{}' is not a finite number", reading_type));
    }
    let (min, max): (f64, f64) = match reading_type {
        "temp_c" => (-40.0, 100.0),
        "humidity_pct" => (0.0, 100.0),
        "co2_ppm" => (0.0, 50_000.0),
        "light_lux" => (0.0, 200_000.0),
        "ph" => (0.0, 14.0),
        "custom" => return Ok(()),
        other => return Err(format!("Unknown reading_type '{}'", other)),
    };
    if value < min || value > max {
        return Err(format!(
            "Reading value {} for '{}' is outside the plausible range [{}, {}]",
            value, reading_type, min, max
        ));
    }
    Ok(())
}

/// `req.source` defaults to `"manual"` when omitted and is checked against
/// [`validate_source`] otherwise — see the module doc comment's "Trust gap in
/// `source`" note: this confirms the value is a recognized label, not that it
/// truthfully describes how the reading was collected.
pub fn create_environmental_reading(
    conn: &Connection,
    req: &CreateEnvironmentalReadingRequest,
    created_by: Option<&str>,
) -> Result<String, String> {
    if req.specimen_id.is_none() && req.subculture_id.is_none() {
        return Err("A reading must be linked to a specimen or a subculture".to_string());
    }
    validate_reading_value(&req.reading_type, req.value)?;

    let source = req.source.as_deref().unwrap_or("manual");
    validate_source(source)?;

    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO environmental_readings \
         (id, specimen_id, subculture_id, reading_type, value, unit, source, recorded_at, notes, created_by) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, COALESCE(?8, datetime('now')), ?9, ?10)",
        params![
            id,
            req.specimen_id,
            req.subculture_id,
            req.reading_type,
            req.value,
            req.unit,
            source,
            req.recorded_at,
            req.notes,
            created_by,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

fn row_to_reading(row: &rusqlite::Row) -> rusqlite::Result<EnvironmentalReading> {
    Ok(EnvironmentalReading {
        id: row.get("id")?,
        specimen_id: row.get("specimen_id")?,
        subculture_id: row.get("subculture_id")?,
        reading_type: row.get("reading_type")?,
        value: row.get("value")?,
        unit: row.get("unit")?,
        source: row.get("source")?,
        recorded_at: row.get("recorded_at")?,
        notes: row.get("notes")?,
        created_by: row.get("created_by")?,
        created_at: row.get("created_at")?,
    })
}

pub fn list_environmental_readings(
    conn: &Connection,
    specimen_id: Option<&str>,
    limit: i64,
) -> DbResult<Vec<EnvironmentalReading>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM environmental_readings WHERE specimen_id = ?1 ORDER BY recorded_at DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![specimen_id, limit], row_to_reading)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// (min, max) thresholds per reading type. `None` in either position means
/// "no bound on that side". Read from `app_settings` with these as fallback
/// defaults, so a lab can override them without a migration.
fn threshold_for(conn: &Connection, reading_type: &str) -> (Option<f64>, Option<f64>) {
    let (default_min, default_max): (Option<f64>, Option<f64>) = match reading_type {
        "temp_c" => (Some(15.0), Some(30.0)),
        "humidity_pct" => (Some(40.0), Some(80.0)),
        "co2_ppm" => (None, Some(1200.0)),
        "ph" => (Some(5.5), Some(7.5)),
        _ => (None, None),
    };
    let min = default_min.map(|d| {
        crate::db::queries::read_setting(conn, &format!("sensor_{}_min", reading_type), &d.to_string())
            .parse()
            .unwrap_or(d)
    });
    let max = default_max.map(|d| {
        crate::db::queries::read_setting(conn, &format!("sensor_{}_max", reading_type), &d.to_string())
            .parse()
            .unwrap_or(d)
    });
    (min, max)
}

/// Checks the most recent reading per (specimen, reading_type) against its
/// configured threshold and returns one alert per out-of-range reading.
pub fn get_environmental_alerts(conn: &Connection) -> DbResult<Vec<EnvironmentalAlert>> {
    let mut stmt = conn.prepare(
        "SELECT er.specimen_id, er.reading_type, er.value, er.recorded_at
         FROM environmental_readings er
         WHERE er.specimen_id IS NOT NULL
           AND er.recorded_at = (
               SELECT MAX(er2.recorded_at) FROM environmental_readings er2
               WHERE er2.specimen_id = er.specimen_id AND er2.reading_type = er.reading_type
           )",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>("specimen_id")?,
            row.get::<_, String>("reading_type")?,
            row.get::<_, f64>("value")?,
            row.get::<_, String>("recorded_at")?,
        ))
    })?;

    let mut alerts = Vec::new();
    for row in rows {
        let (specimen_id, reading_type, value, recorded_at) = row?;
        let (min, max) = threshold_for(conn, &reading_type);
        let out_of_range = min.is_some_and(|m| value < m) || max.is_some_and(|m| value > m);
        if !out_of_range {
            continue;
        }
        let message = match (min, max) {
            (Some(m), _) if value < m => format!("{} reading {} is below the minimum threshold {}", reading_type, value, m),
            (_, Some(m)) if value > m => format!("{} reading {} exceeds the maximum threshold {}", reading_type, value, m),
            _ => format!("{} reading {} is out of range", reading_type, value),
        };
        alerts.push(EnvironmentalAlert {
            specimen_id,
            reading_type,
            value,
            threshold_min: min,
            threshold_max: max,
            message,
            recorded_at,
        });
    }
    Ok(alerts)
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

    /// Inserts a minimal valid species + specimen row so FK-checked inserts
    /// against `environmental_readings.specimen_id` succeed (migrations turn
    /// `PRAGMA foreign_keys = ON` on partway through the migration chain).
    fn insert_minimal_specimen(conn: &Connection, specimen_id: &str) {
        conn.execute(
            "INSERT INTO species (id, genus, species_name, species_code) VALUES ('sp-species-1', 'Testus', 'testus', 'TST-01')",
            [],
        ).ok(); // ignore "already exists" if called more than once in a test
        conn.execute(
            "INSERT INTO specimens (id, accession_number, species_id, initiation_date) \
             VALUES (?1, ?1, 'sp-species-1', '2026-01-01')",
            params![specimen_id],
        )
        .unwrap();
    }

    #[test]
    fn parse_kv_payload_extracts_known_fields() {
        let readings = parse_sensor_payload("temp_c=24.5,humidity_pct=61.2").unwrap();
        assert_eq!(readings.len(), 2);
        assert_eq!(readings[0], ParsedReading { reading_type: "temp_c".to_string(), value: 24.5 });
        assert_eq!(readings[1], ParsedReading { reading_type: "humidity_pct".to_string(), value: 61.2 });
    }

    #[test]
    fn parse_kv_payload_skips_unknown_keys() {
        let readings = parse_sensor_payload("temp_c=24.5,firmware_version=3").unwrap();
        assert_eq!(readings.len(), 1);
        assert_eq!(readings[0].reading_type, "temp_c");
    }

    #[test]
    fn parse_json_payload_extracts_known_fields() {
        let readings = parse_sensor_payload(r#"{"temp_c": 24.5, "co2_ppm": 800}"#).unwrap();
        assert_eq!(readings.len(), 2);
        assert!(readings.iter().any(|r| r.reading_type == "temp_c" && r.value == 24.5));
        assert!(readings.iter().any(|r| r.reading_type == "co2_ppm" && r.value == 800.0));
    }

    #[test]
    fn parse_sensor_payload_rejects_empty_input() {
        assert!(parse_sensor_payload("").is_err());
        assert!(parse_sensor_payload("   ").is_err());
    }

    #[test]
    fn parse_sensor_payload_rejects_payload_with_no_known_fields() {
        assert!(parse_sensor_payload("firmware_version=3").is_err());
        assert!(parse_sensor_payload(r#"{"firmware_version": 3}"#).is_err());
    }

    #[test]
    fn validate_reading_value_accepts_plausible_values() {
        assert!(validate_reading_value("temp_c", 24.5).is_ok());
        assert!(validate_reading_value("humidity_pct", 61.2).is_ok());
        assert!(validate_reading_value("ph", 6.8).is_ok());
    }

    #[test]
    fn validate_reading_value_rejects_out_of_range() {
        assert!(validate_reading_value("humidity_pct", 6512.0).is_err());
        assert!(validate_reading_value("ph", 20.0).is_err());
        assert!(validate_reading_value("temp_c", f64::NAN).is_err());
    }

    #[test]
    fn validate_reading_value_rejects_unknown_type() {
        assert!(validate_reading_value("bogus_type", 1.0).is_err());
    }

    #[test]
    fn validate_source_accepts_all_known_sources() {
        for s in KNOWN_SOURCES {
            assert!(validate_source(s).is_ok(), "'{}' should be a recognized source", s);
        }
    }

    #[test]
    fn validate_source_rejects_unknown_value() {
        let err = validate_source("wifi_sensor").unwrap_err();
        assert!(err.contains("wifi_sensor"), "error should name the rejected value: {err}");
        assert!(err.contains("manual"), "error should list the known values: {err}");
    }

    #[test]
    fn validate_source_rejects_case_mismatch() {
        // Enum values are lowercase in the CHECK constraint; no case-folding.
        assert!(validate_source("Manual").is_err());
        assert!(validate_source("USB_SERIAL").is_err());
    }

    #[test]
    fn create_environmental_reading_requires_specimen_or_subculture() {
        let conn = migrated_db();
        let req = CreateEnvironmentalReadingRequest {
            specimen_id: None,
            subculture_id: None,
            reading_type: "temp_c".to_string(),
            value: 24.0,
            unit: None,
            source: None,
            recorded_at: None,
            notes: None,
        };
        assert!(create_environmental_reading(&conn, &req, None).is_err());
    }

    #[test]
    fn create_and_list_environmental_reading_round_trip() {
        let conn = migrated_db();
        insert_minimal_specimen(&conn, "sp-1");
        let req = CreateEnvironmentalReadingRequest {
            specimen_id: Some("sp-1".to_string()),
            subculture_id: None,
            reading_type: "temp_c".to_string(),
            value: 24.5,
            unit: Some("C".to_string()),
            source: None,
            recorded_at: None,
            notes: Some("manual check".to_string()),
        };
        let id = create_environmental_reading(&conn, &req, Some("user-1")).unwrap();
        let readings = list_environmental_readings(&conn, Some("sp-1"), 10).unwrap();
        assert_eq!(readings.len(), 1);
        assert_eq!(readings[0].id, id);
        assert_eq!(readings[0].source, "manual");
        assert_eq!(readings[0].value, 24.5);
    }

    #[test]
    fn create_environmental_reading_accepts_each_known_source() {
        let conn = migrated_db();
        insert_minimal_specimen(&conn, "sp-src");
        for s in KNOWN_SOURCES {
            let req = CreateEnvironmentalReadingRequest {
                specimen_id: Some("sp-src".to_string()),
                subculture_id: None,
                reading_type: "temp_c".to_string(),
                value: 22.0,
                unit: None,
                source: Some(s.to_string()),
                recorded_at: None,
                notes: None,
            };
            assert!(create_environmental_reading(&conn, &req, None).is_ok(), "'{}' should be accepted", s);
        }
    }

    #[test]
    fn create_environmental_reading_rejects_unknown_source_before_hitting_the_db() {
        let conn = migrated_db();
        insert_minimal_specimen(&conn, "sp-src-2");
        let req = CreateEnvironmentalReadingRequest {
            specimen_id: Some("sp-src-2".to_string()),
            subculture_id: None,
            reading_type: "temp_c".to_string(),
            value: 22.0,
            unit: None,
            source: Some("wifi_sensor".to_string()),
            recorded_at: None,
            notes: None,
        };
        let err = create_environmental_reading(&conn, &req, None).unwrap_err();
        // A clear validation error, not a raw SQLite CHECK-constraint message.
        assert!(err.contains("wifi_sensor"), "error should name the rejected value: {err}");
        assert_eq!(list_environmental_readings(&conn, Some("sp-src-2"), 10).unwrap().len(), 0);
    }

    #[test]
    fn get_environmental_alerts_flags_out_of_range_reading() {
        let conn = migrated_db();
        insert_minimal_specimen(&conn, "sp-1");
        let req = CreateEnvironmentalReadingRequest {
            specimen_id: Some("sp-1".to_string()),
            subculture_id: None,
            reading_type: "temp_c".to_string(),
            value: 45.0, // above default max of 30.0
            unit: None,
            source: None,
            recorded_at: None,
            notes: None,
        };
        create_environmental_reading(&conn, &req, None).unwrap();

        let alerts = get_environmental_alerts(&conn).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].reading_type, "temp_c");
        assert!(alerts[0].message.contains("exceeds"));
    }

    #[test]
    fn get_environmental_alerts_ignores_in_range_reading() {
        let conn = migrated_db();
        insert_minimal_specimen(&conn, "sp-1");
        let req = CreateEnvironmentalReadingRequest {
            specimen_id: Some("sp-1".to_string()),
            subculture_id: None,
            reading_type: "temp_c".to_string(),
            value: 22.0,
            unit: None,
            source: None,
            recorded_at: None,
            notes: None,
        };
        create_environmental_reading(&conn, &req, None).unwrap();
        let alerts = get_environmental_alerts(&conn).unwrap();
        assert!(alerts.is_empty());
    }
}
