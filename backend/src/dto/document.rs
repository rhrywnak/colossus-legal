use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::models::document::Document;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentDto {
    pub id: String,
    pub title: String,
    pub doc_type: String,           // e.g. "complaint", "motion", "court_ruling"
    pub created_at: Option<String>, // ISO-8601 string or None
    pub file_path: Option<String>,  // filename of associated file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl From<Document> for DocumentDto {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id,
            title: doc.title,
            doc_type: doc.doc_type.unwrap_or_default(),
            created_at: doc.created_at.map(|dt| dt.to_string()),
            file_path: doc.file_path,
            notes: doc.notes,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentCreateRequest {
    pub title: String,
    pub doc_type: String,
    pub created_at: Option<String>,
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
    pub created_at: Option<String>,
    pub description: Option<String>,
    pub file_path: Option<String>,
    pub uploaded_at: Option<NaiveDate>,
    pub related_claim_id: Option<String>,
    pub source_url: Option<String>,
}
