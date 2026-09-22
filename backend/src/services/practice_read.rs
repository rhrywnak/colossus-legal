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
//! reads the one stored failure line (`practice_read_failed_line`, v2.2.1: "Your
//! answer is saved…"), `read_abstain_reason` says why in plain English, and
//! `read_error` says which failure it was in the operator's terms. That split is
//! Standing Rule 1 exactly. The attempt loop itself lives in
//! [`super::practice_read_attempts`].
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

use crate::services::practice_model_call::{call_model, elapsed_ms};
use crate::services::practice_read_attempts::{
    run_attempts, AttemptRules, Attempts, AttemptsEnd, TokenCost, MAX_ATTEMPTS,
};
use crate::services::practice_read_outcome::{ReadOutcome, MODEL_ABSTAINED_PREFIX};
use crate::services::practice_read_parse::{
    compose_abstain_text, compose_read_text, Overrun, ReadReply, ReplyRejection,
};
use crate::services::practice_read_payload::{build_user_message, ReadPayload};
use crate::services::practice_read_setup::prepare;
use crate::state::AppState;

/// Judge one typed answer, and never propagate a failure.
///
/// ## Domain note: two lines, two audiences (Roman, 2026-09-21)
///
/// Every SYSTEM failure — set-up, an unreachable model, a reply unusable twice —
/// shows Marie the one `practice_read_failed_line`: her answer is saved, and she
/// can press Answer again or discuss it. The specific cause goes to the
/// operator's columns (`read_error`, `read_abstain_reason`) and the log, never
/// to her screen. Only a MODEL decline shows `practice_read_abstain_line` plus
/// the model's own sentence, because that is the model speaking to her about her
/// answer rather than the system reporting a fault.
///
/// # Panics
/// None. Every path returns a [`ReadOutcome`]; the abstain arms carry the reason.
pub async fn read_answer(state: &AppState, payload: &ReadPayload) -> ReadOutcome {
    let snapshot = state.settings.current();
    let model_id = snapshot.practice_read.model.clone();
    let abstain_line = snapshot.practice_report_wording.read_abstain_line.clone();
    let failed_line = snapshot.practice_report_wording.read_failed_line.clone();

    let setup = match prepare(state, &model_id).await {
        Ok(setup) => setup,
        Err(reason) => {
            tracing::error!(model = %model_id, reason = %reason, "practice read: not attempted");
            return ReadOutcome::abstained(
                &failed_line,
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
    let ctx = AttemptRules {
        rules: setup.rules(),
        citable: &citable,
        corrections: setup.corrections.as_ref(),
        model_id: &model_id,
    };
    // The one plumbing (`practice_model_call`). Its retry is the RATE-LIMIT cap,
    // not the re-request inside `run_attempts`: that one exists because a reply
    // can be badly FORMATTED, which asking again does fix.
    //
    // `move` copies the four references into the closure, and the inner
    // `async move` takes the owned message — so each future owns what it reads.
    let (system, params, provider) = (&setup.system, &setup.params, setup.provider.as_ref());
    let run = run_attempts(&ctx, &user, move |message: String| async move {
        call_model(state, provider, system, &message, params).await
    })
    .await;
    let ms = elapsed_ms(started);

    finish(
        &setup.version,
        run,
        &abstain_line,
        &failed_line,
        model_id,
        ms,
    )
}

/// Turn a finished attempt loop into the row that will be stored.
///
/// Pure — `version` rather than the whole setup, so a test can drive every arm
/// without building a provider.
pub(super) fn finish(
    version: &str,
    run: Attempts,
    abstain_line: &str,
    failed_line: &str,
    model_id: String,
    ms: i32,
) -> ReadOutcome {
    let Attempts {
        end,
        attempts,
        spent,
    } = run;
    let tokens = (spent.input, spent.output);
    let (plain, error, raw) = match end {
        AttemptsEnd::Accepted {
            reply,
            raw,
            overruns,
        } => {
            return accept(
                version,
                reply,
                raw,
                abstain_line,
                model_id,
                ms,
                tokens,
                overruns,
                i16::from(attempts),
            );
        }
        AttemptsEnd::Rejected { rejection, raw } => (
            plain_reason_for(&rejection).to_string(),
            format!("{rejection}"),
            Some(raw),
        ),
        AttemptsEnd::CallFailed { error, last_raw } => (
            "the model could not be reached".to_string(),
            error,
            last_raw,
        ),
        AttemptsEnd::NoAttempt => (
            "the read did not complete".to_string(),
            format!("the attempt loop ended after {MAX_ATTEMPTS} attempts without a verdict"),
            None,
        ),
    };
    let mut outcome = ReadOutcome::abstained(failed_line, plain, error, Some(model_id), Some(ms));
    // What the attempts cost stands whatever the last one did — the 2026-09-17
    // row stored NULL tokens for a read that had spent thousands.
    stamp(spent, &mut outcome);
    // The prompt WAS loaded, so the row records which one: "which prompt was
    // live" is the second question of any morning after.
    if attempts > 0 {
        outcome.version = Some(version.to_string());
        outcome.attempts = Some(i16::from(attempts));
    }
    // A reply that came back unusable is the only diagnosis there is.
    outcome.raw_reply = raw;
    outcome
}

/// Write the running token total onto an outcome that is about to be stored.
///
/// ## Why this is a named function and not two assignments at each arm
///
/// Every abstain arm owes the same honesty, and the one that DIDN'T pay it is
/// how a 2026-09-17 read that spent thousands of tokens stored NULL for both
/// counts: the call-failure arm returned before copying. One named move is a
/// thing a test can hold.
fn stamp(spent: TokenCost, outcome: &mut ReadOutcome) {
    outcome.input_tokens = spent.input;
    outcome.output_tokens = spent.output;
}

/// The plain-English reason one reply could not be used — for `read_abstain_reason`.
///
/// Domain note: since v2.2.1 this is an OPERATOR's column, not Marie's screen.
/// She reads `practice_read_failed_line` whatever the cause (Roman, 2026-09-21);
/// this sentence stays on the row so the morning after can say which it was.
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
        // v2.2.1: keys belong in `keys`; one in the prose would have put a
        // meaningless label on her screen.
        ReplyRejection::KeyInProse { .. } => {
            "the model's answer used internal labels instead of words, so it was not shown"
        }
    }
}

/// Turn a usable reply into the row that will be stored.
#[allow(clippy::too_many_arguments)]
fn accept(
    version: &str,
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
                error: Some(format!("{MODEL_ABSTAINED_PREFIX}{reason}")),
                version: Some(version.to_string()),
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
                version: Some(version.to_string()),
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
#[path = "practice_read_tests.rs"]
mod tests;
