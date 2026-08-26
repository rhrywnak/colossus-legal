//! Truncation detection — the fix for census R-3.
//!
//! ## The defect this closes
//!
//! Measured 2026-08-25 on `doc-penzien-coa-brief-03-14-2011`: pass 1 and pass 2
//! each recorded `output_tokens = 32000`, which is exactly the profile's
//! `max_tokens`. The model was cut off mid-answer both times. Nothing noticed,
//! for two reasons that compound:
//!
//! 1. **`stop_reason` was read nowhere** in either repo — the one field that
//!    says "I was cut off" was discarded at the provider boundary.
//! 2. **`repair_json` is aggressively permissive** (its own doc comment says
//!    so). A response truncated mid-array gets its array closed and its partial
//!    trailing object dropped, and then parses cleanly. Run 171's stored output
//!    is 386 well-formed relationships and valid JSON — an extraction that was
//!    cut off and one that finished are indistinguishable after that point.
//!
//! So a truncated extraction reported success, and the only evidence was a token
//! count that happened to equal a number in a YAML file.
//!
//! ## The fix, and why it lives here
//!
//! Truncation is now **fatal, and detected before the text is parsed**. This
//! module is the decision, kept pure so both directions can be asserted without
//! an API call; `rig_llm_bridge` applies it at the point where the provider's
//! answer becomes a pipeline answer, which is upstream of every parser and
//! shared by pass 1 and pass 2 alike.
//!
//! **No retry.** Failing loudly is the whole fix (ruled 2026-08-25). The
//! operator raises `max_tokens` in the profile and reruns.

/// The Anthropic `stop_reason` meaning "I hit the `max_tokens` ceiling".
///
// STRUCTURAL: this is the Anthropic Messages API's wire vocabulary, not a
// setting. A deployment cannot choose a different spelling for it; if Anthropic
// ever renames the value, that is a code change following an API change, which
// is exactly what a compiled constant should track.
pub const STOP_REASON_MAX_TOKENS: &str = "max_tokens";

/// What a completed LLM call looked like, for the purposes of this check.
///
/// A borrowed view rather than the whole `LlmCallResult`, so the detector can be
/// tested without constructing a provider response — and so it cannot reach for
/// anything else by accident.
#[derive(Debug, Clone, Copy)]
pub struct CallShape<'a> {
    /// The provider's `stop_reason`, if it reported one.
    pub stop_reason: Option<&'a str>,
    /// Output tokens the provider reported, if it reported them.
    pub output_tokens: Option<u64>,
    /// The `max_tokens` this call was configured with.
    pub configured_max_tokens: u32,
    /// The model id, for the failure message.
    pub model: &'a str,
}

/// Was this response cut off at the token ceiling?
///
/// ## Rust Learning: `Option<&str>` and why `None` is not a failure
///
/// `stop_reason` is `Option` because a provider may not report one — a local
/// vLLM model, or a future adapter. `None` means "not reported", which is a
/// different state from "reported something other than max_tokens", and neither
/// is truncation. Only the affirmative value fails the call. Guessing truncation
/// from a token count alone would fail every extraction that legitimately used
/// its whole budget, which is a worse failure than the one being fixed.
pub fn is_truncated(shape: &CallShape<'_>) -> bool {
    shape.stop_reason == Some(STOP_REASON_MAX_TOKENS)
}

/// The operator-facing failure text for a truncated call.
///
/// ## Which kind of sentence this is
///
/// A **run-record error string**, not wording-store content. It follows the
/// convention every other step failure in this pipeline already uses — the text
/// reaches `pipeline_jobs.error` and the step record, never a rendered UI
/// surface — so it is written in code like its neighbours in
/// `rig_llm_bridge::map_engine_error` rather than declared and seeded.
///
/// It names all four things the operator needs to act: which model, that the
/// ceiling was the cause, what the ceiling was, and what was produced against
/// it. The remedy is stated because the fix is a profile edit, not a code change.
pub fn truncation_message(shape: &CallShape<'_>) -> String {
    let produced = match shape.output_tokens {
        Some(n) => n.to_string(),
        // Distinguishable from "0" — the provider not reporting usage and the
        // provider reporting none are different states (Standing Rule 1).
        None => "not reported".to_string(),
    };
    format!(
        "model {model} stopped at the max_tokens ceiling: the response was TRUNCATED, \
         not completed. Produced {produced} output tokens against a configured cap of \
         {cap}. The extraction is discarded rather than parsed, because a truncated \
         response repairs into plausible JSON and would otherwise be stored as a \
         complete result. Raise max_tokens for this document type in its profile YAML \
         and re-run.",
        model = shape.model,
        produced = produced,
        cap = shape.configured_max_tokens,
    )
}

#[cfg(test)]
#[path = "truncation_tests.rs"]
mod tests;
