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
//!
//! ## Each verdict is DURABLE the moment it is judged (task SCAN_SERVER_STATE, B)
//!
//! The loop used to hold every verdict in memory and write the whole set once, at
//! the end, in `theme_scan_persist`. That made the run all-or-nothing from the
//! outside: for the twenty minutes it took, a scenario's queue showed nothing the
//! scan had found, and a run that was stopped — or that died — took every verdict
//! it had already paid for with it.
//!
//! So the group's verdict rows are inserted HERE, one group at a time, right after
//! the progress bump. The end-of-run batch write stays exactly where it was: it is
//! the one write that sees the whole run, and `ON CONFLICT DO NOTHING` on the
//! verdict table's primary key makes replaying it free. Two writers, one row,
//! no duplicates and no double count.
//!
//! ## Stopping (task SCAN_SERVER_STATE, C)
//!
//! The loop checks a [`CancellationToken`] before each LLM call. A cancelled token
//! means the remaining groups are SKIPPED — counted, never judged, never given a
//! verdict row — and the caller writes `status = 'cancelled'` with the counts as
//! they stand. Conservation holds as `judged + skipped = total`, which is why
//! [`JudgeRun`] carries the skipped count rather than letting it be inferred from
//! a shortfall.

use std::sync::Arc;

use colossus_extract::{LlmProvider, LlmResponse, PipelineError};
use futures::stream::{self, StreamExt};
use sqlx::PgPool;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::bias::dto::BiasInstance;
use crate::domain::llm_params::ResolvedLlmParams;
use crate::llm_retry::call_with_rate_limit_retry_params;
use crate::llm_retry_policy::LlmRetryPolicy;
use crate::repositories::pipeline_repository::{
    bump_scan_run_progress, insert_scan_run_verdicts, ProgressBucket,
};
use crate::services::theme_scan_parse::{parse_verdict, Verdict};
use crate::services::theme_scan_persist::verdict_records;
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

/// What one whole judging pass produced — the results, plus what it did NOT do.
///
/// ## Why the skipped count is CARRIED rather than derived
///
/// A caller could subtract `results.len()` from the pool size and get the same
/// number, and that is exactly the kind of arithmetic that goes wrong quietly: it
/// would also be non-zero if a group vanished for any other reason, and it would
/// read as a dropped candidate rather than as a deliberate stop. Counting the
/// skips where they happen keeps `judged + skipped = total` a statement the code
/// makes rather than one a reader has to reconstruct (Standing Rule 1).
pub(crate) struct JudgeRun {
    /// One entry per group that was actually judged. Ordering is not preserved
    /// (`buffer_unordered`), which is irrelevant — the persist pass aggregates and
    /// each result carries its own candidate.
    pub results: Vec<(CandidateGroup, JudgeOutcome)>,
    /// Groups the stop passed over. Never judged, never given a verdict row, and
    /// never counted in any outcome bucket — they simply did not happen.
    pub skipped: usize,
    /// Whether the token was cancelled by the time the pass ended. Read rather
    /// than inferred from `skipped > 0`: a stop that arrives on the very last
    /// group skips nothing and is still a stop, and the run must still be recorded
    /// `cancelled` rather than `completed`.
    pub cancelled: bool,
}

/// Everything one judging pass needs that is the same for every candidate.
///
/// ## Rust Learning: a parameter bundle, and why it is not just tidiness
///
/// This function took eleven parameters and carried
/// `#[allow(clippy::too_many_arguments)]` to say so. The lint is not fussiness:
/// eleven positional arguments of which three are `Arc<...>` and two are `usize`
/// is a call site where two values can be swapped and still compile. Naming them
/// removes that class of mistake and lets the `allow` go.
///
/// Every field is cheap to clone — `Arc` and `PgPool` clones are refcount bumps,
/// `ResolvedLlmParams` and `LlmRetryPolicy` are `Copy`, and a `CancellationToken`
/// clone is a handle on the SAME token (a deep copy would be a token nobody can
/// cancel). So the per-item clones inside the fan-out stay as cheap as they were.
pub(crate) struct JudgeInputs {
    pub provider: Arc<dyn LlmProvider>,
    pub semaphore: Arc<Semaphore>,
    pub concurrency: usize,
    pub scan_prompt: Arc<str>,
    pub scan_criteria: Arc<str>,
    pub params: ResolvedLlmParams,
    pub pool: PgPool,
    pub run_id: Uuid,
    /// The automatic-retry caps, from `LLM_RETRY_MAX` /
    /// `LLM_RATE_LIMIT_RETRY_MAX` — see [`crate::llm_retry_policy`].
    pub policy: LlmRetryPolicy,
    /// The stop handle. Checked BEFORE each call rather than raced against one,
    /// so a stop never abandons an LLM request that has already been paid for.
    pub cancel: CancellationToken,
}

/// Judge every candidate GROUP concurrently, bounded by the dedicated semaphore,
/// reporting LIVE progress and a durable verdict row to the database as each
/// group completes.
///
/// One group is one LLM call. A group of one is an ordinary candidate; a group of
/// two or more is a set of byte-identical quotes the pre-filter folded together
/// (task 2.15 Tier 2) — the same sentence judged once instead of twice. The
/// group's members ride through untouched so the persist pass can write each one
/// its own verdict row.
///
/// `pool` + `run_id` drive two per-group writes. [`bump_scan_run_progress`] is the
/// `chunks_processed` analog and is BEST-EFFORT: a failed bump is logged and the
/// scan continues — losing a progress tick must not lose a verdict (Standing
/// Rule 1: the failure is observable in the log, and the final counts are
/// authoritative regardless). The live bucket is the VERDICT-time classification.
/// The verdict INSERT beside it is best-effort in the same sense and for a
/// stronger reason: the end-of-run batch write will replay it, so one failed
/// insert costs live visibility, never the record.
pub(crate) async fn judge_all(inputs: JudgeInputs, groups: Vec<CandidateGroup>) -> JudgeRun {
    let total = groups.len();
    // ## Rust Learning: `filter_map` over `Option` to drop the skipped groups
    //
    // Each concurrent task returns `Option<(CandidateGroup, JudgeOutcome)>` —
    // `None` for a group the stop passed over. `buffer_unordered` keeps the
    // concurrency bound, and `filter_map(ready)` unwraps the `Some`s and discards
    // the `None`s in one pass. `stream::iter(x).filter_map` wants a FUTURE back,
    // which is what `std::future::ready` supplies for an already-known value.
    let results: Vec<(CandidateGroup, JudgeOutcome)> = stream::iter(groups.into_iter().enumerate())
        .map(|(idx, group)| judge_and_record(&inputs, group, idx, total))
        .buffer_unordered(inputs.concurrency)
        .filter_map(std::future::ready)
        .collect()
        .await;

    JudgeRun {
        skipped: total.saturating_sub(results.len()),
        cancelled: inputs.cancel.is_cancelled(),
        results,
    }
}

/// Judge ONE group and write what it produced, or skip it because a stop landed.
///
/// `None` means skipped — never judged, never given a verdict row, never counted
/// in any outcome bucket. The check is the first thing in the future's body, so a
/// stopped scan stops SPENDING: everything below that line costs money.
///
/// ## Rust Learning: why this returns a future rather than being `async fn`
///
/// The `Stream::map` closure must produce owned futures that outlive the borrow of
/// `inputs`, so the per-item clones happen HERE (synchronously, before the `async
/// move` block) and the block owns them. An `async fn` taking `&JudgeInputs` would
/// hold that borrow across every `.await` in the fan-out, and `buffer_unordered`
/// needs the futures to be independent of each other.
fn judge_and_record(
    inputs: &JudgeInputs,
    group: CandidateGroup,
    idx: usize,
    total: usize,
) -> impl std::future::Future<Output = Option<(CandidateGroup, JudgeOutcome)>> {
    // Cheap per-item clones — see `JudgeInputs` for why each is a refcount bump,
    // a `Copy`, or a handle on the same token.
    let provider = Arc::clone(&inputs.provider);
    let semaphore = Arc::clone(&inputs.semaphore);
    let scan_prompt = Arc::clone(&inputs.scan_prompt);
    let scan_criteria = Arc::clone(&inputs.scan_criteria);
    let pool = inputs.pool.clone();
    let cancel = inputs.cancel.clone();
    let (params, policy, run_id) = (inputs.params, inputs.policy, inputs.run_id);
    async move {
        if cancel.is_cancelled() {
            return None;
        }
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
            policy,
        )
        .await;
        record_group(&pool, run_id, &group, &outcome).await;
        Some((group, outcome))
    }
}

/// The two per-group writes, in the order the screen reads them.
///
/// Progress FIRST, then the verdict rows. A human watching the running view sees
/// the counter move and then the queue grow, which is the right way round: the
/// counter is the claim ("I judged one more") and the verdict is the evidence for
/// it. Reversed, a queue could briefly show a proposal from a run whose counter
/// had not yet admitted judging anything.
///
/// Both are best-effort and both are LOUD — see each one's own doc.
async fn record_group(pool: &PgPool, run_id: Uuid, group: &CandidateGroup, outcome: &JudgeOutcome) {
    report_progress(pool, run_id, outcome).await;
    record_verdict(pool, run_id, group, outcome).await;
}

/// Write this group's verdict rows the moment it is judged.
///
/// Best-effort but LOUD, and safe to fail: `theme_scan_persist::write_verdicts`
/// replays the whole set at the end of the run with the same `ON CONFLICT DO
/// NOTHING` clause, so a failure here costs LIVE visibility of one candidate and
/// never the record. It is logged with the run and the group's representative so
/// a scenario whose queue is not filling as the counter climbs has a reason in the
/// log rather than a mystery.
///
/// A run a human STOPS is the case that makes this write matter: nothing replays
/// for it, so what is on disk when the token fires is what the scan produced.
async fn record_verdict(
    pool: &PgPool,
    run_id: Uuid,
    group: &CandidateGroup,
    outcome: &JudgeOutcome,
) {
    let records = verdict_records(run_id, &group.members, outcome);
    if let Err(e) = insert_scan_run_verdicts(pool, &records).await {
        tracing::warn!(
            %run_id,
            evidence_id = %group.representative.evidence_id,
            members = group.members.len(),
            error = %e,
            "theme scan: live verdict write failed (continuing; the end-of-run \
             batch will replay it)"
        );
    }
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
    policy: LlmRetryPolicy,
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
        policy,
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

// The LIVE-DATABASE proofs for the per-verdict write and the stop (task
// SCAN_SERVER_STATE, parts B and C). A separate sibling because they are a
// different KIND of test — `#[ignore]`d, needing a real pipeline database — and
// mixing them into the pure module above would hide that distinction behind an
// attribute halfway down a file.
#[cfg(test)]
#[path = "theme_scan_judge_live_tests.rs"]
mod live_tests;
