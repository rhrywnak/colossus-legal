//! Theme Scan judging — the concurrent per-quote LLM fan-out (D2b, extended in
//! LLM Config Chunk B).
//!
//! Split from `theme_scan.rs` so neither file exceeds the module-size limit.
//! `theme_scan.rs` owns orchestration; THIS module owns the LLM loop; the sibling
//! `theme_scan_persist` owns the `scenario_fact_refs` / `scan_runs` writes and the
//! summary. Chunk B added per-candidate token + raw-reply capture (for the
//! `scan_run_verdicts` audit rows) and routed the call through the params-aware
//! seam ([`crate::domain::llm_provider_ext::LlmProviderExt`]).
//!
//! ## Per-item failures never abort the batch (Standing Rule 1)
//!
//! A malformed reply, an out-of-set role, or a closed semaphore for ONE candidate
//! is captured in that candidate's [`JudgeOutcome`] (its `verdict` is `Err`) — it
//! does NOT `?` out of the batch. This mirrors the extraction loop's
//! `chunks_failed += 1; continue`: a 94-quote scan must not be lost to one bad
//! reply.

use std::sync::Arc;

use colossus_extract::{LlmProvider, LlmResponse, PipelineError};
use futures::stream::{self, StreamExt};
use sqlx::PgPool;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::bias::dto::BiasInstance;
use crate::domain::llm_params::ResolvedLlmParams;
use crate::llm_retry::call_with_rate_limit_retry_params;
use crate::repositories::pipeline_repository::{bump_scan_run_progress, ProgressBucket};
use crate::services::theme_scan_parse::{parse_verdict, Verdict};
use crate::services::theme_scan_prefilter::CandidateGroup;

/// Everything one judged candidate yields: the parsed verdict (or a per-item
/// failure reason), plus the raw reply and token usage the audit tables record.
///
/// `raw_reply` is `Some` whenever the model returned text (a success OR a
/// parse-failure — both are auditable); it is `None` only when the call itself
/// failed before returning text. `input_tokens` / `output_tokens` are
/// `None`-if-absent (never a fabricated 0 — Standing Rule 1).
pub(crate) struct JudgeOutcome {
    pub verdict: Result<Verdict, String>,
    pub raw_reply: Option<String>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

/// Judge every candidate GROUP concurrently, bounded by the dedicated semaphore,
/// reporting LIVE progress to `scan_runs` as each group completes.
///
/// One group is one LLM call. A group of one is an ordinary candidate; a group of
/// two or more is a set of byte-identical quotes the pre-filter folded together
/// (task 2.15 Tier 2) — the same sentence judged once instead of twice. The
/// group's members ride through untouched so the persist pass can write each one
/// its own verdict row.
///
/// Returns one `(group, outcome)` per input. Ordering is not preserved
/// (`buffer_unordered`), which is irrelevant — the persist pass aggregates and
/// each result carries its own candidate. `params` is [`ResolvedLlmParams`],
/// which is `Copy`, so each concurrent task gets its own cheap copy (no `Arc`).
///
/// `pool` + `run_id` drive the per-candidate [`bump_scan_run_progress`] write
/// (the `chunks_processed` analog). The progress write is BEST-EFFORT: a failed
/// bump is logged and the scan continues — losing a progress tick must not lose a
/// verdict (Standing Rule 1: the failure is observable in the log, and the final
/// counts are authoritative regardless). The live bucket is the VERDICT-time
/// classification; a relevant verdict whose `scenario_fact_refs` write later fails
/// is reclassified to `failed` only at completion (ruling 2 — live is an estimate).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn judge_all(
    provider: Arc<dyn LlmProvider>,
    semaphore: Arc<Semaphore>,
    concurrency: usize,
    scan_prompt: Arc<str>,
    scan_criteria: Arc<str>,
    params: ResolvedLlmParams,
    groups: Vec<CandidateGroup>,
    pool: PgPool,
    run_id: Uuid,
) -> Vec<(CandidateGroup, JudgeOutcome)> {
    let total = groups.len();
    stream::iter(groups.into_iter().enumerate())
        .map(|(idx, group)| {
            // Cheap per-item clones — the underlying provider/semaphore/text are
            // shared, not copied; `params` is `Copy`; `PgPool::clone` is an Arc
            // bump; `run_id` is `Copy`.
            let provider = Arc::clone(&provider);
            let semaphore = Arc::clone(&semaphore);
            let scan_prompt = Arc::clone(&scan_prompt);
            let scan_criteria = Arc::clone(&scan_criteria);
            let pool = pool.clone();
            async move {
                let outcome = judge_one(
                    provider.as_ref(),
                    &semaphore,
                    &scan_prompt,
                    &scan_criteria,
                    &params,
                    // The representative is what the model reads; its twins are
                    // byte-identical, so judging one judges all of them.
                    &group.representative,
                    idx,
                    total,
                )
                .await;
                report_progress(&pool, run_id, &outcome).await;
                (group, outcome)
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await
}

/// The live outcome bucket for one judged candidate (verdict-time classification).
fn outcome_bucket(outcome: &JudgeOutcome) -> ProgressBucket {
    match &outcome.verdict {
        Ok(v) if v.relevant => ProgressBucket::Relevant,
        Ok(_) => ProgressBucket::Irrelevant,
        Err(_) => ProgressBucket::Failed,
    }
}

/// Best-effort per-candidate progress write. A failure is LOGGED (so it is
/// observable) but never aborts the scan — a dropped progress tick is cosmetic;
/// the verdict and the final counts are unaffected.
async fn report_progress(pool: &PgPool, run_id: Uuid, outcome: &JudgeOutcome) {
    let bucket = outcome_bucket(outcome);
    if let Err(e) = bump_scan_run_progress(pool, run_id, bucket).await {
        tracing::warn!(%run_id, error = %e, "theme scan: progress bump failed (continuing)");
    }
}

/// Judge one candidate. Every failure mode is captured in the returned
/// [`JudgeOutcome`] (a `verdict: Err`) rather than propagating, so one bad reply
/// cannot abort the batch.
#[allow(clippy::too_many_arguments)]
async fn judge_one(
    provider: &dyn LlmProvider,
    semaphore: &Semaphore,
    scan_prompt: &str,
    scan_criteria: &str,
    params: &ResolvedLlmParams,
    candidate: &BiasInstance,
    idx: usize,
    total: usize,
) -> JudgeOutcome {
    // Acquire a permit from the dedicated cap for the duration of the call. A
    // closed semaphore (only at shutdown) is a per-item failure, not a panic.
    let _permit = match semaphore.acquire().await {
        Ok(permit) => permit,
        Err(e) => {
            return JudgeOutcome {
                verdict: Err(format!("theme scan semaphore closed: {e}")),
                raw_reply: None,
                input_tokens: None,
                output_tokens: None,
            };
        }
    };

    let user_msg = build_user_message(scan_criteria, candidate);
    // `Some(scan_prompt)` routes through `invoke_with_system_and_params`, so the
    // judging system prompt (theme_scan_prompt_v2.md) survives (Chunk B
    // precondition). `params.max_tokens` (the verdict cap) reaches the wire.
    let result = call_with_rate_limit_retry_params(
        provider,
        Some(scan_prompt),
        &user_msg,
        params,
        idx,
        total,
    )
    .await;
    outcome_from_result(result)
}

/// Turn one LLM call result into a [`JudgeOutcome`]: parse the verdict on
/// success (retaining the raw reply as the audit surface + prose-JSON-compliance
/// signal), or record a per-item failure with no raw reply on a call error.
fn outcome_from_result(result: Result<LlmResponse, PipelineError>) -> JudgeOutcome {
    match result {
        Ok(response) => {
            let verdict = parse_verdict(&response.text).map_err(|reason| {
                let preview: String = response.text.chars().take(500).collect();
                format!("{reason} | raw LLM reply: {preview}")
            });
            JudgeOutcome {
                verdict,
                raw_reply: Some(response.text),
                input_tokens: response.input_tokens,
                output_tokens: response.output_tokens,
            }
        }
        Err(e) => JudgeOutcome {
            verdict: Err(format!("LLM call failed: {e}")),
            raw_reply: None,
            input_tokens: None,
            output_tokens: None,
        },
    }
}

/// Build the per-quote user message: the accusation criterion plus this one
/// quote's speaker, document, and verbatim text. Case-agnostic — all case data
/// comes from the scenario/candidate, none is compiled in.
///
/// ## Two shapes, one function (discovery Q&A pairing)
///
/// Discovery Evidence carries the interrogatory `question` its answer responds
/// to; documentary evidence does not. When `question` is present and non-empty
/// we present the candidate as a Q&A pair — `Question asked` + `Answer under
/// review` — so the judge reads a bare "Yes"/"No" answer in light of the
/// question that gives it meaning. When it is absent (or empty), we keep the
/// original single-`Quote:` shape unchanged, so documentary evidence sees the
/// exact message it always has.
///
/// Domain note: the answer text lives in `verbatim_quote` for BOTH shapes (the
/// discovery pass-1 template writes the sworn answer there); the question is the
/// only added context. An empty-string question is normalized to `None`-
/// equivalent here — a question property that exists but holds `""` carries no
/// interpretive value, so it takes the single-quote path (Standing Rule 1: it
/// reads identically to a genuinely absent question, which is the honest state).
pub(crate) fn build_user_message(scan_criteria: &str, candidate: &BiasInstance) -> String {
    let speaker = candidate
        .stated_by
        .as_ref()
        .map(|a| a.name.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown");
    let document = candidate
        .document
        .as_ref()
        .map(|d| d.title.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown");
    let quote = candidate.verbatim_quote.as_deref().unwrap_or("");

    // `filter(|s| !s.is_empty())` collapses `Some("")` to `None`: an empty
    // question is treated exactly like a missing one (single-quote path).
    let question = candidate
        .question
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    match question {
        Some(question) => format!(
            "ACCUSATION (what the scenario alleges):\n{scan_criteria}\n\n\
             QUOTE UNDER REVIEW:\nSpeaker: {speaker}\nDocument: {document}\n\
             Question asked: \"{question}\"\nAnswer under review: \"{quote}\"\n"
        ),
        None => format!(
            "ACCUSATION (what the scenario alleges):\n{scan_criteria}\n\n\
             QUOTE UNDER REVIEW:\nSpeaker: {speaker}\nDocument: {document}\nQuote: \"{quote}\"\n"
        ),
    }
}

#[cfg(test)]
#[path = "theme_scan_judge_tests.rs"]
mod tests;
