//! Tests for the gather subject filter's vocabulary and its party sets.

use super::*;
use std::str::FromStr;

/// ⚑ `strict` reaches the subject and NOBODY else — today's behaviour exactly.
///
/// This is the conservation baseline. If `strict` ever reached more than the
/// subject, the "strict reproduces today's pool" identity that the whole
/// conservation check rests on would be quietly false.
#[test]
fn strict_reaches_the_subject_and_nobody_else() {
    let reachable = vec![
        "person-george-phillips".to_string(),
        "person-emil-awad".to_string(),
        "org-catholic-family-services".to_string(),
    ];
    assert_eq!(
        GatherSubjectFilter::Strict.parties("person-george-phillips", &reachable),
        Some(vec!["person-george-phillips"]),
        "strict must ignore the widened set entirely"
    );
}

/// ⚑ `widened` reaches Emil Awad, which is what AT-2 turns on.
#[test]
fn widened_reaches_every_party_the_allegations_named() {
    let reachable = vec![
        "org-catholic-family-services".to_string(),
        "person-emil-awad".to_string(),
        "person-george-phillips".to_string(),
    ];
    let parties = GatherSubjectFilter::Widened
        .parties("person-george-phillips", &reachable)
        .expect("widened filters, it does not disable the filter");

    assert!(
        parties.contains(&"person-emil-awad"),
        "the four admissions filed about Emil Awad alone are unreachable without him"
    );
    assert_eq!(parties.len(), 3);
}

/// `off` is the ABSENCE of a filter, not a filter that matches nothing.
///
/// The distinction is the difference between "everything" and "nothing", and
/// collapsing it is how a gather returns an empty pool with no explanation.
#[test]
fn off_is_no_filter_rather_than_an_empty_one() {
    assert_eq!(
        GatherSubjectFilter::Off.parties("person-george-phillips", &[]),
        None
    );
    // And the contrast: widened with an empty reachable set is a filter that
    // matches nothing, which is a different — and reportable — state.
    assert_eq!(
        GatherSubjectFilter::Widened.parties("s", &[]),
        Some(Vec::new())
    );
}

/// The three tokens round-trip, and serde agrees with `as_str`.
#[test]
fn the_three_tokens_round_trip_through_storage() {
    for mode in GatherSubjectFilter::allowed() {
        assert_eq!(GatherSubjectFilter::from_str(mode.as_str()), Ok(mode));
        assert_eq!(
            serde_json::to_value(mode).expect("serializes"),
            serde_json::json!(mode.as_str()),
            "as_str and the serde tag must not drift"
        );
        assert_eq!(mode.to_string(), mode.as_str());
    }
    assert_eq!(
        GatherSubjectFilter::allowed().map(|m| m.as_str()),
        ["strict", "widened", "off"]
    );
}

/// An unknown stored value names every legal one.
///
/// The row is edited by a human in a settings page; a typo must produce a
/// refusal that says what to type instead, not a silent fall back to a default
/// that searches a different pool.
#[test]
fn an_unknown_value_names_the_three_legal_ones() {
    let err = GatherSubjectFilter::from_str("widend").expect_err("a typo must be refused");
    assert!(err.contains("'widend'"), "{err}");
    for legal in ["strict", "widened", "off"] {
        assert!(err.contains(legal), "the refusal must name {legal}: {err}");
    }
    assert!(GatherSubjectFilter::from_str("").is_err());
    assert!(
        GatherSubjectFilter::from_str("Widened").is_err(),
        "the stored vocabulary is lowercase; a near-miss is still a miss"
    );
}
