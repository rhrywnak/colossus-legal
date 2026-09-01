//! Tests for the probe extractor.
//!
//! The rule is meant to be checkable by eye, so these read as examples rather
//! than as properties: what goes in, what comes out, and why.

use super::*;

fn probes(query: &str) -> Vec<String> {
    probes_of(query)
}

/// ⚑ The two probes AT-2 turns on survive the extractor intact.
///
/// If `$50,000` or `Milster` were missing, the trigram half would be searching
/// for the wrong things and no ranking number would say so. The dollar sign and
/// the comma must survive: they are exactly what distinguishes `$50,000` from
/// a bare `50000` in a scrap receipt, which the full-text analyser cannot.
#[test]
fn the_figure_and_the_name_survive_with_their_punctuation() {
    let found = probes("allegations of theft regarding $50,000.00 that Mr. Milster deposited");

    assert!(found.contains(&"$50,000.00".to_string()), "{found:?}");
    assert!(found.contains(&"Milster".to_string()), "{found:?}");
    assert!(
        !found.iter().any(|p| p.contains("50000")),
        "the comma is the distinguishing character and must not be stripped: {found:?}"
    );
}

/// Punctuation clinging to either end is stripped; punctuation inside is not.
#[test]
fn surrounding_punctuation_goes_and_inner_punctuation_stays() {
    let found = probes(r#"the check ("$50,000"), dated 11/1/2013, from Awad."#);

    assert!(
        found.contains(&"$50,000".to_string()),
        "quotes and brackets go: {found:?}"
    );
    assert!(
        found.contains(&"11/1/2013".to_string()),
        "the slashes are the date: {found:?}"
    );
    assert!(
        found.contains(&"Awad".to_string()),
        "the full stop goes: {found:?}"
    );
}

/// Ordinary lower-case prose is left to the full-text half.
///
/// Probing on it would return most of the corpus — trigram has no stemming and
/// no stopwords, so "deposited" would match every card containing "deposit".
#[test]
fn ordinary_prose_produces_no_probes() {
    assert!(probes("the money was never returned to the estate").is_empty());
    assert!(probes("").is_empty());
    assert!(probes("   \n  ").is_empty());
}

/// ⚑ A capitalised sentence-opener does not become a probe.
///
/// "The" would match almost every row in the corpus and drown the ranking. The
/// four-character floor is what excludes it without needing a stopword list —
/// which would be domain vocabulary compiled into shared code.
#[test]
fn a_short_capitalised_word_is_not_a_probe() {
    let found = probes("The CFS was appointed. She saw him.");

    for too_short in ["The", "CFS", "She", "him"] {
        assert!(
            !found.contains(&too_short.to_string()),
            "{too_short} is under the {MIN_PROBE_CHARS}-character floor: {found:?}"
        );
    }
}

/// A digit anywhere in the token qualifies it, not just a leading one.
#[test]
fn a_digit_anywhere_makes_a_probe() {
    let found = probes("case 2013-PR-0041 and exhibit 14b were filed");

    assert!(found.contains(&"2013-PR-0041".to_string()), "{found:?}");
    // "14b" is three characters and falls under the floor — the deliberate
    // trade: a docket number that short carries one trigram and would match
    // arbitrarily. The floor is pinned from BOTH sides for digit tokens, as it
    // is for capitalised ones, so a digit-specific minimum could not drift in
    // unnoticed.
    assert!(
        !found.contains(&"14b".to_string()),
        "3 chars, under the floor: {found:?}"
    );
    assert!(
        probes("exhibit 14bc was filed").contains(&"14bc".to_string()),
        "4 chars, exactly at the floor, and it must be kept"
    );
}

/// Probes are a SET, sorted, so two runs produce the same trigram ranking.
#[test]
fn probes_are_deduplicated_and_ordered() {
    let found = probes("Milster paid Milster and Awad. $50,000 then $50,000 again.");

    assert_eq!(
        found,
        vec!["$50,000", "Awad", "Milster"],
        "deduplicated and sorted, so the ranking is reproducible"
    );
    assert_eq!(probes_of("Awad Milster"), probes_of("Milster Awad"));
}

/// A bare currency symbol is not a probe.
///
/// `$` alone, or `$.`, would otherwise slip through on the strength of being
/// punctuation the cleaner deliberately keeps.
#[test]
fn a_bare_currency_symbol_is_not_a_probe() {
    assert!(probes("$ and $. and $x").is_empty());
}

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
