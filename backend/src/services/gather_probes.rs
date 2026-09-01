//! Short probes pulled out of a composed query, for the trigram half.
//!
//! ## The rule, in one sentence
//!
//! **A probe is any whitespace-separated token — stripped of surrounding
//! punctuation — that either contains a digit or starts with a capital letter,
//! and is at least four characters long.**
//!
//! That is deliberately plain. A cleverer extractor nobody can predict would be
//! worse here than one whose output a human can check by eye, which is exactly
//! what the report does: if `$50,000` and `Milster` are not in S-11's list, the
//! extractor is wrong and no ranking number would say so.
//!
//! ## Why those two classes and no others
//!
//! They are what trigrams are good at and embeddings are bad at.
//!
//! - **Digits** carry currency amounts, dates in figures, docket and form
//!   numbers. The full-text analyser destroys them: `to_tsvector('english',
//!   '$50,000')` is `'50' '000'`, so it cannot tell `$50,000` from `50,000` in
//!   a scrap receipt. Trigrams can.
//! - **Capitals** carry proper names, which are typed from memory and
//!   half-remembered, and which whole-token matching cannot reach a substring
//!   of — `Milste` never reaches `Milster` through a tsvector.
//!
//! Ordinary lower-case prose is left to the full-text half, which stems and
//! stopwords it properly. Probing on it would return most of the corpus.

use std::collections::BTreeSet;

// STRUCTURAL: the shortest run of characters a trigram index can distinguish.
// pg_trgm decomposes a string into three-character runs, so a probe below this
// carries at most one trigram and matches almost anything — it is a property of
// the index's arithmetic, not a tuning dial. Four rather than three so a probe
// carries at least two overlapping trigrams and a capitalised sentence-opener
// like "The" cannot become one.
const MIN_PROBE_CHARS: usize = 4;

/// Extract the probes from a composed query, deduplicated and sorted.
///
/// Sorted rather than kept in query order so two runs produce the same probe
/// list and therefore the same trigram ranking — the same reason the party set
/// and the fused ties are ordered.
pub fn probes_of(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .filter_map(clean_token)
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect()
}

/// One token, stripped and tested against the rule.
///
/// ## Rust Learning: `trim_matches` with a closure
///
/// Punctuation clings to both ends of a word in prose — `("$50,000."` — and a
/// probe carrying a stray bracket matches nothing. `trim_matches` takes a
/// predicate and strips from BOTH ends until it fails, so one call handles any
/// combination. The dollar sign and the comma inside the number survive,
/// because they are not at an end; that matters, since they are precisely what
/// distinguishes `$50,000` from `50000`.
fn clean_token(token: &str) -> Option<String> {
    let trimmed = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '$');
    // `$` is kept as a leading character but is not itself content: a bare "$"
    // or "$." would otherwise pass the digit test on the token it came from.
    let core = trimmed.trim_start_matches('$');
    if core.chars().count() < MIN_PROBE_CHARS {
        return None;
    }
    let has_digit = core.chars().any(|c| c.is_ascii_digit());
    let is_capitalised = core.chars().next().is_some_and(char::is_uppercase);
    (has_digit || is_capitalised).then(|| trimmed.to_string())
}

#[cfg(test)]
#[path = "gather_probes_tests.rs"]
mod tests;
