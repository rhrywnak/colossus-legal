//! Tests for the export's own pure parts.
//!
//! ## Why a bin has tests at all
//!
//! Everything here fails SILENTLY. A wrong scenario list produces a file for
//! the wrong scenario, or no file at all, and both look like success in a
//! terminal. A human then drafts meeting questions from whatever came out. A
//! crash would be kinder than any of these.

use super::*;

/// The forms a human actually types.
#[test]
fn the_scenario_list_accepts_both_spellings() {
    assert_eq!(parse_ordinals("12,13,14").expect("plain"), vec![12, 13, 14]);
    assert_eq!(parse_ordinals("S-12,S-13").expect("prefixed"), vec![12, 13]);
    assert_eq!(
        parse_ordinals(" 12 , S-13 ,14 ").expect("spaced"),
        vec![12, 13, 14],
        "a list pasted out of a document carries spaces"
    );
    assert_eq!(parse_ordinals("9").expect("one"), vec![9]);
}

/// ⚑ A trailing comma yields the list, not a silent short one.
///
/// `12,13,` is what a human types when they mean three and delete one. Parsed
/// carelessly it becomes a parse error at the empty segment, or worse a
/// scenario 0 that matches nothing and writes no file anyone notices.
#[test]
fn empty_segments_are_skipped_rather_than_becoming_a_scenario() {
    assert_eq!(parse_ordinals("12,13,").expect("trailing"), vec![12, 13]);
    assert_eq!(parse_ordinals("12,,14").expect("doubled"), vec![12, 14]);
    assert_eq!(parse_ordinals(",12").expect("leading"), vec![12]);
}

/// ⚑ Nothing to export is an EMPTY list, and the caller must not treat that as
/// a successful run over five scenarios.
#[test]
fn an_empty_argument_yields_no_scenarios() {
    assert!(parse_ordinals("").expect("empty").is_empty());
    assert!(parse_ordinals("  ,  ").expect("only separators").is_empty());
}

/// A typo is refused by name, not silently dropped.
///
/// Dropping it would run four scenarios when five were asked for and report
/// success — the failure this whole file exists to prevent.
#[test]
fn an_unparseable_ordinal_is_refused_and_quoted() {
    let err = parse_ordinals("12,thirteen,14")
        .expect_err("a word is not an ordinal")
        .to_string();
    assert!(err.contains("'thirteen'"), "{err}");
    assert!(
        err.contains("S-12"),
        "the message must show the accepted form: {err}"
    );

    assert!(
        parse_ordinals("S-").is_err(),
        "a bare prefix is not an ordinal"
    );
    assert!(parse_ordinals("12.5").is_err());
}
