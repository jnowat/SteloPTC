//! Pure Work Queue detection logic, extracted from `commands::work_queue`
//! (behavior-preserving move, not a rewrite) so WP-52 notifications can reuse
//! the exact same overdue-detection rules instead of duplicating them.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkQueueItem {
    pub specimen_id: String,
    pub accession_number: String,
    pub species_name: Option<String>,
    pub location: Option<String>,
    pub reason: String,
    pub reason_code: String,
    /// "critical" | "high" | "normal"
    pub urgency: String,
    /// Positive = overdue by N days; negative = due in N days; None = not date-based
    pub days_overdue: Option<i64>,
}

fn urgency_rank(urgency: &str) -> i32 {
    match urgency {
        "critical" => 0,
        "high" => 1,
        _ => 2,
    }
}

/// Returns how many days ago `date_str` was relative to `today_str` (both "YYYY-MM-DD").
/// Returns None if parsing fails.
fn days_ago(date_str: &str, today_str: &str) -> Option<i64> {
    use chrono::NaiveDate;
    let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()?;
    let today = NaiveDate::parse_from_str(today_str, "%Y-%m-%d").ok()?;
    Some((today - date).num_days())
}

pub fn compute_work_queue_items(conn: &Connection) -> Result<Vec<WorkQueueItem>, String> {
    let today = chrono::Local::now().date_naive().to_string(); // "YYYY-MM-DD"

    let mut items: Vec<WorkQueueItem> = Vec::new();

    // ── 1. Unresolved quarantine ──────────────────────────────────────────────
    {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.accession_number, s.location, s.quarantine_release_date,
                    sp.genus || ' ' || sp.species_name AS species_name
             FROM specimens s
             LEFT JOIN species sp ON s.species_id = sp.id
             WHERE s.is_archived = 0
               AND s.quarantine_flag = 1
               AND (s.quarantine_release_date IS NULL OR s.quarantine_release_date <= ?1)",
        )
        .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![today], |row| {
                Ok((
                    row.get::<_, String>("id")?,
                    row.get::<_, String>("accession_number")?,
                    row.get::<_, Option<String>>("location")?,
                    row.get::<_, Option<String>>("species_name")?,
                    row.get::<_, Option<String>>("quarantine_release_date")?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for row in rows.filter_map(|r| r.ok()) {
            let (specimen_id, accession_number, location, species_name, release_date) = row;
            let reason = if release_date.is_none() {
                "In quarantine (no release date set)".to_string()
            } else {
                "Quarantine release date has passed — review required".to_string()
            };
            items.push(WorkQueueItem {
                specimen_id,
                accession_number,
                species_name,
                location,
                reason,
                reason_code: "quarantine".to_string(),
                urgency: "critical".to_string(),
                days_overdue: None,
            });
        }
    }

    // ── 2. Contamination flag on most recent subculture ───────────────────────
    {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.accession_number, s.location,
                    sp.genus || ' ' || sp.species_name AS species_name,
                    sc.date AS last_date
             FROM specimens s
             LEFT JOIN species sp ON s.species_id = sp.id
             JOIN subcultures sc ON sc.specimen_id = s.id
               AND sc.passage_number = (
                     SELECT MAX(passage_number) FROM subcultures WHERE specimen_id = s.id
                   )
             WHERE s.is_archived = 0
               AND sc.contamination_flag = 1",
        )
        .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![], |row| {
                Ok((
                    row.get::<_, String>("id")?,
                    row.get::<_, String>("accession_number")?,
                    row.get::<_, Option<String>>("location")?,
                    row.get::<_, Option<String>>("species_name")?,
                    row.get::<_, String>("last_date")?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for row in rows.filter_map(|r| r.ok()) {
            let (specimen_id, accession_number, location, species_name, last_date) = row;
            // Skip if already added for quarantine
            let already = items.iter().any(|i| i.specimen_id == specimen_id && i.reason_code == "quarantine");
            if already { continue; }

            let days_since = days_ago(&last_date, &today);
            items.push(WorkQueueItem {
                specimen_id,
                accession_number,
                species_name,
                location,
                reason: format!("Contamination detected — check required ({} days ago)", days_since.unwrap_or(0)),
                reason_code: "contamination".to_string(),
                urgency: "critical".to_string(),
                days_overdue: days_since,
            });
        }
    }

    // ── 3. No recorded passages ───────────────────────────────────────────────
    {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.accession_number, s.location,
                    sp.genus || ' ' || sp.species_name AS species_name
             FROM specimens s
             LEFT JOIN species sp ON s.species_id = sp.id
             WHERE s.is_archived = 0
               AND s.subculture_count = 0",
        )
        .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![], |row| {
                Ok((
                    row.get::<_, String>("id")?,
                    row.get::<_, String>("accession_number")?,
                    row.get::<_, Option<String>>("location")?,
                    row.get::<_, Option<String>>("species_name")?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for row in rows.filter_map(|r| r.ok()) {
            let (specimen_id, accession_number, location, species_name) = row;
            items.push(WorkQueueItem {
                specimen_id,
                accession_number,
                species_name,
                location,
                reason: "No passages recorded — initial subculture needed".to_string(),
                reason_code: "no_passages".to_string(),
                urgency: "high".to_string(),
                days_overdue: None,
            });
        }
    }

    // ── 4. Subculture due / overdue ───────────────────────────────────────────
    {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.accession_number, s.location,
                    sp.genus || ' ' || sp.species_name AS species_name,
                    sp.default_subculture_interval_days,
                    MAX(sc.date) AS last_subculture_date
             FROM specimens s
             LEFT JOIN species sp ON s.species_id = sp.id
             LEFT JOIN subcultures sc ON sc.specimen_id = s.id
             WHERE s.is_archived = 0
               AND s.subculture_count > 0
               AND sp.default_subculture_interval_days IS NOT NULL
               AND sp.default_subculture_interval_days > 0
             GROUP BY s.id
             HAVING last_subculture_date IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![], |row| {
                Ok((
                    row.get::<_, String>("id")?,
                    row.get::<_, String>("accession_number")?,
                    row.get::<_, Option<String>>("location")?,
                    row.get::<_, Option<String>>("species_name")?,
                    row.get::<_, i64>("default_subculture_interval_days")?,
                    row.get::<_, String>("last_subculture_date")?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for row in rows.filter_map(|r| r.ok()) {
            let (specimen_id, accession_number, location, species_name, interval, last_date) = row;

            let days_since = match days_ago(&last_date, &today) {
                Some(d) => d,
                None => continue,
            };
            let days_overdue = days_since - interval;

            // Only add if due within 3 days or already overdue
            if days_overdue < -3 { continue; }

            // Skip if already in critical bucket
            let already_critical = items.iter().any(|i| {
                i.specimen_id == specimen_id
                    && (i.reason_code == "quarantine" || i.reason_code == "contamination")
            });
            if already_critical { continue; }

            let (reason, urgency) = if days_overdue >= 14 {
                (format!("Subculture overdue by {} days", days_overdue), "high".to_string())
            } else if days_overdue >= 0 {
                (format!("Subculture overdue by {} day{}", days_overdue, if days_overdue == 1 { "" } else { "s" }), "normal".to_string())
            } else {
                (format!("Subculture due in {} day{}", -days_overdue, if days_overdue == -1 { "" } else { "s" }), "normal".to_string())
            };

            items.push(WorkQueueItem {
                specimen_id,
                accession_number,
                species_name,
                location,
                reason,
                reason_code: "subculture_due".to_string(),
                urgency,
                days_overdue: Some(days_overdue),
            });
        }
    }

    // ── 5. Media change overdue (media batch expired) ─────────────────────────
    {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.accession_number, s.location,
                    sp.genus || ' ' || sp.species_name AS species_name,
                    mb.expiration_date, mb.name AS media_name
             FROM specimens s
             LEFT JOIN species sp ON s.species_id = sp.id
             JOIN subcultures sc ON sc.specimen_id = s.id
               AND sc.passage_number = (
                     SELECT MAX(passage_number) FROM subcultures WHERE specimen_id = s.id
                   )
             JOIN media_batches mb ON sc.media_batch_id = mb.id
             WHERE s.is_archived = 0
               AND mb.expiration_date IS NOT NULL
               AND mb.expiration_date < ?1",
        )
        .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![today], |row| {
                Ok((
                    row.get::<_, String>("id")?,
                    row.get::<_, String>("accession_number")?,
                    row.get::<_, Option<String>>("location")?,
                    row.get::<_, Option<String>>("species_name")?,
                    row.get::<_, String>("expiration_date")?,
                    row.get::<_, String>("media_name")?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for row in rows.filter_map(|r| r.ok()) {
            let (specimen_id, accession_number, location, species_name, expiry, media_name) = row;

            // Skip if already in critical or subculture_due bucket
            let already = items.iter().any(|i| {
                i.specimen_id == specimen_id
                    && (i.reason_code == "quarantine"
                        || i.reason_code == "contamination"
                        || i.reason_code == "subculture_due")
            });
            if already { continue; }

            let days = days_ago(&expiry, &today).unwrap_or(0);
            items.push(WorkQueueItem {
                specimen_id,
                accession_number,
                species_name,
                location,
                reason: format!("Media batch '{}' expired {} day{} ago — media change needed", media_name, days, if days == 1 { "" } else { "s" }),
                reason_code: "media_expired".to_string(),
                urgency: "high".to_string(),
                days_overdue: Some(days),
            });
        }
    }

    // ── Sort: urgency rank ASC, then days_overdue DESC (most overdue first) ──
    items.sort_by(|a, b| {
        let ur = urgency_rank(&a.urgency).cmp(&urgency_rank(&b.urgency));
        if ur != std::cmp::Ordering::Equal { return ur; }
        let da = a.days_overdue.unwrap_or(0);
        let db_val = b.days_overdue.unwrap_or(0);
        db_val.cmp(&da)
    });

    Ok(items)
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
    fn empty_lab_returns_empty_queue() {
        let conn = migrated_db();
        let items = compute_work_queue_items(&conn).unwrap();
        assert!(items.is_empty());
    }
}
