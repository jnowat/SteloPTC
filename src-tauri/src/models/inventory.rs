use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub unit: String,
    pub physical_state: Option<String>,    // 'solid' | 'liquid'
    pub concentration: Option<f64>,        // stock concentration for liquids
    pub concentration_unit: Option<String>, // nM, µM, mM, M, ng/mL, µg/mL, mg/mL, mg/L, g/L, %
    pub current_stock: f64,
    pub minimum_stock: f64,
    pub reorder_point: Option<f64>,
    pub supplier: Option<String>,
    pub catalog_number: Option<String>,
    pub lot_number: Option<String>,
    pub storage_location: Option<String>,
    pub expiration_date: Option<String>,
    pub cost_per_unit: Option<f64>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateInventoryItemRequest {
    pub name: String,
    pub category: String,
    pub unit: String,
    pub physical_state: Option<String>,
    pub concentration: Option<f64>,
    pub concentration_unit: Option<String>,
    pub current_stock: Option<f64>,
    pub minimum_stock: Option<f64>,
    pub reorder_point: Option<f64>,
    pub supplier: Option<String>,
    pub catalog_number: Option<String>,
    pub lot_number: Option<String>,
    pub storage_location: Option<String>,
    pub expiration_date: Option<String>,
    pub cost_per_unit: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateInventoryItemRequest {
    pub id: String,
    pub name: Option<String>,
    pub category: Option<String>,
    pub unit: Option<String>,
    pub physical_state: Option<String>,
    pub concentration: Option<f64>,
    pub concentration_unit: Option<String>,
    pub current_stock: Option<f64>,
    pub minimum_stock: Option<f64>,
    pub reorder_point: Option<f64>,
    pub supplier: Option<String>,
    pub catalog_number: Option<String>,
    pub lot_number: Option<String>,
    pub storage_location: Option<String>,
    pub expiration_date: Option<String>,
    pub cost_per_unit: Option<f64>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LowStockAlert {
    pub id: String,
    pub name: String,
    pub category: String,
    pub current_stock: f64,
    pub minimum_stock: f64,
    pub reorder_point: Option<f64>,
    pub unit: String,
    pub physical_state: Option<String>,
}

// Prepared stock solutions derived from inventory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedSolution {
    pub id: String,
    pub name: String,
    pub source_item_id: Option<String>,
    pub source_item_name: Option<String>,
    pub concentration: f64,
    pub concentration_unit: String,
    pub solvent: Option<String>,
    pub volume_ml: f64,
    pub volume_remaining_ml: f64,
    pub prepared_by: Option<String>,
    pub preparation_date: String,
    pub expiration_date: Option<String>,
    pub storage_conditions: Option<String>,
    pub lot_number: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePreparedSolutionRequest {
    pub name: String,
    pub source_item_id: Option<String>,
    pub concentration: f64,
    pub concentration_unit: String,
    pub solvent: Option<String>,
    pub volume_ml: f64,
    pub prepared_by: Option<String>,
    pub preparation_date: String,
    pub expiration_date: Option<String>,
    pub storage_conditions: Option<String>,
    pub lot_number: Option<String>,
    pub notes: Option<String>,
    /// Amount of source item consumed to make this solution (deducted from inventory)
    pub source_amount_used: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreparedSolutionRequest {
    pub id: String,
    pub volume_remaining_ml: Option<f64>,
    pub storage_conditions: Option<String>,
    pub expiration_date: Option<String>,
    pub notes: Option<String>,
}
