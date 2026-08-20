//! What comes BACK from the read: parsing it, and judging its parts.
//!
//! Pure. No provider, no database, no clock — so every rule below is a unit test
//! instead of a call that costs money to observe. The payload half is
//! [`super::practice_read_payload`]; the impure half is
//! [`super::practice_read`].
//!
//! ## The rule that changed in T1, and why it is a correctness fix
//!
//! v2 returned ONE LINE, and this module took `raw.lines().find(non-empty)` and
//! threw the rest away — no error, no log, nothing kept. A model that replied
//!
//! ```text
//! Fine. Short, and yours.
//! But you left the false premise standing.
//! ```
//!
//! was stored as an unqualified `Fine.`, and the sentence that contradicted it
//! was unrecoverable. That is the opposite of the principle stated two paragraphs
//! below in the old file, which insisted an over-long reply is refused rather
//! than truncated *"because half a sentence about testimony can invert its
//! meaning"*. It was true of the WORD cap and false of the LINE rule.
//!
//! v3 returns three parts and this module parses three parts. Nothing is dropped
//! for being second.
//!
//! ## The second rule that changed: a ceiling no longer discards
//!
//! A 26-word read used to be REFUSED, and Marie saw "no system read this time"
//! for a reply that was one word long. The trade is now inverted, on Roman's
//! ruling: an overrun re-requests ONCE, and a second overrun is stored and shown
//! **as returned**, with the part and the count logged. A formatting slip is not
//! the witness's fault and must not cost her the coaching.
//!
//! ## Why a reply that will not parse also re-requests (Roman, 2026-08-20)
//!
//! §2.5 as written sent an unparseable reply straight to the abstain arm. That
//! was written when the reply was prose, where "unparseable" meant the model had
//! said something strange. With a JSON reply the likeliest parse failure is a
//! formatting slip — a stray fence, a trailing comma — and abstaining for one
//! shows Marie *"I can't read this one"* when there was nothing wrong with her
//! answer. Same rule as the overrun: once more, then abstain.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// The JSON field names v3 teaches the model to produce.
///
/// ## Why these are CODE constants and not settings rows
///
/// The same reasoning Roman gave for keeping the seven tactic counters in the
/// prompt file (2026-08-20): the prompt is version-controlled, diffable and
/// md5-verified on push, while a settings row is editable in a browser with no
/// review — which is exactly how the prompt came to say "George" while the
/// payload said "the defense". A structural field name belongs on the reviewed
/// side of that line. The NUMBERS stay rows, because the numbers are what an
/// operator legitimately tunes.
///
/// The coupling is not left to prose: `the_reply_field_names_are_the_ones_the_prompt_file_teaches`
/// reads v3 off disk and fails the build if any of these is missing from it, and
/// `the_reply_struct_and_the_field_constants_cannot_drift` proves this list is
/// the struct's own serialized shape rather than a second copy of it.
pub const FIELD_CALL: &str = "call";
pub const FIELD_WHY: &str = "why";
pub const FIELD_POINTERS: &str = "pointers";
pub const FIELD_KEYS: &str = "keys";
pub const FIELD_ABSTAIN: &str = "abstain";

/// Every field name the reply may carry.
pub const REPLY_FIELDS: &[&str] = &[
    FIELD_ABSTAIN,
    FIELD_CALL,
    FIELD_KEYS,
    FIELD_POINTERS,
    FIELD_WHY,
];

/// The reply exactly as the model sends it.
///
/// ## Rust Learning: `#[serde(default)]` on every field, and what it decides
///
/// `default` means a missing field deserializes to its type's default rather
/// than failing. That is the right reading here and it is a DOMAIN decision, not
/// a convenience: the design says a part may be omitted when there is nothing to
/// say, so an absent `why` is a legitimate reply and must not be a parse failure.
/// A malformed BODY still fails, which is what the re-request arm is for.
///
/// Unknown fields are tolerated (no `deny_unknown_fields`) because a model that
/// adds a stray `"note"` has still answered the question, and abstaining over it
/// would spend a witness's coaching on our own strictness.
// serde: allows unknown fields because a model that adds a stray `"note"` has
// still answered the question, and refusing the reply over it would spend a
// witness's coaching on our own strictness. A malformed BODY still fails, which
// is what the re-request arm is for.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RawReply {
    #[serde(default)]
    pub call: String,
    #[serde(default)]
    pub why: String,
    #[serde(default)]
    pub pointers: Vec<String>,
    #[serde(default)]
    pub keys: Vec<String>,
    /// Present and non-empty when the model declined to judge.
    #[serde(default)]
    pub abstain: Option<String>,
}

/// A read that was accepted, in the three parts it is stored as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadParts {
    /// The one line naming what happened.
    pub call: String,
    /// The reasoning. Empty is legitimate.
    pub why: String,
    /// The pointers, in the order the model gave them. 0–3.
    pub pointers: Vec<String>,
    /// The citation keys used, every one proven to be a key that was sent.
    pub keys: Vec<String>,
    /// True when the call opens with the OK word. Drives the rail colour and the
    /// `read_ok` column.
    pub ok: bool,
}

/// What one reply turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadReply {
    /// A judgement, in three parts.
    Parts(ReadParts),
    /// The model declined, with its own plain-English reason.
    Abstain(String),
}

/// A part that came back over its ceiling.
///
/// NOT an error. It is carried alongside an accepted reply so the caller can
/// re-request once and, failing that, log what it stored — which is the whole of
/// the inverted trade: the overrun is recorded, never the reason Marie sees
/// nothing.
///
/// `Serialize` because it is STORED, not only logged: the four ceilings are
/// settings rows an operator will move, so recording that a part was long without
/// recording the limit it was long against would leave a later reader unable to
/// say whether a stored `read_call` was ever actually over the line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Overrun {
    /// `call`, `why`, `pointers`, or `pointer 2`.
    pub part: String,
    pub words: usize,
    pub limit: usize,
}

/// Why a reply could not be read at all.
///
/// Each of these re-requests once and then abstains. They become `read_error` —
/// the operator's half of the honesty, while Marie reads the abstain line.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReplyRejection {
    #[error("the model replied with nothing")]
    Empty,

    #[error("the reply was not the JSON v3 asks for: {detail}")]
    Unparseable { detail: String },

    #[error("the reply has no {FIELD_CALL} and no {FIELD_ABSTAIN} — it judged nothing")]
    NothingSaid,

    #[error("the reply cited {key}, which was not among the keys it was sent ({sent})")]
    UnknownKey { key: String, sent: String },
}

/// The ceilings a reply is judged against, read from the store by the caller.
///
/// ## Rust Learning: borrowed fields and a lifetime
///
/// `fine_token: &'a str` borrows the settings snapshot rather than cloning a
/// `String` on every answer. The `'a` tells the compiler the struct may not
/// outlive the snapshot it points into — exactly the guarantee wanted here, since
/// a stale token would judge against wording the store no longer holds.
#[derive(Debug, Clone, Copy)]
pub struct ReadRules<'a> {
    pub max_words_call: usize,
    pub max_words_why: usize,
    pub max_words_pointer: usize,
    pub max_pointers: usize,
    /// The cap on a call that OPENS with the OK word. "Fine." plus a speech is
    /// still a speech.
    pub max_words_after_fine: usize,
    pub fine_token: &'a str,
}

/// Strip a markdown fence a model wrapped its JSON in.
///
/// Models fence JSON whether or not they were asked to, and a fence is a
/// formatting habit rather than a different answer. Stripping it here is the
/// same judgement the old parser made about quotation marks — and it is the
/// difference between a re-request that costs money and a reply that was always
/// fine.
fn unfence(raw: &str) -> &str {
    let trimmed = raw.trim();
    let Some(rest) = trimmed.strip_prefix("```") else {
        return trimmed;
    };
    // ```json / ```JSON / ``` — the language tag runs to the first newline.
    let body = match rest.find('\n') {
        Some(at) => &rest[at + 1..],
        None => rest,
    };
    body.trim_end().strip_suffix("```").unwrap_or(body).trim()
}

/// Count words the way every ceiling in this module counts them.
fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Take a model reply and decide what it is.
///
/// Returns the reply together with every ceiling it exceeded. **Overruns are not
/// errors** — see [`Overrun`].
///
/// # Errors
/// [`ReplyRejection`] when the reply is empty, is not the JSON v3 asks for, says
/// nothing at all, or cites a key it was not sent. Every one of those re-requests
/// once before it becomes an abstain.
pub fn parse_reply(
    raw: &str,
    rules: ReadRules<'_>,
    citable: &BTreeSet<String>,
) -> Result<(ReadReply, Vec<Overrun>), ReplyRejection> {
    let body = unfence(raw);
    if body.is_empty() {
        return Err(ReplyRejection::Empty);
    }

    let reply: RawReply = serde_json::from_str(body).map_err(|e| ReplyRejection::Unparseable {
        detail: e.to_string(),
    })?;

    // The abstain arm wins when the model used it, whatever else it wrote: a
    // model that declines and then fills the shape anyway has still declined, and
    // showing the shape would be this build overriding its own model's refusal.
    if let Some(reason) = reply.abstain.as_deref().map(str::trim) {
        if !reason.is_empty() {
            return Ok((ReadReply::Abstain(reason.to_string()), Vec::new()));
        }
    }

    let call = reply.call.trim();
    if call.is_empty() {
        return Err(ReplyRejection::NothingSaid);
    }

    let keys = validate_keys(&reply.keys, citable)?;
    let pointers: Vec<String> = reply
        .pointers
        .iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();

    let ok = call.starts_with(rules.fine_token);
    let overruns = measure(call, reply.why.trim(), &pointers, rules, ok);

    Ok((
        ReadReply::Parts(ReadParts {
            call: call.to_string(),
            why: reply.why.trim().to_string(),
            pointers,
            keys,
            ok,
        }),
        overruns,
    ))
}

/// Every key the model returned, proven to be a key it was sent.
///
/// ## Domain note: this is the whole point of T1's grounding half
///
/// "A read that cannot cite cannot claim" is only worth saying if something
/// checks the citation. A model that names `R4` on a scenario with three
/// receipts, or cites `S1` on a question with no sworn pair, has invented a
/// document — the exact failure the anchor demand produced before this task —
/// and it must not reach Marie looking like a fact.
fn validate_keys(
    returned: &[String],
    citable: &BTreeSet<String>,
) -> Result<Vec<String>, ReplyRejection> {
    let mut keys = Vec::with_capacity(returned.len());
    for key in returned {
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        if !citable.contains(key) {
            return Err(ReplyRejection::UnknownKey {
                key: key.to_string(),
                sent: citable.iter().cloned().collect::<Vec<_>>().join(" "),
            });
        }
        keys.push(key.to_string());
    }
    Ok(keys)
}

/// Which parts came back over their ceilings.
fn measure(
    call: &str,
    why: &str,
    pointers: &[String],
    rules: ReadRules<'_>,
    ok: bool,
) -> Vec<Overrun> {
    let mut overruns = Vec::new();

    // A call that opens with the OK word is capped by the OK word's own,
    // shorter limit — the v2 rule, kept: "Fine." plus a speech is still a speech.
    let (call_words, call_limit) = if ok {
        let tail = call.strip_prefix(rules.fine_token).unwrap_or(call);
        (words(tail), rules.max_words_after_fine)
    } else {
        (words(call), rules.max_words_call)
    };
    if call_words > call_limit {
        overruns.push(Overrun {
            part: FIELD_CALL.to_string(),
            words: call_words,
            limit: call_limit,
        });
    }

    let why_words = words(why);
    if why_words > rules.max_words_why {
        overruns.push(Overrun {
            part: FIELD_WHY.to_string(),
            words: why_words,
            limit: rules.max_words_why,
        });
    }

    if pointers.len() > rules.max_pointers {
        overruns.push(Overrun {
            part: FIELD_POINTERS.to_string(),
            words: pointers.len(),
            limit: rules.max_pointers,
        });
    }

    for (index, pointer) in pointers.iter().enumerate() {
        let count = words(pointer);
        if count > rules.max_words_pointer {
            overruns.push(Overrun {
                part: format!("pointer {}", index + 1),
                words: count,
                limit: rules.max_words_pointer,
            });
        }
    }

    overruns
}

/// The single line the untouched frontend still renders.
///
/// ## Why this exists at all, and what reads it
///
/// T1 stores the read in parts; T4 renders those parts. Between the two, Marie
/// keeps using the reveal every day, and the reveal prints ONE string from the
/// `read_text` COLUMN — as does the question-review page, which is the half the
/// task's §2.6 missed and Roman corrected (A7). So the parts are composed back
/// into one line and written to that column, and neither screen changes.
///
/// The composition is the CALL, plus the first pointer when there is one: the
/// call says what happened and the first pointer says what to do, which is the
/// pair that was worth reading in the one-sentence era. The `why` is deliberately
/// left out — it is the part that would turn one line into a paragraph on a
/// screen built for a sentence.
pub fn compose_read_text(parts: &ReadParts) -> String {
    let Some(first) = parts.pointers.first() else {
        return parts.call.clone();
    };
    // The model may or may not end the call with a stop. Supplying one when it is
    // missing is the difference between "You let the braid stand Take the second"
    // and a sentence.
    let separator = if parts.call.ends_with(['.', '!', '?', '—', ':']) {
        " "
    } else {
        ". "
    };
    format!("{}{separator}{first}", parts.call)
}

/// What Marie reads when the read declines.
///
/// The stored line always; the model's own reason after it when the model was
/// the one to decline. A code-detected abstain has no sentence of its own here —
/// its cause is a diagnostic and goes to `read_abstain_reason`, for the reason
/// the skip marker gives: a value this build composed FROM A FAILURE IT OBSERVED
/// must not be something an operator can edit after the fact.
pub fn compose_abstain_text(stored_line: &str, model_reason: Option<&str>) -> String {
    match model_reason.map(str::trim).filter(|r| !r.is_empty()) {
        Some(reason) => format!("{stored_line} {reason}"),
        None => stored_line.to_string(),
    }
}

#[cfg(test)]
#[path = "practice_read_parse_tests.rs"]
mod tests;
