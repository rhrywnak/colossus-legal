//! When pressing Answer makes a new VERSION, and when it re-reads the old one.
//!
//! One decision, pure, in a module of its own so it can be tested without a
//! database — the handler that uses it needs a pool for everything else it does.
//!
//! ## Roman's ruling of 2026-08-23
//!
//! > A version is a change she made, not a button she pressed twice.
//!
//! She presses Answer, the read starts, she presses **Stop waiting**, then
//! presses Answer again without touching the text. Or she reads a critique,
//! thinks about it, and presses Answer again out of habit. Before this rule both
//! wrote a second row identical to the first, and her "▸ 2 earlier versions"
//! line began counting things that were not versions of anything.

/// Is this press a RE-READ of the answer that already stands?
///
/// `true` when what she typed is byte-identical to the current answer — reuse
/// that row and ask for the read again. `false` when it differs, or when there
/// is nothing standing yet, and a new version is written.
///
/// ## Domain note: byte-identical, deliberately NOT trimmed-equal
///
/// A trailing space she added and meant is a change, and this code cannot tell
/// which spaces she meant. Comparing trimmed would silently discard an edit
/// whose whole content is whitespace — rare, but the failure is that her change
/// vanishes with the screen reporting success, which is the worst shape a
/// failure can have on this surface.
///
/// ## Rust Learning: `Option::is_some_and`
///
/// Takes the `Option` by value and folds "is there one?" and "does it satisfy
/// this?" into a single expression, so the absent case cannot be forgotten.
/// `map(..).unwrap_or(false)` says the same thing in two steps; this one cannot
/// be half-written.
pub fn is_reread(standing: Option<&str>, typed: &str) -> bool {
    standing.is_some_and(|current| current == typed)
}

#[cfg(test)]
#[path = "practice_answer_version_tests.rs"]
mod tests;
