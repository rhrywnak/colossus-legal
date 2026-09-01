//! The reranker gate's frozen fixture — its shape, and the audit over it.
//!
//! **Pure. No I/O, no database, no graph.** Everything here is a plain data type
//! or a function over one, so the whole shape can be exercised by a unit test in
//! milliseconds. The reading lives in `bin/gate_fixture.rs`; the *meaning* lives
//! here.
//!
//! ## Why this is a library module and not part of the binary
//!
//! Two reasons, and the first is mechanical: cargo auto-discovers **every**
//! `.rs` file directly under `src/bin/` as a separate binary, so a sibling
//! `gate_fixture_tests.rs` beside the bin would compile as a phantom binary with
//! no `main`. Test code for a bin has to live either inside the bin file or in
//! the library.
//!
//! The second reason is the point of the fixture. G0 *writes* these files; G1
//! (`rerank_gate`) *reads* them back and scores them. Both ends must agree on
//! the shape to the byte, and the honest way to hold two ends to one shape is
//! one set of types they both import — not two hand-kept copies that drift the
//! first time a field is added.
//!
//! ## Domain note: what a "gate fixture" is for
//!
//! The scan of a scenario has already been paid for once — 251 Opus calls for
//! S-11 on 2026-08-29, at roughly 2.8¢ each. The reranker gate asks whether a
//! free local cross-encoder, given the same query, would have floated the cards
//! Opus called relevant to the top of the same pool. That question can be asked
//! any number of times for nothing, but only against a *frozen* copy of what the
//! scan saw. This file is that copy's contract.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

// Every wire struct below carries `#[serde(deny_unknown_fields)]`.
//
// ## Why deny, when the usual argument is forward compatibility
//
// These four types are BOTH ends of one contract: G0 writes the fixture and G1
// reads it back, from the same repository at the same version. There is no third
// party sending us a newer shape to tolerate. So the only way an unknown field
// can appear is that the two ends have drifted — a fixture written by one build
// being scored by another — and that is precisely the case where G1 must stop
// rather than silently score a file it half-understands and report a recall
// number nobody can reproduce. A tolerated unknown field would make a drifted
// gate look like a passing one.

/// One candidate quote, as the gate will see it.
///
/// Every field but `id` is optional because every one of them can genuinely be
/// absent in the graph, and a fixture that turned "no page number" into `0` or
/// `""` would hand G1 a fact the record does not contain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateCard {
    /// The Neo4j Evidence node id — the join key for every other read.
    pub id: String,
    /// `C-207`, this scenario's stable handle for the card. `None` when the
    /// scenario has never gathered this node and so never minted an ordinal —
    /// which is the normal state for an `outside_pool` card.
    pub c_number: Option<String>,
    pub title: String,
    pub document: Option<String>,
    pub page: Option<i64>,
    /// `"p. 22"` — the page rendered for a human, or `None` when there is no
    /// page to render.
    pub pinpoint: Option<String>,
    pub quote: Option<String>,
    pub significance: Option<String>,
    /// The names on this Evidence node's ABOUT edges.
    ///
    /// A list, not one string: the subject filter L2 will build is a question
    /// about set membership ("is CFS among the subjects?"), and a joined string
    /// would have to be re-split to answer it.
    pub about: Vec<String>,
}

/// One allegation the scenario BEARS ON, as the query composer will use it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllegationRef {
    /// `A-16` — the complaint paragraph, in the handle humans say out loud.
    pub id: String,
    pub text: String,
}

/// The Stage-0 query: theme statement + linked allegation text + talking points,
/// with the subject kept as a filter beside it rather than folded into the text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatherQuery {
    /// `None` when the scenario has no theme statement written yet — distinct
    /// from `Some("")`, which would be a theme somebody blanked.
    pub theme: Option<String>,
    pub allegations: Vec<AllegationRef>,
    pub talking_points: Vec<String>,
    pub subject: String,
}

/// One scenario's frozen scan, ready for the gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateFixture {
    /// `S-11`.
    pub scenario: String,
    pub scenario_id: String,
    pub run_id: String,
    /// When the frozen run started, RFC 3339 — the operator's proof they got the
    /// run they asked for and not a neighbouring one.
    pub run_started_at: String,
    /// The date this file was extracted, so a stale fixture is visible on sight.
    pub extracted_at: String,
    pub query: GatherQuery,
    pub candidates: Vec<CandidateCard>,
    pub opus_relevant_ids: Vec<String>,
    pub included_ids: Vec<String>,
    /// The cards the scenario's pool cannot see today, which the fix must make
    /// visible — AT-1's C-54 for S-9, AT-2's seven $50,000 admissions for S-11.
    pub outside_pool: Vec<CandidateCard>,
}

/// The counts the operator asserts, supplied per run — never compiled in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedCounts {
    pub candidates: usize,
    pub opus_relevant: usize,
    pub included: usize,
    pub outside_pool: usize,
}

/// One structural check and how it came out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckOutcome {
    pub name: &'static str,
    pub detail: String,
    pub passed: bool,
}

/// Everything the audit found, ready to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureAudit {
    /// The one count line the report pastes verbatim.
    pub count_line: String,
    pub counts_match: bool,
    pub checks: Vec<CheckOutcome>,
    /// Cards Roman Included that Opus never called relevant.
    ///
    /// Reported, never failed. Per the task: "Roman including a card Opus missed
    /// is a finding about the judge prompt, not a bug in this bin."
    pub included_not_relevant: Vec<String>,
}

impl FixtureAudit {
    /// True when every structural check passed. Deliberately says nothing about
    /// the counts — a count mismatch is information about the scan history, and
    /// the caller decides what to do with it.
    pub fn structurally_sound(&self) -> bool {
        self.checks.iter().all(|c| c.passed)
    }
}

/// Render the count line, actual beside expected.
///
/// ## Why the expected number is printed even when it matches
///
/// The gate's whole value is that its inputs were not tuned to produce a
/// pleasing number. A line that prints only the actual count is unfalsifiable on
/// re-read; one that prints both is a claim a later reader can check.
fn count_line(scenario: &str, got: ExpectedCounts, want: ExpectedCounts) -> String {
    let pair = |g: usize, w: usize| {
        if g == w {
            format!("{g}")
        } else {
            format!("{g} (EXPECTED {w})")
        }
    };
    format!(
        "{scenario:<5}: candidates {} · opus_relevant {} · included {} · outside_pool {}",
        pair(got.candidates, want.candidates),
        pair(got.opus_relevant, want.opus_relevant),
        pair(got.included, want.included),
        pair(got.outside_pool, want.outside_pool),
    )
}

/// A check that passed, worded once.
fn ok(name: &'static str, detail: String) -> CheckOutcome {
    CheckOutcome {
        name,
        detail,
        passed: true,
    }
}

/// Every allegation carries text, and every candidate carries a quote.
///
/// ## Domain note: why a blank quote is a hard failure and a blank theme is not
///
/// The reranker scores `(query, quote)` pairs. A candidate with no quote is a
/// pair the cross-encoder cannot form, so it would silently drop out of the
/// ranking and quietly shrink the denominator of `recall@60`. A scenario with no
/// theme statement is a real, legitimate state (`query_basis: theme_only` in the
/// design) and is recorded rather than rejected.
fn content_checks(f: &GateFixture) -> Vec<CheckOutcome> {
    let blank_allegations: Vec<&str> = f
        .query
        .allegations
        .iter()
        .filter(|a| a.text.trim().is_empty())
        .map(|a| a.id.as_str())
        .collect();
    let blank_quotes: Vec<&str> = f
        .candidates
        .iter()
        .chain(f.outside_pool.iter())
        .filter(|c| c.quote.as_deref().unwrap_or("").trim().is_empty())
        .map(|c| c.id.as_str())
        .collect();

    vec![
        CheckOutcome {
            name: "every allegation text is non-empty",
            detail: match blank_allegations.len() {
                0 => format!("{} allegations, all with text", f.query.allegations.len()),
                n => format!("{n} blank: {}", blank_allegations.join(", ")),
            },
            passed: blank_allegations.is_empty(),
        },
        CheckOutcome {
            name: "every candidate has a non-empty quote",
            detail: match blank_quotes.len() {
                0 => format!(
                    "{} cards, all quoted",
                    f.candidates.len() + f.outside_pool.len()
                ),
                n => format!("{n} blank: {}", blank_quotes.join(", ")),
            },
            passed: blank_quotes.is_empty(),
        },
    ]
}

/// One "these ids should all be / should never be in that set" check, worded once.
///
/// ## Rust Learning: why `strays` is `Vec<&str>` and not `Vec<String>`
///
/// The offending ids are borrowed straight out of the fixture and only ever
/// read — joined into one message and dropped. Cloning each into an owned
/// `String` would allocate once per stray on a path whose whole job is to
/// describe a failure. The borrow is safe because `strays` cannot outlive `f`,
/// and the compiler enforces exactly that.
fn membership_check(name: &'static str, clean: String, strays: &[&str]) -> CheckOutcome {
    CheckOutcome {
        name,
        detail: match strays.len() {
            0 => clean,
            n => format!("{n}: {}", strays.join(", ")),
        },
        passed: strays.is_empty(),
    }
}

/// The three set-relation checks: relevant ⊆ candidates, candidates ∩ outside = ∅,
/// and the reported-not-failed included ⊆ relevant.
fn set_checks(f: &GateFixture, included_not_relevant: &[String]) -> Vec<CheckOutcome> {
    // BTreeSet, not HashSet: the membership answer is the same either way, but a
    // BTreeSet yields its stragglers in sorted order, so two runs of this tool
    // produce byte-identical reports and `diff` means something.
    let ids: BTreeSet<&str> = f.candidates.iter().map(|c| c.id.as_str()).collect();
    let strays: Vec<&str> = f
        .opus_relevant_ids
        .iter()
        .map(String::as_str)
        .filter(|id| !ids.contains(id))
        .collect();
    let overlap: Vec<&str> = f
        .outside_pool
        .iter()
        .map(|c| c.id.as_str())
        .filter(|id| ids.contains(id))
        .collect();

    vec![
        membership_check(
            "opus_relevant_ids ⊆ candidate ids",
            format!(
                "all {} relevant ids are in the pool",
                f.opus_relevant_ids.len()
            ),
            &strays,
        ),
        membership_check(
            "no id in both candidates and outside_pool",
            format!(
                "{} outside-pool cards, none in the pool",
                f.outside_pool.len()
            ),
            &overlap,
        ),
        ok(
            "included_ids ⊆ opus_relevant_ids (reported, never failed)",
            match included_not_relevant.len() {
                0 => format!(
                    "all {} Included cards were called relevant",
                    f.included_ids.len()
                ),
                n => format!("{n} Included that Opus did NOT call relevant — see FINDINGS"),
            },
        ),
    ]
}

/// Audit one fixture against the counts the operator expected.
///
/// Pure: it reads the fixture and returns what it found. It does not adjust
/// anything, and there is deliberately no code path that could — the counts
/// arrive from the command line and the cards arrive from the database, and this
/// function's only power is to describe the gap between them.
pub fn audit_fixture(f: &GateFixture, expected: ExpectedCounts) -> FixtureAudit {
    let relevant: BTreeSet<&str> = f.opus_relevant_ids.iter().map(String::as_str).collect();
    let included_not_relevant: Vec<String> = f
        .included_ids
        .iter()
        .filter(|id| !relevant.contains(id.as_str()))
        .cloned()
        .collect();

    let got = ExpectedCounts {
        candidates: f.candidates.len(),
        opus_relevant: f.opus_relevant_ids.len(),
        included: f.included_ids.len(),
        outside_pool: f.outside_pool.len(),
    };

    let mut checks = content_checks(f);
    checks.extend(set_checks(f, &included_not_relevant));

    FixtureAudit {
        count_line: count_line(&f.scenario, got, expected),
        counts_match: got == expected,
        checks,
        included_not_relevant,
    }
}

#[cfg(test)]
#[path = "gate_fixture_tests.rs"]
mod tests;
