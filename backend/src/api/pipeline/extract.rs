//! POST /api/admin/pipeline/documents/:id/extract — LLM extraction (Pass 1).

use std::collections::HashMap;
use std::path::Path;

use axum::{extract::Path as AxumPath, extract::State, Json};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

use super::anthropic::call_anthropic;
use super::ExtractResponse;

/// Loads extraction schema, builds prompt, calls Claude, parses JSON response,
/// stores entities + relationships in the pipeline database.
pub async fn extract_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<ExtractResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST extract");

    let api_key = state.config.anthropic_api_key.as_deref().ok_or_else(|| {
        AppError::Internal { message: "ANTHROPIC_API_KEY not configured".to_string() }
    })?;

    // 1. Fetch document — 404 if not found
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Document '{doc_id}' not found") })?;

    // 2. Check status — must be TEXT_EXTRACTED
    if document.status != "TEXT_EXTRACTED" {
        return Err(AppError::Conflict {
            message: format!("Cannot extract: status is '{}', expected 'TEXT_EXTRACTED'", document.status),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    // 3. Fetch pipeline config
    let pipe_config = pipeline_repository::get_pipeline_config(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("No pipeline config for '{doc_id}'") })?;

    // 4. Fetch document text pages and concatenate
    let pages = pipeline_repository::get_document_text(&state.pipeline_pool, &doc_id)
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

    // 5. Load extraction schema from YAML
    let schema_path = format!("{}/{}", state.config.extraction_schema_dir, pipe_config.schema_file);
    let schema = colossus_extract::ExtractionSchema::from_file(Path::new(&schema_path))
        .map_err(|e| AppError::BadRequest {
            message: format!("Failed to load schema '{}': {e}", pipe_config.schema_file),
            details: serde_json::json!({ "schema_file": pipe_config.schema_file }),
        })?;

    // 6. Build prompt
    let mut builder = colossus_extract::PromptBuilder::new(
        Path::new(&state.config.extraction_template_dir),
    );
    let prompt = builder
        .build_extraction_prompt(&schema, &full_text, None, pipe_config.admin_instructions.as_deref(), Some("global_rules.md"))
        .map_err(|e| AppError::Internal { message: format!("Failed to build prompt: {e}") })?;

    let model_name = pipe_config.pass1_model.clone();
    let max_tokens = pipe_config.pass1_max_tokens as u32;

    // 7. Insert extraction run (status = RUNNING)
    let run_id = pipeline_repository::insert_extraction_run(
        &state.pipeline_pool, &doc_id, 1, &model_name, &schema.document_type,
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Failed to insert extraction run: {e}") })?;

    // 8. Call Anthropic API
    tracing::info!(
        prompt_len = prompt.len(),
        prompt_preview = %&prompt[..prompt.len().min(200)],
        model = %model_name,
        max_tokens = max_tokens,
        "Calling Anthropic API for extraction"
    );
    let api_result = call_anthropic(&state.http_client, api_key, &model_name, max_tokens, &prompt).await;

    let (response_text, usage) = match api_result {
        Ok(r) => r,
        Err(e) => {
            // Mark run as FAILED
            let _ = pipeline_repository::complete_extraction_run(
                &state.pipeline_pool,
                run_id,
                &serde_json::json!({ "error": format!("{e:?}") }),
                None,
                None,
                None,
                "FAILED",
            )
            .await;
            return Err(e);
        }
    };

    // 9. Parse JSON from LLM response
    let parsed: serde_json::Value = match serde_json::from_str(&response_text) {
        Ok(v) => v,
        Err(parse_err) => {
            // Store raw text for debugging, mark as FAILED
            let _ = pipeline_repository::complete_extraction_run(
                &state.pipeline_pool,
                run_id,
                &serde_json::json!({ "raw_text": response_text }),
                Some(usage.input_tokens as i32),
                Some(usage.output_tokens as i32),
                None,
                "FAILED",
            )
            .await;
            return Err(AppError::BadRequest {
                message: format!("LLM returned invalid JSON: {parse_err}"),
                details: serde_json::json!({
                    "parse_error": parse_err.to_string(),
                    "raw_text_preview": &response_text[..response_text.len().min(500)],
                }),
            });
        }
    };

    // 10. Complete the extraction run with raw output
    pipeline_repository::complete_extraction_run(
        &state.pipeline_pool,
        run_id,
        &parsed,
        Some(usage.input_tokens as i32),
        Some(usage.output_tokens as i32),
        None, // Cost calculated later if needed
        "COMPLETED",
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to complete extraction run: {e}"),
    })?;

    // 11-12. Parse entities and relationships, insert into DB
    let (entity_count, rel_count) =
        store_entities_and_relationships(&state, run_id, &doc_id, &parsed).await?;

    // 13. Update document status
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, "EXTRACTED")
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update document status: {e}"),
        })?;

    tracing::info!(
        doc_id = %doc_id, entity_count, rel_count,
        input_tokens = usage.input_tokens, output_tokens = usage.output_tokens,
        "Extraction complete"
    );

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.extract",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({
            "model": model_name,
            "entity_count": entity_count,
            "relationship_count": rel_count,
            "input_tokens": usage.input_tokens,
            "output_tokens": usage.output_tokens,
        })),
    )
    .await;

    Ok(Json(ExtractResponse {
        document_id: doc_id,
        status: "EXTRACTED".to_string(),
        run_id,
        model: model_name,
        entity_count,
        relationship_count: rel_count,
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
    }))
}

// ── Helpers ──────────────────────────────────────────────────────

/// Parse entities and relationships from the LLM JSON output and insert into DB.
/// Returns (entity_count, relationship_count).
async fn store_entities_and_relationships(
    state: &AppState,
    run_id: i32,
    doc_id: &str,
    parsed: &serde_json::Value,
) -> Result<(usize, usize), AppError> {
    // Insert entities, tracking json_id → db_item_id
    let entities = parsed["entities"].as_array();
    let mut id_map: HashMap<String, i32> = HashMap::new();
    let mut entity_count = 0usize;

    if let Some(entities) = entities {
        for entity in entities {
            let entity_type = entity["entity_type"].as_str().unwrap_or("unknown");
            let json_id = entity["id"].as_str().unwrap_or("");
            let verbatim = entity["verbatim_quote"].as_str();

            let db_id = pipeline_repository::insert_extraction_item(
                &state.pipeline_pool,
                run_id,
                doc_id,
                entity_type,
                entity,
                verbatim,
            )
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to insert entity '{json_id}': {e}"),
            })?;

            if !json_id.is_empty() {
                id_map.insert(json_id.to_string(), db_id);
            }
            entity_count += 1;
        }
    }

    // Insert relationships, mapping JSON entity IDs to database item IDs
    let relationships = parsed["relationships"].as_array();
    let mut rel_count = 0usize;

    if let Some(relationships) = relationships {
        for rel in relationships {
            let rel_type = rel["relationship_type"].as_str().unwrap_or("UNKNOWN");
            let from_id_str = rel["from_entity"].as_str().unwrap_or("");
            let to_id_str = rel["to_entity"].as_str().unwrap_or("");

            let from_db_id = match id_map.get(from_id_str) {
                Some(&id) => id,
                None => continue, // Skip if source entity wasn't found
            };
            let to_db_id = match id_map.get(to_id_str) {
                Some(&id) => id,
                None => continue, // Skip if target entity wasn't found
            };

            let props = rel.get("properties");

            pipeline_repository::insert_extraction_relationship(
                &state.pipeline_pool,
                run_id,
                doc_id,
                from_db_id,
                to_db_id,
                rel_type,
                props,
                1, // Tier 1 = Pass 1
            )
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to insert relationship: {e}"),
            })?;
            rel_count += 1;
        }
    }

    Ok((entity_count, rel_count))
}
