//! Tests for what happens to probes AFTER they are extracted: which are worth
//! reading, and which turn out to be the same probe.
//!
//! Split from `gather_probes_tests.rs` when that file passed 300 code lines
//! (Rule 17). The boundary is real and not just a size cut: that file answers
//! "what counts as a probe", this one answers "and which of them earn a vote".

use super::*;

// ---------------------------------------------------------------------------
// Selectivity — which probes are worth reading
// ---------------------------------------------------------------------------

fn counted(pairs: &[(&str, i64)]) -> Vec<ProbeCount> {
    pairs
        .iter()
        .map(|(probe, matches)| ProbeCount {
            probe: (*probe).to_string(),
            matches: *matches,
        })
        .collect()
}

const ONE_THIRD: f64 = 1.0 / 3.0;
/// The shipped `gather_probe_floor`.
const FLOOR: usize = 3;

/// ⚑ The boundary, from all three sides — and never rescued by the floor.
///
/// Every case here carries a companion probe that passes normally, so `kept` is
/// non-empty and the floor path never runs. Without that companion the floor
/// rescues the single over-threshold probe back into `kept`, `dropped` comes
/// back empty, and the assertion passes whatever the comparison does — which is
/// exactly how an earlier version of this test proved nothing.
#[test]
fn the_share_decides_at_the_boundary_and_on_both_sides_of_it() {
    // 1030 admitted, a third allowed: the ceiling is 343.33.
    let counts = counted(&[("under", 343), ("over", 344), ("exact", 343)]);

    let chosen = select_probes(&counts, 1030, ONE_THIRD, FLOOR);

    assert_eq!(chosen.kept, vec!["under", "exact"]);
    assert_eq!(chosen.dropped, counted(&[("over", 344)]));
    assert!(!chosen.floor_applied, "something was kept normally");
}

/// ⚑ Exactly at the ceiling is KEPT — the comparison is `<=`, not `<`.
///
/// The companion probe is what makes this detectable: with `<` the probe at
/// exactly one third would be dropped, and with only one probe the floor would
/// hand it straight back and the test would pass either way.
#[test]
fn a_probe_at_exactly_the_share_is_kept() {
    // 300 admitted, a third allowed: the ceiling is exactly 100, no float slack.
    let chosen = select_probes(
        &counted(&[("at", 100), ("companion", 50)]),
        300,
        ONE_THIRD,
        FLOOR,
    );

    assert_eq!(
        chosen.kept,
        vec!["at", "companion"],
        "100 of 300 IS one third"
    );
    assert!(chosen.dropped.is_empty(), "and nothing was dropped");
    assert!(!chosen.floor_applied, "so the floor never ran to rescue it");
}

/// One over the ceiling is dropped, and genuinely dropped — not floor-rescued.
#[test]
fn a_probe_one_over_the_share_is_dropped() {
    let chosen = select_probes(
        &counted(&[("over", 101), ("companion", 50)]),
        300,
        ONE_THIRD,
        FLOOR,
    );

    assert_eq!(chosen.kept, vec!["companion"]);
    assert_eq!(
        chosen.dropped,
        counted(&[("over", 101)]),
        "101 of 300 is over a third and stays dropped, because something else was kept"
    );
    assert!(!chosen.floor_applied);
}

/// ⚑ The real numbers: `Court` goes, `$50,000` stays.
///
/// Measured on S-11's gather against an admitted set of 1030.
#[test]
fn the_measured_s11_probes_split_the_way_the_ruling_intended() {
    let counts = counted(&[
        ("Court", 534),
        ("$50,000", 73),
        ("Hanley", 64),
        ("Tighe", 35),
    ]);

    let chosen = select_probes(&counts, 1030, ONE_THIRD, FLOOR);

    assert_eq!(
        chosen.kept,
        vec!["$50,000", "Hanley", "Tighe"],
        "the figures and the names survive"
    );
    assert_eq!(
        chosen.dropped,
        counted(&[("Court", 534)]),
        "the probe that agrees with half the pool does not"
    );
}

/// ⚑ A probe matching NOTHING is kept.
///
/// It costs nothing — an empty list is a no-op in the fusion — and it is
/// information: a term the corpus does not contain. The two ends of the range
/// are not the same thing and a rule that dropped both would be saying they are.
#[test]
fn a_probe_matching_nothing_is_kept() {
    let chosen = select_probes(
        &counted(&[("absent", 0), ("Court", 534)]),
        1030,
        ONE_THIRD,
        FLOOR,
    );

    assert!(chosen.kept.contains(&"absent".to_string()));
    assert_eq!(chosen.dropped, counted(&[("Court", 534)]));
}

/// ⚑ Never zero probes, even when EVERY probe saturates.
///
/// A silently empty trigram half would look exactly like a working one that
/// found nothing — the same invisible failure as the truncation and the 0-row
/// `%` operator. The most selective survive instead, and the state is flagged.
#[test]
fn every_probe_over_the_share_still_leaves_the_most_selective() {
    let counts = counted(&[
        ("Court", 534),
        ("Probate", 800),
        ("Attorney", 600),
        ("Plaintiff", 900),
        ("Defendant", 700),
    ]);

    let chosen = select_probes(&counts, 1030, ONE_THIRD, FLOOR);

    assert_eq!(
        chosen.kept,
        vec!["Court", "Attorney", "Defendant"],
        "the three most selective, in ascending match count"
    );
    assert_eq!(chosen.kept.len(), FLOOR);
    assert_eq!(chosen.dropped.len(), 2);
    assert!(
        chosen.floor_applied,
        "and the caller is told, because 'every probe is generic' is a real state"
    );
}

/// Fewer probes than the floor: all of them survive, and nothing panics on the
/// slice.
#[test]
fn fewer_probes_than_the_floor_all_survive() {
    let chosen = select_probes(
        &counted(&[("Court", 534), ("Probate", 800)]),
        1030,
        ONE_THIRD,
        FLOOR,
    );

    assert_eq!(chosen.kept.len(), 2, "fewer than the floor, so all of them");
    assert!(chosen.dropped.is_empty());
    assert!(chosen.floor_applied);
}

/// No probes at all is not an error and not a floor case.
#[test]
fn no_probes_is_an_empty_selection_not_a_floor() {
    let chosen = select_probes(&[], 1030, ONE_THIRD, FLOOR);

    assert!(chosen.kept.is_empty());
    assert!(chosen.dropped.is_empty());
    assert!(
        !chosen.floor_applied,
        "there was nothing to keep; the floor did not rescue anything"
    );
}

/// An empty admitted set drops nothing to the floor rather than dividing by it.
#[test]
fn an_empty_admitted_set_falls_to_the_floor_rather_than_dividing_by_zero() {
    let chosen = select_probes(&counted(&[("a", 1), ("b", 2)]), 0, ONE_THIRD, FLOOR);

    // The ceiling is 0, so a probe matching anything is over it — but the floor
    // keeps them rather than running none.
    assert_eq!(chosen.kept.len(), 2);
    assert!(chosen.floor_applied);
}

/// Ties in the floor path break on the probe text, so two runs agree.
#[test]
fn the_floor_breaks_ties_on_the_probe_text() {
    let counts = counted(&[
        ("zebra", 500),
        ("alpha", 500),
        ("middle", 500),
        ("omega", 500),
    ]);

    let first = select_probes(&counts, 1030, ONE_THIRD, FLOOR);
    let second = select_probes(&counts, 1030, ONE_THIRD, FLOOR);

    assert_eq!(first.kept, vec!["alpha", "middle", "omega"]);
    assert_eq!(first, second);
}

/// The floor comes from configuration, and the invariant survives a bad value.
///
/// "Never zero" is not defeasible: a stored 0 would switch the guard off from
/// the settings page, so it is clamped here as well as bounded in the row.
#[test]
fn the_floor_is_configurable_but_never_zero() {
    let all_over = counted(&[("a", 900), ("b", 800), ("c", 700), ("d", 600)]);

    assert_eq!(select_probes(&all_over, 1030, ONE_THIRD, 1).kept, vec!["d"]);
    assert_eq!(
        select_probes(&all_over, 1030, ONE_THIRD, 2).kept,
        vec!["d", "c"],
        "the most selective first"
    );
    assert_eq!(
        select_probes(&all_over, 1030, ONE_THIRD, 0).kept.len(),
        1,
        "a stored zero is clamped to one — the trigram half must never be empty"
    );
    assert_eq!(
        select_probes(&all_over, 1030, ONE_THIRD, 99).kept.len(),
        4,
        "and a floor above the probe count keeps all of them, not a panic"
    );
}

// ---------------------------------------------------------------------------
// One term, one vote
// ---------------------------------------------------------------------------

fn listed(pairs: &[(&str, &[&str])]) -> Vec<(String, Vec<String>)> {
    pairs
        .iter()
        .map(|(probe, hits)| {
            (
                (*probe).to_string(),
                hits.iter().map(|h| (*h).to_string()).collect(),
            )
        })
        .collect()
}

/// ⚑ The measured duplicates collapse, and the shortest spelling survives.
///
/// `$50,000`, `$50,000.00` and `$500,000.00` returned the identical 65 ids on
/// the real corpus, so every card they matched was scored three times for one
/// match. That is the fusion's central signal — several independent probes
/// agreeing — manufactured out of spelling variants.
#[test]
fn probes_with_identical_result_sets_collapse_to_the_shortest_spelling() {
    let (kept, groups) = collapse_identical(listed(&[
        ("$50,000.00", &["a", "b"]),
        ("$500,000.00", &["a", "b"]),
        ("$50,000", &["a", "b"]),
        ("Tighe", &["c"]),
    ]));

    assert_eq!(
        kept.len(),
        2,
        "three duplicates became one vote, plus Tighe"
    );
    assert!(
        kept.iter().any(|(p, _)| p == "$50,000"),
        "shortest wins: {kept:?}"
    );
    assert!(kept.iter().any(|(p, _)| p == "Tighe"));

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].representative, "$50,000");
    assert_eq!(groups[0].collapsed, vec!["$50,000.00", "$500,000.00"]);
}

/// ⚑ Sets that merely OVERLAP do not collapse, however heavily.
///
/// This is what exact equality buys. `Hanley` and `Higgs` are different people
/// appearing in overlapping documents; a similarity rule loose enough to merge
/// spelling variants would be loose enough to merge them, and one party's
/// evidence would silently vanish from the ranking's reckoning.
#[test]
fn heavily_overlapping_sets_stay_apart() {
    // Ten DISTINCT ids, nine of them shared — a Jaccard of 9/11, far above any
    // threshold anyone would set. An earlier version of this test used ten
    // repeats of one id, so the two sets differed in CARDINALITY (1 vs 2) and
    // it passed without ever reaching the overlap case it claimed to test.
    let shared: Vec<String> = (0..9).map(|n| format!("card-{n}")).collect();
    let mut hanley: Vec<&str> = shared.iter().map(String::as_str).collect();
    let mut higgs = hanley.clone();
    hanley.push("only-hanley");
    higgs.push("only-higgs");

    let (kept, groups) = collapse_identical(listed(&[("Hanley", &hanley), ("Higgs", &higgs)]));

    assert_eq!(
        kept.len(),
        2,
        "nine shared ids out of ten is not equality, and both keep their vote"
    );
    assert!(groups.is_empty(), "{groups:?}");
}

/// Duplicate ids within one probe's list do not defeat the collapse.
///
/// The key is deduplicated, so two probes reaching the same cards collapse
/// however many rows each returned. The read should never produce duplicates —
/// `evidence_id` is the mirror's primary key — but the collapse must not be the
/// thing that breaks if one ever does.
#[test]
fn a_repeated_id_does_not_defeat_the_collapse() {
    let (kept, groups) = collapse_identical(listed(&[
        ("longer-name", &["a", "b", "b"]),
        ("short", &["a", "a", "b"]),
    ]));

    assert_eq!(kept.len(), 1, "the same SET, whatever the row counts");
    assert_eq!(groups[0].representative, "short");
}

/// A single differing element keeps two probes apart, and so does a subset.
#[test]
fn a_subset_is_not_an_identical_set() {
    let (kept, groups) = collapse_identical(listed(&[
        ("wide", &["a", "b", "c"]),
        ("narrow", &["a", "b"]),
    ]));

    assert_eq!(kept.len(), 2);
    assert!(groups.is_empty(), "a subset is not equality: {groups:?}");
}

/// ⚑ Comparison is over SETS, not sequences.
///
/// Two probes with the same matches can order them differently, because each
/// row's `word_similarity` is measured against its own probe. Comparing the
/// ordered lists would miss exactly the duplicates this exists to catch.
#[test]
fn the_same_ids_in_a_different_order_are_the_same_set() {
    let (kept, groups) = collapse_identical(listed(&[
        ("longer-spelling", &["b", "a"]),
        ("short", &["a", "b"]),
    ]));

    assert_eq!(kept.len(), 1, "same ids, different order, one vote");
    assert_eq!(groups[0].representative, "short");
    assert_eq!(
        kept[0].1,
        vec!["a", "b"],
        "and the representative keeps ITS OWN ranked order, not a merge"
    );
}

/// Equal-length spellings break the tie lexicographically, so runs agree.
#[test]
fn an_equal_length_tie_breaks_stably() {
    let input = listed(&[("zebra", &["a"]), ("alpha", &["a"]), ("mango", &["a"])]);

    let (first, groups) = collapse_identical(input.clone());
    let (second, _) = collapse_identical(input);

    assert_eq!(groups[0].representative, "alpha");
    assert_eq!(first, second);
}

/// Nothing to collapse leaves everything alone.
#[test]
fn distinct_probes_are_left_untouched() {
    let (kept, groups) = collapse_identical(listed(&[("a", &["1"]), ("b", &["2"])]));

    assert_eq!(kept.len(), 2);
    assert!(groups.is_empty());
    assert!(collapse_identical(Vec::new()).0.is_empty());
}
