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
//! ## Free-function orchestrator + thin `Step` adapter
//!
//! [`run_pass2_extraction`] is the free-function orchestrator — it's what
//! an API handler or admin CLI calls when triggering pass 2 directly. The
//! [`LlmExtractPass2`] struct is the FSM-facing adapter: its [`Step`]
//! impl delegates to the same orchestrator and returns the FSM edge to
//! Verify, so the Worker can advance the job transparently. Keeping the
//! two layers separate means out-of-band callers don't carry FSM
//! artifacts, and the Step impl doesn't carry FSM-unaware signature
//! cruft.
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
use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::models::document_status::{RUN_STATUS_COMPLETED, RUN_STATUS_FAILED, RUN_STATUS_RUNNING};
use crate::pipeline::config::{resolve_config, CrossDocContextRecord, ProcessingProfile};
use crate::pipeline::context::AppContext;
use crate::pipeline::providers::provider_for_model;
use crate::pipeline::steps::llm_extract::{
    compute_cost, default_profile_name_from_schema, load_global_rules, resolve_effective_mode,
    resolve_max_tokens, sha2_hex, strip_authoring_comments, write_processing_config_snapshot,
    LlmExtractError, SnapshotRuntimeFields,
};
use crate::pipeline::steps::llm_extract_helpers::{
    call_with_rate_limit_retry, mark_run_failed, parse_chunk_response,
};
use crate::pipeline::steps::verify::Verify;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository::{
    self, extraction,
    extraction::{CrossDocEntity, Pass1Entity},
    models,
};

// ── FSM step adapter ────────────────────────────────────────────

/// The LlmExtractPass2 step variant's payload.
///
/// Carries only the document id; pass-2 resolves everything else
/// (profile, template, model, pass-1 entities) from storage at execute
/// time via [`run_pass2_extraction`]. The Worker reaches this step when
/// pass 1 routes to it via `llm_extract::next_step_after_pass1` — i.e.
/// whenever `resolved.run_pass2 == true`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmExtractPass2 {
    pub document_id: String,
}

#[async_trait]
impl Step<DocProcessing> for LlmExtractPass2 {
    async fn execute(
        self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        run_pass2_extraction(&self.document_id, db, context, cancel, progress).await?;
        Ok(StepResult::Next(DocProcessing::Verify(Verify {
            document_id: self.document_id,
        })))
    }

    async fn on_cancel(
        self,
        db: &PgPool,
        _context: &AppContext,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Scope the cleanup to pass-2 rows only so a mid-pass-2 cancel
        // never clobbers the COMPLETED pass-1 run (which is pass 2's
        // input). A pass-2 COMPLETED row is left intact — its
        // relationships are the authoritative output and the orchestrator's
        // idempotency check will short-circuit future retries.
        if let Err(e) = sqlx::query(
            "DELETE FROM extraction_runs \
             WHERE document_id = $1 AND pass_number = 2 \
               AND status IN ($2, $3)",
        )
        .bind(&self.document_id)
        .bind(RUN_STATUS_RUNNING)
        .bind(RUN_STATUS_FAILED)
        .execute(db)
        .await
        {
            tracing::warn!(
                doc_id = %self.document_id, error = %e,
                "LlmExtractPass2::on_cancel: delete of RUNNING/FAILED pass-2 runs failed (non-fatal)"
            );
        }
        Ok(())
    }
}

// ── Public entry point ──────────────────────────────────────────

/// Run the pass-2 (relationship-only) extraction for a document.
///
/// Preconditions:
/// - A COMPLETED pass-1 extraction_run must exist for this document.
/// - The resolved profile must declare `pass2_template_file`.
///
/// Pass 2 is mode-agnostic: it always loads the full document text from
/// `document_text` (stored by ExtractText, before any chunking) and the
/// merged pass-1 entities, then makes a single LLM call. Pass 1's
/// chunking mode (`full` / `chunked` / `structured`) does not affect it.
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
    // Pass 2 always operates on the full document text loaded above,
    // independent of how pass 1 chunked. Log the effective pass-1 mode so
    // operators can see which pass-1 path produced the entities feeding
    // this pass-2 run.
    let effective_mode = resolve_effective_mode(&resolved);
    tracing::info!(
        document_id,
        chunking_mode = %resolved.chunking_mode,
        effective_mode = %effective_mode,
        "Pass 2 running with full document text (Pass 1 used '{}' mode)",
        effective_mode
    );

    // 6. Load pass-1 entities. Empty ⇒ no COMPLETED pass-1 run exists.
    let entities = extraction::load_pass1_entities(db, document_id).await?;
    if entities.is_empty() {
        return Err(LlmExtractError::NoCompletedPass1 {
            document_id: document_id.to_string(),
        }
        .into());
    }

    // 7. Look up the model row and construct its provider. Pass 2 uses
    //    `resolved.pass2_model` when set (operator can pick a stronger
    //    relationship-reasoning model), otherwise falls back to the
    //    pass-1 `model` for backward compatibility.
    let pass2_model_id = resolved
        .pass2_model
        .clone()
        .unwrap_or_else(|| resolved.model.clone());
    tracing::info!(
        document_id,
        pass2_model = %pass2_model_id,
        pass1_model = %resolved.model,
        using_pass2_override = resolved.pass2_model.is_some(),
        "Pass 2: resolved model"
    );
    let model_record = models::get_active_model_by_id(db, &pass2_model_id)
        .await?
        .ok_or_else(|| LlmExtractError::ModelNotFound {
            model_id: pass2_model_id.clone(),
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

    // 8b. Load the global-rules fragment (if the profile names one) and
    //     compute its SHA-256. The Pass-2 prompt substitutes the content
    //     at `{{global_rules}}`, mirroring Pass-1. The hash lands in
    //     `extraction_runs.rules_hash` and `processing_config` JSONB so
    //     two Pass-2 runs against different rules versions are
    //     distinguishable from the database alone (Gap 5 in
    //     AUDIT_PIPELINE_CONFIG_GAPS.md, fixed by this commit).
    let (global_rules_text, global_rules_hash) = load_global_rules(
        Path::new(&context.template_dir),
        resolved.global_rules_file.as_deref(),
    )?;

    // 9a. Load cross-document context from previously PUBLISHED docs.
    //     ComplaintAllegation / LegalCount / Party entities surface here
    //     so the pass-2 LLM can author CORROBORATES, CONTRADICTS, and
    //     other cross-document relationship types against a discovery
    //     response's admissions/denials, a court ruling's holdings, etc.
    //     Ids are prefixed (`ctx:...`) to prevent collisions with the
    //     current doc's local pass-1 ids.
    let cross_doc_entities =
        extraction::load_cross_document_context(db, document_id).await?;
    tracing::info!(
        document_id,
        local_entities = entities.len(),
        cross_doc_entities = cross_doc_entities.len(),
        "Pass 2: loaded entities for prompt"
    );

    // 9a-bis. Reproducibility record of the cross-document entities that
    //     will be inlined into the Pass-2 prompt. Built BEFORE the prompt
    //     assembly so any future filtering between load and prompt-build
    //     would force this code to be moved alongside it (the audit log
    //     must reflect what actually went into the prompt — Gap 3 in
    //     AUDIT_PIPELINE_CONFIG_GAPS.md). Today there is no filtering, so
    //     `cross_doc_entities` IS the prompt input set.
    //
    // Two parallel structures:
    //   - `cross_doc_records`: full triples written into JSONB and into
    //     the `prior_context` TEXT column (full reproducibility).
    //   - `pass2_source_document_ids`: sorted unique list of contributing
    //     document_ids (cheap "which prior runs informed this Pass-2"
    //     queries without parsing the full triple list).
    let cross_doc_records: Vec<CrossDocContextRecord> = cross_doc_entities
        .iter()
        .map(|c| CrossDocContextRecord {
            document_id: c.source_document_id.clone(),
            prefixed_id: c.prefixed_id.clone(),
            item_id: c.item_id,
        })
        .collect();
    let mut pass2_source_document_ids: Vec<String> = cross_doc_entities
        .iter()
        .map(|c| c.source_document_id.clone())
        .collect();
    pass2_source_document_ids.sort();
    pass2_source_document_ids.dedup();

    // Compact JSON encoding for the `extraction_runs.prior_context` TEXT
    // column. `serde_json::to_string` (not `to_string_pretty`) — saves
    // bytes on large cross-doc sets and the column is opaque to humans
    // anyway (the JSONB sub-field is the queryable copy).
    // Empty cross-doc set → `None` so the column stays NULL rather than
    // storing the literal string `"[]"` (a NULL is the unambiguous
    // "nothing to record" signal in the audit log).
    let prior_context_json: Option<String> = if cross_doc_records.is_empty() {
        None
    } else {
        match serde_json::to_string(&cross_doc_records) {
            Ok(s) => Some(s),
            Err(e) => {
                // Serialisation failure on a Vec<CrossDocContextRecord>
                // would mean a serde-derive bug — none of the fields can
                // produce a non-finite float or a non-string map key.
                // Log + degrade to NULL rather than abort; the snapshot
                // JSONB still carries the structured copy if it succeeds.
                tracing::warn!(
                    document_id,
                    error = %e,
                    "Failed to serialize prior_context (non-fatal — JSONB sub-field still written)"
                );
                None
            }
        }
    };

    // 9b. Render entities for the prompt and build the LLM-id → item_id map.
    //     Local entities come first so the LLM's attention order favors
    //     this document's own entities; cross-doc entities follow with a
    //     `source_document` field making their provenance explicit.
    let mut entities_prompt: Vec<serde_json::Value> = Vec::with_capacity(
        entities.len() + cross_doc_entities.len(),
    );
    entities_prompt.extend(entities.iter().map(Pass1Entity::to_prompt_value));
    entities_prompt.extend(cross_doc_entities.iter().map(CrossDocEntity::to_prompt_value));
    let entities_json = serde_json::to_string_pretty(&entities_prompt)?;

    let mut id_map: std::collections::HashMap<String, i32> = entities
        .iter()
        .filter(|e| !e.id.is_empty())
        .map(|e| (e.id.clone(), e.item_id))
        .collect();
    for c in &cross_doc_entities {
        id_map.insert(c.prefixed_id.clone(), c.item_id);
    }

    // 10. Placeholder substitution via the shared `assemble_pass2_prompt`
    //     helper. Substitutes `{{global_rules}}` and
    //     `{{admin_instructions}}` (previously left as literal placeholders
    //     in every pass-2 prompt — Gap 6 in AUDIT_PIPELINE_CONFIG_GAPS.md).
    //     `{{context}}` is still substituted with the empty string here;
    //     pass-2 inlines cross-doc entities into `{{entities_json}}`
    //     above, and a future renderer for `pipe_config.prior_context_doc_ids`
    //     would populate this without changing the substitution code.
    let global_rules_ref: Option<&str> = global_rules_hash
        .as_ref()
        .map(|_| global_rules_text.as_str());
    let prompt = assemble_pass2_prompt(
        &template_text,
        &schema_json,
        &entities_json,
        &full_text,
        global_rules_ref,
        pipe_config.admin_instructions.as_deref(),
        None,
    );

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
        // Record the model actually used for pass 2, not the pass-1
        // model — otherwise the audit log disagrees with the Anthropic /
        // vLLM request that produced this run's output.
        &pass2_model_id,
        &schema.version,
        Some(&prompt),
        Some(pass2_template_file.as_str()),
        Some(&template_hash),
        // F3 audit: rules_name + rules_hash. Previously NULL; populated
        // here so a Pass-2 run is reproducible from the DB alone (Gap 5
        // in AUDIT_PIPELINE_CONFIG_GAPS.md).
        resolved.global_rules_file.as_deref(),
        global_rules_hash.as_deref(),
        None,
        Some(&serde_json::to_value(&schema)?),
        Some(resolved.temperature),
        Some(max_tokens as i32),
        pipe_config.admin_instructions.as_deref(),
        // F3 audit: prior_context is the JSON-encoded list of cross-doc
        // entities actually injected into the Pass-2 prompt. Previously
        // always NULL. AUDIT_PIPELINE_CONFIG_GAPS.md Gap 3.
        prior_context_json.as_deref(),
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
        .map_err(|e| {
            tracing::debug!(error = %e, "Semaphore acquire failed");
            LlmExtractError::SemaphoreClosed
        })?;

    // 13. LLM call with rate-limit retry.
    // best-effort progress update
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
        RUN_STATUS_COMPLETED,
    )
    .await
    .map_err(|e| LlmExtractError::CompleteRunFailed {
        message: format!("{e}"),
    })?;

    // 16. Processing-config snapshot (best-effort).
    // Pass-2 snapshot: effective_pass = 2 triggers the snapshot helper
    // to overwrite `model` / `template_file` with their Pass-2 values
    // (Gap 11 in AUDIT_PIPELINE_CONFIG_GAPS.md). Cross-doc records and
    // the source-doc list are captured here too — same data also lives
    // in `extraction_runs.prior_context` as a TEXT JSON for the bytes-
    // exact reproducibility column.
    write_processing_config_snapshot(
        db,
        run_id,
        &resolved,
        SnapshotRuntimeFields {
            effective_pass: 2,
            template_hash: &template_hash,
            system_prompt_hash: system_prompt_hash.as_deref(),
            global_rules_hash: global_rules_hash.as_deref(),
            pass2_cross_doc_entities: &cross_doc_records,
            pass2_source_document_ids: &pass2_source_document_ids,
        },
    )
    .await;

    progress
        .set_step_result(serde_json::json!({
            "pass": 2,
            "relationship_count": rel_count,
            "local_entities": entities.len(),
            "cross_doc_entities": cross_doc_entities.len(),
            "input_tokens": input_tokens,
            "output_tokens": output_tokens,
            "profile": resolved.profile_name,
            // Report the pass-2-specific model so the UI reflects what
            // actually ran (may differ from pass-1's `resolved.model`).
            "model": pass2_model_id,
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

/// Substitute every placeholder a Pass-2 template references and return
/// the assembled prompt.
///
/// Pass-2 templates (the `pass2_*_v4.md` files) reference six
/// placeholders. Five of them are required for any meaningful run; the
/// sixth (`{{admin_instructions}}`) only appears in
/// `pass2_discovery_response_v4.md` today but is substituted
/// unconditionally so a future template can opt in without a code
/// change. The substitution order matches Pass-1's `assemble_chunk_prompt`:
/// schema first so any rule fragment that references it picks it up;
/// body placeholders last so accidental placeholder syntax inside user
/// content (rules / instructions / context / entity labels) cannot get
/// re-substituted.
///
/// Substituted placeholders (in order):
///
/// 1. `{{schema_json}}` — always
/// 2. `{{entities_json}}` — always (the merged Pass-1 + cross-doc list)
/// 3. `{{global_rules}}` — empty string when `None` so the literal
///    placeholder doesn't leak as text. Mirrors Pass-1 (Gap 6 in
///    AUDIT_PIPELINE_CONFIG_GAPS.md, fixed by this commit).
/// 4. `{{admin_instructions}}` — empty string when `None`. Same
///    rationale.
/// 5. `{{context}}` — empty string when `None`. Pass-2 does not yet
///    render cross-document prior context here (the cross-doc entities
///    are inlined into `entities_json` instead); the placeholder is
///    substituted defensively.
/// 6. `{{document_text}}` — always; the full document body.
///
/// ## Rust Learning: `Option::unwrap_or("")` vs. branching
///
/// Each `Option<&str>` argument flattens through `.unwrap_or("")` so
/// `None` and `Some("")` collapse to the same substitution. That is the
/// audited behaviour: both mean "no rules / no instructions / no context
/// to inject" from the LLM's perspective, and the placeholder must
/// vanish either way (otherwise the literal `{{...}}` leaks into the
/// prompt). Pass-1's `assemble_chunk_prompt` uses the same idiom.
fn assemble_pass2_prompt(
    template_text: &str,
    schema_json: &str,
    entities_json: &str,
    document_text: &str,
    global_rules: Option<&str>,
    admin_instructions: Option<&str>,
    context: Option<&str>,
) -> String {
    // Strip AUTHORING_NOTE comment blocks BEFORE the substitution
    // chain. Mirrors `assemble_chunk_prompt` (Pass-1) so the contract
    // is consistent across passes — see [`strip_authoring_comments`]
    // for the marker convention and the "before substitution" rule.
    let stripped = strip_authoring_comments(template_text);
    stripped
        .replace("{{schema_json}}", schema_json)
        .replace("{{entities_json}}", entities_json)
        .replace("{{global_rules}}", global_rules.unwrap_or(""))
        .replace("{{admin_instructions}}", admin_instructions.unwrap_or(""))
        .replace("{{context}}", context.unwrap_or(""))
        .replace("{{document_text}}", document_text)
}

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
         WHERE document_id = $1 AND pass_number = 2 AND status = $2 \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(document_id)
    .bind(RUN_STATUS_COMPLETED)
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
    fn no_completed_pass1_display_names_document() {
        let e = LlmExtractError::NoCompletedPass1 {
            document_id: "doc-abc".into(),
        };
        assert!(e.to_string().contains("doc-abc"));
    }

    #[test]
    fn llm_extract_pass2_struct_round_trips_through_serde() {
        // pipeline_jobs.current_step_payload round-trips the step struct
        // through JSON — a missing derive would surface here first.
        let a = LlmExtractPass2 {
            document_id: "doc-rt".into(),
        };
        let j = serde_json::to_string(&a).unwrap();
        let b: LlmExtractPass2 = serde_json::from_str(&j).unwrap();
        assert_eq!(a.document_id, b.document_id);
    }

    #[test]
    fn llm_extract_pass2_step_uses_trait_default_retry_settings() {
        // Pass 2 matches pass 1's retry policy: zero Worker-level retries.
        // The LLM call's internal rate-limit retry (MAX_RETRIES_PER_CHUNK)
        // handles the transient-failure class we care about.
        assert_eq!(
            <LlmExtractPass2 as Step<DocProcessing>>::DEFAULT_RETRY_LIMIT,
            0
        );
        assert_eq!(
            <LlmExtractPass2 as Step<DocProcessing>>::DEFAULT_RETRY_DELAY_SECS,
            0
        );
        assert!(<LlmExtractPass2 as Step<DocProcessing>>::DEFAULT_TIMEOUT_SECS.is_none());
    }

    // ── assemble_pass2_prompt ────────────────────────────────────
    //
    // These tests pin down the placeholder contract introduced by
    // AUDIT_PIPELINE_CONFIG_GAPS.md Gaps 5/6: Pass-2 templates that
    // reference `{{global_rules}}` and `{{admin_instructions}}` must
    // get those placeholders substituted, not leaked as literal text.
    // Every shipped pass2_*_v4.md template references {{global_rules}};
    // pass2_discovery_response_v4.md also references {{admin_instructions}}.

    #[test]
    fn assemble_pass2_prompt_substitutes_global_rules_when_some() {
        let template = "Rules:\n{{global_rules}}\n\nDoc: {{document_text}}";
        let prompt = assemble_pass2_prompt(
            template,
            "<SCHEMA>",
            "<ENTITIES>",
            "<DOC BODY>",
            Some("RULE-A; RULE-B"),
            None,
            None,
        );
        assert!(
            prompt.contains("RULE-A; RULE-B"),
            "global_rules content must land in the prompt; got: {prompt}"
        );
        assert!(
            !prompt.contains("{{global_rules}}"),
            "literal placeholder must be gone; got: {prompt}"
        );
    }

    #[test]
    fn assemble_pass2_prompt_substitutes_global_rules_with_empty_when_none() {
        // Template references the placeholder but the profile didn't
        // configure rules — substitution must collapse to empty string,
        // not leak the literal token to the LLM.
        let template = "Rules:\n{{global_rules}}\n\nDoc: {{document_text}}";
        let prompt = assemble_pass2_prompt(
            template,
            "<SCHEMA>",
            "<ENTITIES>",
            "<DOC>",
            None,
            None,
            None,
        );
        assert!(
            !prompt.contains("{{global_rules}}"),
            "absent global_rules must still strip the placeholder; got: {prompt}"
        );
    }

    #[test]
    fn assemble_pass2_prompt_substitutes_admin_instructions_when_some() {
        let template = "Admin: {{admin_instructions}}\n\nDoc: {{document_text}}";
        let prompt = assemble_pass2_prompt(
            template,
            "<SCHEMA>",
            "<ENTITIES>",
            "<DOC>",
            None,
            Some("Focus on dates"),
            None,
        );
        assert!(
            prompt.contains("Focus on dates"),
            "admin_instructions content must land in the prompt; got: {prompt}"
        );
        assert!(!prompt.contains("{{admin_instructions}}"));
    }

    #[test]
    fn assemble_pass2_prompt_substitutes_admin_instructions_with_empty_when_none() {
        let template = "Admin: {{admin_instructions}}\n\nDoc: {{document_text}}";
        let prompt = assemble_pass2_prompt(
            template,
            "<SCHEMA>",
            "<ENTITIES>",
            "<DOC>",
            None,
            None,
            None,
        );
        assert!(
            !prompt.contains("{{admin_instructions}}"),
            "absent admin_instructions must still strip the placeholder; got: {prompt}"
        );
    }

    #[test]
    fn assemble_pass2_prompt_substitutes_every_placeholder_alongside_body() {
        // Integration-style: schema + entities + global_rules +
        // admin_instructions + context + body, all substituted in one call.
        // Mirrors `assemble_chunk_prompt`'s end-to-end test in pass-1.
        let template = "\
Schema:\n{{schema_json}}\n\n\
Entities:\n{{entities_json}}\n\n\
Rules:\n{{global_rules}}\n\n\
Admin:\n{{admin_instructions}}\n\n\
Context:\n{{context}}\n\n\
Doc:\n{{document_text}}";
        let prompt = assemble_pass2_prompt(
            template,
            "<SCHEMA>",
            "<ENTITIES>",
            "<DOC>",
            Some("<RULES>"),
            Some("<ADMIN>"),
            Some("<CTX>"),
        );
        for needle in [
            "<SCHEMA>", "<ENTITIES>", "<RULES>", "<ADMIN>", "<CTX>", "<DOC>",
        ] {
            assert!(prompt.contains(needle), "missing {needle}; got: {prompt}");
        }
        for placeholder in [
            "{{schema_json}}",
            "{{entities_json}}",
            "{{global_rules}}",
            "{{admin_instructions}}",
            "{{context}}",
            "{{document_text}}",
        ] {
            assert!(
                !prompt.contains(placeholder),
                "literal {placeholder} must be gone; got: {prompt}"
            );
        }
    }

    /// Roman's Step 1 directive G #3: behavioural test that the
    /// strip-then-substitute order delivers a correct prompt for a
    /// template with both an AUTHORING_NOTE block (which references
    /// placeholders in its body — that's the documentation it carries)
    /// AND legitimate placeholder lines AND prose that names the
    /// substitution targets without using `{{...}}` syntax.
    ///
    /// Confirms three properties at once:
    ///
    ///   1. The AUTHORING_NOTE block is stripped — its body never
    ///      reaches the LLM, even though it contains `{{context}}`
    ///      and `{{schema_json}}` text that would otherwise be
    ///      replaced.
    ///   2. Prose phrases like "the cross-document context block
    ///      below" survive intact — the substitution doesn't touch
    ///      them because they don't contain `{{...}}` tokens.
    ///   3. The actual placeholder lines (`{{context}}`,
    ///      `{{document_text}}`, etc.) are substituted with the
    ///      provided values.
    ///
    /// Regression-test for the original Instruction-F bug:
    /// pass2_discovery_response_v4.md had four prose lines that
    /// LITERALLY contained `{{context}}` (now rewritten to "the
    /// cross-document context block below"). This test pins the
    /// fix.
    #[test]
    fn assemble_pass2_prompt_strips_authoring_note_and_preserves_prose_references() {
        let template = "\
<!-- AUTHORING_NOTE
Authors: do not put {{context}} or {{schema_json}} in prose; they get substituted.
-->
# Pass 2 Template

Review the complaint allegations provided in the cross-document context block below. \
For each Evidence entity, decide if it CORROBORATES the allegation.

## Entities from Pass 1

{{entities_json}}

## Schema

{{schema_json}}

## Context

{{context}}

## Document Text

{{document_text}}";
        let prompt = assemble_pass2_prompt(
            template,
            "<SCHEMA_BODY>",
            "<ENTITIES_BODY>",
            "<DOC_BODY>",
            None,
            None,
            Some("<CTX_BODY>"),
        );

        // (1) AUTHORING_NOTE block fully stripped — neither the marker
        // nor the placeholder-shaped prose inside it should remain.
        assert!(
            !prompt.contains("AUTHORING_NOTE"),
            "AUTHORING_NOTE marker must be stripped; got:\n{prompt}"
        );
        assert!(
            !prompt.contains("Authors: do not put"),
            "the AUTHORING_NOTE body must be stripped; got:\n{prompt}"
        );

        // (2) Prose phrase survives — the substitution layer doesn't
        // touch text that doesn't contain `{{...}}` tokens. This is
        // the property that broke in the original bug.
        assert!(
            prompt.contains("Review the complaint allegations provided in \
                the cross-document context block below."),
            "the prose reference to the context block must survive intact; \
             got:\n{prompt}"
        );

        // (3) Real placeholders are substituted with the provided values.
        assert!(prompt.contains("<SCHEMA_BODY>"));
        assert!(prompt.contains("<ENTITIES_BODY>"));
        assert!(prompt.contains("<CTX_BODY>"));
        assert!(prompt.contains("<DOC_BODY>"));

        // (4) No raw placeholder syntax leaks into the final prompt.
        for placeholder in [
            "{{schema_json}}",
            "{{entities_json}}",
            "{{context}}",
            "{{document_text}}",
        ] {
            assert!(
                !prompt.contains(placeholder),
                "literal {placeholder} must be gone; got:\n{prompt}"
            );
        }
    }
}
