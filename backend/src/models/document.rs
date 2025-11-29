use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub doc_type: Option<String>,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub uploaded_at: Option<NaiveDate>,
    pub related_claim_id: Option<String>,
    pub source_url: Option<String>,
    // REQUIRED for T3.1a
    pub created_at: Option<String>,
}
