//! POST /api/admin/pipeline/documents/:id/extract — chunk-based LLM extraction (Pass 1).

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use axum::{extract::Path as AxumPath, extract::State, Json};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

use super::chunk_extractor::AnthropicChunkExtractor;
use super::{chunk_orchestration, chunk_storage, ExtractResponse};

/// Optional request body for extraction overrides. Non-null fields override
/// the `pipeline_config` defaults.
#[derive(Debug, serde::Deserialize, Default)]
pub struct ExtractRequest {
    #[serde(default)]
    pub schema_file: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub admin_instructions: Option<String>,
    #[serde(default)]
    pub temperature: Option<f64>,
}

/// Core logic for LLM extraction — callable from handler AND process endpoint.
///
/// Runs chunk-based extraction, validates completeness, stores entities.
/// Does NOT check document status — caller is responsible for validation.
pub(crate) async fn run_extract(
    state: &AppState,
    doc_id: &str,
    username: &str,
    overrides: ExtractRequest,
) -> Result<ExtractResponse, AppError> {
    let step_start = Instant::now();

    let step_id = steps::record_step_start(
        &state.pipeline_pool, doc_id, "extract", username, &serde_json::json!({}),
    ).await.map_err(|e| AppError::Internal { message: format!("Step logging: {e}") })?;

    let api_key = state.config.anthropic_api_key.as_deref().ok_or_else(|| {
        AppError::Internal { message: "ANTHROPIC_API_KEY not configured".to_string() }
    })?;

    let document = pipeline_repository::get_document(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Document '{doc_id}' not found") })?;

    let pipe_config = pipeline_repository::get_pipeline_config(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("No pipeline config for '{doc_id}'") })?;

    let pages = pipeline_repository::get_document_text(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;
    if pages.is_empty() {
        return Err(AppError::BadRequest {
            message: "No text pages found for document".to_string(),
            details: serde_json::json!({}),
        });
    }
    let full_text: String = pages.iter()
        .map(|p| format!("--- Page {} ---\n{}", p.page_number, p.text_content))
        .collect::<Vec<_>>()
        .join("\n\n");

    let has_overrides = overrides.schema_file.is_some()
        || overrides.model.is_some()
        || overrides.max_tokens.is_some()
        || overrides.admin_instructions.is_some();
    let schema_file_owned = overrides.schema_file.unwrap_or_else(|| pipe_config.schema_file.clone());
    let schema_file = schema_file_owned.as_str();
    let model_name = overrides.model.unwrap_or_else(|| pipe_config.pass1_model.clone());
    let max_tokens = overrides.max_tokens.unwrap_or(pipe_config.pass1_max_tokens as u32);
    let admin_instructions_owned = overrides.admin_instructions.or_else(|| pipe_config.admin_instructions.clone());
    let admin_instructions = admin_instructions_owned.as_deref();
    let temperature = overrides.temperature;

    let schema_path = format!("{}/{}", state.config.extraction_schema_dir, schema_file);
    tracing::info!(
        doc_id = %doc_id, schema = %schema_file, document_type = %document.document_type,
        "Loading schema '{}'", schema_file,
    );
    let schema = colossus_extract::ExtractionSchema::from_file(Path::new(&schema_path))
        .map_err(|e| AppError::Internal {
            message: format!("Failed to load schema '{}': {e}", schema_file),
        })?;

    let template_dir = Path::new(&state.config.extraction_template_dir);
    let raw_template = std::fs::read_to_string(template_dir.join("chunk_extract.md"))
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read chunk_extract.md: {e}"),
        })?;
    let mut builder = colossus_extract::PromptBuilder::new(template_dir);
    let artifact = builder
        .build_extraction_prompt(&schema, &full_text, None, admin_instructions, Some("global_rules.md"), Some("chunk_extract.md"))
        .map_err(|e| AppError::Internal { message: format!("Failed to build prompt: {e}") })?;

    let schema_json_value = serde_json::to_value(&schema).ok();
    let run_id = pipeline_repository::insert_extraction_run(
        &state.pipeline_pool, doc_id, 1, &model_name, &schema.document_type,
        Some(&artifact.prompt_text), Some(&artifact.template_name), Some(&artifact.template_hash),
        artifact.rules_name.as_deref(), artifact.rules_hash.as_deref(),
        Some(&artifact.schema_hash), schema_json_value.as_ref(),
        temperature, Some(max_tokens as i32), admin_instructions, None,
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Failed to insert extraction run: {e}") })?;

    tracing::info!(
        doc_id = %doc_id, text_len = full_text.len(), model = %model_name, max_tokens,
        "Running chunk-based extraction"
    );

    let extractor = Arc::new(AnthropicChunkExtractor::new(
        api_key.to_string(), model_name.clone(), max_tokens as u64,
    ));
    let schema_for_chunks = schema_json_value.clone().unwrap_or(serde_json::Value::Null);

    let api_start = Instant::now();
    let summary_result = chunk_orchestration::run_chunk_extraction(
        &state.pipeline_pool, run_id, doc_id, &full_text, &schema_for_chunks,
        &raw_template, Arc::clone(&extractor),
    ).await;
    let elapsed_secs = api_start.elapsed().as_secs_f64();

    let summary = match summary_result {
        Ok(s) => s,
        Err(e) => {
            let _ = pipeline_repository::complete_extraction_run(
                &state.pipeline_pool, run_id,
                &serde_json::json!({ "error": format!("{e:?}") }),
                None, None, None, "FAILED",
            ).await;
            steps::record_step_failure(
                &state.pipeline_pool, step_id, step_start.elapsed().as_secs_f64(),
                &format!("{e:?}"),
            ).await.ok();
            return Err(e);
        }
    };

    pipeline_repository::complete_extraction_run(
        &state.pipeline_pool, run_id, &summary.legacy_json,
        None, None, None, "COMPLETED",
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Failed to complete extraction run: {e}") })?;

    chunk_orchestration::update_run_chunk_stats(
        &state.pipeline_pool, run_id,
        summary.chunk_count, summary.chunks_succeeded, summary.chunks_failed,
    ).await;

    let completeness = super::completeness_validation::validate_completeness(&schema, &summary.legacy_json);
    for (entity_type, count) in &completeness.entity_counts {
        tracing::info!(doc_id = %doc_id, entity_type = %entity_type, count, "Extracted entity count");
    }
    for warning in &completeness.warnings {
        tracing::warn!(doc_id = %doc_id, "Completeness warning: {}", warning);
    }

    if !completeness.passed {
        pipeline_repository::update_document_status(&state.pipeline_pool, doc_id, "EXTRACTION_FAILED")
            .await
            .map_err(|e| AppError::Internal { message: format!("Failed to update status: {e}") })?;
        steps::record_step_failure(
            &state.pipeline_pool, step_id, step_start.elapsed().as_secs_f64(),
            &format!("Completeness validation failed: {:?}", completeness.errors),
        ).await.ok();
        return Err(AppError::BadRequest {
            message: format!("Extraction completeness validation failed: {}", completeness.errors.join("; ")),
            details: serde_json::json!({
                "errors": completeness.errors,
                "warnings": completeness.warnings,
                "entity_counts": completeness.entity_counts,
            }),
        });
    }

    let (entity_count, rel_count) = chunk_storage::store_entities_and_relationships(
        state, run_id, doc_id, &summary.legacy_json,
    ).await?;

    pipeline_repository::update_document_status(&state.pipeline_pool, doc_id, "EXTRACTED")
        .await
        .map_err(|e| AppError::Internal { message: format!("Failed to update document status: {e}") })?;

    tracing::info!(
        doc_id = %doc_id, entity_count, rel_count,
        chunk_count = summary.chunk_count,
        chunks_succeeded = summary.chunks_succeeded,
        chunks_failed = summary.chunks_failed,
        "Extraction complete"
    );

    let action_details = serde_json::json!({
        "model": model_name,
        "entity_count": entity_count,
        "relationship_count": rel_count,
        "chunk_count": summary.chunk_count,
        "chunks_succeeded": summary.chunks_succeeded,
        "chunks_failed": summary.chunks_failed,
    });
    log_admin_action(
        &state.audit_repo, username, "pipeline.document.extract",
        Some("document"), Some(doc_id), Some(action_details.clone()),
    ).await;

    steps::record_step_complete(
        &state.pipeline_pool, step_id, step_start.elapsed().as_secs_f64(), &action_details,
    ).await.ok();

    if has_overrides {
        pipeline_repository::update_pipeline_config(
            &state.pipeline_pool, doc_id, &model_name,
            max_tokens as i32, schema_file, admin_instructions,
        ).await.ok();
    }

    Ok(ExtractResponse {
        document_id: doc_id.to_string(),
        status: "EXTRACTED".to_string(),
        run_id,
        model: model_name,
        entity_count,
        relationship_count: rel_count,
        input_tokens: 0,
        output_tokens: 0,
        elapsed_secs,
    })
}

/// POST /api/admin/pipeline/documents/:id/extract
///
/// HTTP handler — thin wrapper around `run_extract`.
/// Checks admin auth and status guard, then delegates to core logic.
pub async fn extract_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
    body: Option<Json<ExtractRequest>>,
) -> Result<Json<ExtractResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST extract");

    // Status guard
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Document '{doc_id}' not found") })?;

    if document.status != "TEXT_EXTRACTED" {
        return Err(AppError::Conflict {
            message: format!("Cannot extract: status is '{}', expected 'TEXT_EXTRACTED'", document.status),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    let overrides = body.map(|b| b.0).unwrap_or_default();
    let result = run_extract(&state, &doc_id, &user.username, overrides).await?;
    Ok(Json(result))
}
