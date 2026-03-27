//! POST /api/admin/pipeline/documents — Upload a PDF with metadata.
//!
//! ## Rust Learning: Two-phase multipart parsing
//!
//! Unlike a JSON body which is parsed all at once, multipart/form-data arrives
//! as a stream of fields. We loop through all fields, collecting metadata strings
//! and the file bytes separately, then validate that all required fields were
//! present after the loop completes.

use axum::{
    extract::{Multipart, State},
    Json,
};
use sha2::{Digest, Sha256};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, PipelineConfigInput};
use crate::state::AppState;

use super::{field_text, require_field, UploadDocumentResponse, MAX_FILE_SIZE};

/// POST /api/admin/pipeline/documents
///
/// Accepts a multipart form with a PDF file and metadata fields.
/// Saves the PDF to disk, computes SHA-256, and creates document +
/// pipeline_config records in the pipeline database.
pub async fn upload_document(
    user: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(axum::http::StatusCode, Json<UploadDocumentResponse>), AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, "POST /api/admin/pipeline/documents");

    // Collect all multipart fields into local variables.
    // Fields can arrive in any order, so we parse them all first.
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut doc_id: Option<String> = None;
    let mut title: Option<String> = None;
    let mut document_type: Option<String> = None;
    let mut schema_file: Option<String> = None;
    let mut pass1_model: Option<String> = None;
    let mut pass2_model: Option<String> = None;
    let mut admin_instructions: Option<String> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest {
        message: format!("Failed to read multipart field: {e}"),
        details: serde_json::json!({}),
    })? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                file_name = field.file_name().map(|s| s.to_string());
                let data = field.bytes().await.map_err(|e| AppError::BadRequest {
                    message: format!("Failed to read file data: {e}"),
                    details: serde_json::json!({}),
                })?;
                file_data = Some(data.to_vec());
            }
            "id" => doc_id = Some(field_text(&name, field).await?),
            "title" => title = Some(field_text(&name, field).await?),
            "document_type" => document_type = Some(field_text(&name, field).await?),
            "schema_file" => schema_file = Some(field_text(&name, field).await?),
            "pass1_model" => pass1_model = Some(field_text(&name, field).await?),
            "pass2_model" => pass2_model = Some(field_text(&name, field).await?),
            "admin_instructions" => admin_instructions = Some(field_text(&name, field).await?),
            _ => { /* ignore unknown fields */ }
        }
    }

    // Validate required fields
    let doc_id = require_field("id", doc_id)?;
    let title = require_field("title", title)?;
    let document_type = require_field("document_type", document_type)?;
    let schema_file = require_field("schema_file", schema_file)?;
    let file_data = file_data.ok_or_else(|| AppError::BadRequest {
        message: "No 'file' field in multipart upload".to_string(),
        details: serde_json::json!({}),
    })?;

    // Validate file
    if file_data.len() > MAX_FILE_SIZE {
        return Err(AppError::BadRequest {
            message: format!("File too large: {} bytes (max {MAX_FILE_SIZE})", file_data.len()),
            details: serde_json::json!({ "size_bytes": file_data.len(), "max_bytes": MAX_FILE_SIZE }),
        });
    }

    let original_name = file_name.unwrap_or_else(|| format!("{doc_id}.pdf"));
    if !original_name.to_lowercase().ends_with(".pdf") {
        return Err(AppError::BadRequest {
            message: "Only .pdf files are accepted".to_string(),
            details: serde_json::json!({ "filename": original_name }),
        });
    }

    // Compute SHA-256 hash
    let file_hash = format!("{:x}", Sha256::digest(&file_data));

    // Save PDF to disk using the document ID as filename (avoids collisions).
    let storage_filename = format!("{doc_id}.pdf");

    // Prevent path traversal
    if doc_id.contains("..") || doc_id.contains('/') || doc_id.contains('\\') {
        return Err(AppError::BadRequest {
            message: "Invalid document ID — must not contain path separators".to_string(),
            details: serde_json::json!({}),
        });
    }

    let dest_path = format!(
        "{}/{}",
        state.config.document_storage_path.trim_end_matches('/'),
        storage_filename
    );

    // Check for existing document in the database (409 if already exists)
    if pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .is_some()
    {
        return Err(AppError::Conflict {
            message: format!("Document '{doc_id}' already exists"),
            details: serde_json::json!({ "document_id": doc_id }),
        });
    }

    // Write file to disk
    tokio::fs::write(&dest_path, &file_data).await.map_err(|e| AppError::Internal {
        message: format!("Failed to write file to disk: {e}"),
    })?;

    // Insert document record
    pipeline_repository::insert_document(
        &state.pipeline_pool,
        &doc_id,
        &title,
        &storage_filename,
        &file_hash,
        &document_type,
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Failed to insert document: {e}") })?;

    // Insert pipeline config
    let config_input = PipelineConfigInput {
        pass1_model,
        pass2_model,
        pass1_max_tokens: None,
        pass2_max_tokens: None,
        schema_file,
        admin_instructions,
        prior_context_doc_ids: None,
    };
    pipeline_repository::insert_pipeline_config(
        &state.pipeline_pool,
        &doc_id,
        &config_input,
        &user.username,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to insert pipeline config: {e}"),
    })?;

    tracing::info!(user = %user.username, doc_id = %doc_id, size = file_data.len(), "Pipeline document uploaded");

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.upload",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({
            "title": title,
            "document_type": document_type,
            "size_bytes": file_data.len(),
            "file_hash": file_hash,
        })),
    )
    .await;

    // Fetch the inserted record to return it
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::Internal {
            message: "Document was inserted but not found on re-read".to_string(),
        })?;

    Ok((axum::http::StatusCode::CREATED, Json(UploadDocumentResponse { document })))
}
