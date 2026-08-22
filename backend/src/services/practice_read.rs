//! The read: resolving the model, making the call, and judging what came back.
//!
//! The impure half. What is SENT lives in [`super::practice_read_payload`] and
//! [`super::practice_read_gather`]; what is ACCEPTED BACK lives in
//! [`super::practice_read_parse`], where both are unit-tested without a provider.
//!
//! ## The contract this module keeps with the screen
//!
//! There is exactly one way for a judgement to reach Marie: a call that returned
//! parseable parts citing only keys it was sent. Every other outcome — an unknown
//! model, a missing prompt file, a timeout, a rate limit, an input that failed to
//! load, a reply that would not parse twice running — takes the ABSTAIN arm: she
//! reads the stored "I can't read this one." line, `read_abstain_reason` says why
//! in plain English, and `read_error` says which failure it was in the operator's
//! terms. That split is Standing Rule 1 exactly.
//!
//! ## What changed in T1, and why it is architecture rather than polish
//!
//! **The answer row is written and committed BEFORE the read is requested.** It
//! used to be the reverse: the model was called, and the row — her typed answer,
//! her chips, her mark — existed nowhere until the call returned. A vendor being
//! slow could not lose her answer only because the handler happened to await
//! successfully. Now it cannot lose it at all, because the row is already on disk
//! before the first token is sent.
//!
//! **An abstain is a sentence, not a silence.** v2 stored `read_text = NULL` for
//! every failure and the screen printed a fixed "no system read this time" line
//! that said nothing about which of six things had happened. The abstain arm
//! speaks: Marie is told the read declined, and when the MODEL declined it is
//! told in the model's own words.

use std::time::Instant;

use crate::llm_retry::call_with_rate_limit_retry_params;
use crate::services::practice_read_outcome::ReadOutcome;
use crate::services::practice_read_parse::{
    compose_abstain_text, compose_read_text, parse_reply, Overrun, ReadReply, ReplyRejection,
};
use crate::services::practice_read_payload::{build_user_message, ReadPayload};
use crate::services::practice_read_setup::{prepare, ReadSetup};
use crate::state::AppState;

/// How many times one answer may be sent to the model.
///
/// STRUCTURAL, not a tunable: it is the arithmetic of Roman's rule, not a dial.
/// A ceiling overrun or a reply that will not parse is re-requested ONCE — twice
/// total — because a formatting slip is not the witness's fault and must not cost
/// her the coaching, while a third attempt would spend a witness's evening
/// waiting on a model that is not going to comply. Raising this on the Settings
/// page would let an operator turn one answer into an unbounded spend.
//
// STRUCTURAL: the two-attempt bound is the arithmetic of Roman's ruling of
// 2026-08-20, not a per-deployment dial. See the doc comment above.
// CONST: structural — not a tunable; never a settings row.
const MAX_ATTEMPTS: u8 = 2;

/// Judge one typed answer, and never propagate a failure.
///
/// # Panics
/// None. Every path returns a [`ReadOutcome`]; the abstain arms carry the reason.
pub async fn read_answer(state: &AppState, payload: &ReadPayload) -> ReadOutcome {
    let snapshot = state.settings.current();
    let model_id = snapshot.practice_read.model.clone();
    let abstain_line = snapshot.practice_report_wording.read_abstain_line.clone();

    let setup = match prepare(state, &model_id).await {
        Ok(setup) => setup,
        Err(reason) => {
            tracing::error!(model = %model_id, reason = %reason, "practice read: not attempted");
            return ReadOutcome::abstained(
                &abstain_line,
                "the read could not be set up — this is a deployment fault, not your answer"
                    .to_string(),
                reason,
                Some(model_id),
                None,
            );
        }
    };

    let user = build_user_message(payload);
    let citable = payload.citable_keys();
    let started = Instant::now();
    let mut last: Option<String> = None;
    // Accumulated across EVERY attempt, not just the one that succeeded.
    //
    // ## Domain note: what the row's token counts mean
    //
    // A re-request doubles the spend, and `read_ms` already reports the whole
    // wall-clock cost because that is what Marie waited. The tokens follow the
    // same rule for the same reason: a row saying one call's worth after two were
    // made would understate the cost of exactly the answers that were most
    // expensive, and a wave of re-requests would be invisible in any total.
    let mut spent = TokenCost::default();

    for attempt in 1..=MAX_ATTEMPTS {
        let result = call_with_rate_limit_retry_params(
            setup.provider.as_ref(),
            Some(&setup.system),
            &user,
            &setup.params,
            0,
            1,
        )
        .await;
        let ms = elapsed_ms(started);

        let response = match result {
            Ok(response) => response,
            Err(e) => {
                // A call that never returned is not a formatting slip and does not
                // improve by being asked again — the retry that IS worth making
                // (a rate limit) already happened inside the call above.
                let reason = format!("the call failed: {e}");
                tracing::warn!(model = %model_id, ms, attempt, reason = %reason, "practice read: call failed");
                let mut outcome = ReadOutcome::abstained(
                    &abstain_line,
                    "the model could not be reached".to_string(),
                    reason,
                    Some(model_id),
                    Some(ms),
                );
                // The prompt WAS loaded and sent, so the row records which one:
                // "which prompt was live" is the second question of any morning
                // after, and T3's no-op rule keys on this column.
                outcome.version = Some(setup.version.clone());
                outcome.attempts = Some(i16::from(attempt));
                // A first attempt that came back unusable, followed by a second
                // that never came back at all, still leaves something to diagnose
                // from — and this is the only place it survives.
                outcome.raw_reply = last;
                return outcome;
            }
        };
        // best-effort: the columns are INTEGER and a token count above 2^31 is not
        // a number any model in the registry can produce — the largest ceiling
        // here is 128,000. A value that somehow did not fit is recorded as "not
        // reported" rather than failing an answer over a metric, and `ms` and
        // `model` still say the call happened.
        spent.add(
            response.input_tokens.and_then(|n| i32::try_from(n).ok()),
            response.output_tokens.and_then(|n| i32::try_from(n).ok()),
        );
        let tokens = (spent.input, spent.output);

        match parse_reply(&response.text, setup.rules(), &citable) {
            Ok((reply, overruns)) => {
                // An overrun on the FIRST attempt buys one more try at a tidy
                // reply. On the last it is kept as returned — never truncated,
                // never discarded — and logged with the part and the count.
                if !overruns.is_empty() && attempt < MAX_ATTEMPTS {
                    log_overruns(&model_id, attempt, &overruns, "re-requesting once");
                    tracing::warn!(
                        model = %model_id, ms, attempt,
                        input_tokens = tokens.0, output_tokens = tokens.1,
                        "practice read: re-requesting — this attempt's cost stands whatever the next returns"
                    );
                    last = Some(response.text);
                    continue;
                }
                if !overruns.is_empty() {
                    log_overruns(&model_id, attempt, &overruns, "stored as returned");
                }
                return accept(
                    &setup,
                    reply,
                    response.text,
                    &abstain_line,
                    model_id,
                    ms,
                    tokens,
                    overruns,
                    i16::from(attempt),
                );
            }
            Err(rejection) => {
                let reason = format!("{rejection}");
                if attempt < MAX_ATTEMPTS {
                    tracing::warn!(
                        model = %model_id, ms, attempt, reason = %reason,
                        input_tokens = tokens.0, output_tokens = tokens.1,
                        reply = %clip(&response.text),
                        "practice read: reply unusable — re-requesting once"
                    );
                    last = Some(response.text);
                    continue;
                }
                tracing::warn!(
                    model = %model_id, ms, attempt, reason = %reason,
                    reply = %clip(&response.text),
                    "practice read: reply unusable twice — abstaining"
                );
                let mut outcome = ReadOutcome::abstained(
                    &abstain_line,
                    plain_reason_for(&rejection).to_string(),
                    reason,
                    Some(model_id),
                    Some(ms),
                );
                outcome.input_tokens = tokens.0;
                outcome.output_tokens = tokens.1;
                outcome.version = Some(setup.version.clone());
                outcome.attempts = Some(i16::from(attempt));
                outcome.raw_reply = Some(response.text);
                return outcome;
            }
        }
    }

    // Unreachable: the loop returns on every path of its final iteration. Written
    // as an abstain rather than `unreachable!()` because a panic here would be a
    // 500 on an answer a witness has already typed, and MAX_ATTEMPTS is the kind
    // of constant a later edit could set to zero.
    tracing::error!(model = %model_id, "practice read: the attempt loop ended without a verdict");
    let mut outcome = ReadOutcome::abstained(
        &abstain_line,
        "the read did not complete".to_string(),
        format!("the attempt loop ended after {MAX_ATTEMPTS} attempts without a verdict"),
        Some(model_id),
        Some(elapsed_ms(started)),
    );
    outcome.raw_reply = last;
    outcome
}

/// What every attempt on one answer has cost so far.
///
/// ## Rust Learning: `Option` addition that keeps "not reported" distinct from zero
///
/// A provider may report no token count at all. Adding `None` to a running total
/// must not turn it into `Some(0)` — that would claim a call was free — so an
/// unreported attempt leaves the total exactly as it was, and a total that was
/// never reported at all stays `None`. `saturating_add` because the columns are
/// INTEGER and wrapping a cost into a negative number is worse than capping it.
#[derive(Debug, Default, Clone, Copy)]
struct TokenCost {
    input: Option<i32>,
    output: Option<i32>,
}

impl TokenCost {
    fn add(&mut self, input: Option<i32>, output: Option<i32>) {
        self.input = accumulate(self.input, input);
        self.output = accumulate(self.output, output);
    }
}

/// One side of the running total.
fn accumulate(running: Option<i32>, next: Option<i32>) -> Option<i32> {
    match (running, next) {
        (Some(a), Some(b)) => Some(a.saturating_add(b)),
        (Some(a), None) => Some(a),
        (None, next) => next,
    }
}

/// Milliseconds since the first attempt began.
///
/// Domain note: this spans EVERY attempt, so a re-requested read records the
/// whole wall-clock cost rather than only the successful half. That is the honest
/// number — it is what Marie waited.
fn elapsed_ms(started: Instant) -> i32 {
    i32::try_from(started.elapsed().as_millis()).unwrap_or(i32::MAX)
}

/// The first 300 characters of a reply, for a log line.
fn clip(reply: &str) -> String {
    reply.chars().take(300).collect()
}

/// Marie's half of the reason one reply could not be used.
fn plain_reason_for(rejection: &ReplyRejection) -> &'static str {
    match rejection {
        ReplyRejection::Empty => "the model sent nothing back",
        ReplyRejection::Unparseable { .. } | ReplyRejection::NothingSaid => {
            "the model's answer did not come back in a form this build could read"
        }
        // The one that is a JUDGEMENT and not a mechanical failure: the model
        // named a document it was not given, which is the invention this whole
        // task exists to make impossible.
        ReplyRejection::UnknownKey { .. } => {
            "the model cited something it was not given, so the read was not trusted"
        }
    }
}

/// One line per overrun — the part and the count, as the ruling requires.
fn log_overruns(model_id: &str, attempt: u8, overruns: &[Overrun], disposition: &str) {
    for overrun in overruns {
        tracing::warn!(
            model = %model_id,
            attempt,
            part = %overrun.part,
            words = overrun.words,
            limit = overrun.limit,
            disposition,
            "practice read: a part came back over its ceiling"
        );
    }
}

/// Turn a usable reply into the row that will be stored.
#[allow(clippy::too_many_arguments)]
fn accept(
    setup: &ReadSetup,
    reply: ReadReply,
    raw: String,
    abstain_line: &str,
    model_id: String,
    ms: i32,
    tokens: (Option<i32>, Option<i32>),
    overruns: Vec<Overrun>,
    attempts: i16,
) -> ReadOutcome {
    let (input_tokens, output_tokens) = tokens;
    match reply {
        // The MODEL declined. Its reason is plain English by construction — the
        // prompt asks for it in the model's own voice — so Marie reads it after
        // the stored line rather than a sentence this build guessed at.
        ReadReply::Abstain(reason) => {
            tracing::info!(
                model = %model_id, ms, input_tokens, output_tokens, reason = %reason,
                "practice read: the model abstained"
            );
            ReadOutcome {
                text: Some(compose_abstain_text(abstain_line, Some(&reason))),
                abstain_reason: Some(reason.clone()),
                error: Some(format!("the model abstained: {reason}")),
                version: Some(setup.version.clone()),
                attempts: Some(attempts),
                input_tokens,
                output_tokens,
                ms: Some(ms),
                model: Some(model_id),
                // KEPT, unlike the accepted arm. There the parts ARE the reply,
                // column by column, and nothing is lost. Here only the abstain
                // sentence is stored — and `RawReply` tolerates unknown fields, so
                // a model that declined AND wrote something alongside it would
                // have that second half discarded. A wave of abstains is a prompt
                // problem, and diagnosing one means reading what the model wrote.
                raw_reply: Some(raw),
                ..Default::default()
            }
        }
        ReadReply::Parts(parts) => {
            tracing::info!(
                model = %model_id, ms, input_tokens, output_tokens, ok = parts.ok,
                pointers = parts.pointers.len(), keys = %parts.keys.join(" "),
                overruns = overruns.len(),
                "practice read"
            );
            ReadOutcome {
                text: Some(compose_read_text(&parts)),
                ok: Some(parts.ok),
                error: None,
                abstain_reason: None,
                version: Some(setup.version.clone()),
                parts: Some(parts),
                attempts: Some(attempts),
                overruns,
                input_tokens,
                output_tokens,
                ms: Some(ms),
                model: Some(model_id),
                // Nothing to keep on the accepted arm: the parts ARE the model's
                // own words, stored column by column. An overrun kept as returned
                // is likewise stored in full — there is no discarded remainder for
                // a raw copy to rescue, which was not true before T1.
                raw_reply: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{accumulate, TokenCost, MAX_ATTEMPTS};

    /// One answer is sent to the model AT MOST twice.
    ///
    /// The re-request is bounded, and the bound is arithmetic rather than a dial.
    /// A `MAX_ATTEMPTS` raised on a Settings page would let one typed answer
    /// become an unbounded spend against a model that is not going to comply; a
    /// `MAX_ATTEMPTS` of 1 would silently retire the re-request rule, and every
    /// formatting slip would go back to costing Marie her coaching.
    #[test]
    fn one_answer_is_never_sent_to_the_model_more_than_twice() {
        assert_eq!(MAX_ATTEMPTS, 2);
    }

    /// Two attempts cost what two attempts cost.
    ///
    /// `read_ms` already spans every attempt, because that is what Marie waited.
    /// The tokens follow the same rule: a row reporting one call's worth after two
    /// were made understates exactly the answers that were most expensive, and a
    /// wave of re-requests would then be invisible in any total anybody computes.
    #[test]
    fn a_re_requested_read_records_what_both_attempts_cost() {
        let mut spent = TokenCost::default();
        spent.add(Some(2100), Some(180));
        spent.add(Some(2100), Some(240));

        assert_eq!(spent.input, Some(4200));
        assert_eq!(spent.output, Some(420));
    }

    /// An unreported count is not a free call.
    ///
    /// A provider that reports no tokens must leave the running total alone.
    /// Folding `None` in as zero would let one silent attempt make a two-call
    /// answer look like a one-call answer, which is the same lie the test above
    /// exists to prevent, arrived at from the other direction.
    #[test]
    fn an_unreported_token_count_never_reads_as_zero() {
        assert_eq!(accumulate(None, None), None, "never reported stays unknown");
        assert_eq!(accumulate(Some(2100), None), Some(2100));
        assert_eq!(accumulate(None, Some(2100)), Some(2100));
        // And it never wraps into a negative cost.
        assert_eq!(accumulate(Some(i32::MAX), Some(10)), Some(i32::MAX));
    }
}
