//! What the provider actually sent back, in one line.
//!
//! ## The incident this module is (2026-08-28)
//!
//! A post-appeal transcript, pass 1, Opus 5, `max_tokens = 64000`: the model
//! generated for 727 seconds and returned ONLY reasoning blocks. Zero text.
//!
//! The call did not fail in any way the pipeline could describe. It was not a
//! timeout, not a truncation, not a rate limit — it was a complete, well-formed
//! message with nothing in it. The error that reached the UI said the response
//! contained no text content, which was true and told nobody anything: it did
//! not say what the response DID contain, and the container logs did not either.
//! Diagnosing it meant reading Anthropic's thinking documentation rather than
//! reading our own output, which is the definition of a failure that is not
//! observable (Standing Rule 1).
//!
//! So every failure that can be reached with a response in hand now carries one
//! line saying what arrived: how many content blocks of each type, the output
//! tokens the provider billed, and the stop reason. The next incident of this
//! shape diagnoses itself from `pipeline_jobs.error`.
//!
//! ## Why the counts are collected on every call, not just failing ones
//!
//! A count gathered only when something goes wrong is a count nobody can
//! baseline. Knowing that a healthy pass-1 response carries `text ×1` is what
//! makes `thinking ×14, text ×0` legible as a departure rather than as a number
//! with no context.

use std::collections::BTreeMap;
use std::fmt;

/// How many content blocks of each type a response carried.
///
/// ## Rust Learning: a newtype over a `BTreeMap`, not a bare alias
///
/// `BTreeMap` rather than `HashMap` because this is rendered into an operator-
/// facing string: a `HashMap` would order the types differently on every run,
/// and two error messages describing the identical failure would not compare
/// equal by eye. The newtype exists so [`fmt::Display`] can live on it — the one
/// place the rendering is decided — instead of the format being re-invented at
/// each error site.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockCounts(BTreeMap<String, usize>);

impl BlockCounts {
    /// Record one block of the given wire type.
    pub fn record(&mut self, kind: &str) {
        *self.0.entry(kind.to_string()).or_insert(0) += 1;
    }

    /// How many blocks in total.
    pub fn total(&self) -> usize {
        self.0.values().sum()
    }

    /// How many blocks of one type, `0` if none.
    pub fn get(&self, kind: &str) -> usize {
        self.0.get(kind).copied().unwrap_or(0)
    }
}

impl fmt::Display for BlockCounts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("none");
        }
        let parts: Vec<String> = self
            .0
            .iter()
            .map(|(kind, n)| format!("{kind} ×{n}"))
            .collect();
        f.write_str(&parts.join(", "))
    }
}

/// One line describing what the provider actually sent back.
///
/// Composed in ONE place and used by both failure paths — the no-text error and
/// the truncation error — because two hand-written versions of this sentence
/// would describe the same response differently and an operator comparing two
/// incidents would not be able to tell.
///
/// Reads, for the 2026-08-28 incident:
///
/// ```text
/// response anatomy: 14 content blocks (thinking ×14); output_tokens=63997; stop_reason=end_turn
/// ```
///
/// `not reported` rather than a zero for either missing value, because "the
/// provider said nothing" and "the provider said none" are different states and
/// this line exists precisely to be trusted about that.
pub fn anatomy_line(
    blocks: &BlockCounts,
    output_tokens: Option<u64>,
    stop_reason: Option<&str>,
) -> String {
    let produced = match output_tokens {
        Some(n) => n.to_string(),
        None => "not reported".to_string(),
    };
    let stopped = stop_reason.unwrap_or("not reported");
    format!(
        "response anatomy: {total} content blocks ({blocks}); output_tokens={produced}; \
         stop_reason={stopped}",
        total = blocks.total(),
    )
}

#[cfg(test)]
#[path = "response_anatomy_tests.rs"]
mod tests;
