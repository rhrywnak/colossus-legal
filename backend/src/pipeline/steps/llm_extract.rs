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

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::pipeline::context::AppContext;
use crate::pipeline::steps::ingest::Ingest;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository::{self, extraction, steps};

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
        self.run_llm_extract(db, context, cancel, progress)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
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
    /// Internal: perform the full LLM extraction write path. Called from
    /// [`Step::execute`] via the thin wrapper above.
    async fn run_llm_extract(
        &self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, LlmExtractError> {
        let step_start = std::time::Instant::now();
        let doc_id = self.document_id.as_str();

        // [1] Legacy pipeline_steps write — transitional, matches the rest
        //     of Phase 4 while the frontend still reads this table.
        let step_id = steps::record_step_start(
            db,
            doc_id,
            "extract",
            "worker",
            &serde_json::json!({}),
        )
        .await
        .map_err(|e| LlmExtractError::InsertRunFailed {
            message: format!("record_step_start: {e}"),
        })?;

        // [2] Idempotency: if a COMPLETED run already exists, this step's
        //     work is already durable. Emit a skip summary and advance.
        let existing: Option<i32> = sqlx::query_scalar(
            "SELECT id FROM extraction_runs \
             WHERE document_id = $1 AND status = 'COMPLETED' \
             ORDER BY id DESC LIMIT 1",
        )
        .bind(doc_id)
        .fetch_optional(db)
        .await
        .map_err(|e| LlmExtractError::InsertRunFailed {
            message: format!("idempotency check: {e}"),
        })?;

        if let Some(existing_run_id) = existing {
            tracing::info!(
                doc_id = %doc_id, run_id = existing_run_id,
                "LlmExtract: COMPLETED run already exists — skipping"
            );
            if let Err(e) = steps::record_step_complete(
                db,
                step_id,
                step_start.elapsed().as_secs_f64(),
                &serde_json::json!({"idempotent": true, "existing_run_id": existing_run_id}),
            )
            .await
            {
                tracing::warn!(
                    doc_id = %doc_id, step_id, error = %e,
                    "LlmExtract: record_step_complete failed on idempotent path (non-fatal)"
                );
            }
            return Ok(StepResult::Next(DocProcessing::Ingest(Ingest {
                document_id: self.document_id.clone(),
            })));
        }

        // [3] Fetch pipeline_config. Required — NoPipelineConfig is a bug.
        let pipe_config = pipeline_repository::get_pipeline_config(db, doc_id)
            .await
            .map_err(|e| LlmExtractError::InsertRunFailed {
                message: format!("get_pipeline_config: {e}"),
            })?
            .ok_or_else(|| LlmExtractError::NoPipelineConfig {
                document_id: doc_id.to_string(),
            })?;

        // Also verify the document exists — earlier pipeline steps should
        // have established this, but a hostile / racing delete could leave
        // pipeline_config orphaned. Cheap re-verification.
        if pipeline_repository::get_document(db, doc_id)
            .await
            .map_err(|e| LlmExtractError::InsertRunFailed {
                message: format!("get_document: {e}"),
            })?
            .is_none()
        {
            return Err(LlmExtractError::DocumentNotFound {
                document_id: doc_id.to_string(),
            });
        }

        // TODO(P-CONFIG-refactor): source model_name and max_tokens from
        // step_config["LlmExtract"] JSONB → env vars, not just pipeline_config
        // columns. Minimal version uses pipeline_config columns only to match
        // the rest of Phase 4. See tracker follow-up debt section.
        let model_name = pipe_config.pass1_model.clone();
        let max_tokens = pipe_config.pass1_max_tokens as u32;

        // [4] Load extraction schema.
        let schema_path = format!("{}/{}", context.schema_dir, pipe_config.schema_file);
        let schema = colossus_extract::ExtractionSchema::from_file(std::path::Path::new(
            &schema_path,
        ))
        .map_err(|e| LlmExtractError::SchemaLoadFailed {
            schema_file: pipe_config.schema_file.clone(),
            source: e,
        })?;

        // [5] Fetch per-page text and assemble full_text.
        let pages = pipeline_repository::get_document_text(db, doc_id)
            .await
            .map_err(|e| LlmExtractError::InsertRunFailed {
                message: format!("get_document_text: {e}"),
            })?;
        if pages.is_empty() {
            return Err(LlmExtractError::NoTextPages {
                document_id: doc_id.to_string(),
            });
        }
        let full_text = pages
            .iter()
            .map(|p| format!("--- Page {} ---\n{}", p.page_number, p.text_content))
            .collect::<Vec<_>>()
            .join("\n\n");

        // [6] Build prompt via PromptBuilder. Try a document-type-specific
        //     pass1 template first; fall through to the builder's default
        //     ("pass1_template.md") when the specific file isn't present.
        let specific_template = format!("pass1_{}.md", schema.document_type);
        let template_path =
            std::path::Path::new(&context.template_dir).join(&specific_template);
        let template_name = if template_path.exists() {
            Some(specific_template.as_str())
        } else {
            None
        };
        let mut builder = colossus_extract::PromptBuilder::new(std::path::Path::new(
            &context.template_dir,
        ));
        let artifact = builder
            .build_extraction_prompt(
                &schema,
                &full_text,
                None,
                pipe_config.admin_instructions.as_deref(),
                Some("global_rules.md"),
                template_name,
            )
            .map_err(|e| LlmExtractError::PromptBuildFailed { source: e })?;

        let schema_json_value = serde_json::to_value(&schema).ok();

        // [7] Insert extraction_run with full F3 reproducibility.
        let run_id = extraction::insert_extraction_run(
            db,
            doc_id,
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
            None,
            Some(max_tokens as i32),
            pipe_config.admin_instructions.as_deref(),
            None,
        )
        .await
        .map_err(|e| LlmExtractError::InsertRunFailed {
            message: format!("{e:?}")
        })?;

        // [8] Cancel check before we pay for a call.
        if cancel.is_cancelled().await {
            mark_run_failed(db, run_id, "cancelled before LLM call").await;
            mark_step_failed(db, step_id, step_start, "cancelled before LLM call").await;
            return Err(LlmExtractError::LlmCallFailed {
                source: colossus_extract::PipelineError::LlmProvider(
                    "cancelled before LLM call".to_string(),
                ),
            });
        }

        // [9] Acquire LLM semaphore — bounds concurrent LLM API calls
        //     across all jobs. Drops automatically on return.
        let _permit = context
            .llm_semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| LlmExtractError::SemaphoreClosed)?;

        // [10] Cancel check after permit acquired.
        if cancel.is_cancelled().await {
            mark_run_failed(db, run_id, "cancelled after permit acquire").await;
            mark_step_failed(db, step_id, step_start, "cancelled after permit acquire").await;
            return Err(LlmExtractError::LlmCallFailed {
                source: colossus_extract::PipelineError::LlmProvider(
                    "cancelled after permit acquire".to_string(),
                ),
            });
        }

        if let Err(e) = progress
            .report(serde_json::json!({"status": "llm_extracting", "model": &model_name}))
            .await
        {
            tracing::warn!(doc_id = %doc_id, error = %e, "LlmExtract: progress.report failed (non-fatal)");
        }

        // [11] Single LLM call.
        let api_start = std::time::Instant::now();
        let llm_result = context
            .llm_provider
            .invoke(&artifact.prompt_text, max_tokens)
            .await;

        let response = match llm_result {
            Ok(r) => r,
            Err(e) => {
                mark_run_failed(db, run_id, &format!("llm invoke: {e}")).await;
                mark_step_failed(db, step_id, step_start, &format!("llm invoke: {e}")).await;
                return Err(LlmExtractError::LlmCallFailed { source: e });
            }
        };
        let _llm_elapsed = api_start.elapsed();

        // [12] Parse JSON. Note: if parsing fails the extraction_run stays
        //      at RUNNING. That is intentional for the minimal version —
        //      on_cancel or a subsequent reprocess will sweep it; the raw
        //      text is NOT persisted because extraction_runs.raw_output
        //      expects JSON, not arbitrary strings. llm_json repair is the
        //      deferred follow-up that will render this branch unreachable.
        let parsed: serde_json::Value = serde_json::from_str(&response.text).map_err(|e| {
            let preview: String = response.text.chars().take(500).collect();
            LlmExtractError::ResponseNotJson { preview, source: e }
        })?;

        // [13] Cost estimate + [14] finalize run.
        let input_tokens_i32 = response.input_tokens.map(|v| v as i32);
        let output_tokens_i32 = response.output_tokens.map(|v| v as i32);
        let cost = crate::api::pipeline::constants::estimate_cost(
            response.input_tokens.unwrap_or(0) as i64,
            response.output_tokens.unwrap_or(0) as i64,
        );

        extraction::complete_extraction_run(
            db,
            run_id,
            &parsed,
            input_tokens_i32,
            output_tokens_i32,
            Some(cost),
            "COMPLETED",
        )
        .await
        .map_err(|e| LlmExtractError::CompleteRunFailed {
            message: format!("{e:?}")
        })?;

        // [15] Cancel check before storage is messy by design: the run is
        //      now COMPLETED and the tokens are billed. Prefer to store
        //      the results (avoiding wasted spend) and let downstream
        //      cancellation surface after the records are persisted.
        if cancel.is_cancelled().await {
            tracing::warn!(
                doc_id = %doc_id, run_id,
                "LlmExtract: cancel requested post-LLM-call — proceeding with storage so paid tokens aren't wasted"
            );
        }

        // [16] Store entities + relationships.
        let (entity_count, rel_count) = extraction::store_entities_and_relationships(
            db,
            run_id,
            &self.document_id,
            &parsed,
        )
        .await
        .map_err(|e| LlmExtractError::StoreFailed {
            message: format!("{e:?}")
        })?;

        // [17] Legacy pipeline_steps complete.
        if let Err(e) = steps::record_step_complete(
            db,
            step_id,
            step_start.elapsed().as_secs_f64(),
            &serde_json::json!({
                "model": model_name,
                "entity_count": entity_count,
                "relationship_count": rel_count,
                "input_tokens": response.input_tokens,
                "output_tokens": response.output_tokens,
            }),
        )
        .await
        {
            tracing::warn!(
                doc_id = %doc_id, step_id, error = %e,
                "LlmExtract: record_step_complete failed (legacy table; non-fatal)"
            );
        }

        tracing::info!(
            doc_id = %doc_id, run_id, entity_count, rel_count,
            input_tokens = ?response.input_tokens, output_tokens = ?response.output_tokens,
            "LlmExtract complete"
        );

        // [18] Advance to Ingest.
        Ok(StepResult::Next(DocProcessing::Ingest(Ingest {
            document_id: self.document_id.clone(),
        })))
    }
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
