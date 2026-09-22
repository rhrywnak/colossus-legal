//! What one read produced, and the row it writes.
//!
//! Pure: no provider, no database, no clock. Split from [`super::practice_read`]
//! in T1 when the attempt loop and the abstain arms took that module past the
//! 300-line limit (Rule 17). The seam is the one the type already draws — that
//! module DECIDES what happened, and this is what "what happened" IS, plus the
//! fourteen columns it becomes.
//!
//! Keeping the mapping here also means the test that matters most — that no two
//! outcomes write the same row — needs nothing but this file.

use crate::repositories::pipeline_repository::practice_answers::AnswerRead;
use crate::services::practice_read_gather::PayloadFailure;
use crate::services::practice_read_parse::{compose_abstain_text, Overrun, ReadParts};

/// What one read attempt produced, in the shape the answer row stores.
///
/// ## Rust Learning: one struct instead of `Result`
///
/// A `Result` would push the caller into `match`ing two shapes for something the
/// database stores as one row either way. Every field here maps to a column, and
/// the token counts must survive BOTH arms — a call that succeeded and was then
/// judged an abstain still cost money and still must be logged.
#[derive(Debug, Clone, Default)]
pub struct ReadOutcome {
    /// The single composed line the untouched frontend renders, from
    /// `compose_read_text` or `compose_abstain_text`. `None` only when no read
    /// was asked for at all.
    pub text: Option<String>,
    /// `Some(true)` = the OK word, `Some(false)` = a fault was named,
    /// `None` = an abstain or no read. The neutral rail is the third state.
    pub ok: Option<bool>,
    /// The operator's reason there is no judgement. `None` when there is one.
    pub error: Option<String>,
    /// Marie's reason, in plain English. `Some` exactly on the abstain arm.
    pub abstain_reason: Option<String>,
    /// The three parts, when there are three parts.
    pub parts: Option<ReadParts>,
    /// Which prompt produced this. Recorded even on an abstain — "which prompt
    /// was live" is the second question of any morning after, right behind
    /// "which model".
    pub version: Option<String>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    /// Wall-clock milliseconds across every attempt, whether they succeeded.
    pub ms: Option<i32>,
    pub model: Option<String>,
    /// What the model said, when this build could not use it.
    pub raw_reply: Option<String>,
    /// How many times the model was asked. `None` when it was never asked.
    ///
    /// Stored, not merely logged, because the token counts on this row are the
    /// SUM across attempts: 4,200 input tokens is one expensive call or two
    /// ordinary ones, and nothing else on the row can say which.
    pub attempts: Option<i16>,
    /// The parts stored OVER their ceiling, with the ceiling they were over.
    ///
    /// Empty in the ordinary case. Kept on the row because a ceiling is a settings
    /// row an operator will move, and "was this read long?" is unanswerable later
    /// unless the limit that was in force is recorded beside the count.
    pub overruns: Vec<Overrun>,
}

/// The stamp a read that never went to a model wears in `read_version`.
///
/// §2.4 allows "the prompt file name OR AN EQUIVALENT STAMP", and this is the
/// equivalent: naming a prompt file here would claim a model produced a line this
/// build wrote itself. It is also what lets T3's no-op rule tell a stored line
/// from a judgement without re-reading the text.
pub const STORED_READ_VERSION: &str = "stored:dont-recall";

/// What a row wears when NOBODY ASKED for a read.
///
/// ## Domain note: the fourth state of an answer row, named rather than implied
///
/// Three already existed: a read that succeeded, a read that abstained, and a
/// read IN FLIGHT (`practice_answer_read::READ_IN_FLIGHT`, which is also the shape of
/// a backend that died mid-read). The Answer-analysis switch creates a fourth —
/// the answer was recorded and no model was ever asked — and it must not borrow
/// the in-flight marker: an operator reading `no read yet: … the model is being
/// asked` on a row nobody asked anything about would go looking for a vendor
/// outage that never happened.
///
/// STRUCTURAL, exactly like its sibling above: a DIAGNOSTIC in a log column, not
/// a sentence anybody reads on a screen. Nothing on the wire carries it, and the
/// browser draws no critique block at all for such a row — see
/// `practiceCritique::critiqueFor`, whose `none` arm is this state's rendering.
// STRUCTURAL: a diagnostic marker in a log column, never wording on a screen.
pub const READ_NOT_REQUESTED: &str =
    "no read: the answer analysis switch was off, so no model was asked";

/// How `read_error` opens when the MODEL declined — composed by
/// `practice_read::accept`, and read back by [`is_failed_read`].
///
/// STRUCTURAL: a diagnostic prefix in a log column that this build writes and
/// this build parses; one constant so the writer and the reader cannot drift.
// STRUCTURAL: the writer/reader contract for model-decline rows in read_error.
pub const MODEL_ABSTAINED_PREFIX: &str = "the model abstained: ";

/// Whether a stored read is a SYSTEM failure — the answer analysis did not come
/// back — as opposed to a judgement, a model's decline, or no read at all.
///
/// ## Domain note: why this is derived and not a column (ADDENDUM_1)
///
/// Every abstain carries `read_abstain_reason`; the MODEL's own decline is the
/// one whose `read_error` opens with [`MODEL_ABSTAINED_PREFIX`]. Deriving the
/// flag from those two columns needs no migration and classifies every row
/// already on disk the same way: a pre-v2.2.1 failure ("I can't read this one."
/// with a system cause) is a failure too, and renders as one.
///
/// Used by the answer response, the one-page critique's neutral rail, and the
/// two chat context packages — one rule, one function.
pub fn is_failed_read(abstain_reason: Option<&str>, error: Option<&str>) -> bool {
    abstain_reason.is_some() && !error.is_some_and(|e| e.starts_with(MODEL_ABSTAINED_PREFIX))
}

impl ReadOutcome {
    /// Whether this outcome is a system failure — see [`is_failed_read`].
    pub fn failed(&self) -> bool {
        is_failed_read(self.abstain_reason.as_deref(), self.error.as_deref())
    }

    /// A read this build wrote itself, with no model call.
    ///
    /// ## Domain note: why this exists rather than calling the model anyway
    ///
    /// The "I don't recall." button sends a sentence THIS SYSTEM WROTE. Paying a
    /// model to judge our own words bought a sentence about a sentence, at full
    /// token cost, on a one-click control **[measured: ~2,090 input tokens per
    /// press]**. The verdict was never in doubt either: "I don't recall" is a
    /// complete answer when it is true, which is what the stored line says.
    pub fn stored(line: String) -> Self {
        let parts = ReadParts {
            call: line.clone(),
            why: String::new(),
            pointers: Vec::new(),
            keys: Vec::new(),
            ok: true,
        };
        ReadOutcome {
            text: Some(line),
            ok: Some(true),
            parts: Some(parts),
            version: Some(STORED_READ_VERSION.to_string()),
            ..Default::default()
        }
    }

    /// A read nobody asked for: the switch was off.
    ///
    /// ## ⚑ Every field but the marker is `None`, and that is the whole point
    ///
    /// No text, no verdict, no parts, no prompt version, no tokens, no model —
    /// because none of those happened. `ok: None` is the neutral rail, which the
    /// screen renders as nothing at all rather than as a judgement. The one thing
    /// the row records is WHY there is nothing: [`READ_NOT_REQUESTED`].
    ///
    /// Domain note: this is written when a NEW answer version is stored with the
    /// switch off. It is never attached over an existing row — see
    /// `post_practice_answer`, whose re-read arm leaves a standing read alone
    /// rather than erasing a critique Marie already has.
    pub fn not_requested() -> Self {
        ReadOutcome {
            error: Some(READ_NOT_REQUESTED.to_string()),
            ..Default::default()
        }
    }

    /// A read that never happened because an input could not be loaded.
    ///
    /// This is the blind-read defect's replacement. The old code logged an error,
    /// carried on with an empty vector, and returned a sentence indistinguishable
    /// from a good one.
    pub fn from_payload_failure(stored_line: &str, failure: &PayloadFailure) -> Self {
        ReadOutcome::abstained(
            stored_line,
            failure.plain_reason().to_string(),
            failure.to_string(),
            None,
            None,
        )
    }

    /// The row this outcome writes.
    ///
    /// ## Rust Learning: `serde_json::json!` on a `Vec<String>`
    ///
    /// The pointers and keys are stored as JSONB arrays, and `json!` turns a
    /// `Vec<String>` into one directly. They are `None` — a SQL NULL — rather than
    /// an empty array when there are no parts at all, because "this read had no
    /// pointers" and "this row has no read" are different facts and the column
    /// keeps them different.
    pub fn to_row(&self) -> AnswerRead {
        let parts = self.parts.as_ref();
        AnswerRead {
            read_text: self.text.clone(),
            read_ok: self.ok,
            read_error: self.error.clone(),
            read_abstain_reason: self.abstain_reason.clone(),
            read_call: parts.map(|p| p.call.clone()),
            read_why: parts.map(|p| p.why.clone()).filter(|why| !why.is_empty()),
            read_pointers: parts.map(|p| serde_json::json!(p.pointers)),
            read_keys: parts.map(|p| serde_json::json!(p.keys)),
            read_version: self.version.clone(),
            read_input_tokens: self.input_tokens,
            read_output_tokens: self.output_tokens,
            read_ms: self.ms,
            read_model: self.model.clone(),
            read_raw_reply: self.raw_reply.clone(),
            read_attempts: self.attempts,
            // NULL and not `[]` when nothing overran: the ordinary case should not
            // put a row in every operator's "show me the overruns" query.
            read_overruns: (!self.overruns.is_empty()).then(|| serde_json::json!(self.overruns)),
        }
    }

    /// A read that declined, with both halves of the reason.
    ///
    /// `plain` is Marie's sentence and `error` is the operator's. Requiring both
    /// at every construction site is what stops an abstain shipping with one of
    /// them empty — which would be a screen saying nothing, or a log saying
    /// nothing, depending which was forgotten.
    pub(crate) fn abstained(
        stored_line: &str,
        plain: String,
        error: String,
        model: Option<String>,
        ms: Option<i32>,
    ) -> Self {
        ReadOutcome {
            text: Some(compose_abstain_text(stored_line, None)),
            abstain_reason: Some(plain),
            error: Some(error),
            model,
            ms,
            ..Default::default()
        }
    }
}

#[cfg(test)]
#[path = "practice_read_outcome_tests.rs"]
mod tests;
