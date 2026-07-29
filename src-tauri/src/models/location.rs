use serde::{Deserialize, Serialize};

/// WP-57: a physical lab location (room/rack/area) with an optional
/// floor-plan pin. Purely additive to the existing free-text
/// `specimens.location` / `location_details` fields — this is a separate,
/// optional entity used only by the interactive lab map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Base64-encoded floor-plan image (inline storage, matching the existing
    /// `attachments` convention), `None` until a plan is uploaded.
    pub floor_plan_image: Option<String>,
    /// Fractional (0.0-1.0) position on the floor-plan image.
    pub floor_plan_x: Option<f64>,
    pub floor_plan_y: Option<f64>,
    /// The drawn floor plan for this room, as the JSON document
    /// `src/lib/labLayout.ts` reads and writes: a grid size plus a list of
    /// furniture, each with a footprint and a shelf breakdown.
    ///
    /// Stored as an opaque blob rather than normalised tables on purpose. The
    /// geometry is a *document* — it is always read whole, written whole, and
    /// never queried across rooms — and normalising it would buy a join nobody
    /// needs at the cost of a migration every time the editor grows a field.
    /// What *is* queried, specimen placement, keeps living in the existing
    /// `specimens.location` path string; the layout generates those paths.
    pub layout_json: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateLocationRequest {
    pub name: String,
    pub description: Option<String>,
    pub floor_plan_image: Option<String>,
    pub floor_plan_x: Option<f64>,
    pub floor_plan_y: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLocationRequest {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub floor_plan_image: Option<String>,
    pub floor_plan_x: Option<f64>,
    pub floor_plan_y: Option<f64>,
}

/// One row of the map's data feed: a location's pin position plus the
/// aggregates needed to render density / contamination-risk / age heat-maps
/// without the frontend having to fetch every specimen individually.
#[derive(Debug, Clone, Serialize)]
pub struct LocationMapPoint {
    pub location_id: String,
    pub name: String,
    pub floor_plan_x: Option<f64>,
    pub floor_plan_y: Option<f64>,
    pub specimen_count: i64,
    pub contaminated_count: i64,
    pub avg_age_days: Option<f64>,
}

/// Live specimen counts for one recorded location path, e.g.
/// `"Growth Room B / Rack A / Shelf 3 / B2"`.
///
/// The lab-map editor folds these onto the drawn furniture to shade the plan by
/// how full each rack is. Grouping happens in SQL so the client never has to
/// pull every specimen to colour a picture.
#[derive(Debug, Clone, Serialize)]
pub struct LocationOccupancy {
    /// The free-text location path exactly as recorded on the specimens.
    pub location: String,
    pub specimen_count: i64,
    pub contaminated_count: i64,
}
