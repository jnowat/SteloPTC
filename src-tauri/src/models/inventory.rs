use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub unit: String,
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
}
