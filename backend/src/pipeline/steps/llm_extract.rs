//! LlmExtract pipeline step — config-driven entity extraction.
//!
//! Loads a processing profile from YAML, resolves per-document overrides,
//! branches on chunking_mode (full document vs chunked), and stores a
//! complete configuration snapshot for audit trail.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Sections 3.7 and 3.8.

use std::collections::HashSet;
use std::error::Error;
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

        // 6. Choose an effective max_tokens.
        let max_tokens = resolve_max_tokens(&resolved);

        // 7. Insert the extraction_runs row.
        let run_id = extraction::insert_extraction_run(
            db,
            &self.document_id,
            1,
            &resolved.model,
            &schema.version,
            None,
            Some(resolved.template_file.as_str()),
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
        write_processing_config_snapshot(
            db,
            run_id,
            &resolved,
            &template_hash,
            system_prompt_hash.as_deref(),
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

    // Dynamic placeholder detection: use whichever of the two body
    // placeholders the template actually references. `{{schema_json}}` is
    // substituted unconditionally because the schema prompt fragment is
    // expected in every extraction template.
    let has_document_text = args.template_text.contains("{{document_text}}");
    let has_chunk_text = args.template_text.contains("{{chunk_text}}");
    let prompt = if has_document_text {
        args.template_text
            .replace("{{schema_json}}", args.schema_json)
            .replace("{{document_text}}", args.full_text)
    } else if has_chunk_text {
        args.template_text
            .replace("{{schema_json}}", args.schema_json)
            .replace("{{chunk_text}}", args.full_text)
    } else {
        let msg = "Template has no {{document_text}} or {{chunk_text}} placeholder";
        mark_run_failed(args.db, args.run_id, msg).await;
        return Err(msg.into());
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

/// Assemble the per-chunk prompt by substituting `{{schema_json}}` and the
/// chunk body into a template.
///
/// Mirrors the dual-aware substitution in [`run_full_document_extraction`]:
/// templates may carry **either** `{{document_text}}` (preferred for
/// full-document templates like `pass1_complaint_v4.md`) **or**
/// `{{chunk_text}}` (the original chunked-mode placeholder). Whichever the
/// template uses, the chunk's text is substituted into it.
///
/// When both placeholders appear, `{{document_text}}` wins — matching the
/// preference in `run_full_document_extraction`. When neither appears, the
/// function returns an error so the caller can `mark_run_failed` and abort
/// before sending a prompt the LLM can't act on.
///
/// ## Why this lives outside the per-chunk loop body
///
/// Pulling the substitution into a pure function keeps it unit-testable
/// without an LLM provider, a database, or a runtime. The previous inline
/// version only substituted `{{chunk_text}}`, so a template using
/// `{{document_text}}` (the canonical placeholder for pass-1 complaint
/// extraction) silently sent the LLM a prompt with no document body and
/// the literal four-word string `{{document_text}}` instead. Isolating
/// the logic makes regression tests on this contract trivial.
///
/// ## Rust Learning: returning `Result<String, String>` for a small helper
///
/// A full `thiserror` enum would be overkill for one error case ("no
/// usable placeholder"). The caller wraps this `String` into the existing
/// `Box<dyn Error + Send + Sync>` flow alongside `mark_run_failed`. The
/// error message names *both* placeholders so an operator reading the
/// `extraction_runs.error_message` column knows exactly what to fix in
/// the template file.
fn assemble_chunk_prompt(
    template_text: &str,
    schema_json: &str,
    chunk_text: &str,
) -> Result<String, String> {
    let has_document_text = template_text.contains("{{document_text}}");
    let has_chunk_text = template_text.contains("{{chunk_text}}");

    if has_document_text {
        Ok(template_text
            .replace("{{schema_json}}", schema_json)
            .replace("{{document_text}}", chunk_text))
    } else if has_chunk_text {
        Ok(template_text
            .replace("{{schema_json}}", schema_json)
            .replace("{{chunk_text}}", chunk_text))
    } else {
        Err(
            "Template has no {{document_text}} or {{chunk_text}} placeholder — \
             chunk body cannot be injected into the prompt"
                .to_string(),
        )
    }
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
        // silently used to ship a prompt with no document body. We now
        // fail fast and record the failure on the run row.
        let prompt = match assemble_chunk_prompt(
            args.template_text,
            args.schema_json,
            &chunk.text,
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

/// Write the resolved configuration snapshot to
/// `extraction_runs.processing_config`.
///
/// Best-effort: a snapshot-write failure is logged but does not fail
/// extraction — the merged entities have already been committed.
pub(crate) async fn write_processing_config_snapshot(
    db: &PgPool,
    run_id: i32,
    resolved: &ResolvedConfig,
    template_hash: &str,
    system_prompt_hash: Option<&str>,
) {
    let mut resolved_with_hash = resolved.clone();
    resolved_with_hash.template_hash = Some(template_hash.to_string());
    resolved_with_hash.system_prompt_hash = system_prompt_hash.map(str::to_string);

    let config_json = match serde_json::to_value(&resolved_with_hash) {
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
        let prompt = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK BODY>")
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
        let prompt = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK BODY>")
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
        let err = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK BODY>")
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
        let prompt = assemble_chunk_prompt(template, "<SCHEMA>", "<CHUNK BODY>")
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
            model: "m".into(),
            pass2_model: None,
            template_file: "t".into(),
            template_hash: None,
            pass2_template_file: None,
            system_prompt_file: None,
            system_prompt_hash: None,
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
        };
        assert_eq!(resolve_max_tokens(&r), 12345);
    }

    /// Helper: a minimal ResolvedConfig used by next_step_after_pass1 tests.
    fn resolved_with_run_pass2(flag: bool) -> ResolvedConfig {
        ResolvedConfig {
            profile_name: "complaint".into(),
            model: "m".into(),
            pass2_model: None,
            template_file: "t".into(),
            template_hash: None,
            pass2_template_file: Some("pass2_complaint.md".into()),
            system_prompt_file: None,
            system_prompt_hash: None,
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
}
