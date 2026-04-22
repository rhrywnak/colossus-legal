//! LlmExtract pipeline step — config-driven entity extraction.
//!
//! Loads a processing profile from YAML, resolves per-document overrides,
//! branches on chunking_mode (full document vs chunked), and stores a
//! complete configuration snapshot for audit trail.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Sections 3.7 and 3.8.

use std::error::Error;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

use colossus_extract::{FixedSizeSplitter, LlmProvider, TextSplitter};
use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::pipeline::config::{resolve_config, ProcessingProfile, ResolvedConfig};
use crate::pipeline::context::AppContext;
use crate::pipeline::providers::provider_for_model;
use crate::pipeline::steps::llm_extract_helpers::{
    call_with_rate_limit_retry, mark_run_failed, parse_chunk_response,
};
use crate::pipeline::steps::verify::Verify;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository::{self, documents, extraction, models};

// ── Constants ───────────────────────────────────────────────────

/// Fallback max tokens per LLM call when neither the profile nor
/// step_config / env var provides one.
const DEFAULT_CHUNK_MAX_TOKENS: u32 = 8000;

/// Chunking-mode string recognised for single-call (no chunking) extraction.
const CHUNKING_MODE_FULL: &str = "full";

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
        // Best-effort: wipe any RUNNING/FAILED runs this step left behind.
        // A COMPLETED run is left intact — its rows are the authoritative
        // output and the idempotency check will short-circuit future retries.
        if let Err(e) = sqlx::query(
            "DELETE FROM extraction_runs WHERE document_id = $1 AND status IN ('RUNNING', 'FAILED')",
        )
        .bind(&self.document_id)
        .execute(db)
        .await
        {
            tracing::warn!(
                doc_id = %self.document_id, error = %e,
                "LlmExtract::on_cancel: delete of RUNNING/FAILED runs failed (non-fatal)"
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
        // 1. Idempotency: short-circuit if a COMPLETED run already exists.
        if extraction_already_complete(db, &self.document_id).await? {
            tracing::info!(
                document_id = %self.document_id,
                "Completed extraction run exists, skipping"
            );
            return Ok(StepResult::Next(DocProcessing::Verify(Verify {
                document_id: self.document_id.clone(),
            })));
        }

        // 2. Pipeline config + document + schema + text pages.
        let pipe_config = pipeline_repository::get_pipeline_config(db, &self.document_id)
            .await?
            .ok_or_else(|| LlmExtractError::NoPipelineConfig {
                document_id: self.document_id.clone(),
            })?;

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

        // 3. Resolve three-level config hierarchy.
        let overrides =
            pipeline_repository::get_pipeline_config_overrides(db, &self.document_id).await?;
        let profile_name = overrides
            .profile_name
            .clone()
            .unwrap_or_else(|| default_profile_name_from_schema(&pipe_config.schema_file));
        let profile = ProcessingProfile::load(&context.profile_dir, &profile_name)
            .map_err(|e| LlmExtractError::ProfileLoadFailed { message: e })?;
        let resolved = resolve_config(&profile, &overrides);

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
            .map_err(|_| LlmExtractError::SemaphoreClosed)?;

        // 10. Dispatch on chunking_mode.
        let system_prompt_ref = system_prompt.as_deref();
        let outcome = if resolved.chunking_mode == CHUNKING_MODE_FULL {
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
        } else {
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
            )
            .await?
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
            extraction::update_run_chunk_stats(db, run_id, total, ok, failed)
                .await
                .ok();
        }

        extraction::complete_extraction_run(
            db,
            run_id,
            &merged,
            Some(outcome.total_input_tokens as i32),
            Some(outcome.total_output_tokens as i32),
            cost_usd,
            "COMPLETED",
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

        Ok(StepResult::Next(DocProcessing::Verify(Verify {
            document_id: self.document_id.clone(),
        })))
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
    args.progress
        .report(serde_json::json!({
            "status": "extracting",
            "mode": "full",
        }))
        .await
        .ok();

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

// ── Chunked extraction ──────────────────────────────────────────

/// Perform extraction over text chunks with per-chunk observability.
///
/// Splits the full document text using `FixedSizeSplitter` with the
/// resolved chunk_size / chunk_overlap, then iterates each chunk with
/// rate-limit retry, JSON repair fallback, and per-chunk progress
/// reporting. A chunk failure is logged and counted but does not abort
/// the run unless *every* chunk fails.
async fn run_chunked_extraction(
    args: RunArgs<'_>,
    resolved: &ResolvedConfig,
) -> Result<ExtractionOutcome, Box<dyn Error + Send + Sync>> {
    let chunk_size = resolved.chunk_size.unwrap_or(8000).max(1) as usize;
    let chunk_overlap = resolved.chunk_overlap.unwrap_or(500).max(0) as usize;
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

    let mut all_entities: Vec<serde_json::Value> = Vec::new();
    let mut all_relationships: Vec<serde_json::Value> = Vec::new();
    let mut chunks_succeeded: i32 = 0;
    let mut chunks_failed: i32 = 0;
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;

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

        args.progress
            .report(serde_json::json!({
                "status": "extracting",
                "chunk": i + 1,
                "total": chunks.len(),
                "chars": chunk.text.len(),
            }))
            .await
            .ok();

        let chunk_id =
            extraction::insert_extraction_chunk(args.db, args.run_id, i as i32, &chunk.text)
                .await
                .map_err(|e| format!("Failed to insert chunk record: {e}"))?;

        let chunk_start = Instant::now();

        let prompt = args
            .template_text
            .replace("{{schema_json}}", args.schema_json)
            .replace("{{chunk_text}}", &chunk.text);

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

                        if let Some(entities) = parsed["entities"].as_array() {
                            all_entities.extend(entities.iter().cloned());
                        }
                        if let Some(rels) = parsed["relationships"].as_array() {
                            all_relationships.extend(rels.iter().cloned());
                        }

                        chunks_succeeded += 1;
                        extraction::complete_extraction_chunk(
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
                        .ok();

                        tracing::info!(
                            chunk = i,
                            entities = entity_count,
                            relationships = rel_count,
                            "Chunk extraction succeeded"
                        );

                        let percent =
                            ((chunks_succeeded as f64 / chunks.len() as f64) * 100.0) as i32;
                        documents::update_processing_progress(
                            args.db,
                            args.document_id,
                            "LlmExtract",
                            &format!("Extracting chunk {} of {}...", i + 1, chunks.len()),
                            chunks.len() as i32,
                            chunks_succeeded,
                            all_entities.len() as i32,
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
                        extraction::complete_extraction_chunk(
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
                        .ok();

                        let processed = chunks_succeeded + chunks_failed;
                        let percent = ((processed as f64 / chunks.len() as f64) * 100.0) as i32;
                        documents::update_processing_progress(
                            args.db,
                            args.document_id,
                            "LlmExtract",
                            &format!("Extracting chunk {} of {}...", i + 1, chunks.len()),
                            chunks.len() as i32,
                            processed,
                            all_entities.len() as i32,
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
                extraction::complete_extraction_chunk(
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
                .ok();

                let processed = chunks_succeeded + chunks_failed;
                let percent = ((processed as f64 / chunks.len() as f64) * 100.0) as i32;
                documents::update_processing_progress(
                    args.db,
                    args.document_id,
                    "LlmExtract",
                    &format!("Extracting chunk {} of {}...", i + 1, chunks.len()),
                    chunks.len() as i32,
                    processed,
                    all_entities.len() as i32,
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

// ── Small helpers ───────────────────────────────────────────────

/// True if a COMPLETED extraction_run already exists for this document.
async fn extraction_already_complete(
    db: &PgPool,
    document_id: &str,
) -> Result<bool, sqlx::Error> {
    let existing: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM extraction_runs \
         WHERE document_id = $1 AND status = 'COMPLETED' \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(document_id)
    .fetch_optional(db)
    .await?;
    Ok(existing.is_some())
}

/// Fall back to deriving a profile name from the schema filename.
///
/// Legacy rows in `pipeline_config` may not have a `profile_name` set yet.
/// `complaint_v2.yaml` → `complaint`. This keeps migration-era documents
/// processable without forcing a backfill.
fn default_profile_name_from_schema(schema_file: &str) -> String {
    schema_file
        .trim_end_matches(".yaml")
        .trim_end_matches("_v2")
        .to_string()
}

/// Resolve an effective `max_tokens` for LLM calls.
///
/// Priority: `ResolvedConfig.max_tokens` (from profile) → `LLM_MAX_TOKENS`
/// env var → [`DEFAULT_CHUNK_MAX_TOKENS`]. Values ≤ 0 in the profile are
/// treated as unset.
fn resolve_max_tokens(resolved: &ResolvedConfig) -> u32 {
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
fn compute_cost(
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
async fn write_processing_config_snapshot(
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
fn sha2_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ─────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
    fn default_profile_name_strips_yaml_and_v2() {
        assert_eq!(default_profile_name_from_schema("complaint_v2.yaml"), "complaint");
        assert_eq!(default_profile_name_from_schema("brief.yaml"), "brief");
        assert_eq!(default_profile_name_from_schema("custom"), "custom");
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
            template_file: "t".into(),
            template_hash: None,
            system_prompt_file: None,
            system_prompt_hash: None,
            schema_file: "s".into(),
            chunking_mode: "chunked".into(),
            chunk_size: None,
            chunk_overlap: None,
            max_tokens: 12345,
            temperature: 0.0,
            auto_approve_grounded: true,
            run_pass2: false,
            overrides_applied: vec![],
        };
        assert_eq!(resolve_max_tokens(&r), 12345);
    }
}
