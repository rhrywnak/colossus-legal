//! Theme Scan judging + persistence (D2b).
//!
//! The concurrent per-quote judging fan-out and the write-and-summarize pass,
//! split out of `theme_scan.rs` so neither file exceeds the module-size limit.
//! `theme_scan.rs` owns orchestration; this module owns the LLM loop and the
//! `scenario_fact_refs` writes.
//!
//! ## Per-item failures never abort the batch (Standing Rule 1)
//!
//! A malformed reply, an out-of-set role, or a failed write for ONE candidate is
//! counted in `failed` and logged with its `evidence_id` — it does not `?` out of
//! the batch. This mirrors the extraction loop's `chunks_failed += 1; continue`:
//! a 94-quote scan must not be lost to one bad reply.

use std::sync::Arc;

use colossus_extract::LlmProvider;
use futures::stream::{self, StreamExt};
use sqlx::PgPool;
use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::bias::dto::BiasInstance;
use crate::domain::fact_status::FactStatus;
use crate::dto::{ThemeScanRejected, ThemeScanSuggestion, ThemeScanSummary};
use crate::llm_retry::call_with_rate_limit_retry;
use crate::repositories::pipeline_repository::{upsert_fact_ref, PipelineRepoError};
use crate::services::theme_scan::THEME_SCAN_MAX_TOKENS;
use crate::services::theme_scan_parse::{parse_verdict, Verdict};

// CONST: the honesty-check sample size is a fixed UX constant, not a deployment
// knob. It bounds how many rejected quotes ride inline in the scan response for a
// human spot-check; ten is a reviewable handful. Changing it is a product
// decision (a code change), and making it per-environment would let a deployment
// silently weaken the honesty check — so it is pinned in code on purpose.
const THEME_SCAN_REJECTED_SAMPLE_SIZE: usize = 10;

/// Running tallies accumulated while writing verdicts. Bundling them keeps
/// [`persist_and_summarize`] short and lets [`record_outcome`] mutate one value
/// instead of five out-parameters.
#[derive(Default)]
struct ScanTally {
    relevant_written: usize,
    irrelevant: usize,
    failed: usize,
    suggestions: Vec<ThemeScanSuggestion>,
    rejected: Vec<ThemeScanRejected>,
}

/// Judge every candidate concurrently, bounded by the dedicated semaphore.
///
/// Returns one `(candidate, verdict-or-failure)` per input. Ordering is not
/// preserved (`buffer_unordered`), which is irrelevant — the summary aggregates
/// and the per-item result carries its own candidate.
pub(crate) async fn judge_all(
    provider: Arc<dyn LlmProvider>,
    semaphore: Arc<Semaphore>,
    concurrency: usize,
    scan_prompt: Arc<str>,
    attack_meaning: Arc<str>,
    candidates: Vec<BiasInstance>,
) -> Vec<(BiasInstance, Result<Verdict, String>)> {
    let total = candidates.len();
    stream::iter(candidates.into_iter().enumerate())
        .map(|(idx, candidate)| {
            // Cheap Arc clones per item — the underlying provider/semaphore/text
            // are shared, not copied.
            let provider = Arc::clone(&provider);
            let semaphore = Arc::clone(&semaphore);
            let scan_prompt = Arc::clone(&scan_prompt);
            let attack_meaning = Arc::clone(&attack_meaning);
            async move {
                let verdict = judge_one(
                    provider.as_ref(),
                    &semaphore,
                    &scan_prompt,
                    &attack_meaning,
                    &candidate,
                    idx,
                    total,
                )
                .await;
                (candidate, verdict)
            }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await
}

/// Judge one candidate. Every failure mode returns `Err(reason)` — a counted
/// per-item failure — rather than propagating, so one bad reply cannot abort the
/// batch.
async fn judge_one(
    provider: &dyn LlmProvider,
    semaphore: &Semaphore,
    scan_prompt: &str,
    attack_meaning: &str,
    candidate: &BiasInstance,
    idx: usize,
    total: usize,
) -> Result<Verdict, String> {
    // Acquire a permit from the dedicated cap for the duration of the call. A
    // closed semaphore (only at shutdown) is a per-item failure, not a panic.
    let _permit = semaphore
        .acquire()
        .await
        .map_err(|e| format!("theme scan semaphore closed: {e}"))?;

    let user_msg = build_user_message(attack_meaning, candidate);
    let response = call_with_rate_limit_retry(
        provider,
        Some(scan_prompt),
        &user_msg,
        THEME_SCAN_MAX_TOKENS,
        idx,
        total,
    )
    .await
    .map_err(|e| format!("LLM call failed: {e}"))?;

    // Retain the raw reply in the failure reason so a malformed or surprising
    // verdict is diagnosable from the logs (persist_and_summarize logs the reason
    // with evidence_id + scenario_id). This stateless scan has no run table by
    // design (D2b: no migrations), so the log IS its audit surface for the raw
    // model output — the parsed verdict alone would hide what the model said.
    parse_verdict(&response.text).map_err(|reason| {
        let preview: String = response.text.chars().take(500).collect();
        format!("{reason} | raw LLM reply: {preview}")
    })
}

/// Build the per-quote user message: the accusation criterion plus this one
/// quote's speaker, document, and verbatim text. Case-agnostic — all case data
/// comes from the scenario/candidate, none is compiled in.
fn build_user_message(attack_meaning: &str, candidate: &BiasInstance) -> String {
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

/// Write relevant verdicts, count every outcome, and build the summary.
///
/// Writes are per-row and idempotent (`upsert_fact_ref` is `ON CONFLICT`), so a
/// mid-batch write failure is counted and logged without rolling back the
/// already-written suggestions — a best-effort scan keeps the good verdicts.
pub(crate) async fn persist_and_summarize(
    pool: &PgPool,
    scenario_id: Uuid,
    results: Vec<(BiasInstance, Result<Verdict, String>)>,
) -> ThemeScanSummary {
    let candidates_read = results.len();
    let mut tally = ScanTally::default();
    for (candidate, verdict_res) in results {
        record_outcome(pool, scenario_id, candidate, verdict_res, &mut tally).await;
    }
    ThemeScanSummary {
        candidates_read,
        relevant_written: tally.relevant_written,
        irrelevant: tally.irrelevant,
        failed: tally.failed,
        suggestions: tally.suggestions,
        rejected_sample: sample_rejected(tally.rejected, THEME_SCAN_REJECTED_SAMPLE_SIZE),
    }
}

/// Classify one judged candidate into the tally: write + record a relevant
/// verdict, collect an irrelevant one for the sample, or count a failure. Every
/// failure path logs with `evidence_id` + `scenario_id` (Standing Rule 1).
async fn record_outcome(
    pool: &PgPool,
    scenario_id: Uuid,
    candidate: BiasInstance,
    verdict_res: Result<Verdict, String>,
    tally: &mut ScanTally,
) {
    match verdict_res {
        Ok(v) if v.relevant => match write_relevant(pool, scenario_id, &candidate, &v).await {
            Ok(()) => {
                tally.relevant_written += 1;
                tally.suggestions.push(to_suggestion(candidate, &v));
            }
            Err(e) => {
                tally.failed += 1;
                tracing::error!(
                    evidence_id = %candidate.evidence_id,
                    %scenario_id,
                    error = %e,
                    "theme scan: writing a relevant verdict failed"
                );
            }
        },
        Ok(v) => {
            tally.irrelevant += 1;
            tally.rejected.push(ThemeScanRejected {
                graph_node_id: candidate.evidence_id.clone(),
                reason: v.reason,
                confidence: v.confidence,
                content: candidate,
            });
        }
        Err(reason) => {
            tally.failed += 1;
            tracing::error!(
                evidence_id = %candidate.evidence_id,
                %scenario_id,
                reason = %reason,
                "theme scan: producing a verdict failed"
            );
        }
    }
}

/// Upsert one relevant verdict as an `undecided` suggestion.
async fn write_relevant(
    pool: &PgPool,
    scenario_id: Uuid,
    candidate: &BiasInstance,
    verdict: &Verdict,
) -> Result<(), PipelineRepoError> {
    upsert_fact_ref(
        pool,
        scenario_id,
        &candidate.evidence_id,
        Some(verdict.proposed_role.code()),
        // A scan suggestion is UNDECIDED: it awaits a human include/drop ruling.
        FactStatus::Undecided,
        Some(&verdict.reason),
        Some(verdict.confidence),
    )
    .await
}

/// Map a written verdict to its wire suggestion (carries the graph card content).
fn to_suggestion(candidate: BiasInstance, verdict: &Verdict) -> ThemeScanSuggestion {
    ThemeScanSuggestion {
        graph_node_id: candidate.evidence_id.clone(),
        proposed_role: verdict.proposed_role.code().to_string(),
        reason: verdict.reason.clone(),
        confidence: verdict.confidence,
        content: candidate,
    }
}

/// Take an evenly-spread sample of at most `max` rejected quotes.
///
/// A strided pick (indices `k * n / max`) spreads the sample across the whole
/// reject set, which is ordered by `evidence_id`; the first-`max` alternative
/// would bias the honesty check toward one end of the id space. This needs no
/// RNG dependency — the check wants a representative spread, not cryptographic
/// randomness.
fn sample_rejected(rejected: Vec<ThemeScanRejected>, max: usize) -> Vec<ThemeScanRejected> {
    let n = rejected.len();
    if n <= max {
        return rejected;
    }
    // `k * n / max` for k in 0..max is strictly increasing and always < n, so
    // each index is distinct and in-bounds.
    (0..max).map(|k| rejected[k * n / max].clone()).collect()
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

    fn rejected(id: &str) -> ThemeScanRejected {
        ThemeScanRejected {
            graph_node_id: id.to_string(),
            reason: "r".to_string(),
            confidence: 0.1,
            content: instance(id),
        }
    }

    #[test]
    fn sample_returns_all_when_under_max() {
        let set = vec![rejected("a"), rejected("b")];
        let out = sample_rejected(set, 10);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].graph_node_id, "a");
        assert_eq!(out[1].graph_node_id, "b");
    }

    #[test]
    fn sample_caps_and_spreads_when_over_max() {
        let set: Vec<_> = (0..100).map(|i| rejected(&format!("e{i:03}"))).collect();
        let out = sample_rejected(set, 5);
        assert_eq!(out.len(), 5);
        // Strided indices 0, 20, 40, 60, 80 — spread across the set, not the
        // first five.
        assert_eq!(out[0].graph_node_id, "e000");
        assert_eq!(out[1].graph_node_id, "e020");
        assert_eq!(out[4].graph_node_id, "e080");
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
