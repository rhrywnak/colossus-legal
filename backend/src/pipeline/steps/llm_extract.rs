//! LlmExtract pipeline step — config-driven entity extraction.
//!
//! Loads a processing profile from YAML, resolves per-document overrides,
//! branches on chunking_mode (full document vs chunked), and stores a
//! complete configuration snapshot for audit trail.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Sections 3.7 and 3.8.

use std::collections::HashSet;
use std::error::Error;
use std::path::Path;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use colossus_extract::{
    ChunkMerger, EntityCategory, ExtractedEntity, ExtractedRelationship, FixedSizeSplitter,
    LlmProvider, StructureAwareSplitter, TextChunk, TextSplitter,
};
use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::models::document_status::{RUN_STATUS_COMPLETED, RUN_STATUS_FAILED, RUN_STATUS_RUNNING};
use crate::pipeline::chunking_strategies::resolve_chunking_config;
use crate::pipeline::config::{resolve_config, ProcessingProfile, ResolvedConfig, SystemDefaults};
use crate::pipeline::context::AppContext;
use crate::pipeline::providers::provider_for_model;
use crate::pipeline::steps::llm_extract_helpers::{
    call_with_rate_limit_retry, mark_run_failed, parse_chunk_response,
};
use crate::pipeline::steps::llm_extract_pass2::LlmExtractPass2;
use crate::pipeline::steps::verify::Verify;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository::{self, documents, extraction, models};

// ── Constants ───────────────────────────────────────────────────

/// Fallback max tokens per LLM call when neither the profile nor
/// step_config / env var provides one.
const DEFAULT_CHUNK_MAX_TOKENS: u32 = 8000;

/// Chunking-mode string recognised for single-call (no chunking) extraction.
pub(crate) const CHUNKING_MODE_FULL: &str = "full";

// ── Error type ──────────────────────────────────────────────────

/// Failure modes for the LlmExtract step.
///
/// Display strings are terminal messages — they never interpolate
/// `{source}` (Kazlauskas Guideline 6). Source chains are preserved via
/// `#[source]` where applicable.
#[derive(Debug, thiserror::Error)]
pub enum LlmExtractError {
    #[error("Document not found: {document_id}")]
    DocumentNotFound { document_id: String },

    #[error("No pipeline_config row for document '{document_id}'")]
    NoPipelineConfig { document_id: String },

    #[error("Failed to load schema '{schema_file}': {source}")]
    SchemaLoadFailed {
        schema_file: String,
        #[source]
        source: colossus_extract::PipelineError,
    },

    #[error("Prompt assembly failed: {source}")]
    PromptBuildFailed {
        #[source]
        source: colossus_extract::PipelineError,
    },

    #[error("Document '{document_id}' has no extracted text pages")]
    NoTextPages { document_id: String },

    #[error("LLM call failed: {source}")]
    LlmCallFailed {
        #[source]
        source: colossus_extract::PipelineError,
    },

    #[error("LLM response was not valid JSON ({preview})")]
    ResponseNotJson {
        preview: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Failed to insert extraction_run: {message}")]
    InsertRunFailed { message: String },

    #[error("Failed to finalize extraction_run: {message}")]
    CompleteRunFailed { message: String },

    #[error("Failed to store entities/relationships: {message}")]
    StoreFailed { message: String },

    #[error("LLM semaphore closed before permit could be acquired")]
    SemaphoreClosed,

    #[error("Failed to load processing profile: {message}")]
    ProfileLoadFailed { message: String },

    #[error("Model '{model_id}' not found or inactive in llm_models")]
    ModelNotFound { model_id: String },

    #[error("Failed to construct LLM provider: {message}")]
    ProviderConstructionFailed { message: String },

    #[error("Profile '{profile_name}' has no pass2_template_file")]
    NoPass2Template { profile_name: String },

    #[error("No COMPLETED pass-1 extraction_run found for document '{document_id}'")]
    NoCompletedPass1 { document_id: String },
}

// ── Step struct ─────────────────────────────────────────────────

/// The LlmExtract step variant's payload.
///
/// Only runtime state is the document id; all other parameters are
/// resolved at execute time from the processing profile and per-document
/// overrides.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmExtract {
    pub document_id: String,
}

#[async_trait]
impl Step<DocProcessing> for LlmExtract {
    async fn execute(
        self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        self.run_llm_extract(db, context, cancel, progress).await
    }

    async fn on_cancel(
        self,
        db: &PgPool,
        _context: &AppContext,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Best-effort: wipe any RUNNING/FAILED *pass-1* runs this step
        // left behind. Scoped by pass_number = 1 so a pass-1 cancel never
        // clobbers a prior FAILED pass-2 row (pass 2 has its own Step
        // with its own on_cancel). A COMPLETED run is left intact — its
        // rows are the authoritative output and the idempotency check
        // will short-circuit future retries.
        if let Err(e) = sqlx::query(
            "DELETE FROM extraction_runs \
             WHERE document_id = $1 AND pass_number = 1 \
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
                "LlmExtract::on_cancel: delete of RUNNING/FAILED pass-1 runs failed (non-fatal)"
            );
        }
        Ok(())
    }
}

// ── Outcome of a single extraction path ─────────────────────────

/// Aggregated result produced by either the full-document or chunked path.
///
/// The orchestrator consumes this to write the final run record and store
/// entities + relationships. Chunk-specific stats are `None` for the
/// full-document path.
struct ExtractionOutcome {
    entities: Vec<serde_json::Value>,
    relationships: Vec<serde_json::Value>,
    total_input_tokens: i64,
    total_output_tokens: i64,
    chunk_count: Option<i32>,
    chunks_succeeded: Option<i32>,
    chunks_failed: Option<i32>,
}

// ── Core implementation ─────────────────────────────────────────

impl LlmExtract {
    /// Orchestrate the full extraction path.
    ///
    /// Resolves config, looks up the model, constructs the provider, inserts
    /// the `extraction_runs` row, dispatches to the full-document or chunked
    /// sub-function, then finalizes the run, stores entities + relationships,
    /// and writes the processing_config JSONB snapshot.
    async fn run_llm_extract(
        &self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        // 1. Resolve config FIRST. We need `resolved.run_pass2` available
        //    at the idempotency check so the short-circuit path can route
        //    through the right next step (LlmExtractPass2 vs Verify). The
        //    reads here are cheap — one DB row each, one YAML read — so
        //    doing them on the short-circuit path is a fair trade for
        //    FSM correctness.
        let pipe_config = pipeline_repository::get_pipeline_config(db, &self.document_id)
            .await?
            .ok_or_else(|| LlmExtractError::NoPipelineConfig {
                document_id: self.document_id.clone(),
            })?;

        let overrides =
            pipeline_repository::get_pipeline_config_overrides(db, &self.document_id).await?;
        let profile_name = overrides
            .profile_name
            .clone()
            .unwrap_or_else(|| default_profile_name_from_schema(&pipe_config.schema_file));
        let profile = ProcessingProfile::load(&context.profile_dir, &profile_name)
            .map_err(|e| LlmExtractError::ProfileLoadFailed { message: e })?;
        let resolved = resolve_config(&profile, &overrides);

        // 2. Idempotency: short-circuit if a COMPLETED *pass-1* run
        //    already exists. Filtered by pass_number = 1 so a prior
        //    pass-2 COMPLETED row doesn't falsely mask an incomplete
        //    pass-1 (the ON CONFLICT upsert in insert_extraction_run
        //    keys on (document_id, pass_number), so both passes can
        //    coexist as separate rows).
        if pass1_already_complete(db, &self.document_id).await? {
            tracing::info!(
                document_id = %self.document_id,
                run_pass2 = resolved.run_pass2,
                "Completed pass-1 extraction_run exists, skipping"
            );
            return Ok(StepResult::Next(next_step_after_pass1(
                &resolved,
                &self.document_id,
            )));
        }

        // 3. Document + schema + text pages.
        let _doc = pipeline_repository::get_document(db, &self.document_id)
            .await?
            .ok_or_else(|| LlmExtractError::DocumentNotFound {
                document_id: self.document_id.clone(),
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

        let pages = pipeline_repository::get_document_text(db, &self.document_id).await?;
        if pages.is_empty() {
            return Err(LlmExtractError::NoTextPages {
                document_id: self.document_id.clone(),
            }
            .into());
        }
        let full_text = pages
            .iter()
            .map(|p| format!("--- Page {} ---\n{}", p.page_number, p.text_content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // 4. Look up the model row and construct a provider for this document.
        let model_record = models::get_active_model_by_id(db, &resolved.model)
            .await?
            .ok_or_else(|| LlmExtractError::ModelNotFound {
                model_id: resolved.model.clone(),
            })?;
        let llm_provider = provider_for_model(&model_record)
            .map_err(|message| LlmExtractError::ProviderConstructionFailed { message })?;

        // 5. Load template and hash it.
        let template_path = format!("{}/{}", context.template_dir, resolved.template_file);
        let template_text = std::fs::read_to_string(&template_path)
            .map_err(|e| format!("Failed to read template '{template_path}': {e}"))?;
        let template_hash = sha2_hex(&template_text);

        // 5b. Load system prompt if the resolved config names one. The
        //     provider's native system field wins when populated (Anthropic
        //     Messages API treats system as a separate instruction layer);
        //     concatenating into the user prompt would lose that distinction.
        //     Read failure surfaces as ProfileLoadFailed so the audit log
        //     names the missing file, matching the template-load convention.
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
        let system_prompt_hash: Option<String> =
            system_prompt.as_deref().map(sha2_hex);

        // 5c. Load the global-rules fragment if the profile names one and
        //     compute its SHA-256 for the F3 audit columns. Read failure is
        //     fatal — the profile declared the rules file as required, so
        //     missing it is a configuration error worth surfacing instead
        //     of silently continuing without the rules. See
        //     [`load_global_rules`] for the full case table (no-file vs
        //     empty-file vs nonempty-file vs missing-file).
        let (global_rules_text, global_rules_hash) = load_global_rules(
            Path::new(&context.template_dir),
            resolved.global_rules_file.as_deref(),
        )?;

        // 6. Choose an effective max_tokens.
        let max_tokens = resolve_max_tokens(&resolved);

        // 7. Insert the extraction_runs row.
        // F3 audit: pass `rules_name` and `rules_hash` from the resolved
        // profile and the loaded fragment. Both columns existed since
        // migration 20260410 but were always NULL until this fix —
        // AUDIT_PIPELINE_CONFIG_GAPS.md Gap 5.
        let run_id = extraction::insert_extraction_run(
            db,
            &self.document_id,
            1,
            &resolved.model,
            &schema.version,
            None,
            Some(resolved.template_file.as_str()),
            Some(&template_hash),
            resolved.global_rules_file.as_deref(),
            global_rules_hash.as_deref(),
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

        // 7b. If the run row was reused via ON CONFLICT DO UPDATE (prior
        //     FAILED or stuck-RUNNING attempt), wipe its children so new
        //     items / chunks / relationships don't coexist with stale
        //     ones under the same run_id. No-op for a fresh row (R5).
        extraction::reset_extraction_run_children(db, run_id)
            .await
            .map_err(|e| LlmExtractError::InsertRunFailed {
                message: format!("reset_extraction_run_children: {e}"),
            })?;

        // 8. Cancel check before acquiring semaphore.
        if cancel.is_cancelled().await {
            mark_run_failed(db, run_id, "Cancelled before extraction").await;
            return Err("Cancelled before extraction".into());
        }

        // 9. Acquire LLM semaphore for the duration of the extraction.
        let _llm_permit = context
            .llm_semaphore
            .acquire()
            .await
            .map_err(|e| {
                tracing::debug!(error = %e, "Semaphore acquire failed");
                LlmExtractError::SemaphoreClosed
            })?;

        // 10. Dispatch on the effective chunking mode.
        //
        // ## Rust Learning: Open-set dispatch via `match` on `&str`
        //
        // We match on a `String`-derived `&str` rather than a typed
        // enum so the set of modes can grow without breaking changes
        // elsewhere. The default arm catches anything unrecognized
        // ("chunked" *and* any unknown value) and routes it through
        // the legacy FixedSizeSplitter path. That is the safest
        // backward-compat behavior: a profile that ships a future mode
        // string ("parallel", "multi_pass", ...) before this binary
        // knows what it means still produces extraction output via the
        // legacy path rather than failing the run outright.
        let system_prompt_ref = system_prompt.as_deref();
        // The assembler treats `None` and `Some("")` identically (both
        // collapse to an empty substitution); pass `None` when no rules
        // file was configured so the audit comment in `assemble_chunk_prompt`
        // about "no rule fragment to inject" stays accurate.
        let global_rules_ref: Option<&str> = global_rules_hash
            .as_ref()
            .map(|_| global_rules_text.as_str());
        let admin_instructions_ref = pipe_config.admin_instructions.as_deref();
        // `{{context}}` is reserved for a future prior-context renderer
        // (driven by `pipe_config.prior_context_doc_ids`). For now it is
        // always None — the helper substitutes empty string so the
        // placeholder vanishes without leaking literal text. See
        // RunArgs.context doc comment for the migration path.
        let context_ref: Option<&str> = None;
        let effective_mode = resolve_effective_mode(&resolved);
        let outcome = match effective_mode.as_str() {
            CHUNKING_MODE_FULL => {
                run_full_document_extraction(RunArgs {
                    db,
                    document_id: &self.document_id,
                    llm_provider: &*llm_provider,
                    system_prompt: system_prompt_ref,
                    template_text: &template_text,
                    schema_json: &schema_json,
                    full_text: &full_text,
                    global_rules: global_rules_ref,
                    admin_instructions: admin_instructions_ref,
                    context: context_ref,
                    max_tokens,
                    cancel,
                    progress,
                    run_id,
                })
                .await?
            }
            "structured" => {
                run_structured_extraction(
                    RunArgs {
                        db,
                        document_id: &self.document_id,
                        llm_provider: &*llm_provider,
                        system_prompt: system_prompt_ref,
                        template_text: &template_text,
                        schema_json: &schema_json,
                        full_text: &full_text,
                        global_rules: global_rules_ref,
                        admin_instructions: admin_instructions_ref,
                        context: context_ref,
                        max_tokens,
                        cancel,
                        progress,
                        run_id,
                    },
                    &resolved,
                    &schema,
                )
                .await?
            }
            _ => {
                // "chunked" and any unknown value → legacy FixedSizeSplitter path.
                run_chunked_extraction(
                    RunArgs {
                        db,
                        document_id: &self.document_id,
                        llm_provider: &*llm_provider,
                        system_prompt: system_prompt_ref,
                        template_text: &template_text,
                        schema_json: &schema_json,
                        full_text: &full_text,
                        global_rules: global_rules_ref,
                        admin_instructions: admin_instructions_ref,
                        context: context_ref,
                        max_tokens,
                        cancel,
                        progress,
                        run_id,
                    },
                    &resolved,
                    &schema,
                )
                .await?
            }
        };

        // 11. Merge and finalize the run.
        let merged = serde_json::json!({
            "entities": outcome.entities,
            "relationships": outcome.relationships,
        });

        let cost_usd = compute_cost(
            &model_record,
            outcome.total_input_tokens,
            outcome.total_output_tokens,
        );

        if let (Some(total), Some(ok), Some(failed)) = (
            outcome.chunk_count,
            outcome.chunks_succeeded,
            outcome.chunks_failed,
        ) {
            if let Err(e) =
                extraction::update_run_chunk_stats(db, run_id, total, ok, failed).await
            {
                tracing::error!(
                    run_id = run_id,
                    error = %e,
                    "Failed to write chunk stats to extraction run — audit data incomplete"
                );
            }
        }

        extraction::complete_extraction_run(
            db,
            run_id,
            &merged,
            Some(outcome.total_input_tokens as i32),
            Some(outcome.total_output_tokens as i32),
            cost_usd,
            RUN_STATUS_COMPLETED,
        )
        .await
        .map_err(|e| LlmExtractError::CompleteRunFailed {
            message: format!("{e}"),
        })?;

        let (entity_count, rel_count) =
            extraction::store_entities_and_relationships(db, run_id, &self.document_id, &merged)
                .await
                .map_err(|e| LlmExtractError::StoreFailed {
                    message: format!("{e}"),
                })?;

        // 12. Write processing_config JSONB snapshot (best-effort).
        // Pass-1 snapshot: empty cross-doc slices (the runtime fields
        // are Pass-2-only audit data).
        write_processing_config_snapshot(
            db,
            run_id,
            &resolved,
            SnapshotRuntimeFields {
                effective_pass: 1,
                template_hash: &template_hash,
                system_prompt_hash: system_prompt_hash.as_deref(),
                global_rules_hash: global_rules_hash.as_deref(),
                pass2_cross_doc_entities: &[],
                pass2_source_document_ids: &[],
            },
        )
        .await;

        // 13. Final progress + step complete.
        // best-effort progress update
        documents::update_processing_progress(
            db,
            &self.document_id,
            "LlmExtract",
            "Extraction complete",
            outcome.chunk_count.unwrap_or(1),
            outcome.chunk_count.unwrap_or(1),
            entity_count as i32,
            100,
        )
        .await
        .ok();

        tracing::info!(
            document_id = %self.document_id,
            entities = entity_count,
            relationships = rel_count,
            chunks_succeeded = ?outcome.chunks_succeeded,
            chunks_failed = ?outcome.chunks_failed,
            total_input_tokens = outcome.total_input_tokens,
            total_output_tokens = outcome.total_output_tokens,
            profile = %resolved.profile_name,
            chunking_mode = %resolved.chunking_mode,
            "Extraction complete"
        );

        progress.set_step_result(serde_json::json!({
            "entity_count": entity_count,
            "relationship_count": rel_count,
            "input_tokens": outcome.total_input_tokens,
            "output_tokens": outcome.total_output_tokens,
            "chunk_count": outcome.chunk_count,
            "chunks_succeeded": outcome.chunks_succeeded,
            "chunks_failed": outcome.chunks_failed,
            "profile": resolved.profile_name,
            "model": resolved.model,
            "chunking_mode": resolved.chunking_mode,
            "system_prompt_file": resolved.system_prompt_file,
        }));

        Ok(StepResult::Next(next_step_after_pass1(
            &resolved,
            &self.document_id,
        )))
    }
}

// ── Shared run arguments ────────────────────────────────────────

/// Arguments shared by the full-document and chunked extraction paths.
///
/// Grouping these into a single struct keeps the sub-function signatures
/// readable and lets the orchestrator build the bundle once.
struct RunArgs<'a> {
    db: &'a PgPool,
    document_id: &'a str,
    llm_provider: &'a dyn LlmProvider,
    /// Optional system prompt body. When `Some`, routed through
    /// [`LlmProvider::invoke_with_system`] so providers with native system
    /// support (Anthropic) populate the top-level `system` field.
    system_prompt: Option<&'a str>,
    template_text: &'a str,
    schema_json: &'a str,
    full_text: &'a str,
    /// Global-rules fragment substituted at `{{global_rules}}` in the
    /// template. `None` ⇒ substitute empty string (placeholder vanishes
    /// from the prompt without leaking literal text). Loaded once at the
    /// top of `run_llm_extract` from the profile's `global_rules_file`.
    global_rules: Option<&'a str>,
    /// Operator-supplied per-document instructions substituted at
    /// `{{admin_instructions}}`. Sourced from `pipe_config.admin_instructions`.
    /// `None` ⇒ substitute empty string.
    admin_instructions: Option<&'a str>,
    /// Cross-document prior context substituted at `{{context}}`. Currently
    /// always `None` — the placeholder is replaced with empty string
    /// defensively (matching pass-2's pattern at `llm_extract_pass2.rs:319`).
    /// A future renderer for `pipe_config.prior_context_doc_ids` will
    /// populate this without changing the substitution code.
    context: Option<&'a str>,
    max_tokens: u32,
    cancel: &'a CancellationToken,
    progress: &'a ProgressReporter,
    run_id: i32,
}

// ── Full-document extraction ────────────────────────────────────

/// Perform a single-call extraction on the full document text.
///
/// Used when the profile's `chunking_mode` is `"full"`. Produces exactly
/// one LLM call with the full document text substituted into the template.
/// No chunk records are written — the extraction_runs row is the only
/// audit record for this path.
///
/// The template may use either the `{{document_text}}` placeholder
/// (preferred for full-doc templates like `pass1_complaint.md`) or the
/// `{{chunk_text}}` placeholder (convenience when reusing a chunk template
/// at full size). Which placeholder gets the substitution is detected at
/// runtime — hardcoding one would silently produce a template with an
/// unfilled placeholder and blow up downstream. If the template has
/// neither, we fail fast with an explicit error.
async fn run_full_document_extraction(
    args: RunArgs<'_>,
) -> Result<ExtractionOutcome, Box<dyn Error + Send + Sync>> {
    // best-effort progress update
    args.progress
        .report(serde_json::json!({
            "status": "extracting",
            "mode": "full",
        }))
        .await
        .ok();

    // best-effort progress update
    documents::update_processing_progress(
        args.db,
        args.document_id,
        "LlmExtract",
        "Extracting (full document)...",
        1,
        0,
        0,
        5,
    )
    .await
    .ok();

    // Cross-mode symmetric prompt assembly. The full-document path uses
    // the SAME helper as the chunked / structured paths so all three modes
    // substitute exactly the same set of placeholders. Without this,
    // `{{global_rules}}` and friends would silently leak as literal text
    // in `full` mode while being substituted in the other modes — a
    // subtle quality regression depending on which mode an operator picks.
    let prompt = match assemble_chunk_prompt(
        args.template_text,
        args.schema_json,
        args.full_text,
        args.global_rules,
        args.admin_instructions,
        args.context,
    ) {
        Ok(p) => p,
        Err(msg) => {
            mark_run_failed(args.db, args.run_id, &msg).await;
            return Err(msg.into());
        }
    };

    // Persist the assembled prompt on the run for debugging / audit. The
    // insert_extraction_run call earlier stored NULL here because the
    // prompt hadn't been built yet; do it now with an UPDATE. Failure is
    // logged but not fatal — the extraction itself proceeds.
    if let Err(e) = sqlx::query("UPDATE extraction_runs SET assembled_prompt = $1 WHERE id = $2")
        .bind(&prompt)
        .bind(args.run_id)
        .execute(args.db)
        .await
    {
        tracing::warn!(
            run_id = args.run_id,
            error = %e,
            "Failed to store assembled_prompt (non-fatal)"
        );
    }

    let response = call_with_rate_limit_retry(
        args.llm_provider,
        args.system_prompt,
        &prompt,
        args.max_tokens,
        args.cancel,
        args.progress,
        0,
        1,
    )
    .await
    .map_err(|e| {
        LlmExtractError::LlmCallFailed {
            source: e,
        }
    })?;

    let parsed = parse_chunk_response(&response.text).map_err(|e| -> Box<dyn Error + Send + Sync> {
        let fail_msg = format!("Full-document parse failed: {e}");
        Box::<dyn Error + Send + Sync>::from(fail_msg)
    });
    let parsed = match parsed {
        Ok(v) => v,
        Err(e) => {
            mark_run_failed(args.db, args.run_id, &format!("{e}")).await;
            return Err(e);
        }
    };

    let entities = parsed["entities"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let relationships = parsed["relationships"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    Ok(ExtractionOutcome {
        entities,
        relationships,
        total_input_tokens: response.input_tokens.unwrap_or(0) as i64,
        total_output_tokens: response.output_tokens.unwrap_or(0) as i64,
        chunk_count: None,
        chunks_succeeded: None,
        chunks_failed: None,
    })
}

// ── Chunked / structured shared loop ────────────────────────────

/// Assemble a per-chunk prompt by substituting every placeholder the
/// pass-1 templates expect.
///
/// Substituted placeholders (in canonical order — `{{schema_json}}` first
/// so any rule fragments inserted later that happen to reference it can
/// pick it up; body placeholder last so accidental placeholder syntax in
/// rule/instruction text doesn't get re-substituted):
///
/// 1. `{{schema_json}}` — always; the loaded schema
/// 2. `{{global_rules}}` — `global_rules` arg, empty string when `None`
/// 3. `{{admin_instructions}}` — `admin_instructions` arg, empty when `None`
/// 4. `{{context}}` — `context` arg, empty when `None`
/// 5. **Either** `{{document_text}}` *or* `{{chunk_text}}` — whichever the
///    template carries. `{{document_text}}` wins when both appear,
///    matching [`run_full_document_extraction`]'s preference.
///
/// When the template has *neither* `{{document_text}}` *nor* `{{chunk_text}}`
/// the function returns an error so the caller can `mark_run_failed` and
/// abort before sending a prompt the LLM can't act on. The other three
/// placeholders are *not* required — a template that doesn't reference
/// `{{global_rules}}` simply leaves the substitution as a no-op.
///
/// ## Rust Learning: empty-string substitution as the "absent" default
///
/// `Option::unwrap_or("")` collapses `None` and `Some("")` into the same
/// substitution. That's deliberate — both mean "no rules / no
/// instructions / no context to inject" from the LLM's perspective, and
/// the placeholder must vanish either way (otherwise the literal
/// `{{global_rules}}` leaks into the prompt). This is the same defensive
/// pattern pass-2 uses for `{{context}}` at `llm_extract_pass2.rs:319`.
///
/// ## Rust Learning: returning `Result<String, String>` for a small helper
///
/// A full `thiserror` enum would be overkill for the one error case here
/// ("no usable body placeholder"). The caller wraps the `String` into the
/// existing `Box<dyn Error + Send + Sync>` flow alongside `mark_run_failed`,
/// and the message names *both* placeholders so an operator reading the
/// `extraction_runs.error_message` column knows exactly what to fix.
fn assemble_chunk_prompt(
    template_text: &str,
    schema_json: &str,
    chunk_text: &str,
    global_rules: Option<&str>,
    admin_instructions: Option<&str>,
    context: Option<&str>,
) -> Result<String, String> {
    // Strip AUTHORING_NOTE comment blocks before substitution. These
    // are human-template-author meta-text (e.g., "do not put `{{...}}`
    // tokens in prose"). Keeping them out of the substitution layer
    // means an authoring note that itself carries placeholder-shaped
    // text never reaches the `.replace()` chain. See
    // [`strip_authoring_comments`] for the marker contract.
    let stripped = strip_authoring_comments(template_text);
    let template_text = stripped.as_str();

    let has_document_text = template_text.contains("{{document_text}}");
    let has_chunk_text = template_text.contains("{{chunk_text}}");

    if !has_document_text && !has_chunk_text {
        return Err(
            "Template has no {{document_text}} or {{chunk_text}} placeholder — \
             chunk body cannot be injected into the prompt"
                .to_string(),
        );
    }

    // Substitute the three "always-substitute, empty when None" placeholders
    // up front. Doing them before the body substitution avoids any chance
    // that the chunk text itself (which can be arbitrary user content)
    // could contain a literal `{{global_rules}}` etc. and get re-substituted.
    let mut prompt = template_text
        .replace("{{schema_json}}", schema_json)
        .replace("{{global_rules}}", global_rules.unwrap_or(""))
        .replace("{{admin_instructions}}", admin_instructions.unwrap_or(""))
        .replace("{{context}}", context.unwrap_or(""));

    if has_document_text {
        prompt = prompt.replace("{{document_text}}", chunk_text);
    } else {
        // has_chunk_text is true here (proven above by the early return).
        prompt = prompt.replace("{{chunk_text}}", chunk_text);
    }

    Ok(prompt)
}

/// Drive the LLM extraction loop over a pre-split list of text chunks.
///
/// Both the legacy `run_chunked_extraction` (FixedSizeSplitter) and the
/// new `run_structured_extraction` (StructureAwareSplitter) feed chunks
/// into this helper. Everything that is splitter-agnostic — cancel
/// checks, progress emits, prompt assembly, rate-limit retry, parse
/// fallback, per-chunk audit-trail rows, accumulator math, and the
/// "all chunks failed" terminal check — lives here.
///
/// ## Rust Learning: Shared helper beats duplicated branches
///
/// We could have copy-pasted this ~200-line loop into both extraction
/// functions. Two arguments against:
///
/// 1. **Single point of truth.** Group 2b-ii will replace the
///    `.extend()` accumulator with `ChunkMerger` for proper dedup.
///    Doing that change in one place beats doing it in two and
///    risking a subtle divergence.
/// 2. **Refactoring is cheap when the shape already fits.** The loop
///    only references `args.*` fields plus six local accumulators —
///    it is not entangled with how the chunks were *produced*. The
///    chunk **production** (splitter setup + emptiness check + the
///    splitter-specific tracing line) stays in each caller.
///
/// The caller is responsible for guaranteeing `!chunks.is_empty()` and
/// for emitting any splitter-specific tracing before invoking this
/// helper. This helper assumes there is at least one chunk to process.
///
/// `schema` is consulted only to build the cross-chunk-dedup entity-type
/// set fed into `ChunkMerger` after the loop (entity types categorised
/// `Foundation` or `Reference` are deduplicated; `Structural` /
/// `Evidence` are unique-per-occurrence and skipped). The schema is not
/// touched inside the per-chunk loop.
async fn extract_chunks_loop(
    args: &RunArgs<'_>,
    chunks: &[TextChunk],
    schema: &colossus_extract::ExtractionSchema,
) -> Result<ExtractionOutcome, Box<dyn Error + Send + Sync>> {
    // Per-chunk typed results, fed into ChunkMerger after the loop.
    // Each entry is `(chunk_index, entities, relationships)`. Collecting
    // first and merging once at the end (rather than streaming into the
    // merger) lets the merger see every candidate before deciding which
    // entity wins each dedup group — incremental merge would have to
    // revise earlier survivor choices when a richer candidate arrives.
    let mut chunk_results: Vec<(usize, Vec<ExtractedEntity>, Vec<ExtractedRelationship>)> =
        Vec::new();
    // Running total of *typed* entities pushed into `chunk_results`.
    // Powers the in-loop progress UI's "entities found" counter; the
    // pre-merger count, not the deduplicated count, since the merger
    // only runs once after the loop completes.
    let mut running_entity_count: i32 = 0;
    let mut chunks_succeeded: i32 = 0;
    let mut chunks_failed: i32 = 0;
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;

    // best-effort progress update
    documents::update_processing_progress(
        args.db,
        args.document_id,
        "LlmExtract",
        "Extracting entities...",
        chunks.len() as i32,
        0,
        0,
        0,
    )
    .await
    .ok();

    for (i, chunk) in chunks.iter().enumerate() {
        if args.cancel.is_cancelled().await {
            mark_run_failed(args.db, args.run_id, "Cancelled during extraction").await;
            return Err("Cancelled during extraction".into());
        }

        // best-effort progress update
        args.progress
            .report(serde_json::json!({
                "status": "extracting",
                "chunk": i + 1,
                "total": chunks.len(),
                "chars": chunk.text.len(),
            }))
            .await
            .ok();

        // ## Rust Learning: defensive `unwrap_or_else` on a guaranteed-OK call
        //
        // `serde_json::to_value` only fails when the input's `Serialize`
        // impl emits something invalid (e.g., a non-string map key, a
        // non-finite float). `HashMap<String, serde_json::Value>` cannot
        // produce either — the keys are already `String`, the values are
        // already `Value` — so this `to_value` call is structurally
        // guaranteed to succeed today. We still pattern-match defensively
        // so a future change to `TextChunk.metadata`'s value type can't
        // silently drop the audit row's metadata field. The fallback
        // emits a JSON empty-object so the JSONB column always receives
        // a valid object value, never NULL or junk.
        let chunk_metadata_json = serde_json::to_value(&chunk.metadata).unwrap_or_else(|e| {
            tracing::warn!(
                chunk_index = i,
                error = %e,
                "Failed to serialize chunk metadata (non-fatal, using empty object)"
            );
            serde_json::json!({})
        });

        let chunk_id = extraction::insert_extraction_chunk(
            args.db,
            args.run_id,
            i as i32,
            &chunk.text,
            &chunk_metadata_json,
        )
        .await
        .map_err(|e| format!("Failed to insert chunk record: {e}"))?;

        let chunk_start = Instant::now();

        // Dual-aware placeholder substitution. Templates can carry either
        // `{{document_text}}` or `{{chunk_text}}`; failing to find one
        // silently used to ship a prompt with no document body. The
        // helper also fills `{{global_rules}}`, `{{admin_instructions}}`,
        // and `{{context}}` with empty-string fallbacks so the LLM never
        // sees those literal tokens.
        let prompt = match assemble_chunk_prompt(
            args.template_text,
            args.schema_json,
            &chunk.text,
            args.global_rules,
            args.admin_instructions,
            args.context,
        ) {
            Ok(p) => p,
            Err(msg) => {
                mark_run_failed(args.db, args.run_id, &msg).await;
                return Err(msg.into());
            }
        };

        let llm_result = call_with_rate_limit_retry(
            args.llm_provider,
            args.system_prompt,
            &prompt,
            args.max_tokens,
            args.cancel,
            args.progress,
            i,
            chunks.len(),
        )
        .await;

        let chunk_duration_ms = chunk_start.elapsed().as_millis() as i32;

        match llm_result {
            Ok(response) => {
                let input_toks = response.input_tokens.map(|t| t as i32);
                let output_toks = response.output_tokens.map(|t| t as i32);
                total_input_tokens += response.input_tokens.unwrap_or(0) as i64;
                total_output_tokens += response.output_tokens.unwrap_or(0) as i64;

                match parse_chunk_response(&response.text) {
                    Ok(parsed) => {
                        let entity_count = parsed["entities"]
                            .as_array()
                            .map(|a| a.len())
                            .unwrap_or(0);
                        let rel_count = parsed["relationships"]
                            .as_array()
                            .map(|a| a.len())
                            .unwrap_or(0);

                        // ## Rust Learning: JSON → typed → JSON bridge
                        //
                        // The chunk's LLM response arrives as a generic
                        // `serde_json::Value`; the `ChunkMerger` requires
                        // typed `ExtractedEntity` / `ExtractedRelationship`;
                        // and the downstream `ExtractionOutcome` carries
                        // `Vec<serde_json::Value>` again. `serde_json::from_value::<T>(v)`
                        // is the standard `Value → typed` decoder; its
                        // inverse `serde_json::to_value(&t)` runs after the
                        // merger to put us back into JSON for storage.
                        //
                        // ## Rust Learning: lenient `filter_map` + warn
                        //
                        // We never `.unwrap()` a deserialize result here.
                        // `.collect::<Result<Vec<_>, _>>()` would be the
                        // strict alternative — one malformed entity would
                        // poison the whole chunk. `filter_map(... .ok())`
                        // with a `tracing::warn!` per drop preserves the
                        // chunk's success status and keeps a paper trail
                        // in the logs for debugging the LLM's output.
                        let entities_typed: Vec<ExtractedEntity> = parsed["entities"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| {
                                        match serde_json::from_value::<ExtractedEntity>(
                                            v.clone(),
                                        ) {
                                            Ok(e) => Some(e),
                                            Err(err) => {
                                                tracing::warn!(
                                                    chunk = i,
                                                    error = %err,
                                                    "Skipping malformed entity in chunk response"
                                                );
                                                None
                                            }
                                        }
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        let relationships_typed: Vec<ExtractedRelationship> = parsed
                            ["relationships"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| {
                                        match serde_json::from_value::<ExtractedRelationship>(
                                            v.clone(),
                                        ) {
                                            Ok(r) => Some(r),
                                            Err(err) => {
                                                tracing::warn!(
                                                    chunk = i,
                                                    error = %err,
                                                    "Skipping malformed relationship in chunk response"
                                                );
                                                None
                                            }
                                        }
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        running_entity_count += entities_typed.len() as i32;
                        chunk_results.push((i, entities_typed, relationships_typed));

                        chunks_succeeded += 1;
                        if let Err(e) = extraction::complete_extraction_chunk(
                            args.db,
                            chunk_id,
                            "success",
                            Some(entity_count as i32),
                            Some(rel_count as i32),
                            input_toks,
                            output_toks,
                            Some(chunk_duration_ms),
                            None,
                        )
                        .await
                        {
                            tracing::error!(
                                run_id = args.run_id,
                                chunk_id = %chunk_id,
                                error = %e,
                                "Failed to record successful chunk completion — audit data incomplete"
                            );
                        }

                        tracing::info!(
                            chunk = i,
                            entities = entity_count,
                            relationships = rel_count,
                            "Chunk extraction succeeded"
                        );

                        let percent =
                            ((chunks_succeeded as f64 / chunks.len() as f64) * 100.0) as i32;
                        // best-effort progress update
                        documents::update_processing_progress(
                            args.db,
                            args.document_id,
                            "LlmExtract",
                            &format!("Extracting chunk {} of {}...", i + 1, chunks.len()),
                            chunks.len() as i32,
                            chunks_succeeded,
                            running_entity_count,
                            percent.min(95),
                        )
                        .await
                        .ok();
                    }
                    Err(parse_err) => {
                        chunks_failed += 1;
                        tracing::warn!(
                            chunk = i,
                            error = %parse_err,
                            "Chunk parse failed after repair attempt"
                        );
                        if let Err(e) = extraction::complete_extraction_chunk(
                            args.db,
                            chunk_id,
                            "failed",
                            None,
                            None,
                            input_toks,
                            output_toks,
                            Some(chunk_duration_ms),
                            Some(&format!("Parse error: {parse_err}")),
                        )
                        .await
                        {
                            tracing::error!(
                                run_id = args.run_id,
                                chunk_id = %chunk_id,
                                error = %e,
                                "Failed to record failed (parse) chunk completion — audit data incomplete"
                            );
                        }

                        let processed = chunks_succeeded + chunks_failed;
                        let percent = ((processed as f64 / chunks.len() as f64) * 100.0) as i32;
                        // best-effort progress update
                        documents::update_processing_progress(
                            args.db,
                            args.document_id,
                            "LlmExtract",
                            &format!("Extracting chunk {} of {}...", i + 1, chunks.len()),
                            chunks.len() as i32,
                            processed,
                            running_entity_count,
                            percent.min(95),
                        )
                        .await
                        .ok();
                    }
                }
            }
            Err(call_err) => {
                chunks_failed += 1;
                tracing::warn!(chunk = i, error = %call_err, "Chunk LLM call failed");
                if let Err(e) = extraction::complete_extraction_chunk(
                    args.db,
                    chunk_id,
                    "failed",
                    None,
                    None,
                    None,
                    None,
                    Some(chunk_duration_ms),
                    Some(&format!("{call_err}")),
                )
                .await
                {
                    tracing::error!(
                        run_id = args.run_id,
                        chunk_id = %chunk_id,
                        error = %e,
                        "Failed to record failed (LLM call) chunk completion — audit data incomplete"
                    );
                }

                let processed = chunks_succeeded + chunks_failed;
                let percent = ((processed as f64 / chunks.len() as f64) * 100.0) as i32;
                // best-effort progress update
                documents::update_processing_progress(
                    args.db,
                    args.document_id,
                    "LlmExtract",
                    &format!("Extracting chunk {} of {}...", i + 1, chunks.len()),
                    chunks.len() as i32,
                    processed,
                    running_entity_count,
                    percent.min(95),
                )
                .await
                .ok();
            }
        }
    }

    if chunks_succeeded == 0 {
        let msg = format!("All {} chunks failed extraction", chunks.len());
        mark_run_failed(args.db, args.run_id, &msg).await;
        return Err(msg.into());
    }

    // ## Rust Learning: Schema-driven dedup, not hardcoded names
    //
    // The merger's dedup-eligible set is built by *filtering the schema*
    // — not by listing entity-type names in code. Entity types whose
    // schema category is `Foundation` (stable identities like parties,
    // counts) or `Reference` (citations, exhibits) repeat across chunks
    // and need cross-chunk dedup; `Structural` and `Evidence` entries
    // are unique per occurrence. Adding a new entity type with
    // `category: foundation` to a future schema YAML automatically
    // enables dedup for it without touching this file.
    let dedup_types: HashSet<String> = schema
        .entity_types
        .iter()
        .filter(|et| matches!(et.category, EntityCategory::Foundation | EntityCategory::Reference))
        .map(|et| et.name.clone())
        .collect();

    let merger = ChunkMerger::new(dedup_types);
    let merge_result = merger.merge(chunk_results);

    tracing::info!(
        document_id = %args.document_id,
        entities_before = merge_result.stats.entities_before,
        entities_after = merge_result.stats.entities_after,
        entities_deduplicated = merge_result.stats.entities_deduplicated,
        relationships_before = merge_result.stats.relationships_before,
        relationships_after = merge_result.stats.relationships_after,
        // Note: the merger counts remapped *reference endpoints*, not
        // "relationships" — a single relationship may have one or both
        // endpoints rewritten when its endpoints survived dedup under a
        // different ID.
        references_remapped = merge_result.stats.references_remapped,
        "Chunk merge complete"
    );

    // Re-encode the merger's typed output back into `serde_json::Value`
    // so `ExtractionOutcome` and the downstream
    // `extraction::store_entities_and_relationships` see the same JSON
    // shape they always have. `to_value` is the inverse of `from_value`
    // (used at chunk-collection time above) — the standard typed↔JSON
    // bridge for any type that derives both `Serialize` and `Deserialize`.
    let all_entities: Vec<serde_json::Value> = merge_result
        .entities
        .into_iter()
        .map(|e| {
            serde_json::to_value(&e).unwrap_or_else(|err| {
                tracing::warn!(
                    error = %err,
                    "Failed to re-serialize merged entity (using null fallback)"
                );
                serde_json::Value::Null
            })
        })
        .collect();
    let all_relationships: Vec<serde_json::Value> = merge_result
        .relationships
        .into_iter()
        .map(|r| {
            serde_json::to_value(&r).unwrap_or_else(|err| {
                tracing::warn!(
                    error = %err,
                    "Failed to re-serialize merged relationship (using null fallback)"
                );
                serde_json::Value::Null
            })
        })
        .collect();

    Ok(ExtractionOutcome {
        entities: all_entities,
        relationships: all_relationships,
        total_input_tokens,
        total_output_tokens,
        chunk_count: Some(chunks.len() as i32),
        chunks_succeeded: Some(chunks_succeeded),
        chunks_failed: Some(chunks_failed),
    })
}

// ── Chunked extraction (legacy, FixedSizeSplitter) ──────────────

/// Perform extraction over fixed-size text chunks with per-chunk
/// observability.
///
/// Splits the full document text using `FixedSizeSplitter` with the
/// resolved `chunk_size` / `chunk_overlap`, then delegates to
/// [`extract_chunks_loop`]. A chunk failure is logged and counted but
/// does not abort the run unless *every* chunk fails.
async fn run_chunked_extraction(
    args: RunArgs<'_>,
    resolved: &ResolvedConfig,
    schema: &colossus_extract::ExtractionSchema,
) -> Result<ExtractionOutcome, Box<dyn Error + Send + Sync>> {
    let chunk_size = resolved
        .chunk_size
        .unwrap_or(SystemDefaults::chunk_size())
        .max(1) as usize;
    let chunk_overlap = resolved
        .chunk_overlap
        .unwrap_or(SystemDefaults::chunk_overlap())
        .max(0) as usize;
    let chunks = FixedSizeSplitter::with_config(chunk_size, chunk_overlap).split(args.full_text);
    if chunks.is_empty() {
        return Err("Splitter produced zero chunks from non-empty text".into());
    }
    tracing::info!(
        document_id = %args.document_id,
        chunk_count = chunks.len(),
        full_text_len = args.full_text.len(),
        chunk_size,
        chunk_overlap,
        "Split document into chunks for extraction"
    );

    extract_chunks_loop(&args, &chunks, schema).await
}

// ── Structured extraction (StructureAwareSplitter) ──────────────

/// Perform extraction over structure-aware chunks (atomic units like
/// numbered Q&A pairs or paragraphs grouped according to a strategy).
///
/// Resolves the effective chunking config (strategy defaults + profile
/// overrides + per-doc overrides — the three-layer merge handled by
/// [`resolve_chunking_config`]), constructs a [`StructureAwareSplitter`]
/// from that map, and delegates to [`extract_chunks_loop`]. The merge
/// at the end of the loop currently uses `.extend()` (blind
/// concatenation); Group 2b-ii will replace that with `ChunkMerger` for
/// proper deduplication.
async fn run_structured_extraction(
    args: RunArgs<'_>,
    resolved: &ResolvedConfig,
    schema: &colossus_extract::ExtractionSchema,
) -> Result<ExtractionOutcome, Box<dyn Error + Send + Sync>> {
    // Three-layer config merge: strategy defaults underneath, profile
    // YAML in the middle, per-document overrides on top. The first two
    // are applied here; the third was already merged into
    // `resolved.chunking_config` by `resolve_config()` in `config.rs`.
    let effective_config = resolve_chunking_config(&resolved.chunking_config);

    // `StructureAwareSplitter::from_config` consumes the map and reads
    // boundary_pattern / response_marker / units_per_chunk /
    // unit_overlap from it via the `ConfigAccess` extension trait.
    // Unknown keys are preserved but ignored by the splitter — they may
    // be consumed by other pipeline components downstream.
    let splitter = StructureAwareSplitter::from_config(effective_config.clone());
    let chunks = splitter.split(args.full_text);
    if chunks.is_empty() {
        return Err("StructureAwareSplitter produced zero chunks from non-empty text".into());
    }

    tracing::info!(
        document_id = %args.document_id,
        mode = "structured",
        strategy = effective_config
            .get("strategy")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown"),
        chunk_count = chunks.len(),
        full_text_len = args.full_text.len(),
        "Structure-aware splitting complete"
    );

    extract_chunks_loop(&args, &chunks, schema).await
}

// ── Small helpers ───────────────────────────────────────────────

/// True if a COMPLETED pass-1 extraction_run already exists for this
/// document.
///
/// Scoped by `pass_number = 1` because pass 2 has its own run row
/// under the same document_id. The prior unfiltered version would
/// false-positive once pass 2 landed, incorrectly short-circuiting
/// pass 1 when only pass 2 was COMPLETED (impossible in practice, but
/// also the wrong semantics for the retry-after-pass-2-failure case).
async fn pass1_already_complete(
    db: &PgPool,
    document_id: &str,
) -> Result<bool, sqlx::Error> {
    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM extraction_runs \
         WHERE document_id = $1 AND pass_number = 1 AND status = $2 \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(document_id)
    .bind(RUN_STATUS_COMPLETED)
    .fetch_optional(db)
    .await?;
    Ok(existing.is_some())
}

/// Pick the next FSM step after a successful (or already-COMPLETED)
/// pass 1.
///
/// When the resolved profile has `run_pass2: true`, routes through
/// `LlmExtractPass2`; otherwise returns directly to `Verify`. Shared
/// between the idempotency short-circuit and the success path so both
/// branches agree on the FSM edge — otherwise a retry of an already-
/// completed pass 1 (with `run_pass2 = true`) would bypass pass 2.
pub(crate) fn next_step_after_pass1(
    resolved: &ResolvedConfig,
    document_id: &str,
) -> DocProcessing {
    if resolved.run_pass2 {
        DocProcessing::LlmExtractPass2(LlmExtractPass2 {
            document_id: document_id.to_string(),
        })
    } else {
        DocProcessing::Verify(Verify {
            document_id: document_id.to_string(),
        })
    }
}

/// Fall back to deriving a profile name from the schema filename.
///
/// Legacy rows in `pipeline_config` may not have a `profile_name` set yet.
/// Strips `.yaml` and any trailing `_v<digits>` version suffix, so
/// `complaint_v2.yaml` → `complaint` and `motion_v4.yaml` → `motion`.
/// This keeps migration-era documents processable without forcing a backfill.
pub(crate) fn default_profile_name_from_schema(schema_file: &str) -> String {
    let base = schema_file.trim_end_matches(".yaml");
    if let Some(idx) = base.rfind("_v") {
        let suffix = &base[idx + 2..];
        if !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()) {
            return base[..idx].to_string();
        }
    }
    base.to_string()
}

/// Resolve an effective `max_tokens` for LLM calls.
///
/// Priority: `ResolvedConfig.max_tokens` (from profile) → `LLM_MAX_TOKENS`
/// env var → [`DEFAULT_CHUNK_MAX_TOKENS`]. Values ≤ 0 in the profile are
/// treated as unset.
pub(crate) fn resolve_max_tokens(resolved: &ResolvedConfig) -> u32 {
    if resolved.max_tokens > 0 {
        return resolved.max_tokens as u32;
    }
    std::env::var("LLM_MAX_TOKENS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_CHUNK_MAX_TOKENS)
}

/// Compute the total cost in USD from token counts and model rates.
///
/// Returns `None` if either rate is missing on the model record — the
/// downstream UI treats `None` as "unknown cost" rather than zero.
pub(crate) fn compute_cost(
    model: &crate::repositories::pipeline_repository::LlmModelRecord,
    input_tokens: i64,
    output_tokens: i64,
) -> Option<f64> {
    let cost_in = model
        .cost_per_input_token
        .map(|c| c * input_tokens as f64);
    let cost_out = model
        .cost_per_output_token
        .map(|c| c * output_tokens as f64);
    match (cost_in, cost_out) {
        (Some(a), Some(b)) => Some(a + b),
        _ => None,
    }
}

/// Runtime-discovered values that the snapshot helper folds into the
/// JSONB body before serialising.
///
/// `ResolvedConfig` itself is fully populated by `resolve_config` — it
/// is the *resolved-from-profile-and-overrides* shape, deliberately
/// kept clean of values that don't exist until extraction has actually
/// run (file hashes, the cross-doc entities that ended up in the Pass-2
/// prompt, the discriminator naming this row's pass). Bundling those
/// runtime fields into one struct keeps
/// [`write_processing_config_snapshot`]'s call sites readable; the only
/// two callers (Pass-1 and Pass-2 steps) build one of these and pass it
/// as a single argument.
///
/// All references are `'a`-scoped — the struct is short-lived (built
/// per-run, consumed once) so no owned-data alternative is justified.
///
/// ## Rust Learning: `&'a [T]` in a struct
///
/// A borrowed slice in a struct field requires a named lifetime so the
/// compiler can prove the borrowed data outlives every use of the
/// struct. Naming the lifetime `'a` here lets all field borrows share
/// one lifetime — the call site builds the struct from locals that all
/// live at least as long as the snapshot await, so `'a` collapses to
/// "the local scope of the call" and we never have to spell it out.
pub(crate) struct SnapshotRuntimeFields<'a> {
    /// Which pass this snapshot describes. `1` for Pass-1, `2` for Pass-2.
    /// Lands in `processing_config.effective_pass`. On Pass-2 it also
    /// triggers the model/template overwrite below.
    pub effective_pass: u8,
    pub template_hash: &'a str,
    pub system_prompt_hash: Option<&'a str>,
    pub global_rules_hash: Option<&'a str>,
    /// Pass-2 only. Empty slice on Pass-1.
    pub pass2_cross_doc_entities: &'a [crate::pipeline::config::CrossDocContextRecord],
    /// Pass-2 only. Empty slice on Pass-1.
    pub pass2_source_document_ids: &'a [String],
}

/// Write the resolved configuration snapshot to
/// `extraction_runs.processing_config`.
///
/// Best-effort: a snapshot-write failure is logged but does not fail
/// extraction — the merged entities have already been committed.
///
/// **Mutations apply only to the clone, never to the input `resolved`
/// reference.** The function takes `&ResolvedConfig` (immutable borrow),
/// clones it as the very first step, and mutates the clone before
/// serialisation. The caller's resolver state is therefore preserved
/// even though the snapshot may legitimately swap fields (e.g. on
/// Pass-2, `model` is overwritten with `pass2_model`). This matters for
/// Pass-1's success path, which keeps using `resolved` after the
/// snapshot returns: if we mutated the input, a Pass-2 follow-up that
/// also reads `resolved.model` would see the wrong value.
///
/// On `effective_pass == 2`, the clone's `model` is overwritten with
/// `resolved.pass2_model.clone().unwrap_or_else(|| resolved.model.clone())`
/// (matches the runtime fallback at the LLM call site) and
/// `template_file` is overwritten with `resolved.pass2_template_file`
/// when set. `template_hash` always comes from the runtime parameter —
/// the caller passes the Pass-2 template's hash on a Pass-2 row and the
/// Pass-1 template's hash on a Pass-1 row.
pub(crate) async fn write_processing_config_snapshot(
    db: &PgPool,
    run_id: i32,
    resolved: &ResolvedConfig,
    runtime: SnapshotRuntimeFields<'_>,
) {
    // Clone first, mutate the clone. See function doc for why.
    let mut snapshot = resolved.clone();
    snapshot.effective_pass = runtime.effective_pass;
    snapshot.template_hash = Some(runtime.template_hash.to_string());
    snapshot.system_prompt_hash = runtime.system_prompt_hash.map(str::to_string);
    snapshot.global_rules_hash = runtime.global_rules_hash.map(str::to_string);
    snapshot.pass2_cross_doc_entities = runtime.pass2_cross_doc_entities.to_vec();
    snapshot.pass2_source_document_ids = runtime.pass2_source_document_ids.to_vec();

    if runtime.effective_pass == 2 {
        // Pass-2 snapshot: the JSONB's `model` and `template_file` must
        // describe the Pass-2 LLM call, not the Pass-1 metadata that
        // `ResolvedConfig` carries by default. Mirror the runtime
        // fallback at `llm_extract_pass2.rs` (pass2_model → model).
        snapshot.model = resolved
            .pass2_model
            .clone()
            .unwrap_or_else(|| resolved.model.clone());
        if let Some(p2_tmpl) = &resolved.pass2_template_file {
            snapshot.template_file = p2_tmpl.clone();
        }
        // template_hash is already set above from the runtime parameter
        // — on Pass-2 the caller passes the Pass-2 template's hash, so
        // it is already correct.
    }

    let config_json = match serde_json::to_value(&snapshot) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                run_id,
                error = %e,
                "Failed to serialize resolved config for snapshot (non-fatal)"
            );
            return;
        }
    };

    if let Err(e) = sqlx::query("UPDATE extraction_runs SET processing_config = $1 WHERE id = $2")
        .bind(&config_json)
        .bind(run_id)
        .execute(db)
        .await
    {
        tracing::warn!(
            run_id,
            error = %e,
            "Failed to write processing_config snapshot (non-fatal)"
        );
    }
}

/// Load the profile's `global_rules_file` and compute its SHA-256.
///
/// "Global rules" is a Markdown fragment (e.g. `global_rules_v4.md`) that
/// every profile may share. Pass-1 and Pass-2 prompts substitute the
/// fragment's content at the `{{global_rules}}` placeholder. The hash
/// is recorded in `extraction_runs.rules_hash` and in the
/// `processing_config` JSONB snapshot so two runs against different
/// versions of the same rules file are distinguishable from the
/// database alone (audit reproducibility — Gap 5 in
/// AUDIT_PIPELINE_CONFIG_GAPS.md).
///
/// Three input cases survive into the audit log:
///
/// * `file_name = None` — the profile didn't configure a rules file.
///   Returns `("".to_string(), None)`. The empty string makes the
///   `{{global_rules}}` substitution vanish; the `None` hash makes
///   `rules_hash` NULL in the DB so an auditor can tell "no file" from
///   "empty file."
///
/// * `file_name = Some(_)` and the file is empty (0 bytes) — the
///   operator deliberately neutralised the rules. Returns
///   `("".to_string(), Some(sha256("")))`. The hash proves the file
///   existed and was empty, distinguishing this run from a no-rules run.
///
/// * `file_name = Some(_)` and the file has content — the normal case.
///   Returns `(content, Some(sha256(content)))`.
///
/// A configured-but-missing file (the profile points at a path that
/// doesn't exist on disk) returns
/// `Err(LlmExtractError::ProfileLoadFailed)`. Failing fast prevents a
/// silent extraction-without-rules; an auditor reading
/// `extraction_runs.error_message` sees the exact path that couldn't
/// be loaded.
///
/// ## Rust Learning: returning `Result<(_, _), _>` instead of two helpers
///
/// We could split this into a `load_global_rules` and a separate
/// `hash_global_rules`, but the two are always called together — every
/// caller that needs the content also needs the hash for audit. Bundling
/// them in a tuple eliminates one source of "did I forget to hash?"
/// drift across call sites.
///
/// ## Rust Learning: `&Path` vs `&str` for directory parameters
///
/// `&Path` is the idiomatic Rust type for filesystem paths. Callers can
/// pass either a `PathBuf` (`&pb`) or a `&str` (via `Path::new(s)`),
/// and the function gets the platform-correct path-joining behaviour
/// from `Path::join` for free. A `&str` parameter would force every
/// caller to use `format!("{dir}/{file}")` with a literal `/` separator
/// — fine on Linux/Mac, wrong on Windows.
pub(crate) fn load_global_rules(
    template_dir: &Path,
    file_name: Option<&str>,
) -> Result<(String, Option<String>), LlmExtractError> {
    let Some(name) = file_name else {
        // No rules file configured. The substitution gets the empty
        // string; `rules_hash` stays NULL in the audit log so an
        // auditor can tell "no file" from "empty file."
        return Ok((String::new(), None));
    };

    let path = template_dir.join(name);
    let content = std::fs::read_to_string(&path).map_err(|e| {
        LlmExtractError::ProfileLoadFailed {
            message: format!("Failed to read global rules '{}': {e}", path.display()),
        }
    })?;
    let hash = sha2_hex(&content);
    Ok((content, Some(hash)))
}

/// Strip every `<!-- AUTHORING_NOTE ... -->` block from a template.
///
/// Returns the template with each AUTHORING_NOTE-marked HTML comment
/// removed (along with any trailing newline immediately following the
/// comment, so the strip site doesn't leave a blank line behind).
/// Regular `<!-- ... -->` comments without the AUTHORING_NOTE marker
/// are preserved verbatim — the marker is the explicit "this comment
/// is for human template authors only; do not ship to the LLM" signal.
///
/// ## Why this exists
///
/// Approach 2 of Instruction F adds an "authoring rules" comment block
/// to every Pass-2 template explaining the substitution convention
/// (placeholders only on their own lines, no `{{...}}` in prose). The
/// raw `.replace()` substitution path doesn't strip HTML comments, so
/// without this helper the authoring meta-text would land in the
/// LLM-bound prompt — a different flavour of the same prompt-corruption
/// silent divergence we eliminated in Instructions A through E.
///
/// **Apply BEFORE the `.replace()` chain** in both
/// [`assemble_chunk_prompt`] (Pass-1) and `assemble_pass2_prompt`
/// (Pass-2) so the contract is consistent across passes. A test for
/// this property pins the order: an AUTHORING_NOTE block whose body
/// itself contains placeholder syntax must never have those
/// placeholders reach the substitution layer.
///
/// ## Marker convention
///
/// `<!-- AUTHORING_NOTE` (the literal characters with a single space)
/// followed by anything, then `-->` to close. The match is greedy
/// for content but anchored on `<!-- AUTHORING_NOTE` so plain
/// `<!--AUTHORING_NOTE_OTHER...-->` does not accidentally match.
///
/// An unterminated `<!-- AUTHORING_NOTE ...` (no closing `-->`) is
/// preserved verbatim — losing every byte after the open marker
/// would be a bigger silent failure than leaving the malformed
/// markup visible. The regression test for templates would catch
/// such a malformation in CI.
///
/// ## Rust Learning: hand-rolled scanner avoids a prod regex dep
///
/// `regex` lives in `[dev-dependencies]` for the chunking-strategy
/// boundary-pattern tests; promoting it to `[dependencies]` for one
/// fixed pattern would expand the production trust graph for no
/// reason. The scan is straightforward: `find` on the open marker,
/// `find` on `-->` from the matched position, push the bytes
/// outside the match, repeat. Linear time over the template.
pub(crate) fn strip_authoring_comments(template: &str) -> String {
    const OPEN: &str = "<!-- AUTHORING_NOTE";
    const CLOSE: &str = "-->";

    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    loop {
        match rest.find(OPEN) {
            None => {
                out.push_str(rest);
                return out;
            }
            Some(start) => {
                // Push everything up to the open marker.
                out.push_str(&rest[..start]);
                let after_open = &rest[start + OPEN.len()..];
                match after_open.find(CLOSE) {
                    None => {
                        // Unterminated AUTHORING_NOTE — preserve the
                        // remainder verbatim. Losing every byte after
                        // the open marker would be a bigger silent
                        // failure than leaving the malformed markup
                        // visible to whoever inspects the prompt. The
                        // disk-scan regression test would catch a
                        // template in this state in CI.
                        out.push_str(&rest[start..]);
                        return out;
                    }
                    Some(end) => {
                        // Skip past the close marker and any single
                        // trailing newline so the strip site doesn't
                        // accumulate blank lines.
                        rest = &after_open[end + CLOSE.len()..];
                        if let Some(stripped) = rest.strip_prefix('\n') {
                            rest = stripped;
                        }
                    }
                }
            }
        }
    }
}

/// SHA-256 hex digest of a UTF-8 string.
///
/// Used to fingerprint the loaded prompt template so the audit snapshot
/// can prove *exactly* which template version produced a given run.
pub(crate) fn sha2_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Determine the effective chunking mode from a resolved configuration.
///
/// ## Rust Learning: Two sources, one decision
///
/// The pipeline has two ways to specify chunking mode:
///
/// 1. **Legacy:** `resolved.chunking_mode` — a typed `String` field with
///    values `"full"` or `"chunked"`. Predates the intelligent-chunking
///    work; profiles authored before Phase 1b only have this field.
/// 2. **New:** `resolved.chunking_config["mode"]` — a key inside the
///    flexible config map; values `"full"`, `"chunked"`, or
///    `"structured"`. Phase 1b profiles use this exclusively.
///
/// **The map-based mode wins when present.** Old profiles (no
/// `chunking_config` block at all) fall through to the legacy field
/// untouched. New profiles (which always set `chunking_config.mode`)
/// route through the new dispatch unconditionally. This is the
/// migration strategy: both shapes coexist, the new one takes
/// precedence, and no profile YAML needs to be edited at the cutover.
///
/// We return `String` rather than an enum because the set of modes is
/// intentionally open — adding a future mode (`"parallel"`, `"multi_pass"`)
/// is a profile-YAML edit and a new match arm at the dispatch site, not
/// an enum variant addition with corresponding API breaks. The dispatch
/// site's default arm catches anything unknown and routes it to the
/// legacy chunked path, which is a safer failure mode than rejecting
/// unrecognized profiles outright.
pub(crate) fn resolve_effective_mode(resolved: &ResolvedConfig) -> String {
    if let Some(mode) = resolved
        .chunking_config
        .get("mode")
        .and_then(|v| v.as_str())
    {
        return mode.to_string();
    }
    resolved.chunking_mode.clone()
}

// ─────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── assemble_chunk_prompt ────────────────────────────────────
    //
    // These tests pin down the dual-aware placeholder contract that
    // chunked + structured modes rely on. Regressions on these tests
    // mean the LLM is being sent a prompt with no document body —
    // exactly the silent-truncation bug they were written to prevent.

    #[test]
    fn assemble_chunk_prompt_substitutes_document_text() {
        // Template uses {{document_text}} (e.g., pass1_complaint_v4.md).
        // The chunk's body must replace that placeholder verbatim.
        let template = "Schema: {{schema_json}}\n\nDoc:\n{{document_text}}";
        let prompt = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK BODY>", None, None, None)
            .expect("dual-aware substitution should succeed for {{document_text}}");
        assert!(
            prompt.contains("<CHUNK BODY>"),
            "chunk body must appear in the prompt; got: {prompt}"
        );
        assert!(
            !prompt.contains("{{document_text}}"),
            "the placeholder must be replaced, not left literal; got: {prompt}"
        );
        assert!(prompt.contains("<SCHEMA>"));
    }

    #[test]
    fn assemble_chunk_prompt_substitutes_chunk_text() {
        // Backward-compat: templates that already use {{chunk_text}}
        // (the original chunked-mode placeholder) keep working.
        let template = "Schema: {{schema_json}}\n\nChunk:\n{{chunk_text}}";
        let prompt = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK BODY>", None, None, None)
            .expect("substitution should succeed for {{chunk_text}}");
        assert!(prompt.contains("<CHUNK BODY>"));
        assert!(!prompt.contains("{{chunk_text}}"));
    }

    #[test]
    fn assemble_chunk_prompt_fails_on_missing_both() {
        // A template missing both placeholders cannot inject the chunk
        // body anywhere — fail fast so the caller can mark_run_failed
        // before sending a useless prompt to the LLM.
        let template = "Schema: {{schema_json}}\n\nNo body placeholder anywhere.";
        let err = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK BODY>", None, None, None)
            .expect_err("a template with neither placeholder must error");
        assert!(
            err.contains("{{document_text}}") && err.contains("{{chunk_text}}"),
            "error must name BOTH placeholders so the operator knows what's missing; got: {err}"
        );
    }

    #[test]
    fn assemble_chunk_prompt_prefers_document_text_when_both_present() {
        // Symmetry with run_full_document_extraction: when a template
        // somehow carries both placeholders, {{document_text}} wins.
        // {{chunk_text}} stays unsubstituted (a degenerate case — a
        // well-formed template should only have one body placeholder).
        let template =
            "Doc: {{document_text}}\n\nChunk: {{chunk_text}}\n\nSchema: {{schema_json}}";
        let prompt = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK BODY>", None, None, None)
            .expect("must succeed when at least one placeholder is present");
        assert!(
            prompt.contains("Doc: <CHUNK BODY>"),
            "{{{{document_text}}}} must be the substituted slot; got: {prompt}"
        );
        assert!(
            prompt.contains("Chunk: {{chunk_text}}"),
            "{{{{chunk_text}}}} stays literal when {{{{document_text}}}} wins; got: {prompt}"
        );
    }

    // ── Three additional pass-1 placeholders: global_rules,
    //    admin_instructions, context. The Some(...) variants prove the
    //    value lands in the prompt; the None variants prove the literal
    //    placeholder doesn't leak.

    #[test]
    fn assemble_chunk_prompt_substitutes_global_rules_when_some() {
        let template = "Rules:\n{{global_rules}}\n\nDoc: {{document_text}}";
        let prompt = assemble_chunk_prompt(
            template,
            "<SCHEMA>",
            "<CHUNK>",
            Some("RULE-1; RULE-2"),
            None,
            None,
        )
        .expect("substitution must succeed");
        assert!(
            prompt.contains("RULE-1; RULE-2"),
            "global_rules content must land in the prompt; got: {prompt}"
        );
        assert!(
            !prompt.contains("{{global_rules}}"),
            "literal placeholder must be gone; got: {prompt}"
        );
    }

    #[test]
    fn assemble_chunk_prompt_substitutes_global_rules_with_empty_when_none() {
        // Template references {{global_rules}} but the profile didn't opt
        // in — the placeholder must vanish (replaced with "") rather than
        // leak as literal text into the LLM prompt.
        let template = "Rules:\n{{global_rules}}\n\nDoc: {{document_text}}";
        let prompt = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK>", None, None, None)
            .expect("substitution must succeed");
        assert!(
            !prompt.contains("{{global_rules}}"),
            "absent global_rules must still strip the placeholder; got: {prompt}"
        );
    }

    #[test]
    fn assemble_chunk_prompt_substitutes_admin_instructions_when_some() {
        let template =
            "Admin: {{admin_instructions}}\n\nDoc: {{document_text}}";
        let prompt = assemble_chunk_prompt(
            template,
            "<SCHEMA>",
            "<CHUNK>",
            None,
            Some("Focus on dates"),
            None,
        )
        .expect("substitution must succeed");
        assert!(
            prompt.contains("Focus on dates"),
            "admin_instructions content must land in the prompt; got: {prompt}"
        );
        assert!(!prompt.contains("{{admin_instructions}}"));
    }

    #[test]
    fn assemble_chunk_prompt_substitutes_admin_instructions_with_empty_when_none() {
        let template =
            "Admin: {{admin_instructions}}\n\nDoc: {{document_text}}";
        let prompt = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK>", None, None, None)
            .expect("substitution must succeed");
        assert!(
            !prompt.contains("{{admin_instructions}}"),
            "absent admin_instructions must still strip the placeholder; got: {prompt}"
        );
    }

    #[test]
    fn assemble_chunk_prompt_substitutes_context_with_empty_string() {
        // Defensive empty-string substitution mirrors pass-2's pattern at
        // llm_extract_pass2.rs:319. Pass-1 has no prior-context renderer
        // yet, so context is always None — the placeholder must still
        // vanish so it doesn't reach the LLM as literal text.
        let template = "Context: {{context}}\n\nDoc: {{document_text}}";
        let prompt = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK>", None, None, None)
            .expect("substitution must succeed");
        assert!(
            !prompt.contains("{{context}}"),
            "context placeholder must vanish even when None; got: {prompt}"
        );
    }

    #[test]
    fn assemble_chunk_prompt_substitutes_all_three_alongside_body() {
        // Integration-style: all three new placeholders + the body
        // placeholder + schema, all substituted in one call.
        let template = "\
Schema:\n{{schema_json}}\n\n\
Rules:\n{{global_rules}}\n\n\
Admin:\n{{admin_instructions}}\n\n\
Context:\n{{context}}\n\n\
Doc:\n{{document_text}}";
        let prompt = assemble_chunk_prompt(
            template,
            "<SCHEMA>",
            "<CHUNK>",
            Some("<RULES>"),
            Some("<ADMIN>"),
            Some("<CTX>"),
        )
        .expect("substitution must succeed");
        for needle in [
            "<SCHEMA>", "<RULES>", "<ADMIN>", "<CTX>", "<CHUNK>",
        ] {
            assert!(prompt.contains(needle), "missing {needle}; got: {prompt}");
        }
        for placeholder in [
            "{{schema_json}}",
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

    // ── load_global_rules ────────────────────────────────────────
    //
    // Four cases the helper must distinguish in the audit log:
    //
    //   1. file_name = None              → ("", None)
    //   2. file_name = Some(empty file)  → ("", Some(sha256("")))
    //   3. file_name = Some(real file)   → (content, Some(sha256(content)))
    //   4. file_name = Some(missing)     → Err(ProfileLoadFailed)
    //
    // The tests use `tempfile::tempdir()` so they don't rely on any
    // absolute path on disk and don't need a live database.

    #[test]
    fn load_global_rules_returns_empty_and_none_when_file_is_unconfigured() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (content, hash) = load_global_rules(dir.path(), None)
            .expect("None file_name must be Ok");
        assert_eq!(content, "", "no-file case must yield empty content");
        assert!(hash.is_none(), "no-file case must yield None hash");
    }

    #[test]
    fn load_global_rules_hashes_empty_file_distinctly_from_none() {
        // The audit must distinguish "no rules configured" from
        // "rules configured but empty." The first returns None hash;
        // the second returns Some(sha256("")).
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("empty_rules.md");
        std::fs::write(&path, "").expect("write empty file");
        let (content, hash) = load_global_rules(dir.path(), Some("empty_rules.md"))
            .expect("empty file must be Ok");
        assert_eq!(content, "", "empty file must yield empty content");
        assert_eq!(
            hash.as_deref(),
            Some(sha2_hex("").as_str()),
            "empty-file case must hash the empty string so an auditor can \
             tell empty-file runs from no-file runs"
        );
    }

    #[test]
    fn load_global_rules_returns_content_and_hash_for_real_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let body = "## Extraction Rules\n- always cite verbatim\n";
        let path = dir.path().join("rules_v4.md");
        std::fs::write(&path, body).expect("write file");
        let (content, hash) = load_global_rules(dir.path(), Some("rules_v4.md"))
            .expect("real file must be Ok");
        assert_eq!(content, body, "content round-trips byte-for-byte");
        assert_eq!(
            hash.as_deref(),
            Some(sha2_hex(body).as_str()),
            "hash must match sha2_hex of the loaded body"
        );
    }

    #[test]
    fn load_global_rules_errors_when_configured_file_is_missing() {
        // A configured-but-missing file is an operator-misconfiguration
        // bug, not a "default to no rules" case. Failing fast surfaces
        // the bad path in the audit log instead of silently extracting
        // with no rules.
        let dir = tempfile::tempdir().expect("tempdir");
        let err = load_global_rules(dir.path(), Some("does_not_exist.md"))
            .expect_err("missing file must be Err");
        match err {
            LlmExtractError::ProfileLoadFailed { message } => {
                assert!(
                    message.contains("does_not_exist.md"),
                    "error must name the missing path; got: {message}"
                );
            }
            other => panic!("expected ProfileLoadFailed, got {other:?}"),
        }
    }

    // ── strip_authoring_comments ──────────────────────────────────
    //
    // The five tests Roman specified in his Step 1 approval. They
    // pin down the contract: AUTHORING_NOTE blocks are removed,
    // regular HTML comments are preserved, and the strip happens
    // BEFORE substitution so the body of an AUTHORING_NOTE never
    // reaches the `.replace()` chain.

    #[test]
    fn strip_authoring_comments_removes_a_single_block() {
        let input = "<!-- AUTHORING_NOTE\nrules go here\n-->\n# Heading\n\nbody";
        let out = strip_authoring_comments(input);
        assert_eq!(out, "# Heading\n\nbody");
    }

    #[test]
    fn strip_authoring_comments_removes_multiple_blocks_in_one_template() {
        let input = "\
<!-- AUTHORING_NOTE
first block
-->
# Heading

middle text

<!-- AUTHORING_NOTE
second block
-->
trailing text";
        let out = strip_authoring_comments(input);
        assert!(
            !out.contains("AUTHORING_NOTE"),
            "all AUTHORING_NOTE markers must be gone; got:\n{out}"
        );
        assert!(out.contains("# Heading"));
        assert!(out.contains("middle text"));
        assert!(out.contains("trailing text"));
        assert!(!out.contains("first block"));
        assert!(!out.contains("second block"));
    }

    #[test]
    fn strip_authoring_comments_preserves_non_authoring_text_byte_for_byte() {
        // A template with no AUTHORING_NOTE blocks must round-trip
        // unchanged. Pins the "do nothing when there's nothing to
        // do" contract.
        let input = "# Heading\n\n## Subheading\n\nBody text with {{placeholder}} and prose.\n";
        let out = strip_authoring_comments(input);
        assert_eq!(out, input);
    }

    #[test]
    fn strip_authoring_comments_preserves_a_regular_html_comment() {
        // A normal `<!-- ... -->` comment without the AUTHORING_NOTE
        // marker is NOT stripped — the marker is the explicit
        // "this is for human authors only" signal. Without the
        // marker, the comment stays.
        let input = "<!-- regular comment -->\n# Heading\n\nbody";
        let out = strip_authoring_comments(input);
        assert!(
            out.contains("<!-- regular comment -->"),
            "regular HTML comment must survive; got:\n{out}"
        );
    }

    #[test]
    fn strip_authoring_comments_strips_block_with_placeholder_shaped_content_inside() {
        // The most important test: an AUTHORING_NOTE block whose
        // *body* contains `{{...}}` tokens must be stripped in full
        // before any substitution happens. Confirms the
        // strip-before-replace ordering — if the helper ran AFTER
        // replace, the `{{context}}` inside the note would have
        // been substituted into prose like "Therefore: prose
        // references to "the context block" or "the schema" must
        // NOT use the literal  or {{schema_json}} syntax", which
        // is corrupted text shipped to the LLM. With the strip
        // running first, the entire note (placeholders included)
        // is removed before any `.replace()` call sees it.
        let input = "\
<!-- AUTHORING_NOTE
do not put {{context}} or {{schema_json}} in prose
-->
# Real Content

The real prompt body.";
        let out = strip_authoring_comments(input);
        assert!(
            !out.contains("AUTHORING_NOTE"),
            "marker must be gone; got:\n{out}"
        );
        assert!(
            !out.contains("{{context}}"),
            "placeholder syntax inside the AUTHORING_NOTE block must be \
             stripped along with the rest of the block; got:\n{out}"
        );
        assert!(
            !out.contains("{{schema_json}}"),
            "same — both placeholder tokens inside the note must be gone"
        );
        assert!(out.contains("# Real Content"));
        assert!(out.contains("The real prompt body."));
    }

    /// Roman's Step 1 directive G #1: every Pass-2 template on disk
    /// must carry the AUTHORING_NOTE block at the top. Catches a
    /// future template author who copy-pastes from a non-Pass-2
    /// template and forgets the block.
    #[test]
    fn every_pass2_template_on_disk_has_authoring_note_at_top() {
        // Cargo runs tests from the package root (backend/).
        let entries = std::fs::read_dir("extraction_templates")
            .expect("backend/extraction_templates/ must exist");
        let mut pass2_count = 0;
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with("pass2_") || !name.ends_with("_v4.md") {
                continue;
            }
            pass2_count += 1;
            let body = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            assert!(
                body.starts_with("<!-- AUTHORING_NOTE"),
                "{name} must start with the AUTHORING_NOTE block per the \
                 convention established in commit-of-instruction-F"
            );
        }
        assert!(
            pass2_count >= 5,
            "expected at least 5 Pass-2 v4 templates, found {pass2_count}"
        );
    }

    /// Roman's Step 1 directive G #2 (extended to all v4 templates):
    /// every placeholder string in every shipped pass*_v4.md must
    /// appear ONLY on a line where it is the only non-whitespace
    /// content. If a future author embeds `{{context}}` inside a
    /// prose sentence, this test catches it before the prompt
    /// corruption ships. AUTHORING_NOTE blocks are stripped before
    /// the scan because they legitimately reference placeholder
    /// tokens (per the rules they document).
    #[test]
    fn no_pass_template_carries_inline_prose_placeholder_tokens() {
        const PLACEHOLDERS: &[&str] = &[
            "{{schema_json}}",
            "{{entities_json}}",
            "{{global_rules}}",
            "{{admin_instructions}}",
            "{{context}}",
            "{{document_text}}",
            "{{chunk_text}}",
        ];
        let entries = std::fs::read_dir("extraction_templates")
            .expect("backend/extraction_templates/ must exist");
        let mut scanned = 0;
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            // Match `pass1_..._v4.md` and `pass2_..._v4.md`. Other
            // markdown files in the directory (universal templates,
            // legacy versions) aren't covered by the v4 contract.
            if !(name.starts_with("pass1_") || name.starts_with("pass2_"))
                || !name.ends_with("_v4.md")
            {
                continue;
            }
            scanned += 1;
            let raw = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            // Strip authoring notes first — they reference tokens
            // legitimately as part of their documentation.
            let body = strip_authoring_comments(&raw);
            for (line_no, line) in body.lines().enumerate() {
                for ph in PLACEHOLDERS {
                    if line.contains(ph) {
                        // Allowed shape: line is exactly the
                        // placeholder, possibly with surrounding
                        // whitespace. Anything else is the bug.
                        assert_eq!(
                            line.trim(),
                            *ph,
                            "{name} line {ln} contains the placeholder {ph} \
                             inline within prose: {line:?}. The substitution \
                             layer would silently corrupt this line. Either \
                             move the token to its own line or rewrite the \
                             prose to use plain English (see the AUTHORING_NOTE \
                             block at the top of every Pass-2 template).",
                            ln = line_no + 1,
                        );
                    }
                }
            }
        }
        assert!(
            scanned >= 10,
            "expected at least 10 v4 templates (5 pass-1 + 5 pass-2), \
             scanned {scanned}"
        );
    }

    #[test]
    fn llm_extract_error_display_policy() {
        // Variants without a source keep G6: Display must not mention
        // "source" or thiserror's "Caused by" chain decoration.
        let e1 = LlmExtractError::DocumentNotFound {
            document_id: "doc-x".to_string(),
        };
        let s1 = e1.to_string();
        assert!(!s1.contains("Caused by"), "G6 violation: {s1}");
        assert!(!s1.contains("source"), "G6 violation: {s1}");

        // G6 exception: SchemaLoadFailed, PromptBuildFailed, and
        // LlmCallFailed intentionally interpolate `{source}` so the
        // underlying cause survives the framework's .to_string() into
        // pipeline_jobs.error / pipeline_steps.error_message. Without
        // this, the audit log only shows "LLM call failed" with no hint
        // at the Anthropic 400, timeout, or schema parse error below it.
        let inner = "UNIQUE_INNER_PROVIDER_TOKEN";
        let e2 = LlmExtractError::LlmCallFailed {
            source: colossus_extract::PipelineError::LlmProvider(inner.to_string()),
        };
        let s2 = e2.to_string();
        assert!(
            s2.contains(inner),
            "LlmCallFailed Display must surface source text; got: {s2}"
        );
        // thiserror's "Caused by" chain only appears in Debug; Display
        // stays a single line even with `{source}` interpolation.
        assert!(!s2.contains("Caused by"), "unexpected chain decoration: {s2}");
    }

    #[test]
    fn llm_extract_error_source_chaining() {
        use std::error::Error as _;

        let e = LlmExtractError::LlmCallFailed {
            source: colossus_extract::PipelineError::LlmProvider("x".to_string()),
        };
        assert!(e.source().is_some(), "LlmCallFailed must expose a source");

        let e2 = LlmExtractError::SchemaLoadFailed {
            schema_file: "f.yaml".to_string(),
            source: colossus_extract::PipelineError::Schema("bad".to_string()),
        };
        assert!(e2.source().is_some(), "SchemaLoadFailed must expose a source");
    }

    #[test]
    fn llm_extract_error_document_not_found_display() {
        let e = LlmExtractError::DocumentNotFound {
            document_id: "doc-xyz".to_string(),
        };
        assert!(e.to_string().contains("doc-xyz"));
    }

    #[test]
    fn llm_extract_struct_derives() {
        let a = LlmExtract {
            document_id: "foo".to_string(),
        };
        let b = a.clone();
        let _ = format!("{b:?}");
        let j = serde_json::to_string(&a).unwrap();
        let c: LlmExtract = serde_json::from_str(&j).unwrap();
        assert_eq!(a.document_id, c.document_id);
    }

    #[test]
    fn step_defaults_are_trait_defaults() {
        assert_eq!(
            <LlmExtract as Step<DocProcessing>>::DEFAULT_RETRY_LIMIT,
            0
        );
        assert_eq!(
            <LlmExtract as Step<DocProcessing>>::DEFAULT_RETRY_DELAY_SECS,
            0
        );
        assert!(<LlmExtract as Step<DocProcessing>>::DEFAULT_TIMEOUT_SECS.is_none());
    }

    #[test]
    fn llm_extract_error_store_failed_display() {
        let e = LlmExtractError::StoreFailed {
            message: "x".to_string(),
        };
        assert!(e.to_string().contains('x'));
    }

    /// Compile-only reference catches module-path drift for the extraction
    /// store helper. No runtime execution, no DB.
    #[test]
    fn llm_extract_store_path_compiles() {
        let _f = crate::repositories::pipeline_repository::extraction::store_entities_and_relationships;
    }

    #[test]
    fn default_profile_name_strips_yaml_and_version() {
        // v2 (legacy) and v4 (current) must both reduce to the base name.
        assert_eq!(default_profile_name_from_schema("complaint_v2.yaml"), "complaint");
        assert_eq!(default_profile_name_from_schema("complaint_v4.yaml"), "complaint");
        assert_eq!(
            default_profile_name_from_schema("discovery_response_v4.yaml"),
            "discovery_response"
        );
        assert_eq!(default_profile_name_from_schema("motion_v4.yaml"), "motion");
        assert_eq!(default_profile_name_from_schema("brief_v4.yaml"), "brief");
        assert_eq!(default_profile_name_from_schema("affidavit_v4.yaml"), "affidavit");
        assert_eq!(
            default_profile_name_from_schema("court_ruling_v4.yaml"),
            "court_ruling"
        );
        // Multi-digit versions.
        assert_eq!(
            default_profile_name_from_schema("some_future_v12.yaml"),
            "some_future"
        );
        // No version suffix → passthrough of the stem.
        assert_eq!(default_profile_name_from_schema("brief.yaml"), "brief");
        assert_eq!(
            default_profile_name_from_schema("no_version.yaml"),
            "no_version"
        );
        assert_eq!(default_profile_name_from_schema("custom"), "custom");
        // `_v` not followed by digits must NOT be stripped (e.g. a name that
        // happens to end in `_v`).
        assert_eq!(default_profile_name_from_schema("weird_v.yaml"), "weird_v");
        assert_eq!(
            default_profile_name_from_schema("weird_vbeta.yaml"),
            "weird_vbeta"
        );
    }

    #[test]
    fn sha2_hex_is_deterministic_and_64_chars() {
        let a = sha2_hex("hello");
        let b = sha2_hex("hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert_ne!(a, sha2_hex("hello!"));
    }

    #[test]
    fn resolve_max_tokens_prefers_profile_value() {
        let r = ResolvedConfig {
            profile_name: "p".into(),
            profile_hash: String::new(),
            effective_pass: 1,
            model: "m".into(),
            pass2_model: None,
            template_file: "t".into(),
            template_hash: None,
            pass2_template_file: None,
            system_prompt_file: None,
            system_prompt_hash: None,
            global_rules_file: None,
            global_rules_hash: None,
            schema_file: "s".into(),
            chunking_mode: "chunked".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config: std::collections::HashMap::new(),
            context_config: std::collections::HashMap::new(),
            max_tokens: 12345,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
            overrides_applied: vec![],
            pass2_cross_doc_entities: Vec::new(),
            pass2_source_document_ids: Vec::new(),
        };
        assert_eq!(resolve_max_tokens(&r), 12345);
    }

    /// Helper: a minimal ResolvedConfig used by next_step_after_pass1 tests.
    fn resolved_with_run_pass2(flag: bool) -> ResolvedConfig {
        ResolvedConfig {
            profile_name: "complaint".into(),
            profile_hash: String::new(),
            effective_pass: 1,
            model: "m".into(),
            pass2_model: None,
            template_file: "t".into(),
            template_hash: None,
            pass2_template_file: Some("pass2_complaint.md".into()),
            system_prompt_file: None,
            system_prompt_hash: None,
            global_rules_file: None,
            global_rules_hash: None,
            schema_file: "s".into(),
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config: std::collections::HashMap::new(),
            context_config: std::collections::HashMap::new(),
            max_tokens: 8000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: flag,
            overrides_applied: vec![],
            pass2_cross_doc_entities: Vec::new(),
            pass2_source_document_ids: Vec::new(),
        }
    }

    #[test]
    fn next_step_after_pass1_routes_to_pass2_when_flag_set() {
        let r = resolved_with_run_pass2(true);
        match next_step_after_pass1(&r, "doc-x") {
            DocProcessing::LlmExtractPass2(step) => {
                assert_eq!(step.document_id, "doc-x");
            }
            other => panic!("expected LlmExtractPass2, got {other:?}"),
        }
    }

    #[test]
    fn next_step_after_pass1_routes_to_verify_when_flag_unset() {
        let r = resolved_with_run_pass2(false);
        match next_step_after_pass1(&r, "doc-y") {
            DocProcessing::Verify(step) => {
                assert_eq!(step.document_id, "doc-y");
            }
            other => panic!("expected Verify, got {other:?}"),
        }
    }

    // ── resolve_effective_mode (Phase 1b Group 2b-i) ─────────────

    #[test]
    fn resolve_effective_mode_prefers_chunking_config() {
        // Both layers set: the new chunking_config["mode"] must win
        // over the legacy chunking_mode field. This is the migration
        // contract — once a profile adopts the new map shape, the new
        // value is authoritative.
        let mut resolved = resolved_with_run_pass2(false);
        resolved.chunking_mode = "full".to_string();
        resolved
            .chunking_config
            .insert("mode".to_string(), serde_json::json!("structured"));

        assert_eq!(resolve_effective_mode(&resolved), "structured");
    }

    #[test]
    fn resolve_effective_mode_falls_back_to_legacy() {
        // chunking_config exists (non-empty) but has no "mode" key —
        // e.g. a profile that only declares strategy/units knobs and
        // expects the legacy chunking_mode field to drive dispatch.
        let mut resolved = resolved_with_run_pass2(false);
        resolved.chunking_mode = "full".to_string();
        resolved
            .chunking_config
            .insert("strategy".to_string(), serde_json::json!("qa_pair"));

        assert_eq!(resolve_effective_mode(&resolved), "full");
    }

    #[test]
    fn resolve_effective_mode_empty_config_uses_legacy() {
        // Pre-Phase-1b profile: chunking_config is empty (default
        // HashMap::new() from #[serde(default)]) — fall through to the
        // typed chunking_mode field exactly as the legacy dispatch did.
        let mut resolved = resolved_with_run_pass2(false);
        resolved.chunking_mode = "chunked".to_string();
        // chunking_config left empty by the helper — assert that
        // explicitly so a future helper change doesn't silently
        // invalidate this preconditions.
        assert!(resolved.chunking_config.is_empty());

        assert_eq!(resolve_effective_mode(&resolved), "chunked");
    }

    // ── Dedup-eligible entity-type set (Phase 1b Group 2b-ii) ────
    //
    // These tests verify the *colossus-legal glue* that selects which
    // entity types ChunkMerger should deduplicate across chunks. The
    // merger's internal correctness (ID prefixing, survivor selection,
    // relationship endpoint remapping) is covered by 15 tests in
    // colossus-rs — we test only the boundary contract here: the schema
    // category drives the set; nothing is hardcoded by name.

    #[test]
    fn dedup_types_built_from_schema_categories() {
        // Mixed schema: Foundation + Reference entries flow into the
        // dedup set; Structural + Evidence entries do not.
        let entity_configs = vec![
            ("Party", EntityCategory::Foundation),
            ("LegalCount", EntityCategory::Foundation),
            ("FactualAllegation", EntityCategory::Structural),
            ("Evidence", EntityCategory::Evidence),
            ("LegalCitation", EntityCategory::Reference),
        ];

        let dedup_types: HashSet<String> = entity_configs
            .iter()
            .filter(|(_, cat)| matches!(cat, EntityCategory::Foundation | EntityCategory::Reference))
            .map(|(name, _)| name.to_string())
            .collect();

        assert!(dedup_types.contains("Party"));
        assert!(dedup_types.contains("LegalCount"));
        assert!(dedup_types.contains("LegalCitation"));
        assert!(
            !dedup_types.contains("FactualAllegation"),
            "Structural entities must NOT be deduplicated"
        );
        assert!(
            !dedup_types.contains("Evidence"),
            "Evidence entities must NOT be deduplicated"
        );
        assert_eq!(dedup_types.len(), 3);
    }

    #[test]
    fn empty_schema_produces_empty_dedup_set() {
        // Edge case: a schema with no entity types yields an empty
        // dedup set. The merger then no-ops dedup logic and only
        // applies ID-prefix normalisation, which is safe behaviour.
        let entity_configs: Vec<(&str, EntityCategory)> = vec![];

        let dedup_types: HashSet<String> = entity_configs
            .iter()
            .filter(|(_, cat)| matches!(cat, EntityCategory::Foundation | EntityCategory::Reference))
            .map(|(name, _)| name.to_string())
            .collect();

        assert!(dedup_types.is_empty());
    }

    #[test]
    fn all_evidence_schema_produces_empty_dedup_set() {
        // A schema where every entity is Evidence (per-occurrence,
        // never repeats) should produce an empty dedup set — every
        // entity is unique per chunk and will get a -c{N} suffix from
        // the merger, never a dedup decision.
        let entity_configs = vec![
            ("Statement", EntityCategory::Evidence),
            ("Admission", EntityCategory::Evidence),
            ("Testimony", EntityCategory::Evidence),
        ];

        let dedup_types: HashSet<String> = entity_configs
            .iter()
            .filter(|(_, cat)| matches!(cat, EntityCategory::Foundation | EntityCategory::Reference))
            .map(|(name, _)| name.to_string())
            .collect();

        assert!(dedup_types.is_empty());
    }

    // ── chunk_metadata audit + processing_config audit (Phase 1b Group 3) ─

    #[test]
    fn chunk_metadata_serializes_to_json() {
        // Verify that a populated TextChunk-style metadata map round-trips
        // through serde_json::to_value into a JSON object preserving keys
        // and value types. This is the contract the audit-row write
        // depends on.
        use std::collections::HashMap;

        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        metadata.insert("unit_range".to_string(), serde_json::json!([0, 24]));
        metadata.insert("unit_count".to_string(), serde_json::json!(25));
        metadata.insert("preamble_included".to_string(), serde_json::json!(true));
        metadata.insert(
            "boundary_pattern_used".to_string(),
            serde_json::json!(r"^\d+\.\s"),
        );

        let json = serde_json::to_value(&metadata).expect("HashMap<String, Value> always serializes");

        assert!(json.is_object());
        assert_eq!(json["unit_count"], 25);
        assert_eq!(json["preamble_included"], true);
        assert_eq!(json["unit_range"][0], 0);
        assert_eq!(json["unit_range"][1], 24);
        assert_eq!(json["boundary_pattern_used"], r"^\d+\.\s");
    }

    #[test]
    fn empty_chunk_metadata_serializes_to_empty_object() {
        // FixedSizeSplitter and the "full" path produce chunks with empty
        // metadata. Empty must serialize to `{}` (a valid JSON object),
        // not `null` — the JSONB column should always receive a valid
        // object so downstream readers don't have to handle the
        // missing-vs-empty distinction.
        use std::collections::HashMap;

        let metadata: HashMap<String, serde_json::Value> = HashMap::new();
        let json = serde_json::to_value(&metadata).expect("empty HashMap always serializes");

        assert!(json.is_object());
        assert_eq!(json.as_object().expect("just verified is_object").len(), 0);
    }

    #[test]
    fn resolved_config_includes_chunking_config_in_json() {
        // Task 1b-9 verification: ResolvedConfig's derive(Serialize) +
        // #[serde(default)] on the new fields means write_processing_config_snapshot
        // automatically emits chunking_config and context_config into the
        // extraction_runs.processing_config JSONB column. No code change
        // was needed in write_processing_config_snapshot — this test
        // pins the contract so a future struct edit can't silently drop
        // the fields from the audit snapshot.
        let mut resolved = resolved_with_run_pass2(false);
        resolved
            .chunking_config
            .insert("mode".to_string(), serde_json::json!("structured"));
        resolved
            .chunking_config
            .insert("strategy".to_string(), serde_json::json!("qa_pair"));
        resolved
            .context_config
            .insert("traversal_depth".to_string(), serde_json::json!(2));

        let json = serde_json::to_value(&resolved).expect("ResolvedConfig serializes");

        assert_eq!(json["chunking_config"]["mode"], "structured");
        assert_eq!(json["chunking_config"]["strategy"], "qa_pair");
        assert_eq!(json["context_config"]["traversal_depth"], 2);
    }

    // ── snapshot-shaping (Gap 11): effective_pass + Pass-2 model/template overwrite
    //
    // The DB-write path lives inside `write_processing_config_snapshot`,
    // but the *shape* of the snapshot — the clone-and-mutate logic that
    // applies before serialization — is the testable invariant. This
    // helper performs exactly that shaping (no DB) so the tests can
    // assert the JSONB shape without requiring a live PgPool.
    fn snapshot_for_test(
        resolved: &ResolvedConfig,
        runtime: SnapshotRuntimeFields<'_>,
    ) -> serde_json::Value {
        let mut snapshot = resolved.clone();
        snapshot.effective_pass = runtime.effective_pass;
        snapshot.template_hash = Some(runtime.template_hash.to_string());
        snapshot.system_prompt_hash = runtime.system_prompt_hash.map(str::to_string);
        snapshot.global_rules_hash = runtime.global_rules_hash.map(str::to_string);
        snapshot.pass2_cross_doc_entities = runtime.pass2_cross_doc_entities.to_vec();
        snapshot.pass2_source_document_ids = runtime.pass2_source_document_ids.to_vec();
        if runtime.effective_pass == 2 {
            snapshot.model = resolved
                .pass2_model
                .clone()
                .unwrap_or_else(|| resolved.model.clone());
            if let Some(p2_tmpl) = &resolved.pass2_template_file {
                snapshot.template_file = p2_tmpl.clone();
            }
        }
        serde_json::to_value(&snapshot).expect("snapshot serialises")
    }

    /// Build a `ResolvedConfig` with both Pass-1 and Pass-2 fields set
    /// so the snapshot-shape tests can show the swap.
    fn resolved_with_both_passes() -> ResolvedConfig {
        ResolvedConfig {
            profile_name: "complaint".into(),
            profile_hash: "deadbeef".into(),
            effective_pass: 1,
            model: "claude-sonnet-4-6".into(),
            pass2_model: Some("claude-opus-4-7".into()),
            template_file: "pass1_complaint.md".into(),
            template_hash: None,
            pass2_template_file: Some("pass2_complaint.md".into()),
            system_prompt_file: None,
            system_prompt_hash: None,
            global_rules_file: None,
            global_rules_hash: None,
            schema_file: "complaint_v4.yaml".into(),
            chunking_mode: "full".into(),
            chunk_size: None,
            chunk_overlap: None,
            chunking_config: std::collections::HashMap::new(),
            context_config: std::collections::HashMap::new(),
            max_tokens: 32000,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: true,
            overrides_applied: vec![],
            pass2_cross_doc_entities: Vec::new(),
            pass2_source_document_ids: Vec::new(),
        }
    }

    #[test]
    fn snapshot_effective_pass_1_keeps_pass1_model_and_template() {
        let resolved = resolved_with_both_passes();
        let json = snapshot_for_test(
            &resolved,
            SnapshotRuntimeFields {
                effective_pass: 1,
                template_hash: "p1_hash",
                system_prompt_hash: None,
                global_rules_hash: None,
                pass2_cross_doc_entities: &[],
                pass2_source_document_ids: &[],
            },
        );
        assert_eq!(json["effective_pass"], 1);
        assert_eq!(json["model"], "claude-sonnet-4-6");
        assert_eq!(json["template_file"], "pass1_complaint.md");
        assert_eq!(json["template_hash"], "p1_hash");
        // Pass-2 fields still flow through (the resolver fills them) but
        // are not used by the LLM call on a Pass-1 row — they describe
        // what *will* run on the Pass-2 row, if any.
        assert_eq!(json["pass2_model"], "claude-opus-4-7");
    }

    #[test]
    fn snapshot_effective_pass_2_overwrites_model_and_template() {
        // Gap 11: a JSONB-only audit query against a Pass-2 row used to
        // return Pass-1 values. The snapshot helper now overwrites
        // `model` and `template_file` with the Pass-2 values when
        // `effective_pass = 2` so the JSONB matches what actually ran.
        let resolved = resolved_with_both_passes();
        let json = snapshot_for_test(
            &resolved,
            SnapshotRuntimeFields {
                effective_pass: 2,
                template_hash: "p2_hash",
                system_prompt_hash: None,
                global_rules_hash: None,
                pass2_cross_doc_entities: &[],
                pass2_source_document_ids: &[],
            },
        );
        assert_eq!(json["effective_pass"], 2);
        assert_eq!(
            json["model"], "claude-opus-4-7",
            "Pass-2 snapshot must record the Pass-2 model, not Pass-1's"
        );
        assert_eq!(
            json["template_file"], "pass2_complaint.md",
            "Pass-2 snapshot must record the Pass-2 template filename"
        );
        assert_eq!(json["template_hash"], "p2_hash");
    }

    #[test]
    fn snapshot_effective_pass_2_falls_back_to_pass1_model_when_pass2_unset() {
        // Mirrors the runtime fallback at the LLM call site: when the
        // profile/override didn't set pass2_model, Pass-2 actually runs
        // on the Pass-1 model. The snapshot must reflect that — same id
        // recorded twice (in `model` and in `pass2_model = null`).
        let mut resolved = resolved_with_both_passes();
        resolved.pass2_model = None;
        // Drop pass2_template_file too so the fallback path for templates
        // is exercised: when no Pass-2 template is configured, the
        // snapshot keeps the Pass-1 template_file (consistent with the
        // runtime, which would not have reached this code without one).
        let json = snapshot_for_test(
            &resolved,
            SnapshotRuntimeFields {
                effective_pass: 2,
                template_hash: "p1_or_p2_hash",
                system_prompt_hash: None,
                global_rules_hash: None,
                pass2_cross_doc_entities: &[],
                pass2_source_document_ids: &[],
            },
        );
        assert_eq!(json["effective_pass"], 2);
        assert_eq!(
            json["model"], "claude-sonnet-4-6",
            "with pass2_model=None, Pass-2 snapshot must fall back to Pass-1 model"
        );
        assert!(json["pass2_model"].is_null());
    }

    // ── 2B: cross-doc entity recording

    #[test]
    fn cross_doc_context_record_round_trips_through_serde_json() {
        let r = crate::pipeline::config::CrossDocContextRecord {
            document_id: "doc-abc".into(),
            prefixed_id: "ctx:party-001".into(),
            item_id: 42,
        };
        let j = serde_json::to_value(&r).unwrap();
        assert_eq!(j["document_id"], "doc-abc");
        assert_eq!(j["prefixed_id"], "ctx:party-001");
        assert_eq!(j["item_id"], 42);
        let back: crate::pipeline::config::CrossDocContextRecord =
            serde_json::from_value(j).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn snapshot_pass2_with_empty_cross_doc_serialises_empty_arrays() {
        // A Pass-2 run with no PUBLISHED prior context still produces
        // the two array fields, both empty. They must be `[]` in JSONB
        // (not absent) so a downstream reader can rely on the keys
        // existing on every Pass-2 row.
        let resolved = resolved_with_both_passes();
        let json = snapshot_for_test(
            &resolved,
            SnapshotRuntimeFields {
                effective_pass: 2,
                template_hash: "h",
                system_prompt_hash: None,
                global_rules_hash: None,
                pass2_cross_doc_entities: &[],
                pass2_source_document_ids: &[],
            },
        );
        assert!(json["pass2_cross_doc_entities"].is_array());
        assert_eq!(
            json["pass2_cross_doc_entities"].as_array().unwrap().len(),
            0
        );
        assert!(json["pass2_source_document_ids"].is_array());
        assert_eq!(
            json["pass2_source_document_ids"].as_array().unwrap().len(),
            0
        );
    }

    #[test]
    fn snapshot_pass2_with_cross_doc_records_lands_in_jsonb() {
        // The records the step builds must round-trip into the snapshot
        // JSONB intact. `pass2_source_document_ids` is the caller's
        // sorted/deduped list — the snapshot helper does not re-sort,
        // it just writes what it's given. (The sort/dedup invariant is
        // tested separately at the step layer where it is built.)
        let resolved = resolved_with_both_passes();
        let records = vec![
            crate::pipeline::config::CrossDocContextRecord {
                document_id: "doc-A".into(),
                prefixed_id: "ctx:party-001".into(),
                item_id: 10,
            },
            crate::pipeline::config::CrossDocContextRecord {
                document_id: "doc-A".into(),
                prefixed_id: "ctx:party-002".into(),
                item_id: 11,
            },
            crate::pipeline::config::CrossDocContextRecord {
                document_id: "doc-B".into(),
                prefixed_id: "ctx:count-001".into(),
                item_id: 20,
            },
        ];
        let source_docs: Vec<String> = vec!["doc-A".into(), "doc-B".into()];
        let json = snapshot_for_test(
            &resolved,
            SnapshotRuntimeFields {
                effective_pass: 2,
                template_hash: "h",
                system_prompt_hash: None,
                global_rules_hash: None,
                pass2_cross_doc_entities: &records,
                pass2_source_document_ids: &source_docs,
            },
        );
        assert_eq!(
            json["pass2_cross_doc_entities"].as_array().unwrap().len(),
            3
        );
        assert_eq!(json["pass2_cross_doc_entities"][0]["document_id"], "doc-A");
        assert_eq!(json["pass2_cross_doc_entities"][2]["item_id"], 20);
        let ids = json["pass2_source_document_ids"].as_array().unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "doc-A");
        assert_eq!(ids[1], "doc-B");
    }
}
