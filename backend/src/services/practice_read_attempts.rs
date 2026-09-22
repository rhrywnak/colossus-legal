//! The attempt loop: ask, judge the reply, and ask ONCE more — with a
//! correction — when the reply was unusable.
//!
//! Split from [`super::practice_read`] in v2.2.1, for two reasons. Rule 17:
//! the corrective re-request took that module past 300 lines. And testability:
//! the loop is generic over the call itself (`ask`), so a unit test can hand it
//! recorded replies and assert what attempt 2 SENT — which is the one claim the
//! corrective re-request makes, and one no test could see while the loop called
//! the provider directly.
//!
//! What a finished loop MEANS for the stored row — the failure line, the model's
//! own decline, the parts — stays in `practice_read`. This module never builds a
//! [`super::practice_read_outcome::ReadOutcome`].

use std::collections::BTreeSet;
use std::fmt::Display;
use std::future::Future;

use colossus_extract::LlmResponse;

use crate::services::practice_model_call::response_tokens;
use crate::services::practice_read_corrections::Corrections;
use crate::services::practice_read_parse::{
    parse_reply, Overrun, ReadReply, ReadRules, ReplyRejection,
};

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
pub(crate) const MAX_ATTEMPTS: u8 = 2;

/// What one reply is judged against, borrowed from the caller's setup.
pub(crate) struct AttemptRules<'a> {
    pub(crate) rules: ReadRules<'a>,
    pub(crate) citable: &'a BTreeSet<String>,
    /// `None` when the prompt file carries no corrections section (v4): the
    /// re-request is then the plain one, as it was before v2.2.1.
    pub(crate) corrections: Option<&'a Corrections>,
    /// For the log lines only.
    pub(crate) model_id: &'a str,
}

/// How the loop ended.
#[derive(Debug)]
pub(crate) enum AttemptsEnd {
    /// A reply was usable — parts or the model's own decline.
    Accepted {
        reply: ReadReply,
        raw: String,
        overruns: Vec<Overrun>,
    },
    /// The last reply was unusable, and there are no attempts left.
    Rejected {
        rejection: ReplyRejection,
        raw: String,
    },
    /// A call never returned. `last_raw` is the earlier reply, if one came back
    /// unusable first — the only place it survives for diagnosis.
    CallFailed {
        error: String,
        last_raw: Option<String>,
    },
    /// The loop made no attempt at all — only reachable if `MAX_ATTEMPTS` were
    /// edited to zero. An arm rather than a panic: a panic here would be a 500
    /// on an answer a witness has already typed.
    NoAttempt,
}

/// The loop's result: how it ended, how many calls it made, what they cost.
#[derive(Debug)]
pub(crate) struct Attempts {
    pub(crate) end: AttemptsEnd,
    /// How many times the model was asked (0 only on `NoAttempt`).
    pub(crate) attempts: u8,
    pub(crate) spent: TokenCost,
}

/// Ask, judge, and re-request at most once.
///
/// `ask` makes one call with the user message it is given. Attempt 1 sends
/// `user`. Attempt 2 sends:
///
/// - after a REJECTED reply, with corrections loaded: `user`, then the rejected
///   reply and the one-sentence correction (`Corrections::resend`);
/// - after an overrun, an empty reply, or with no corrections section: `user`
///   again, unchanged — logged, so a plain re-request is never a silent one.
///
/// ## Rust Learning: a generic `FnMut` returning a `Future`
///
/// `F: FnMut(String) -> Fut` with `Fut: Future<Output = …>` is how Rust spells
/// "an async callback". `FnMut` because the caller's closure may be called twice
/// and a test's closure records each request into a `Vec` it owns (mutation).
/// The message is passed BY VALUE (`String`) so the future can own it: a future
/// holding a `&str` into this function's local would not outlive the call that
/// built it. The error type is generic (`E: Display`) because this loop only
/// ever formats it — the provider's own error type stays the provider's.
pub(crate) async fn run_attempts<F, Fut, E>(
    ctx: &AttemptRules<'_>,
    user: &str,
    mut ask: F,
) -> Attempts
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<LlmResponse, E>>,
    E: Display,
{
    let mut spent = TokenCost::default();
    let mut message = user.to_string();
    let mut last: Option<String> = None;

    for attempt in 1..=MAX_ATTEMPTS {
        let response = match ask(message.clone()).await {
            Ok(response) => response,
            Err(e) => {
                // A call that never returned is not a formatting slip and does
                // not improve by being asked again — the retry that IS worth
                // making (a rate limit) already happened inside the call.
                let error = format!("the call failed: {e}");
                tracing::warn!(model = %ctx.model_id, attempt, reason = %error, "practice read: call failed");
                return Attempts {
                    end: AttemptsEnd::CallFailed {
                        error,
                        last_raw: last,
                    },
                    attempts: attempt,
                    spent,
                };
            }
        };
        let (input, output) = response_tokens(&response);
        spent.add(input, output);
        let last_attempt = attempt == MAX_ATTEMPTS;

        match parse_reply(&response.text, ctx.rules, ctx.citable) {
            Ok((reply, overruns)) => {
                // An overrun on the FIRST attempt buys one more try at a tidy
                // reply. On the last it is kept as returned — never truncated,
                // never discarded — and logged with the part and the count.
                if !overruns.is_empty() && !last_attempt {
                    log_overruns(ctx.model_id, attempt, &overruns, "re-requesting once");
                    tracing::warn!(
                        model = %ctx.model_id, attempt,
                        input_tokens = spent.input, output_tokens = spent.output,
                        "practice read: re-requesting — this attempt's cost stands whatever the next returns"
                    );
                    last = Some(response.text);
                    continue;
                }
                if !overruns.is_empty() {
                    log_overruns(ctx.model_id, attempt, &overruns, "stored as returned");
                }
                return Attempts {
                    end: AttemptsEnd::Accepted {
                        reply,
                        raw: response.text,
                        overruns,
                    },
                    attempts: attempt,
                    spent,
                };
            }
            Err(rejection) if !last_attempt => {
                tracing::warn!(
                    model = %ctx.model_id, attempt, reason = %rejection,
                    input_tokens = spent.input, output_tokens = spent.output,
                    reply = %clip(&response.text),
                    "practice read: reply unusable — re-requesting once"
                );
                message = next_message(ctx, user, &response.text, &rejection);
                last = Some(response.text);
            }
            Err(rejection) => {
                tracing::warn!(
                    model = %ctx.model_id, attempt, reason = %rejection,
                    reply = %clip(&response.text),
                    "practice read: reply unusable twice — abstaining"
                );
                return Attempts {
                    end: AttemptsEnd::Rejected {
                        rejection,
                        raw: response.text,
                    },
                    attempts: attempt,
                    spent,
                };
            }
        }
    }

    tracing::error!(model = %ctx.model_id, "practice read: the attempt loop ended without a verdict");
    Attempts {
        end: AttemptsEnd::NoAttempt,
        attempts: 0,
        spent,
    }
}

/// Attempt 2's message after a rejected reply: corrected when the prompt file
/// carries corrections and one fits, plain otherwise — and the log says which.
fn next_message(
    ctx: &AttemptRules<'_>,
    user: &str,
    rejected: &str,
    rejection: &ReplyRejection,
) -> String {
    let Some(corrections) = ctx.corrections else {
        tracing::warn!(
            model = %ctx.model_id,
            "practice read: the prompt file carries no corrections section — re-requesting plainly"
        );
        return user.to_string();
    };
    match corrections.correction_for(rejection) {
        Some(correction) => {
            tracing::info!(model = %ctx.model_id, correction = %correction, "practice read: re-requesting with a correction");
            corrections.resend(user, rejected, &correction)
        }
        // An EMPTY reply: nothing to send back and nothing to correct.
        None => {
            tracing::info!(model = %ctx.model_id, "practice read: the reply was empty — re-requesting plainly");
            user.to_string()
        }
    }
}

/// What every attempt on one answer has cost so far.
///
/// ## Domain note: accumulated across EVERY attempt, not just the one that succeeded
///
/// A re-request doubles the spend, and `read_ms` already reports the whole
/// wall-clock cost because that is what Marie waited. The tokens follow the same
/// rule for the same reason: a row saying one call's worth after two were made
/// would understate the cost of exactly the answers that were most expensive.
///
/// ## Rust Learning: `Option` addition that keeps "not reported" distinct from zero
///
/// A provider may report no token count at all. Adding `None` to a running total
/// must not turn it into `Some(0)` — that would claim a call was free — so an
/// unreported attempt leaves the total exactly as it was, and a total that was
/// never reported at all stays `None`. `saturating_add` because the columns are
/// INTEGER and wrapping a cost into a negative number is worse than capping it.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TokenCost {
    pub(crate) input: Option<i32>,
    pub(crate) output: Option<i32>,
}

impl TokenCost {
    pub(crate) fn add(&mut self, input: Option<i32>, output: Option<i32>) {
        self.input = accumulate(self.input, input);
        self.output = accumulate(self.output, output);
    }
}

/// One side of the running total.
pub(crate) fn accumulate(running: Option<i32>, next: Option<i32>) -> Option<i32> {
    match (running, next) {
        (Some(a), Some(b)) => Some(a.saturating_add(b)),
        (Some(a), None) => Some(a),
        (None, next) => next,
    }
}

/// The first 300 characters of a reply, for a log line.
fn clip(reply: &str) -> String {
    reply.chars().take(300).collect()
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

#[cfg(test)]
#[path = "practice_read_attempts_tests.rs"]
mod tests;
