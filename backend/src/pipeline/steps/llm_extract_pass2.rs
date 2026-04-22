//! Pass-2 (relationship-only) extraction path.
//!
//! Pass 1 (see `llm_extract.rs`) produces entities. This module is the
//! manually-invokable second pass: it takes the COMPLETED pass-1 entities
//! as LLM input and extracts ONLY relationships, storing them under a new
//! `extraction_runs` row where `pass_number = 2`.
//!
//! The two-pass strategy follows industry best practice (KGGen, Microsoft
//! GraphRAG, CORE-KG): letting the LLM focus on one task at a time yields
//! dramatically better relationship quality than asking it to do both at
//! once.
//!
//! ## Why a free function instead of a `Step` impl?
//!
//! Task 3 owns wiring pass 2 into the pipeline FSM. Until that lands,
//! pass 2 is invoked out-of-band (e.g., an API handler or an admin CLI),
//! so exposing a `Step<DocProcessing>` would falsely advertise FSM
//! integration that doesn't exist yet. A free function returning the
//! relationship count keeps the surface honest.
//!
//! ## Rust Learning: sibling-module helper reuse with `pub(crate)`
//!
//! Helpers originally private to `llm_extract.rs` (`sha2_hex`,
//! `resolve_max_tokens`, `compute_cost`, `write_processing_config_snapshot`,
//! `default_profile_name_from_schema`) are exposed here via `pub(crate)`
//! so the pass-2 orchestrator can call them without duplicating logic.
//! `pub(crate)` keeps them invisible outside the backend crate — still
//! not part of the public API, just shared across sibling step modules.

use std::error::Error;

use sqlx::PgPool;

use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;

use crate::pipeline::config::{resolve_config, ProcessingProfile};
use crate::pipeline::context::AppContext;
use crate::pipeline::providers::provider_for_model;
use crate::pipeline::steps::llm_extract::{
    compute_cost, default_profile_name_from_schema, resolve_max_tokens, sha2_hex,
    write_processing_config_snapshot, LlmExtractError, CHUNKING_MODE_FULL,
};
use crate::pipeline::steps::llm_extract_helpers::{
    call_with_rate_limit_retry, mark_run_failed, parse_chunk_response,
};
use crate::repositories::pipeline_repository::{
    self, extraction, extraction::Pass1Entity, models,
};

// ── Public entry point ──────────────────────────────────────────

/// Run the pass-2 (relationship-only) extraction for a document.
///
/// Preconditions:
/// - A COMPLETED pass-1 extraction_run must exist for this document.
/// - The resolved profile must declare `pass2_template_file`.
/// - `chunking_mode` must be `"full"` — relationship extraction needs
///   whole-document context, so chunked dispatch is intentionally
///   refused rather than silently degraded.
///
/// Returns the number of relationships stored. Returns `0` (and does no
/// work) when a pass-2 run is already COMPLETED for this document — the
/// idempotency guard matches pass 1's design.
pub async fn run_pass2_extraction(
    document_id: &str,
    db: &PgPool,
    context: &AppContext,
    cancel: &CancellationToken,
    progress: &ProgressReporter,
) -> Result<usize, Box<dyn Error + Send + Sync>> {
    // 1. Idempotency: short-circuit on an existing COMPLETED pass-2 row.
    if pass2_already_complete(db, document_id).await? {
        tracing::info!(
            document_id, "Pass 2 already COMPLETED for document, skipping"
        );
        return Ok(0);
    }

    // 2. Load pipeline config, document, and schema.
    let pipe_config = pipeline_repository::get_pipeline_config(db, document_id)
        .await?
        .ok_or_else(|| LlmExtractError::NoPipelineConfig {
            document_id: document_id.to_string(),
        })?;

    let _doc = pipeline_repository::get_document(db, document_id)
        .await?
        .ok_or_else(|| LlmExtractError::DocumentNotFound {
            document_id: document_id.to_string(),
        })?;

    let schema_path = format!("{}/{}", context.schema_dir, pipe_config.schema_file);
    let schema = colossus_extract::ExtractionSchema::from_file(std::path::Path::new(
        &schema_path,
    ))
    .map_err(|e| LlmExtractError::SchemaLoadFailed {
        schema_file: pipe_config.schema_file.clone(),
        source: e,
    })?;
    let schema_json = serde_json::to_string_pretty(&schema)?;

    // 3. Full document text — pass 2 is always single-call.
    let pages = pipeline_repository::get_document_text(db, document_id).await?;
    if pages.is_empty() {
        return Err(LlmExtractError::NoTextPages {
            document_id: document_id.to_string(),
        }
        .into());
    }
    let full_text = pages
        .iter()
        .map(|p| format!("--- Page {} ---\n{}", p.page_number, p.text_content))
        .collect::<Vec<_>>()
        .join("\n\n");

    // 4. Resolve profile + overrides.
    let overrides =
        pipeline_repository::get_pipeline_config_overrides(db, document_id).await?;
    let profile_name = overrides
        .profile_name
        .clone()
        .unwrap_or_else(|| default_profile_name_from_schema(&pipe_config.schema_file));
    let profile = ProcessingProfile::load(&context.profile_dir, &profile_name)
        .map_err(|e| LlmExtractError::ProfileLoadFailed { message: e })?;
    let resolved = resolve_config(&profile, &overrides);

    // 5. Enforce pass-2 preconditions on the resolved config.
    let pass2_template_file = resolved.pass2_template_file.clone().ok_or_else(|| {
        LlmExtractError::NoPass2Template {
            profile_name: resolved.profile_name.clone(),
        }
    })?;
    if resolved.chunking_mode != CHUNKING_MODE_FULL {
        return Err(LlmExtractError::InvalidPass2ChunkingMode {
            mode: resolved.chunking_mode.clone(),
        }
        .into());
    }

    // 6. Load pass-1 entities. Empty ⇒ no COMPLETED pass-1 run exists.
    let entities = extraction::load_pass1_entities(db, document_id).await?;
    if entities.is_empty() {
        return Err(LlmExtractError::NoCompletedPass1 {
            document_id: document_id.to_string(),
        }
        .into());
    }

    // 7. Look up the model row and construct its provider.
    let model_record = models::get_active_model_by_id(db, &resolved.model)
        .await?
        .ok_or_else(|| LlmExtractError::ModelNotFound {
            model_id: resolved.model.clone(),
        })?;
    let llm_provider = provider_for_model(&model_record)
        .map_err(|message| LlmExtractError::ProviderConstructionFailed { message })?;

    // 8. Load pass-2 template + optional system prompt.
    let template_path = format!("{}/{}", context.template_dir, pass2_template_file);
    let template_text = std::fs::read_to_string(&template_path)
        .map_err(|e| format!("Failed to read pass-2 template '{template_path}': {e}"))?;
    let template_hash = sha2_hex(&template_text);

    let system_prompt: Option<String> = match &resolved.system_prompt_file {
        Some(filename) => {
            let path = format!("{}/{}", context.system_prompt_dir, filename);
            let text = std::fs::read_to_string(&path).map_err(|e| {
                LlmExtractError::ProfileLoadFailed {
                    message: format!("Failed to read system prompt '{path}': {e}"),
                }
            })?;
            Some(text)
        }
        None => None,
    };
    let system_prompt_hash: Option<String> = system_prompt.as_deref().map(sha2_hex);

    // 9. Render entities for the prompt and build the LLM-id → item_id map.
    let entities_prompt: Vec<serde_json::Value> =
        entities.iter().map(Pass1Entity::to_prompt_value).collect();
    let entities_json = serde_json::to_string_pretty(&entities_prompt)?;
    let id_map: std::collections::HashMap<String, i32> = entities
        .iter()
        .filter(|e| !e.id.is_empty())
        .map(|e| (e.id.clone(), e.item_id))
        .collect();

    // 10. Placeholder substitution. We do NOT fill {{global_rules}} /
    //     {{admin_instructions}} / {{context}} — pass 1's current path
    //     leaves them unfilled too, and this task is scoped to mirror
    //     pass 1's substitution behavior.
    let prompt = template_text
        .replace("{{schema_json}}", &schema_json)
        .replace("{{entities_json}}", &entities_json)
        .replace("{{document_text}}", &full_text);

    let max_tokens = resolve_max_tokens(&resolved);

    // 11. Insert the pass-2 extraction_runs row (pass_number = 2). The
    //     upsert in insert_extraction_run keys on (document_id,
    //     pass_number), so a prior FAILED pass-2 attempt gets reused.
    //     reset_extraction_run_children then wipes children of just this
    //     run_id — pass-1's children on the separate pass-1 run are
    //     untouched.
    // The assembled prompt is passed in directly (pass 1 has to UPDATE
    // it afterward only because it builds per-chunk prompts post-insert).
    let run_id = extraction::insert_extraction_run(
        db,
        document_id,
        2,
        &resolved.model,
        &schema.version,
        Some(&prompt),
        Some(pass2_template_file.as_str()),
        Some(&template_hash),
        None,
        None,
        None,
        Some(&serde_json::to_value(&schema)?),
        Some(resolved.temperature),
        Some(max_tokens as i32),
        pipe_config.admin_instructions.as_deref(),
        None,
    )
    .await
    .map_err(|e| LlmExtractError::InsertRunFailed {
        message: format!("{e}"),
    })?;

    extraction::reset_extraction_run_children(db, run_id)
        .await
        .map_err(|e| LlmExtractError::InsertRunFailed {
            message: format!("reset_extraction_run_children: {e}"),
        })?;

    // 12. Cancel check before acquiring the semaphore.
    if cancel.is_cancelled().await {
        mark_run_failed(db, run_id, "Cancelled before pass 2 extraction").await;
        return Err("Cancelled before pass 2 extraction".into());
    }

    let _llm_permit = context
        .llm_semaphore
        .acquire()
        .await
        .map_err(|_| LlmExtractError::SemaphoreClosed)?;

    // 13. LLM call with rate-limit retry.
    progress
        .report(serde_json::json!({ "status": "extracting", "mode": "pass2_full" }))
        .await
        .ok();

    let response = call_with_rate_limit_retry(
        &*llm_provider,
        system_prompt.as_deref(),
        &prompt,
        max_tokens,
        cancel,
        progress,
        0,
        1,
    )
    .await
    .map_err(|e| LlmExtractError::LlmCallFailed { source: e })?;

    // 14. Parse + store. Pass 2 output is relationships-only; absent
    //     `entities` is fine, absent `relationships` yields a 0-count
    //     run that still COMPLETEs (so the idempotency guard triggers
    //     on future calls).
    let parsed = match parse_chunk_response(&response.text) {
        Ok(v) => v,
        Err(e) => {
            mark_run_failed(db, run_id, &format!("Pass 2 parse failed: {e}")).await;
            return Err(format!("Pass 2 parse failed: {e}").into());
        }
    };

    let rel_count =
        extraction::store_pass2_relationships(db, run_id, document_id, &parsed, &id_map)
            .await
            .map_err(|e| LlmExtractError::StoreFailed {
                message: format!("{e}"),
            })?;

    // 15. Finalize the run.
    let input_tokens = response.input_tokens.unwrap_or(0) as i64;
    let output_tokens = response.output_tokens.unwrap_or(0) as i64;
    let cost_usd = compute_cost(&model_record, input_tokens, output_tokens);

    extraction::complete_extraction_run(
        db,
        run_id,
        &parsed,
        Some(input_tokens as i32),
        Some(output_tokens as i32),
        cost_usd,
        "COMPLETED",
    )
    .await
    .map_err(|e| LlmExtractError::CompleteRunFailed {
        message: format!("{e}"),
    })?;

    // 16. Processing-config snapshot (best-effort).
    write_processing_config_snapshot(
        db,
        run_id,
        &resolved,
        &template_hash,
        system_prompt_hash.as_deref(),
    )
    .await;

    progress
        .set_step_result(serde_json::json!({
            "pass": 2,
            "relationship_count": rel_count,
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "profile": resolved.profile_name,
            "model": resolved.model,
            "pass2_template_file": pass2_template_file,
        }));

    tracing::info!(
        document_id,
        relationships = rel_count,
        input_tokens,
        output_tokens,
        profile = %resolved.profile_name,
        "Pass 2 extraction complete"
    );

    Ok(rel_count)
}

// ── Helpers ─────────────────────────────────────────────────────

/// Has a COMPLETED `pass_number = 2` extraction_run landed for this document?
///
/// Scoped to pass 2 explicitly — pass 1's helper
/// (`extraction_already_complete` in `llm_extract.rs`) matches any pass,
/// which would false-positive here if pass 1 had completed but pass 2
/// hadn't.
async fn pass2_already_complete(
    db: &PgPool,
    document_id: &str,
) -> Result<bool, sqlx::Error> {
    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM extraction_runs \
         WHERE document_id = $1 AND pass_number = 2 AND status = 'COMPLETED' \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(document_id)
    .fetch_optional(db)
    .await?;
    Ok(existing.is_some())
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass2_error_display_no_caused_by_chain() {
        // G6: Display must stay a single line, never leak "Caused by".
        let e = LlmExtractError::NoPass2Template {
            profile_name: "complaint".into(),
        };
        let s = e.to_string();
        assert!(s.contains("complaint"), "should name the profile: {s}");
        assert!(!s.contains("Caused by"), "G6 violation: {s}");
    }

    #[test]
    fn invalid_pass2_chunking_mode_names_the_bad_mode() {
        let e = LlmExtractError::InvalidPass2ChunkingMode {
            mode: "chunked".into(),
        };
        let s = e.to_string();
        assert!(s.contains("chunked"), "should name the offending mode: {s}");
        assert!(s.contains("full"), "should name the required mode: {s}");
    }

    #[test]
    fn no_completed_pass1_display_names_document() {
        let e = LlmExtractError::NoCompletedPass1 {
            document_id: "doc-abc".into(),
        };
        assert!(e.to_string().contains("doc-abc"));
    }
}
