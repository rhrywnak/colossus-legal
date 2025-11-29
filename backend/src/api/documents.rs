use axum::response::IntoResponse;
use axum::Json;

use crate::dto::DocumentDto;

pub async fn list_documents() -> impl IntoResponse {
    let documents = vec![
        DocumentDto {
            id: "doc-1".to_string(),
            title: "Sample Document A".to_string(),
            doc_type: "pdf".to_string(),
            created_at: Some("2025-01-01T00:00:00Z".to_string()),
        },
        DocumentDto {
            id: "doc-2".to_string(),
            title: "Sample Document B".to_string(),
            doc_type: "motion".to_string(),
            created_at: None,
        },
    ];

    Json(documents)
}
