use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::models::document::Document;

/// DTO for Document — v1 API format for backward compatibility.
///
/// The internal Document model is v2 (with DocumentType enum, DateTime<Utc>, etc.)
/// but the API still returns the v1 DTO format until the frontend is updated.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentDto {
    pub id: String,
    pub title: String,
    pub doc_type: String,           // v2→v1 bridge: enum converted to string
    pub created_at: Option<String>, // v2→v1 bridge: DateTime converted to ISO string
}

/// Bridge: Convert v2 Document model to v1 DocumentDto for API backward compatibility.
impl From<Document> for DocumentDto {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id,
            title: doc.title,
            // v2→v1 bridge: Convert DocumentType enum to snake_case string
            doc_type: doc.doc_type.to_string(),
            // v2→v1 bridge: Convert DateTime<Utc> to ISO-8601 string
            created_at: Some(doc.created_at.to_rfc3339()),
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
