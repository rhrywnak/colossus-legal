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
use crate::pipeline::config::{PipelineConfigOverrides, ProcessingProfile};
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps, PipelineConfigInput};
use crate::state::AppState;

use super::{field_text, require_field, UploadDocumentResponse, MAX_FILE_SIZE};

/// Map a document type name to its extraction schema filename.
///
/// This is the single authoritative mapping between document types and
/// schemas. Used by both upload (to set initial pipeline_config) and
/// extract_text (to update pipeline_config after auto-detection).
///
/// ## Why this function exists
///
/// Previously, the mapping existed in two places (upload.rs and a comment
/// in extract_text.rs) and they were inconsistent. Centralizing it here
/// ensures both use the same mapping and both are updated when schemas change.
/// Derive the default processing-profile name from a schema filename.
///
/// Mirrors the two private copies in `pipeline::steps::llm_extract` and
/// `api::pipeline::config_endpoints::preview` so upload-time pre-population
/// picks the same profile the extraction step would load at run time.
/// Strips `.yaml` and any trailing `_v<digits>` version suffix.
/// Examples: `"complaint_v2.yaml"` → `"complaint"`, `"motion_v4.yaml"` → `"motion"`.
fn default_profile_name_from_schema(schema_file: &str) -> String {
    let base = schema_file.trim_end_matches(".yaml");
    if let Some(idx) = base.rfind("_v") {
        let suffix = &base[idx + 2..];
        if !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()) {
            return base[..idx].to_string();
        }
    }
    base.to_string()
}

pub fn schema_for_document_type(document_type: &str) -> &'static str {
    match document_type {
        "complaint" => "complaint_v4.yaml",
        "discovery_response" => "discovery_response_v4.yaml",
        "motion" => "motion_v4.yaml",
        "brief" => "brief_v4.yaml",
        "motion_brief" => "motion_v4.yaml",
        "affidavit" => "affidavit_v4.yaml",
        "court_ruling" => "court_ruling_v4.yaml",
        _ => "complaint_v4.yaml",
    }
}

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
    let start = std::time::Instant::now();
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

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest {
            message: format!("Failed to read multipart field: {e}"),
            details: serde_json::json!({}),
        })?
    {
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
    let document_type = document_type.unwrap_or_else(|| "auto".to_string());
    // Schema file selection: use client-provided value if present,
    // otherwise derive from document type. The extract handler will
    // also select the schema based on document_type, so this is mainly
    // for recording the intended schema in pipeline_config.
    let schema_file =
        schema_file.unwrap_or_else(|| schema_for_document_type(&document_type).to_string());
    let file_data = file_data.ok_or_else(|| AppError::BadRequest {
        message: "No 'file' field in multipart upload".to_string(),
        details: serde_json::json!({}),
    })?;

    // Complaint-first enforcement: the first document must be a complaint.
    // The complaint establishes parties, claims, and legal context that all
    // other documents reference.
    let has_complaint =
        pipeline_repository::documents::has_document_of_type(&state.pipeline_pool, "complaint")
            .await
            .map_err(|e| AppError::Internal {
                message: format!("DB error: {e}"),
            })?;

    if !has_complaint && document_type != "complaint" && document_type != "auto" {
        return Err(AppError::BadRequest {
            message: "A Complaint document must be uploaded first. The Complaint establishes the parties, claims, and legal context that all other documents reference.".to_string(),
            details: serde_json::json!({ "document_type": document_type }),
        });
    }

    // Validate file
    if file_data.len() > MAX_FILE_SIZE {
        return Err(AppError::BadRequest {
            message: format!(
                "File too large: {} bytes (max {MAX_FILE_SIZE})",
                file_data.len()
            ),
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
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .is_some()
    {
        return Err(AppError::Conflict {
            message: format!("Document '{doc_id}' already exists"),
            details: serde_json::json!({ "document_id": doc_id }),
        });
    }

    // Write file to disk
    tokio::fs::write(&dest_path, &file_data)
        .await
        .map_err(|e| AppError::Internal {
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
    .map_err(|e| AppError::Internal {
        message: format!("Failed to insert document: {e}"),
    })?;

    // Classify PDF content — this populates text/scanned/mixed, per-page
    // OCR flags, and character counts. Failures are logged but MUST NOT
    // block the upload: the default `content_type = 'unknown'` stays on
    // the row and ExtractText handles discovery the hard way at processing
    // time. Design: PDF_CONTENT_CLASSIFICATION_DESIGN_v2.md Phase B.
    let classification = match colossus_pdf::PdfTextExtractor::open(&dest_path) {
        Ok(mut extractor) => match extractor.classify() {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::warn!(
                    doc_id = %doc_id, error = %e,
                    "PDF classification failed, defaulting to unknown"
                );
                None
            }
        },
        Err(e) => {
            tracing::warn!(
                doc_id = %doc_id, error = %e,
                "Failed to open PDF for classification"
            );
            None
        }
    };

    if let Some(ref c) = classification {
        let pages_ocr: Vec<i32> = c.pages_needing_ocr.iter().map(|&p| p as i32).collect();
        if let Err(e) = sqlx::query(
            "UPDATE documents SET content_type = $1, page_count = $2, \
             text_pages = $3, scanned_pages = $4, pages_needing_ocr = $5, \
             total_chars = $6 WHERE id = $7",
        )
        .bind(c.content_type.as_str())
        .bind(c.page_count as i32)
        .bind(c.text_pages as i32)
        .bind(c.scanned_pages as i32)
        .bind(&pages_ocr)
        .bind(c.total_chars as i32)
        .bind(&doc_id)
        .execute(&state.pipeline_pool)
        .await
        {
            tracing::warn!(
                doc_id = %doc_id, error = %e,
                "Failed to store PDF classification — upload succeeds, content_type stays 'unknown'"
            );
        }
    }

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

    // Problem 4: pre-populate the per-document override columns from the
    // matched processing profile. The Configuration Panel reads those
    // columns — without this, it shows compiled defaults until the user
    // manually overrides each dropdown. Profile-load failure is non-fatal:
    // the extraction step will re-attempt and fail loudly if the profile
    // truly does not exist.
    let profile_name = default_profile_name_from_schema(&config_input.schema_file);
    match ProcessingProfile::load(&state.config.processing_profile_dir, &profile_name) {
        Ok(profile) => {
            let overrides = PipelineConfigOverrides {
                profile_name: Some(profile.name.clone()),
                extraction_model: Some(profile.extraction_model.clone()),
                pass2_extraction_model: profile.pass2_extraction_model.clone(),
                template_file: Some(profile.template_file.clone()),
                system_prompt_file: profile.system_prompt_file.clone(),
                chunking_mode: Some(profile.chunking_mode.clone()),
                chunk_size: profile.chunk_size,
                chunk_overlap: profile.chunk_overlap,
                max_tokens: Some(profile.max_tokens),
                temperature: Some(profile.temperature),
                run_pass2: Some(profile.run_pass2),
            };
            if let Err(e) = pipeline_repository::patch_pipeline_config_overrides(
                &state.pipeline_pool,
                &doc_id,
                &overrides,
            )
            .await
            {
                tracing::warn!(
                    doc_id = %doc_id, error = %e,
                    "Failed to persist profile overrides at upload (non-fatal) — \
                     Configuration Panel may show compiled defaults until user edits"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                doc_id = %doc_id, profile = %profile_name, error = %e,
                "Profile load failed at upload (non-fatal) — Configuration Panel \
                 will show compiled defaults; extraction will re-attempt load"
            );
        }
    }

    tracing::info!(user = %user.username, doc_id = %doc_id, size = file_data.len(), "Pipeline document uploaded");

    // Record step (after document exists in DB so FK is satisfied)
    if let Ok(step_id) = steps::record_step_start(
        &state.pipeline_pool,
        &doc_id,
        "upload",
        &user.username,
        &serde_json::json!({"filename": original_name, "document_type": document_type}),
    )
    .await
    {
        if let Err(e) = steps::record_step_complete(
            &state.pipeline_pool,
            step_id,
            start.elapsed().as_secs_f64(),
            &serde_json::json!({
                "file_name": original_name,
                "file_size_bytes": file_data.len(),
                "file_hash": file_hash,
                "document_type": document_type,
            }),
        )
        .await
        {
            tracing::error!(
                document_id = %doc_id,
                step_id = step_id,
                error = %e,
                "Failed to record upload step completion — audit trail gap"
            );
        }
    }

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
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::Internal {
            message: "Document was inserted but not found on re-read".to_string(),
        })?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(UploadDocumentResponse { document }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── schema_for_document_type tests ──────────────────────────

    /// These tests document the authoritative mapping between document
    /// types and schema files. If a schema file is renamed, these tests
    /// will fail — which is the correct behavior (the mapping must be
    /// updated explicitly, not silently broken).

    #[test]
    fn test_complaint_maps_to_v4_schema() {
        assert_eq!(
            schema_for_document_type("complaint"),
            "complaint_v4.yaml",
            "Complaint must use complaint_v4.yaml"
        );
    }

    #[test]
    fn test_auto_maps_to_complaint_schema() {
        // "auto" documents default to complaint schema because this system
        // processes legal complaints as its primary document type.
        assert_eq!(
            schema_for_document_type("auto"),
            "complaint_v4.yaml",
            "Auto-detect defaults to complaint schema for this system"
        );
    }

    #[test]
    fn test_unknown_maps_to_complaint_schema() {
        assert_eq!(schema_for_document_type("unknown"), "complaint_v4.yaml");
    }

    #[test]
    fn test_discovery_response_schema() {
        assert_eq!(
            schema_for_document_type("discovery_response"),
            "discovery_response_v4.yaml"
        );
    }

    #[test]
    fn test_motion_schema() {
        assert_eq!(schema_for_document_type("motion"), "motion_v4.yaml");
        assert_eq!(schema_for_document_type("brief"), "brief_v4.yaml");
        assert_eq!(schema_for_document_type("motion_brief"), "motion_v4.yaml");
    }

    #[test]
    fn test_affidavit_schema() {
        assert_eq!(schema_for_document_type("affidavit"), "affidavit_v4.yaml");
    }

    #[test]
    fn test_court_ruling_schema() {
        assert_eq!(
            schema_for_document_type("court_ruling"),
            "court_ruling_v4.yaml"
        );
    }

    #[test]
    fn test_completely_unknown_type_defaults_to_complaint() {
        // Any unrecognized document type defaults to complaint schema.
        // This prevents general_legal.yaml from being used, which produces
        // generic Statement/Party entities instead of legal-specific ones.
        assert_eq!(schema_for_document_type("garbage"), "complaint_v4.yaml");
        assert_eq!(schema_for_document_type(""), "complaint_v4.yaml");
    }

    #[test]
    fn test_schema_mapping_is_exhaustive() {
        // Verify all known document types return a .yaml file.
        // This test catches typos in schema filenames.
        let known_types = [
            "complaint",
            "discovery_response",
            "motion",
            "brief",
            "motion_brief",
            "affidavit",
            "court_ruling",
            "auto",
            "unknown",
        ];
        for doc_type in &known_types {
            let schema = schema_for_document_type(doc_type);
            assert!(
                schema.ends_with(".yaml"),
                "Schema for '{}' must end with .yaml, got: {}",
                doc_type,
                schema
            );
            assert!(
                !schema.is_empty(),
                "Schema for '{}' must not be empty",
                doc_type
            );
        }
    }
}
