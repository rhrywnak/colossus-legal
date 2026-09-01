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

// Split from `tests` above: extraction and selection are two subjects, and the
// one file was over Rule 17's 300 lines. `#[path]` keeps both out of this
// module's own count.
#[cfg(test)]
#[path = "gather_probes_selection_tests.rs"]
mod selection_tests;

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

// ---------------------------------------------------------------------------
// One term, one vote
// ---------------------------------------------------------------------------

/// Probes that turned out to be the same probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeGroup {
    /// The spelling kept — the shortest, ties broken lexicographically.
    pub representative: String,
    /// The other spellings, which contributed nothing further.
    pub collapsed: Vec<String>,
}

/// Collapse probes whose result sets are identical into one.
///
/// ## ⚑ Why this is a defect and not a parameter
///
/// The fusion's central claim is that a card SEVERAL INDEPENDENT probes found is
/// a better bet than a card one probe found. Measured on S-11, three probes were
/// not independent at all: `$50,000`, `$50,000.00` and `$500,000.00` returned
/// the identical 65 ids, so every card they matched was scored three times for
/// one match. `Plaintiff`, `Plaintiff's` and `Plaintiffs` did the same at 40;
/// `Hanley` and `Hanley's` at 44. The agreement the ranking rewards was being
/// manufactured out of spelling variants.
///
/// ## Exact set equality, deliberately
///
/// Two probes collapse only when their result sets are EQUAL — not similar, not
/// overlapping. Exactness is what makes this safe: it cannot merge two probes
/// that genuinely differ, because if either reaches one card the other does not,
/// they stay apart and both keep their vote.
///
/// A similarity rule (say, Jaccard above some threshold) would be strictly more
/// dangerous and buy little: `Hanley` (44) and `Higgs` (38) are different people
/// who appear in overlapping documents, and a loose rule that merged them would
/// silently delete one party's evidence from the ranking's reckoning. There is
/// no threshold at which that risk is worth a few duplicate lists.
///
/// ## Sets, not sequences
///
/// The comparison is over the SET of ids — sorted AND deduplicated — not the
/// ordered list. Two probes with
/// the same matches can order them differently, because each row's
/// `word_similarity` is measured against its own probe — `$50,000.00` scores
/// the same cards slightly differently from `$50,000`. Comparing sequences would
/// therefore miss exactly the duplicates this exists to catch.
///
/// The representative keeps ITS OWN ranked list, so the order that survives is a
/// real one and not a merge of several.
pub fn collapse_identical(
    lists: Vec<(String, Vec<String>)>,
) -> (Vec<(String, Vec<String>)>, Vec<ProbeGroup>) {
    // Keyed by the sorted id set. BTreeMap so the output order is stable across
    // runs, which the fused ranking downstream depends on.
    let mut by_result: std::collections::BTreeMap<Vec<String>, Vec<(String, Vec<String>)>> =
        std::collections::BTreeMap::new();
    for (probe, hits) in lists {
        // `sort` THEN `dedup`: the key must be a SET, and a bare sort would make
        // it a sorted multiset. Two probes reaching the same cards but with a
        // different number of duplicate rows would then key differently and
        // fail to collapse — the exact thing this exists to catch, missed
        // silently.
        //
        // Duplicates should not occur: `evidence_id` is the mirror's primary
        // key and the read projects it from a single table, so one card is one
        // row. `dedup` is here because the invariant is the READ's and this
        // function should not inherit it — a future read that joins would break
        // the collapse rather than the join, and nothing would say so.
        let mut key = hits.clone();
        key.sort();
        key.dedup();
        by_result.entry(key).or_default().push((probe, hits));
    }

    let mut kept = Vec::with_capacity(by_result.len());
    let mut groups = Vec::new();
    for (_, mut members) in by_result {
        // Shortest spelling wins; ties break lexicographically so two runs agree.
        members.sort_by(|a, b| {
            a.0.chars()
                .count()
                .cmp(&b.0.chars().count())
                .then(a.0.cmp(&b.0))
        });
        let (representative, hits) = members.remove(0);
        if !members.is_empty() {
            groups.push(ProbeGroup {
                representative: representative.clone(),
                collapsed: members.into_iter().map(|(probe, _)| probe).collect(),
            });
        }
        kept.push((representative, hits));
    }
    (kept, groups)
}
