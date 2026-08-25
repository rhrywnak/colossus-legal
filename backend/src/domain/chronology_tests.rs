//! Unit tests for the chronology vocabularies.
//!
//! Behavioural, with named inputs and stated expected outputs — a compile is not
//! verification. The vocabulary's CONTENT is pinned here; its agreement with the
//! seed file and with the migrations is pinned by the permanent guard in
//! `chronology::seed_tests`.

use super::*;
use crate::domain::date_precision::DatePrecision;

#[test]
fn the_five_seed_tags_are_the_vocabulary() {
    let tokens: Vec<&str> = CHRONOLOGY_TAGS.iter().map(|(t, _)| *t).collect();
    assert_eq!(
        tokens,
        vec![
            "financial",
            "court_action",
            "filing",
            "discovery",
            "personal"
        ],
        "the seed tag vocabulary changed; the timeline.json categories must change with it"
    );
}

#[test]
fn every_tag_carries_a_distinct_label() {
    let mut labels: Vec<&str> = CHRONOLOGY_TAGS.iter().map(|(_, l)| *l).collect();
    labels.sort_unstable();
    let before = labels.len();
    labels.dedup();
    assert_eq!(before, labels.len(), "two tags share a display label");
}

#[test]
fn is_known_tag_accepts_a_member_and_refuses_a_stranger() {
    assert!(is_known_tag("court_action"));
    // The LABEL is not the token — storing "Court Action" would be the bug this
    // asserts against.
    assert!(!is_known_tag("Court Action"));
    assert!(!is_known_tag("hearsay"));
    assert!(!is_known_tag(""));
}

#[test]
fn tag_label_resolves_a_member_and_returns_none_for_a_stranger() {
    assert_eq!(tag_label("filing"), Some("Filing"));
    assert_eq!(tag_label("court_action"), Some("Court Action"));
    assert_eq!(tag_label("hearsay"), None);
}

#[test]
fn chronology_precisions_are_the_three_that_expect_a_date() {
    let precisions = chronology_precisions();
    assert_eq!(
        precisions,
        vec![
            DatePrecision::Day,
            DatePrecision::Month,
            DatePrecision::Year
        ],
        "the chronology's precision subset drifted from expects_a_date()"
    );
    assert!(
        !precisions.contains(&DatePrecision::Unknown),
        "an event always has a date; 'unknown' can never be a chronology precision"
    );
}

#[test]
fn the_precision_subset_matches_the_migration_check() {
    // The three tokens the CHECK constraint lists, spelled out here so a drift in
    // either direction is a test failure rather than a runtime constraint
    // violation on the first write.
    let tokens: Vec<&str> = chronology_precisions().iter().map(|p| p.as_str()).collect();
    assert_eq!(tokens, vec!["day", "month", "year"]);
}

#[test]
fn validate_precision_accepts_each_member() {
    assert_eq!(validate_precision("day"), Ok(DatePrecision::Day));
    assert_eq!(validate_precision("month"), Ok(DatePrecision::Month));
    assert_eq!(validate_precision("year"), Ok(DatePrecision::Year));
}

#[test]
fn validate_precision_refuses_unknown_differently_from_a_typo() {
    // 'unknown' is a token this build understands and refuses on purpose.
    let refused = validate_precision("unknown").expect_err("'unknown' must be refused");
    assert!(
        matches!(refused, ChronologyPrecisionError::NeedsADate { .. }),
        "got {refused:?}; 'unknown' is a known token, not an unrecognised one"
    );
    assert!(
        refused.to_string().contains("an event always has a date"),
        "the message must say WHY, got: {refused}"
    );

    // A typo is a different refusal with a different message.
    let typo = validate_precision("moth").expect_err("'moth' must be refused");
    assert!(
        matches!(typo, ChronologyPrecisionError::Unknown { .. }),
        "got {typo:?}"
    );
    assert!(
        typo.to_string().contains("day, month, year"),
        "the message must list what IS valid, got: {typo}"
    );
}

#[test]
fn validate_precision_refuses_an_empty_token() {
    assert!(matches!(
        validate_precision(""),
        Err(ChronologyPrecisionError::Unknown { .. })
    ));
}
