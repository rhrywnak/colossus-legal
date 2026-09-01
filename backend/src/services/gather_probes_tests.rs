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
