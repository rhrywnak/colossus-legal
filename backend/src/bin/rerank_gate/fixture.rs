//! The gate fixture on disk, and the STOP-3 count assertion.
//!
//! A fixture is one scenario's paid-for scan, frozen: the query block it was
//! composed from, the 292 candidate cards the gather actually produced, the ids
//! Opus called relevant, the ids Roman Included, and the `outside_pool` cards
//! that the gather could NOT see today but that a widened reach would surface.
//!
//! Nothing here scores anything. Loading is separated from scoring so that a
//! malformed fixture fails before a single pair is sent to the model.

use anyhow::{bail, Context};
use serde::Deserialize;

/// One evidence card, exactly as the fixture stores it.
///
/// ## Rust Learning: `#[serde(deny_unknown_fields)]`
///
/// By default serde IGNORES a JSON key it has no field for. That is the wrong
/// default here: if the fixture generator gains a field — say a second quote
/// surface — a silently-ignored key would mean this gate scored something other
/// than what the generator intended, and the run would look clean. `deny_unknown_fields`
/// turns that into a load error naming the key (standing Rule 1: a missing
/// surface and an ignored surface must not look the same).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    /// The graph node id — the join key against `opus_relevant_ids` /
    /// `included_ids`.
    pub id: String,
    /// `C-1` … the handle a human says out loud.
    ///
    /// `None` for every `outside_pool` card: those were never in the numbered
    /// pool, so they were never assigned a C-number. That is a real distinction,
    /// not a gap to paper over, so it stays an `Option` all the way to the CSV.
    pub c_number: Option<String>,
    pub title: String,
    pub document: String,
    // Why `allow(dead_code)`: `deny_unknown_fields` above means every key the
    // fixture carries MUST be declared here or loading fails. `page` and `about`
    // are carried by the file and are not scored — the CSV cites `pinpoint`, and
    // `about` is the party half that L2a's widening owns, not this gate. Deleting
    // them to silence the warning would make the loader reject a valid fixture.
    #[allow(dead_code)]
    pub page: Option<i64>,
    pub pinpoint: Option<String>,
    /// The verbatim words from the record.
    ///
    /// Domain note: 22 of S-11's 292 quotes are bare "Admitted." or "Denied as
    /// untrue." — a response to a request for admission carries its substance in
    /// the REQUEST, which the extractor put in `title` and `significance`. This
    /// is the whole reason surface S1 (quote alone) is expected to be blind and
    /// the verdict is read on S2.
    pub quote: String,
    pub significance: String,
    /// Party names the card is filed ABOUT. Carried for completeness; the gate
    /// does not score it.
    #[allow(dead_code)]
    pub about: Vec<String>,
}

/// One allegation as the fixture stores it.
///
/// Note this is a NARROWER shape than L2a's `AllegationForQuery`, which also
/// carries `label` and `parties`. The gate composes query TEXT only — the party
/// widening is L2a's other half and is not on trial here — so the extra fields
/// are absent from the fixture and absent from this struct.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Allegation {
    pub id: String,
    pub text: String,
}

/// The scenario's query block — the composer's raw material.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryBlock {
    pub theme: String,
    pub allegations: Vec<Allegation>,
    pub talking_points: Vec<String>,
    /// The subject filter. Kept beside the text, never folded into it
    /// (design §4 Stage 0); the gate does not score it.
    pub subject: String,
}

/// One fixture file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    /// `S-11`, `S-9` — also the CSV filename stem.
    pub scenario: String,
    pub scenario_id: String,
    pub run_id: String,
    pub run_started_at: String,
    pub extracted_at: String,
    pub query: QueryBlock,
    pub candidates: Vec<Card>,
    pub opus_relevant_ids: Vec<String>,
    pub included_ids: Vec<String>,
    pub outside_pool: Vec<Card>,
}

/// The four counts a caller asserts on load — STOP condition 3.
///
/// ## Why this is a parameter and not a constant
///
/// 292/44/10/7 is true of one file on one day. Baking it in would make the bin
/// refuse the next scenario's fixture and would be exactly the hardcoded
/// case-specific value standing Rule 2 forbids. The caller passes what it
/// expects on the command line, so the assertion is real and the bin stays
/// case-agnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedCounts {
    pub candidates: usize,
    pub relevant: usize,
    pub included: usize,
    pub outside_pool: usize,
}

impl std::str::FromStr for ExpectedCounts {
    type Err = anyhow::Error;

    /// Parses `candidates,relevant,included,outside_pool` — e.g. `292,44,10,7`.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(',').map(str::trim).collect();
        if parts.len() != 4 {
            bail!("expected four comma-separated counts (candidates,relevant,included,outside_pool), got {s:?}");
        }
        let n = |i: usize| -> anyhow::Result<usize> {
            parts[i]
                .parse::<usize>()
                .with_context(|| format!("count {} in {s:?} is not a number", i + 1))
        };
        Ok(ExpectedCounts {
            candidates: n(0)?,
            relevant: n(1)?,
            included: n(2)?,
            outside_pool: n(3)?,
        })
    }
}

impl Fixture {
    /// STOP 3: assert the four counts AND the two subset invariants.
    ///
    /// `relevant ⊆ candidates` and `included ⊆ relevant` are what make the gate
    /// arithmetic meaningful: a relevant id that is not in the pool could never
    /// be recalled at any k, and would quietly depress the numerator forever.
    /// Checking it here means the gate can never report a recall miss that was
    /// really a fixture defect.
    pub fn assert_counts(&self, expected: ExpectedCounts) -> anyhow::Result<()> {
        let actual = ExpectedCounts {
            candidates: self.candidates.len(),
            relevant: self.opus_relevant_ids.len(),
            included: self.included_ids.len(),
            outside_pool: self.outside_pool.len(),
        };
        if actual != expected {
            bail!(
                "STOP 3 — {} fixture counts are {}/{}/{}/{} (candidates/relevant/included/outside_pool), expected {}/{}/{}/{}",
                self.scenario,
                actual.candidates,
                actual.relevant,
                actual.included,
                actual.outside_pool,
                expected.candidates,
                expected.relevant,
                expected.included,
                expected.outside_pool
            );
        }

        let pool: std::collections::HashSet<&str> =
            self.candidates.iter().map(|c| c.id.as_str()).collect();
        let relevant: std::collections::HashSet<&str> =
            self.opus_relevant_ids.iter().map(String::as_str).collect();

        for id in &self.opus_relevant_ids {
            if !pool.contains(id.as_str()) {
                bail!(
                    "STOP 3 — {}: opus_relevant id {id:?} is not among the candidates; \
                     `relevant ⊆ candidates` is violated and no k could ever recall it",
                    self.scenario
                );
            }
        }
        for id in &self.included_ids {
            if !relevant.contains(id.as_str()) {
                bail!(
                    "STOP 3 — {}: included id {id:?} is not in opus_relevant_ids; \
                     `included ⊆ relevant` is violated",
                    self.scenario
                );
            }
        }
        Ok(())
    }

    /// Positions (indices into `candidates`) of the cards carrying the given ids.
    ///
    /// Returns POSITIONS rather than ids because every downstream metric works
    /// on the score/rank vectors, which are indexed by fixture order.
    pub fn positions_of(&self, ids: &[String]) -> Vec<usize> {
        let index: std::collections::HashMap<&str, usize> = self
            .candidates
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.as_str(), i))
            .collect();
        // `assert_counts` has already proved every id is present, so a miss here
        // is impossible; `filter_map` rather than an index-and-panic keeps the
        // production path free of `unwrap` regardless (standing Rule 1).
        ids.iter()
            .filter_map(|id| index.get(id.as_str()).copied())
            .collect()
    }
}

/// Read and parse one fixture file.
pub fn load(path: &std::path::Path) -> anyhow::Result<Fixture> {
    let bytes =
        std::fs::read(path).with_context(|| format!("reading gate fixture {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing gate fixture {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str) -> Card {
        Card {
            id: id.to_string(),
            c_number: Some("C-1".to_string()),
            title: "t".to_string(),
            document: "doc-x".to_string(),
            page: Some(1),
            pinpoint: None,
            quote: "Admitted.".to_string(),
            significance: "s".to_string(),
            about: Vec::new(),
        }
    }

    fn fixture(relevant: &[&str], included: &[&str], pool: &[&str]) -> Fixture {
        Fixture {
            scenario: "S-11".to_string(),
            scenario_id: "sid".to_string(),
            run_id: "rid".to_string(),
            run_started_at: "t0".to_string(),
            extracted_at: "t1".to_string(),
            query: QueryBlock {
                theme: "theme".to_string(),
                allegations: Vec::new(),
                talking_points: Vec::new(),
                subject: "subject".to_string(),
            },
            candidates: pool.iter().map(|id| card(id)).collect(),
            opus_relevant_ids: relevant.iter().map(|s| (*s).to_string()).collect(),
            included_ids: included.iter().map(|s| (*s).to_string()).collect(),
            outside_pool: Vec::new(),
        }
    }

    fn counts(candidates: usize, relevant: usize, included: usize) -> ExpectedCounts {
        ExpectedCounts {
            candidates,
            relevant,
            included,
            outside_pool: 0,
        }
    }

    #[test]
    fn expected_counts_parse_from_the_flag_form() {
        let parsed: ExpectedCounts = "292,44,10,7".parse().expect("a well-formed count string");
        assert_eq!(
            parsed,
            ExpectedCounts {
                candidates: 292,
                relevant: 44,
                included: 10,
                outside_pool: 7
            }
        );
        // Whitespace around the commas is tolerated — an operator types this.
        assert_eq!("292, 44, 10, 7".parse::<ExpectedCounts>().unwrap(), parsed);
    }

    #[test]
    fn the_wrong_number_of_counts_is_rejected() {
        for bad in ["292,44,10", "292,44,10,7,1", "", "292"] {
            let error = bad
                .parse::<ExpectedCounts>()
                .expect_err("must reject {bad:?}");
            assert!(
                error.to_string().contains("four comma-separated counts"),
                "got: {error}"
            );
        }
    }

    #[test]
    fn a_count_that_is_not_a_number_names_which_one() {
        let error = "292,forty,10,7"
            .parse::<ExpectedCounts>()
            .expect_err("`forty` is not a number");
        // The position matters: an operator with four numbers needs to know
        // which one it choked on.
        assert!(error.to_string().contains("count 2"), "got: {error}");
    }

    #[test]
    fn matching_counts_and_satisfied_subsets_pass() {
        let f = fixture(&["a", "b"], &["a"], &["a", "b", "c"]);
        assert!(f.assert_counts(counts(3, 2, 1)).is_ok());
    }

    #[test]
    fn a_count_mismatch_stops_and_prints_both_shapes() {
        let f = fixture(&["a", "b"], &["a"], &["a", "b", "c"]);
        let error = f
            .assert_counts(counts(292, 2, 1))
            .expect_err("the candidate count is wrong");
        let rendered = error.to_string();
        assert!(rendered.contains("STOP 3"));
        assert!(rendered.contains("S-11"), "the scenario must be named");
        assert!(rendered.contains("3/2/1"), "actual: {rendered}");
        assert!(rendered.contains("292/2/1"), "expected: {rendered}");
    }

    /// `relevant ⊆ candidates`, the invariant that makes recall mean anything.
    ///
    /// A relevant id that is not in the pool cannot be recalled at ANY k, so it
    /// depresses every recall number by a constant nobody can see. A fixture
    /// defect would read as a reranker failure.
    #[test]
    fn a_relevant_id_outside_the_candidate_pool_stops() {
        let f = fixture(&["a", "ghost"], &["a"], &["a", "b"]);
        let error = f
            .assert_counts(counts(2, 2, 1))
            .expect_err("`ghost` is not among the candidates");
        let rendered = error.to_string();
        assert!(rendered.contains("\"ghost\""), "got: {rendered}");
        assert!(rendered.contains("no k could ever recall it"));
    }

    /// `included ⊆ relevant`. Gate B asks whether every INCLUDED card is
    /// recalled; an included id the fixture never called relevant makes that
    /// question unanswerable.
    #[test]
    fn an_included_id_that_is_not_relevant_stops() {
        let f = fixture(&["a"], &["a", "b"], &["a", "b"]);
        let error = f
            .assert_counts(counts(2, 1, 2))
            .expect_err("`b` is included but not relevant");
        assert!(error.to_string().contains("included ⊆ relevant"));
    }
}
