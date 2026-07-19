// WP-78: Environmental out-of-range monitoring.
//
// WP-54 shipped the environmental-reading foundation (parsing, validation,
// manual entry) but no evaluation — a reading was stored and displayed, never
// checked against a healthy range. This module supplies that missing piece: a
// pure evaluation of a reading against a per-type acceptable range, surfaced to
// the operator as a compliance flag through the WP-74 rule engine
// (`environmental_out_of_range`), so it automatically inherits the flag UI and
// the WP-77 waiver workflow.
//
// The default ranges below are sensible plant/cell/fungal culture-room bounds;
// they are deliberately conservative so a reading only flags when it is clearly
// outside a healthy envelope. Everything here is pure and unit-tested; the
// command layer maps the latest reading per specimen through `evaluate`.

use serde::Serialize;

/// An inclusive acceptable range for a reading type.
#[derive(Debug, Clone, Copy)]
pub struct Threshold {
    pub min: f64,
    pub max: f64,
}

/// Whether a reading falls below or above its acceptable range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RangeStatus {
    Low,
    High,
}

/// Default acceptable range for a known reading type, or `None` for a type with
/// no meaningful universal bound (`custom`, or an unrecognized type — which must
/// never produce a false alert).
pub fn default_threshold(reading_type: &str) -> Option<Threshold> {
    let t = match reading_type {
        "temp_c" => Threshold { min: 18.0, max: 28.0 },
        "humidity_pct" => Threshold { min: 40.0, max: 95.0 },
        "co2_ppm" => Threshold { min: 300.0, max: 5000.0 },
        "light_lux" => Threshold { min: 500.0, max: 15000.0 },
        "ph" => Threshold { min: 5.4, max: 6.2 },
        _ => return None,
    };
    Some(t)
}

/// Evaluate a value against a threshold: `None` when in range, otherwise which
/// side it violated.
pub fn evaluate(value: f64, threshold: &Threshold) -> Option<RangeStatus> {
    if value < threshold.min {
        Some(RangeStatus::Low)
    } else if value > threshold.max {
        Some(RangeStatus::High)
    } else {
        None
    }
}

/// Evaluate a reading by type using the default threshold. `None` when the type
/// has no default range or the value is within range.
pub fn evaluate_reading(reading_type: &str, value: f64) -> Option<RangeStatus> {
    let t = default_threshold(reading_type)?;
    evaluate(value, &t)
}

/// A human-readable label for a reading type (for flag messages).
pub fn reading_label(reading_type: &str) -> &str {
    match reading_type {
        "temp_c" => "Temperature",
        "humidity_pct" => "Humidity",
        "co2_ppm" => "CO₂",
        "light_lux" => "Light",
        "ph" => "pH",
        other => other,
    }
}

/// Build a flag message for an out-of-range reading, e.g.
/// "Temperature 31.5 above the 18.0–28.0 range".
pub fn range_message(reading_type: &str, value: f64, status: RangeStatus, threshold: &Threshold) -> String {
    let side = match status {
        RangeStatus::Low => "below",
        RangeStatus::High => "above",
    };
    format!(
        "{} {} {} the {:.1}–{:.1} range",
        reading_label(reading_type),
        value,
        side,
        threshold.min,
        threshold.max
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_range_is_none() {
        let t = default_threshold("temp_c").unwrap();
        assert_eq!(evaluate(24.0, &t), None);
        assert_eq!(evaluate(18.0, &t), None, "min boundary is inclusive");
        assert_eq!(evaluate(28.0, &t), None, "max boundary is inclusive");
    }

    #[test]
    fn below_and_above_are_detected() {
        let t = default_threshold("temp_c").unwrap();
        assert_eq!(evaluate(17.9, &t), Some(RangeStatus::Low));
        assert_eq!(evaluate(28.1, &t), Some(RangeStatus::High));
    }

    #[test]
    fn custom_and_unknown_types_never_flag() {
        assert!(default_threshold("custom").is_none());
        assert!(default_threshold("not_a_type").is_none());
        // A value that would be extreme for any real sensor still yields None,
        // because there is no default range to compare against.
        assert_eq!(evaluate_reading("custom", 999999.0), None);
        assert_eq!(evaluate_reading("not_a_type", -50.0), None);
    }

    #[test]
    fn evaluate_reading_covers_each_known_type() {
        assert_eq!(evaluate_reading("temp_c", 35.0), Some(RangeStatus::High));
        assert_eq!(evaluate_reading("humidity_pct", 20.0), Some(RangeStatus::Low));
        assert_eq!(evaluate_reading("co2_ppm", 200.0), Some(RangeStatus::Low));
        assert_eq!(evaluate_reading("light_lux", 20000.0), Some(RangeStatus::High));
        assert_eq!(evaluate_reading("ph", 7.0), Some(RangeStatus::High));
        // Healthy mid-range values stay clear.
        assert_eq!(evaluate_reading("ph", 5.8), None);
        assert_eq!(evaluate_reading("humidity_pct", 70.0), None);
    }

    #[test]
    fn message_reads_naturally() {
        let t = default_threshold("temp_c").unwrap();
        let msg = range_message("temp_c", 31.5, RangeStatus::High, &t);
        assert!(msg.contains("Temperature"));
        assert!(msg.contains("above"));
        assert!(msg.contains("18.0"));
        assert!(msg.contains("28.0"));
    }
}
