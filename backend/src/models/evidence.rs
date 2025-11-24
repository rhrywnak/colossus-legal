use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Evidence {
    pub id: String,
    pub claim_id: Option<String>,
    pub document_id: Option<String>,
    pub description: Option<String>,
    pub evidence_type: Option<String>,
    pub is_supporting: Option<bool>,
    pub collected_on: Option<NaiveDate>,
}
