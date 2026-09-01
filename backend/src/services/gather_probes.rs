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

// ---------------------------------------------------------------------------
// Selectivity — which probes are worth reading
// ---------------------------------------------------------------------------

/// One probe and how much of the admitted set it matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeCount {
    pub probe: String,
    /// Rows in the admitted set this probe matches — counted, not read.
    pub matches: i64,
}

/// What the selectivity rule decided, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeSelection {
    /// Probes worth reading, in the order given.
    pub kept: Vec<String>,
    /// Probes dropped for matching too much, each with its count.
    pub dropped: Vec<ProbeCount>,
    /// True when every probe was over the share and the floor kept the most
    /// selective anyway — a state worth reporting, since it means the query's
    /// vocabulary is entirely generic.
    pub floor_applied: bool,
}

/// Drop the probes that match more than `max_share` of the admitted set.
///
/// ## ⚑ Why a probe matching everything is worse than no probe
///
/// The trigram half fuses one ranked list per probe, so a card several probes
/// agree on outranks one a single probe found. A probe matching most of the
/// pool agrees with EVERYTHING, so it contributes a near-uniform list that
/// crowds out the figures and names the scenario turns on. Measured on S-11:
/// `Court` matched 534 of 1030 admitted cards; `$50,000` matched 73.
///
/// ## The two guards
///
/// - **A probe matching NOTHING is kept.** It costs nothing to read — an empty
///   list is a no-op in the fusion — and it is information: a term the corpus
///   does not contain. The two ends of the range are not the same thing and
///   must not be collapsed.
/// - **Never zero probes.** If every probe is over the share, the most
///   selective `floor` of them survive anyway. A silently empty trigram half
///   would look exactly like a working one.
///
/// `floor` comes from the `gather_probe_floor` settings row. It is clamped to
/// at least 1 here as well as bounded in the row: "never zero" is the invariant
/// the whole guard exists for, and it must not be defeasible by a stored value
/// however the row is bounded today.
pub fn select_probes(
    counts: &[ProbeCount],
    admitted: usize,
    max_share: f64,
    floor: usize,
) -> ProbeSelection {
    let ceiling = admitted as f64 * max_share;

    let (kept, dropped): (Vec<&ProbeCount>, Vec<&ProbeCount>) =
        counts.iter().partition(|c| (c.matches as f64) <= ceiling);

    if !kept.is_empty() {
        return ProbeSelection {
            kept: kept.iter().map(|c| c.probe.clone()).collect(),
            dropped: dropped.into_iter().cloned().collect(),
            floor_applied: false,
        };
    }

    // Everything was over the share. Keep the most selective few rather than
    // running none. Ties break on the probe text so two runs agree.
    let mut by_selectivity: Vec<&ProbeCount> = counts.iter().collect();
    by_selectivity.sort_by(|a, b| a.matches.cmp(&b.matches).then(a.probe.cmp(&b.probe)));
    let floor = floor.max(1).min(by_selectivity.len());

    ProbeSelection {
        kept: by_selectivity[..floor]
            .iter()
            .map(|c| c.probe.clone())
            .collect(),
        dropped: by_selectivity[floor..]
            .iter()
            .map(|c| (*c).clone())
            .collect(),
        floor_applied: !by_selectivity.is_empty(),
    }
}
