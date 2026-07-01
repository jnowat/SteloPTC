use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalReading {
    pub id: String,
    pub specimen_id: Option<String>,
    pub subculture_id: Option<String>,
    /// "temp_c" | "humidity_pct" | "co2_ppm" | "light_lux" | "ph" | "custom"
    pub reading_type: String,
    pub value: f64,
    pub unit: Option<String>,
    /// "manual" | "usb_serial" | "bluetooth" | "mqtt"
    pub source: String,
    pub recorded_at: String,
    pub notes: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateEnvironmentalReadingRequest {
    pub specimen_id: Option<String>,
    pub subculture_id: Option<String>,
    pub reading_type: String,
    pub value: f64,
    pub unit: Option<String>,
    /// Defaults to "manual" when omitted.
    pub source: Option<String>,
    /// Defaults to now() when omitted.
    pub recorded_at: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentalAlert {
    pub specimen_id: Option<String>,
    pub reading_type: String,
    pub value: f64,
    pub threshold_min: Option<f64>,
    pub threshold_max: Option<f64>,
    pub message: String,
    pub recorded_at: String,
}

/// One reading extracted from a raw transport payload (a serial line, an MQTT
/// message body, etc.) before it is persisted. Transport-agnostic by design —
/// see `db::sensors` module docs for what is and isn't wired to real hardware.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedReading {
    pub reading_type: String,
    pub value: f64,
}
