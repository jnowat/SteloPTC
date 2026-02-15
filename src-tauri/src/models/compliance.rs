use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRecord {
    pub id: String,
    pub specimen_id: String,
    pub specimen_accession: Option<String>,
    pub record_type: String,
    pub agency: Option<String>,
    pub permit_number: Option<String>,
    pub permit_expiry: Option<String>,
    pub test_type: Option<String>,
    pub test_method: Option<String>,
    pub test_date: Option<String>,
    pub test_lab: Option<String>,
    pub test_result: Option<String>,
    pub status: String,
    pub flag_reason: Option<String>,
    pub chain_of_custody: Option<String>,
    pub notes: Option<String>,
    pub document_path: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateComplianceRequest {
    pub specimen_id: String,
    pub record_type: String,
    pub agency: Option<String>,
    pub permit_number: Option<String>,
    pub permit_expiry: Option<String>,
    pub test_type: Option<String>,
    pub test_method: Option<String>,
    pub test_date: Option<String>,
    pub test_lab: Option<String>,
    pub test_result: Option<String>,
    pub status: Option<String>,
    pub chain_of_custody: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateComplianceRequest {
    pub id: String,
    pub test_result: Option<String>,
    pub status: Option<String>,
    pub flag_reason: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ComplianceFlag {
    pub specimen_id: String,
    pub accession_number: String,
    pub species_code: String,
    pub flag_type: String,
    pub message: String,
    pub severity: String,
}
