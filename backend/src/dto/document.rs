use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentCreateRequest {
    pub id: Option<String>,
    pub title: String,
    pub doc_type: Option<String>,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub uploaded_at: Option<NaiveDate>,
    pub related_claim_id: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentUpdateRequest {
    pub title: Option<String>,
    pub doc_type: Option<String>,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub uploaded_at: Option<NaiveDate>,
    pub related_claim_id: Option<String>,
    pub source_url: Option<String>,
}
