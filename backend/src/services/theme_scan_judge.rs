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

/// Judge every candidate concurrently, bounded by the dedicated semaphore,
/// reporting LIVE progress to `scan_runs` as each candidate completes.
///
/// Returns one `(candidate, outcome)` per input. Ordering is not preserved
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
    attack_meaning: Arc<str>,
    params: ResolvedLlmParams,
    candidates: Vec<BiasInstance>,
    pool: PgPool,
    run_id: Uuid,
) -> Vec<(BiasInstance, JudgeOutcome)> {
    let total = candidates.len();
    stream::iter(candidates.into_iter().enumerate())
        .map(|(idx, candidate)| {
            // Cheap per-item clones — the underlying provider/semaphore/text are
            // shared, not copied; `params` is `Copy`; `PgPool::clone` is an Arc
            // bump; `run_id` is `Copy`.
            let provider = Arc::clone(&provider);
            let semaphore = Arc::clone(&semaphore);
            let scan_prompt = Arc::clone(&scan_prompt);
            let attack_meaning = Arc::clone(&attack_meaning);
            let pool = pool.clone();
            async move {
                let outcome = judge_one(
                    provider.as_ref(),
                    &semaphore,
                    &scan_prompt,
                    &attack_meaning,
                    &params,
                    &candidate,
                    idx,
                    total,
                )
                .await;
                report_progress(&pool, run_id, &outcome).await;
                (candidate, outcome)
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
    attack_meaning: &str,
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

    let user_msg = build_user_message(attack_meaning, candidate);
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
pub(crate) fn build_user_message(attack_meaning: &str, candidate: &BiasInstance) -> String {
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
            "ACCUSATION (what the scenario alleges):\n{attack_meaning}\n\n\
             QUOTE UNDER REVIEW:\nSpeaker: {speaker}\nDocument: {document}\n\
             Question asked: \"{question}\"\nAnswer under review: \"{quote}\"\n"
        ),
        None => format!(
            "ACCUSATION (what the scenario alleges):\n{attack_meaning}\n\n\
             QUOTE UNDER REVIEW:\nSpeaker: {speaker}\nDocument: {document}\nQuote: \"{quote}\"\n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fact_role::FactRole;

    fn instance(id: &str) -> BiasInstance {
        BiasInstance {
            evidence_id: id.to_string(),
            title: String::new(),
            verbatim_quote: None,
            question: None,
            page_number: None,
            pattern_tags: Vec::new(),
            stated_by: None,
            about: Vec::new(),
            document: None,
        }
    }

    fn outcome(verdict: Result<Verdict, String>) -> JudgeOutcome {
        JudgeOutcome {
            verdict,
            raw_reply: None,
            input_tokens: None,
            output_tokens: None,
        }
    }

    fn verdict(relevant: bool) -> Verdict {
        Verdict {
            relevant,
            proposed_role: FactRole::Supports,
            reason: "r".to_string(),
            confidence: 0.9,
        }
    }

    #[test]
    fn outcome_bucket_classifies_the_three_live_buckets() {
        // The live progress bucket is the VERDICT-time classification (ruling 2).
        assert_eq!(
            outcome_bucket(&outcome(Ok(verdict(true)))),
            ProgressBucket::Relevant
        );
        assert_eq!(
            outcome_bucket(&outcome(Ok(verdict(false)))),
            ProgressBucket::Irrelevant
        );
        assert_eq!(
            outcome_bucket(&outcome(Err("call failed".to_string()))),
            ProgressBucket::Failed
        );
    }

    #[test]
    fn user_message_uses_unknown_for_missing_speaker_and_document() {
        let msg = build_user_message("the accusation", &instance("ev-1"));
        assert!(msg.contains("Speaker: unknown"));
        assert!(msg.contains("Document: unknown"));
        assert!(msg.contains("the accusation"));
    }

    #[test]
    fn user_message_includes_speaker_document_and_quote() {
        let mut c = instance("ev-2");
        c.verbatim_quote = Some("I never said that".to_string());
        c.stated_by = Some(crate::bias::dto::ActorOption {
            id: "p1".to_string(),
            name: "Marie".to_string(),
            actor_type: "Person".to_string(),
            tagged_statement_count: 0,
        });
        c.document = Some(crate::bias::dto::DocumentRef {
            id: "d1".to_string(),
            title: "Affidavit".to_string(),
            document_type: None,
        });
        let msg = build_user_message("the accusation", &c);
        assert!(msg.contains("Speaker: Marie"));
        assert!(msg.contains("Document: Affidavit"));
        assert!(msg.contains("I never said that"));
    }

    #[test]
    fn user_message_pairs_question_and_answer_when_question_present() {
        // Discovery Q&A pairing: a bare answer is presented WITH its question,
        // so the judge can interpret "Yes" against what was actually asked.
        let mut c = instance("ev-3");
        c.verbatim_quote = Some("Yes".to_string());
        c.question = Some("Did you receive the funds on June 1?".to_string());
        let msg = build_user_message("the accusation", &c);

        assert!(
            msg.contains("Question asked: \"Did you receive the funds on June 1?\""),
            "question line missing: {msg}"
        );
        assert!(
            msg.contains("Answer under review: \"Yes\""),
            "answer line missing: {msg}"
        );
        // The single-quote label must NOT appear in the Q&A shape.
        assert!(
            !msg.contains("Quote: \""),
            "Q&A shape must not use the single-quote label: {msg}"
        );
    }

    #[test]
    fn user_message_uses_single_quote_shape_when_question_absent() {
        // Documentary evidence (no question) keeps the ORIGINAL message shape
        // unchanged — no "Question asked" / "Answer under review" lines.
        let mut c = instance("ev-4");
        c.verbatim_quote = Some("The ledger shows a transfer.".to_string());
        // question stays None (documentary evidence).
        let msg = build_user_message("the accusation", &c);

        assert!(
            msg.contains("Quote: \"The ledger shows a transfer.\""),
            "single-quote shape expected: {msg}"
        );
        assert!(
            !msg.contains("Question asked:"),
            "no question line when question is None: {msg}"
        );
        assert!(
            !msg.contains("Answer under review:"),
            "no answer line when question is None: {msg}"
        );
    }

    #[test]
    fn user_message_treats_empty_question_as_absent() {
        // A `question` property that exists but holds "" (or only whitespace)
        // carries no interpretive value — it must read identically to a missing
        // question (the single-quote shape), not emit an empty `Question asked`.
        let mut c = instance("ev-5");
        c.verbatim_quote = Some("No".to_string());
        c.question = Some("   ".to_string());
        let msg = build_user_message("the accusation", &c);

        assert!(
            msg.contains("Quote: \"No\""),
            "empty question must fall back to single-quote shape: {msg}"
        );
        assert!(
            !msg.contains("Question asked:"),
            "empty question must not produce a question line: {msg}"
        );
    }
}
