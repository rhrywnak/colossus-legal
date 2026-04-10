//! POST /api/admin/pipeline/documents/:id/extract — LLM extraction (Pass 1).

use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use axum::{extract::Path as AxumPath, extract::State, Json};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

use super::anthropic::call_anthropic;
use super::constants;
use super::ExtractResponse;

/// Optional request body for extraction overrides.
///
/// If no body is sent, all values come from `pipeline_config`.
/// If a body is sent, non-null fields override the pipeline defaults.
#[derive(Debug, serde::Deserialize, Default)]
pub struct ExtractRequest {
    /// Override the schema file (e.g. "complaint_v2.yaml")
    #[serde(default)]
    pub schema_file: Option<String>,
    /// Override the LLM model (e.g. "claude-sonnet-4-6")
    #[serde(default)]
    pub model: Option<String>,
    /// Override max tokens
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Override custom instructions
    #[serde(default)]
    pub admin_instructions: Option<String>,
    /// Override temperature
    #[serde(default)]
    pub temperature: Option<f64>,
}

/// Loads extraction schema, builds prompt, calls Claude, parses JSON response,
/// stores entities + relationships in the pipeline database.
pub async fn extract_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
    body: Option<Json<ExtractRequest>>,
) -> Result<Json<ExtractResponse>, AppError> {
    require_admin(&user)?;
    let step_start = Instant::now();
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST extract");

    let step_id = steps::record_step_start(
        &state.pipeline_pool, &doc_id, "extract", &user.username, &serde_json::json!({}),
    ).await.map_err(|e| AppError::Internal { message: format!("Step logging: {e}") })?;

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

    // 5. Apply request body overrides (if any) on top of pipeline config defaults.
    let overrides = body.map(|b| b.0).unwrap_or_default();
    let has_overrides = overrides.schema_file.is_some()
        || overrides.model.is_some()
        || overrides.max_tokens.is_some()
        || overrides.admin_instructions.is_some();
    let schema_file_owned = overrides.schema_file
        .unwrap_or_else(|| pipe_config.schema_file.clone());
    let schema_file = schema_file_owned.as_str();
    let model_name = overrides.model
        .unwrap_or_else(|| pipe_config.pass1_model.clone());
    let max_tokens = overrides.max_tokens
        .unwrap_or(pipe_config.pass1_max_tokens as u32);
    let admin_instructions_owned = overrides.admin_instructions
        .or_else(|| pipe_config.admin_instructions.clone());
    let admin_instructions = admin_instructions_owned.as_deref();
    let temperature = overrides.temperature;

    // 6. Load extraction schema.
    //    Entity type names in the schema become Neo4j labels directly.
    let schema_path = format!("{}/{}", state.config.extraction_schema_dir, schema_file);
    tracing::info!(
        doc_id = %doc_id, schema = %schema_file, document_type = %document.document_type,
        "Using schema '{}' for document {} (type: {})",
        schema_file, doc_id, document.document_type
    );
    let schema = colossus_extract::ExtractionSchema::from_file(Path::new(&schema_path))
        .map_err(|e| AppError::Internal {
            message: format!("Failed to load schema '{}': {e}", schema_file),
        })?;

    // 7. Build prompt.
    //    Select prompt template based on document type. Document-type-specific
    //    templates are named pass1_{document_type}.md. Falls back to
    //    pass1_template.md (the generic template) if not found.
    let specific_template = format!("pass1_{}.md", schema.document_type);
    let template_path = Path::new(&state.config.extraction_template_dir).join(&specific_template);
    let template_name = if template_path.exists() {
        tracing::info!(doc_id = %doc_id, template = %specific_template, "Using document-type-specific prompt template");
        Some(specific_template.as_str())
    } else {
        tracing::info!(doc_id = %doc_id, "No specific template for '{}', using default pass1_template.md", schema.document_type);
        None // PromptBuilder defaults to pass1_template.md
    };

    let mut builder = colossus_extract::PromptBuilder::new(
        Path::new(&state.config.extraction_template_dir),
    );
    let artifact = builder
        .build_extraction_prompt(&schema, &full_text, None, admin_instructions, Some("global_rules.md"), template_name)
        .map_err(|e| AppError::Internal { message: format!("Failed to build prompt: {e}") })?;
    let prompt = &artifact.prompt_text;

    // 8. Insert extraction run (status = RUNNING) with reproducibility metadata
    let schema_json_value = serde_json::to_value(&schema).ok();
    let run_id = pipeline_repository::insert_extraction_run(
        &state.pipeline_pool,
        &doc_id,
        1,
        &model_name,
        &schema.document_type,
        Some(&artifact.prompt_text),
        Some(&artifact.template_name),
        Some(&artifact.template_hash),
        artifact.rules_name.as_deref(),
        artifact.rules_hash.as_deref(),
        Some(&artifact.schema_hash),
        schema_json_value.as_ref(),
        temperature,
        Some(max_tokens as i32),
        admin_instructions,
        None, // prior_context — not implemented yet (F7)
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
    let api_start = Instant::now();
    let api_result = call_anthropic(api_key, &model_name, max_tokens, prompt).await;
    let elapsed_secs = api_start.elapsed().as_secs_f64();

    let (response_text, input_tokens, output_tokens) = match api_result {
        Ok(r) => (r.text, r.input_tokens, r.output_tokens),
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

    // 9. Parse JSON from LLM response
    let parsed: serde_json::Value = match serde_json::from_str(&response_text) {
        Ok(v) => v,
        Err(parse_err) => {
            let _ = pipeline_repository::complete_extraction_run(
                &state.pipeline_pool, run_id,
                &serde_json::json!({ "raw_text": response_text }),
                Some(input_tokens as i32), Some(output_tokens as i32),
                Some(constants::estimate_cost(input_tokens as i64, output_tokens as i64)),
                "FAILED",
            ).await;
            steps::record_step_failure(
                &state.pipeline_pool, step_id, step_start.elapsed().as_secs_f64(),
                &format!("LLM returned invalid JSON: {parse_err}"),
            ).await.ok();
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
        Some(input_tokens as i32),
        Some(output_tokens as i32),
        Some(constants::estimate_cost(input_tokens as i64, output_tokens as i64)),
        "COMPLETED",
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to complete extraction run: {e}"),
    })?;

    // 10b. Run completeness validation against schema rules
    let completeness = super::completeness_validation::validate_completeness(&schema, &parsed);

    // Log entity counts
    for (entity_type, count) in &completeness.entity_counts {
        tracing::info!(doc_id = %doc_id, entity_type = %entity_type, count, "Extracted entity count");
    }

    // Log warnings
    for warning in &completeness.warnings {
        tracing::warn!(doc_id = %doc_id, "Completeness warning: {}", warning);
    }

    // If validation failed, update status and return error
    if !completeness.passed {
        pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, "EXTRACTION_FAILED")
            .await
            .map_err(|e| AppError::Internal { message: format!("Failed to update status: {e}") })?;

        let error_detail = serde_json::json!({
            "errors": completeness.errors,
            "warnings": completeness.warnings,
            "entity_counts": completeness.entity_counts,
        });

        steps::record_step_failure(
            &state.pipeline_pool, step_id, step_start.elapsed().as_secs_f64(),
            &format!("Completeness validation failed: {:?}", completeness.errors),
        ).await.ok();

        return Err(AppError::BadRequest {
            message: format!("Extraction completeness validation failed: {}", completeness.errors.join("; ")),
            details: error_detail,
        });
    }

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
        input_tokens = input_tokens, output_tokens = output_tokens,
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
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
        })),
    )
    .await;

    steps::record_step_complete(
        &state.pipeline_pool, step_id, step_start.elapsed().as_secs_f64(),
        &serde_json::json!({"entity_count": entity_count, "relationship_count": rel_count,
            "input_tokens": input_tokens, "output_tokens": output_tokens}),
    ).await.ok();

    // 15. If overrides were provided, persist them to pipeline_config for next run
    if has_overrides {
        pipeline_repository::update_pipeline_config(
            &state.pipeline_pool,
            &doc_id,
            &model_name,
            max_tokens as i32,
            schema_file,
            admin_instructions,
        )
        .await
        .ok(); // Best effort — don't fail extraction if config update fails
    }

    Ok(Json(ExtractResponse {
        document_id: doc_id,
        status: "EXTRACTED".to_string(),
        run_id,
        model: model_name,
        entity_count,
        relationship_count: rel_count,
        input_tokens,
        output_tokens,
        elapsed_secs,
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
            let verbatim = entity["verbatim_quote"].as_str()
                    .or_else(|| entity["properties"]["verbatim_quote"].as_str());

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
