//! backend/src/pipeline/steps/llm_extract.rs
//!
//! LlmExtract pipeline step — single-call LLM entity and relationship
//! extraction. Reads the document's per-page text from `document_text`,
//! assembles a prompt via `colossus_extract::PromptBuilder`, invokes the
//! configured `LlmProvider` once, parses the response JSON, and stores the
//! resulting entities and relationships into `extraction_items` /
//! `extraction_relationships` via the new
//! `pipeline_repository::extraction::store_entities_and_relationships`
//! helper.
//!
//! ## Research-grounded deviation from v5_2 Part 10.2
//!
//! The v5_2 spec sketches a chunked architecture using FixedSizeSplitter
//! with per-chunk observability (record_chunk_success / record_chunk_failure
//! / store_extraction_results / complete_extraction_run called inside a
//! loop). This implementation deviates from that sketch. Rationale:
//!
//! 1. The chunking helpers named in v5_2 Part 10.2 (build_chunk_prompt,
//!    record_chunk_success, record_chunk_failure, store_extraction_results)
//!    do not exist in the codebase. They lived in chunk_extractor.rs,
//!    chunk_orchestration.rs, and chunk_storage.rs, which P2-Cleanup
//!    commit 1414838 deleted on 2026-04-16 because the chunking
//!    implementation had a double-storage FK violation bug and
//!    rate-limiting misbehavior that required a full rewrite.
//!
//! 2. That same P2-Cleanup commit also deleted
//!    backend/src/api/pipeline/extract.rs (the old HTTP extract_handler
//!    and run_extract function), going beyond v5_2 Part 15's published
//!    cleanup map. As of 2026-04-16 there has been no working LLM
//!    extraction path in the codebase, HTTP or pipeline. This step is
//!    the first working LLM extraction path since that deletion.
//!
//! 3. DOCUMENT_PROCESSING_PIPELINE_DESIGN_v1.md section 13 decision 2
//!    specified: "Our documents are 7K-10K tokens, fitting in a single
//!    LLM call. If colossus-ai encounters 50K+ token documents, we'll
//!    need chunking. Design the LlmExtractor to accept a TextSplitter
//!    trait but don't implement chunking until needed." Original design
//!    intent was single-call.
//!
//! 4. Anthropic Sonnet 4.x supports 64K output tokens and 200K input
//!    context, which comfortably accommodates the OCR'd legal corpus.
//!
//! Chunking, llm_json repair fallback, rate-limit retry on
//! PipelineError::RateLimited, and stop_reason=max_tokens monitoring are
//! DELIBERATELY DEFERRED as additive follow-ups (tracked in follow-up
//! debt section) to be added after first successful end-to-end DEV test
//! run. These features layer onto the existing call site and parse path
//! without restructuring this file.

use std::error::Error;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::time::Duration;

use colossus_extract::{FixedSizeSplitter, LlmProvider, LlmResponse, PipelineError, TextSplitter};
use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::pipeline::context::AppContext;
use crate::pipeline::steps::ingest::Ingest;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository::{self, extraction, steps};

// ── Constants ───────────────────────────────────────────────────

/// Max tokens per chunk LLM call. 8000 is sufficient for a 4000-char chunk
/// producing structured JSON output. Overridable via step_config or LLM_MAX_TOKENS env.
const DEFAULT_CHUNK_MAX_TOKENS: u32 = 8000;

/// Maximum retry attempts per chunk on rate-limit (429) errors.
const MAX_RETRIES_PER_CHUNK: u32 = 3;

// ── Error type (Kazlauskas G6 — Display omits {source}) ─────────

/// Failure modes for the LlmExtract step.
///
/// Display strings are terminal messages — they never interpolate
/// `{source}` (Kazlauskas Guideline 6). Source chains are preserved via
/// `#[source]` where applicable. Mirrors the P4-3 / P4-5 / P4-6 / P4-7
/// error-type discipline exactly.
///
/// `InsertRunFailed` / `CompleteRunFailed` / `StoreFailed` collapse their
/// inner repo error to a `String` rather than threading `PipelineRepoError`
/// directly, to match the existing `IngestError::Helper` pattern (the
/// underlying repo error is already stringly-typed so there is nothing to
/// source-chain).
#[derive(Debug, thiserror::Error)]
pub enum LlmExtractError {
    #[error("Document not found: {document_id}")]
    DocumentNotFound { document_id: String },

    #[error("No pipeline_config row for document '{document_id}'")]
    NoPipelineConfig { document_id: String },

    #[error("Failed to load schema '{schema_file}'")]
    SchemaLoadFailed {
        schema_file: String,
        #[source]
        source: colossus_extract::PipelineError,
    },

    #[error("Prompt assembly failed")]
    PromptBuildFailed {
        #[source]
        source: colossus_extract::PipelineError,
    },

    #[error("Document '{document_id}' has no extracted text pages")]
    NoTextPages { document_id: String },

    #[error("LLM call failed")]
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
}

// ── Step struct ─────────────────────────────────────────────────

/// The LlmExtract step variant's payload.
///
/// Like every other Phase 4 step, the only runtime state is the document
/// id. Everything else (model, max_tokens, schema, templates) is fetched
/// from `pipeline_config` / `AppContext` at execute time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LlmExtract {
    pub document_id: String,
}

/// Retry and timeout configuration is deliberately NOT set via trait
/// associated constants. These values should be read from deployment
/// config (pipeline_config.step_config["LlmExtract"] JSONB then env
/// vars) via the framework's resolve_step_config machinery. Setting
/// compile-time defaults here would violate the no-hardcoded-operational
/// -values discipline. The existing P4-5/P4-6/P4-7 steps DO currently
/// set these constants — that is tracked as follow-up debt
/// P-CONFIG-refactor to be addressed after Phase 5 integration testing
/// surfaces the real operational values.
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
        if let Err(e) =
            sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1 AND status IN ('RUNNING', 'FAILED')")
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

// ── Core implementation ─────────────────────────────────────────

impl LlmExtract {
    /// Internal: perform the full chunked LLM extraction write path.
    /// Called from [`Step::execute`] via the thin wrapper above.
    async fn run_llm_extract(
        &self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        let step_start = Instant::now();
        let step_id = steps::record_step_start(
            db,
            &self.document_id,
            "LlmExtract",
            "worker",
            &serde_json::json!({}),
        )
        .await?;

        // ── 1. Idempotency check ──
        let existing: Option<i32> = sqlx::query_scalar(
            "SELECT id FROM extraction_runs \
             WHERE document_id = $1 AND status = 'COMPLETED' \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(&self.document_id)
        .fetch_optional(db)
        .await?;

        if existing.is_some() {
            tracing::info!(
                document_id = %self.document_id,
                "Completed extraction run exists, skipping"
            );
            return Ok(StepResult::Next(DocProcessing::Ingest(Ingest {
                document_id: self.document_id.clone(),
            })));
        }

        // ── 2. Fetch pipeline config ──
        let pipe_config = pipeline_repository::get_pipeline_config(db, &self.document_id)
            .await?
            .ok_or_else(|| LlmExtractError::NoPipelineConfig {
                document_id: self.document_id.clone(),
            })?;

        // ── 3. Resolve max_tokens: step_config → env → default ──
        let max_tokens: u32 = {
            let step_cfg: Option<serde_json::Value> = sqlx::query_scalar(
                "SELECT step_config->'LlmExtract' FROM pipeline_config WHERE document_id = $1",
            )
            .bind(&self.document_id)
            .fetch_optional(db)
            .await?
            .flatten();

            step_cfg
                .as_ref()
                .and_then(|c| c.get("max_tokens"))
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
                .unwrap_or_else(|| {
                    std::env::var("LLM_MAX_TOKENS")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(DEFAULT_CHUNK_MAX_TOKENS)
                })
        };

        // ── 4. Verify document exists ──
        let _doc = pipeline_repository::get_document(db, &self.document_id)
            .await?
            .ok_or_else(|| LlmExtractError::DocumentNotFound {
                document_id: self.document_id.clone(),
            })?;

        // ── 5. Load schema ──
        let schema_path = format!("{}/{}", context.schema_dir, pipe_config.schema_file);
        let schema = colossus_extract::ExtractionSchema::from_file(std::path::Path::new(
            &schema_path,
        ))
        .map_err(|e| LlmExtractError::SchemaLoadFailed {
            schema_file: pipe_config.schema_file.clone(),
            source: e,
        })?;
        let schema_json = serde_json::to_string_pretty(&schema)?;

        // ── 6. Fetch document text pages ──
        let pages = pipeline_repository::get_document_text(db, &self.document_id).await?;
        if pages.is_empty() {
            return Err(LlmExtractError::NoTextPages {
                document_id: self.document_id.clone(),
            }
            .into());
        }

        // ── 7. Concatenate pages with page markers ──
        let full_text = pages
            .iter()
            .map(|p| format!("--- Page {} ---\n{}", p.page_number, p.text_content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // ── 8. Split into chunks ──
        let chunks = FixedSizeSplitter::new().split(&full_text);
        if chunks.is_empty() {
            return Err("Splitter produced zero chunks from non-empty text".into());
        }
        tracing::info!(
            document_id = %self.document_id,
            chunk_count = chunks.len(),
            full_text_len = full_text.len(),
            "Split document into chunks for extraction"
        );

        // ── 9. Load chunk prompt template ──
        let template_path = format!("{}/chunk_extract.md", context.template_dir);
        let template_text = std::fs::read_to_string(&template_path)
            .map_err(|e| format!("Failed to read chunk_extract.md: {e}"))?;

        // ── 10. Insert extraction run ──
        let model_name = context.llm_provider.model_name().to_string();
        let run_id = extraction::insert_extraction_run(
            db,
            &self.document_id,
            1,                          // pass_number
            &model_name,
            &schema.version,
            None,                       // assembled_prompt (per-chunk, not stored here)
            Some("chunk_extract.md"),   // template_name
            None,                       // template_hash
            None,                       // rules_name
            None,                       // rules_hash
            None,                       // schema_hash
            Some(&serde_json::to_value(&schema)?), // schema_content
            None,                       // temperature
            Some(max_tokens as i32),    // max_tokens_requested
            pipe_config.admin_instructions.as_deref(),
            None,                       // prior_context
        )
        .await
        .map_err(|e| LlmExtractError::InsertRunFailed {
            message: format!("{e}"),
        })?;

        // ── 11. Cancel check before acquiring semaphore ──
        if cancel.is_cancelled().await {
            mark_run_failed(db, run_id, "Cancelled before extraction").await;
            return Err("Cancelled before extraction".into());
        }

        // ── 12. Acquire LLM semaphore ──
        let _llm_permit = context
            .llm_semaphore
            .acquire()
            .await
            .map_err(|_| LlmExtractError::SemaphoreClosed)?;

        // ── 13. Chunked extraction loop ──
        let mut all_entities: Vec<serde_json::Value> = Vec::new();
        let mut all_relationships: Vec<serde_json::Value> = Vec::new();
        let mut chunks_succeeded: i32 = 0;
        let mut chunks_failed: i32 = 0;
        let mut total_input_tokens: i64 = 0;
        let mut total_output_tokens: i64 = 0;

        for (i, chunk) in chunks.iter().enumerate() {
            // Cancel check before each chunk
            if cancel.is_cancelled().await {
                mark_run_failed(db, run_id, "Cancelled during extraction").await;
                mark_step_failed(db, step_id, step_start, "Cancelled during extraction").await;
                return Err("Cancelled during extraction".into());
            }

            // Progress event
            progress
                .report(serde_json::json!({
                    "status": "extracting",
                    "chunk": i + 1,
                    "total": chunks.len(),
                    "chars": chunk.text.len(),
                }))
                .await
                .ok();

            // Insert chunk record (pending)
            let chunk_id = extraction::insert_extraction_chunk(db, run_id, i as i32, &chunk.text)
                .await
                .map_err(|e| format!("Failed to insert chunk record: {e}"))?;

            let chunk_start = Instant::now();

            // Build prompt for this chunk
            let prompt = template_text
                .replace("{{schema_json}}", &schema_json)
                .replace("{{chunk_text}}", &chunk.text);

            // Call LLM with rate-limit retry
            let llm_result = call_with_rate_limit_retry(
                &*context.llm_provider,
                &prompt,
                max_tokens,
                cancel,
                progress,
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

                    // Parse response JSON (with repair fallback)
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

                            // Accumulate entities and relationships
                            if let Some(entities) = parsed["entities"].as_array() {
                                all_entities.extend(entities.iter().cloned());
                            }
                            if let Some(rels) = parsed["relationships"].as_array() {
                                all_relationships.extend(rels.iter().cloned());
                            }

                            chunks_succeeded += 1;
                            extraction::complete_extraction_chunk(
                                db, chunk_id, "success",
                                Some(entity_count as i32), Some(rel_count as i32),
                                input_toks, output_toks,
                                Some(chunk_duration_ms), None,
                            )
                            .await
                            .ok();

                            tracing::info!(
                                chunk = i, entities = entity_count, relationships = rel_count,
                                "Chunk extraction succeeded"
                            );
                        }
                        Err(parse_err) => {
                            chunks_failed += 1;
                            tracing::warn!(
                                chunk = i, error = %parse_err,
                                "Chunk parse failed after repair attempt"
                            );
                            extraction::complete_extraction_chunk(
                                db, chunk_id, "failed",
                                None, None, input_toks, output_toks,
                                Some(chunk_duration_ms),
                                Some(&format!("Parse error: {parse_err}")),
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
                        db, chunk_id, "failed",
                        None, None, None, None,
                        Some(chunk_duration_ms),
                        Some(&format!("{call_err}")),
                    )
                    .await
                    .ok();
                }
            }
        }

        // ── 14. Update chunk stats on the run ──
        extraction::update_run_chunk_stats(
            db,
            run_id,
            chunks.len() as i32,
            chunks_succeeded,
            chunks_failed,
        )
        .await
        .ok();

        // ── 15. Check if ALL chunks failed ──
        if chunks_succeeded == 0 {
            let msg = format!("All {} chunks failed extraction", chunks.len());
            mark_run_failed(db, run_id, &msg).await;
            mark_step_failed(db, step_id, step_start, &msg).await;
            return Err(msg.into());
        }

        // ── 16. Build merged result and store ──
        let merged = serde_json::json!({
            "entities": all_entities,
            "relationships": all_relationships,
        });

        // Compute cost from aggregated token usage across all chunks.
        let cost_usd = {
            let cost_in = context
                .llm_provider
                .cost_per_input_token()
                .map(|c| c * total_input_tokens as f64);
            let cost_out = context
                .llm_provider
                .cost_per_output_token()
                .map(|c| c * total_output_tokens as f64);
            match (cost_in, cost_out) {
                (Some(a), Some(b)) => Some(a + b),
                _ => None,
            }
        };

        extraction::complete_extraction_run(
            db,
            run_id,
            &merged,
            Some(total_input_tokens as i32),
            Some(total_output_tokens as i32),
            cost_usd,
            "COMPLETED",
        )
        .await
        .map_err(|e| LlmExtractError::CompleteRunFailed {
            message: format!("{e}"),
        })?;

        // Store individual entities and relationships.
        let (entity_count, rel_count) =
            extraction::store_entities_and_relationships(db, run_id, &self.document_id, &merged)
                .await
                .map_err(|e| LlmExtractError::StoreFailed {
                    message: format!("{e}"),
                })?;

        tracing::info!(
            document_id = %self.document_id,
            entities = entity_count,
            relationships = rel_count,
            chunks_succeeded,
            chunks_failed,
            total_input_tokens,
            total_output_tokens,
            "Chunked extraction complete"
        );

        // ── 17. Record step complete ──
        steps::record_step_complete(
            db,
            step_id,
            step_start.elapsed().as_secs_f64(),
            &serde_json::json!({
                "entity_count": entity_count,
                "relationship_count": rel_count,
                "chunk_count": chunks.len(),
                "chunks_succeeded": chunks_succeeded,
                "chunks_failed": chunks_failed,
                "input_tokens": total_input_tokens,
                "output_tokens": total_output_tokens,
            }),
        )
        .await
        .ok();

        // ── 18. Advance to Ingest ──
        Ok(StepResult::Next(DocProcessing::Ingest(Ingest {
            document_id: self.document_id.clone(),
        })))
    }
}

// ── Helpers: rate-limit retry and JSON repair ───────────────────

/// Call the LLM provider with rate-limit-aware retry.
///
/// On `PipelineError::RateLimited`, sleeps exactly `retry_after_secs` and retries.
/// Max `MAX_RETRIES_PER_CHUNK` attempts. Any other error returns immediately.
/// Emits progress events during waits so the UI shows status.
async fn call_with_rate_limit_retry(
    provider: &dyn LlmProvider,
    prompt: &str,
    max_tokens: u32,
    cancel: &CancellationToken,
    progress: &ProgressReporter,
    chunk_idx: usize,
    chunk_total: usize,
) -> Result<LlmResponse, PipelineError> {
    let mut attempt = 0u32;
    loop {
        match provider.invoke(prompt, max_tokens).await {
            Ok(response) => return Ok(response),
            Err(PipelineError::RateLimited { retry_after_secs }) => {
                attempt += 1;
                if attempt > MAX_RETRIES_PER_CHUNK {
                    return Err(PipelineError::LlmProvider(format!(
                        "chunk {}/{}: exhausted {} rate-limit retries",
                        chunk_idx + 1,
                        chunk_total,
                        MAX_RETRIES_PER_CHUNK
                    )));
                }

                progress
                    .report(serde_json::json!({
                        "status": "rate_limited",
                        "chunk": chunk_idx + 1,
                        "total": chunk_total,
                        "retry_after_secs": retry_after_secs,
                        "attempt": attempt,
                    }))
                    .await
                    .ok();

                tracing::warn!(
                    chunk = chunk_idx,
                    retry_after_secs,
                    attempt,
                    "Rate limited, sleeping before retry"
                );

                // Sleep with cancel awareness. Poll is_cancelled every second.
                let mut remaining = retry_after_secs;
                while remaining > 0 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    remaining -= 1;
                    if cancel.is_cancelled().await {
                        return Err(PipelineError::LlmProvider(
                            "Cancelled during rate-limit wait".into(),
                        ));
                    }
                }
                // Loop continues — retry the call
            }
            Err(other) => return Err(other),
        }
    }
}

/// Parse an LLM response as a JSON Value containing entities and relationships.
///
/// Tries direct `serde_json` parse first. On failure, strips markdown fences
/// and uses `llm_json::repair_json` for repair, then retries parse.
fn parse_chunk_response(text: &str) -> Result<serde_json::Value, String> {
    let stripped = strip_markdown_fences(text);

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stripped) {
        return Ok(val);
    }

    match llm_json::repair_json(&stripped, &Default::default()) {
        Ok(repaired) => serde_json::from_str::<serde_json::Value>(&repaired)
            .map_err(|e| format!("JSON repair succeeded but parse still failed: {e}")),
        Err(repair_err) => {
            let preview = &stripped[..stripped.len().min(200)];
            Err(format!(
                "JSON parse and repair both failed. Repair error: {repair_err}. Preview: {preview}"
            ))
        }
    }
}

/// Strip leading/trailing markdown code fences.
fn strip_markdown_fences(text: &str) -> String {
    let t = text.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim().to_string()
}

// ── Helpers: best-effort failure recording ──────────────────────

/// Log-and-ignore writer used when the step is about to return Err: we want
/// the DB record to reflect the failure but we don't want a secondary write
/// error to mask the primary cause.
async fn mark_run_failed(db: &PgPool, run_id: i32, reason: &str) {
    if let Err(e) = extraction::complete_extraction_run(
        db,
        run_id,
        &serde_json::json!({"error": reason}),
        None,
        None,
        None,
        "FAILED",
    )
    .await
    {
        tracing::warn!(run_id, error = %e, reason, "mark_run_failed: DB write failed (non-fatal)");
    }
}

async fn mark_step_failed(
    db: &PgPool,
    step_id: i32,
    step_start: std::time::Instant,
    error_message: &str,
) {
    if let Err(e) = steps::record_step_failure(
        db,
        step_id,
        step_start.elapsed().as_secs_f64(),
        error_message,
    )
    .await
    {
        tracing::warn!(step_id, error = %e, "mark_step_failed: DB write failed (non-fatal)");
    }
}

// ─────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Error discipline ──────────────────────────────────────────

    #[test]
    fn llm_extract_error_display_g6_compliance() {
        let e1 = LlmExtractError::DocumentNotFound {
            document_id: "doc-x".to_string(),
        };
        let s1 = e1.to_string();
        assert!(!s1.contains("Caused by"), "G6 violation: {s1}");
        assert!(!s1.contains("source"), "G6 violation: {s1}");

        let inner = "UNIQUE_INNER_PROVIDER_TOKEN";
        let e2 = LlmExtractError::LlmCallFailed {
            source: colossus_extract::PipelineError::LlmProvider(inner.to_string()),
        };
        let s2 = e2.to_string();
        assert!(
            !s2.contains(inner),
            "Display must not interpolate source text (G6); got: {s2}"
        );
        assert!(!s2.contains("Caused by"), "G6 violation: {s2}");
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
        // This test enforces the no-hardcoded-values discipline. If a future
        // change sets these constants to numeric values, this test MUST fail
        // until the P-CONFIG-refactor framework fix lands.
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

    /// Compile-only reference to the new repository helper — catches any
    /// module-path drift (e.g., if the helper is ever moved). No runtime
    /// execution, no DB.
    #[test]
    fn llm_extract_store_path_compiles() {
        let _f = crate::repositories::pipeline_repository::extraction::store_entities_and_relationships;
    }
}
