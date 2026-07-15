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
use tokio::sync::Semaphore;

use crate::bias::dto::BiasInstance;
use crate::domain::llm_params::ResolvedLlmParams;
use crate::llm_retry::call_with_rate_limit_retry_params;
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

/// Judge every candidate concurrently, bounded by the dedicated semaphore.
///
/// Returns one `(candidate, outcome)` per input. Ordering is not preserved
/// (`buffer_unordered`), which is irrelevant — the persist pass aggregates and
/// each result carries its own candidate. `params` is [`ResolvedLlmParams`],
/// which is `Copy`, so each concurrent task gets its own cheap copy (no `Arc`).
#[allow(clippy::too_many_arguments)]
pub(crate) async fn judge_all(
    provider: Arc<dyn LlmProvider>,
    semaphore: Arc<Semaphore>,
    concurrency: usize,
    scan_prompt: Arc<str>,
    attack_meaning: Arc<str>,
    params: ResolvedLlmParams,
    candidates: Vec<BiasInstance>,
) -> Vec<(BiasInstance, JudgeOutcome)> {
    let total = candidates.len();
    stream::iter(candidates.into_iter().enumerate())
        .map(|(idx, candidate)| {
            // Cheap per-item clones — the underlying provider/semaphore/text are
            // shared, not copied; `params` is `Copy` and moves a small value.
            let provider = Arc::clone(&provider);
            let semaphore = Arc::clone(&semaphore);
            let scan_prompt = Arc::clone(&scan_prompt);
            let attack_meaning = Arc::clone(&attack_meaning);
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
                (candidate, outcome)
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await
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
    // judging system prompt (theme_scan_prompt_v1.md) survives (Chunk B
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

    format!(
        "ACCUSATION (what the scenario alleges):\n{attack_meaning}\n\n\
         QUOTE UNDER REVIEW:\nSpeaker: {speaker}\nDocument: {document}\nQuote: \"{quote}\"\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(id: &str) -> BiasInstance {
        BiasInstance {
            evidence_id: id.to_string(),
            title: String::new(),
            verbatim_quote: None,
            page_number: None,
            pattern_tags: Vec::new(),
            stated_by: None,
            about: Vec::new(),
            document: None,
        }
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
}
