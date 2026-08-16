//! The mandatory-with-override rule, asserted. Every way of getting a blank date
//! past intake is a way of losing a document's date silently.

use super::*;

#[test]
fn every_token_round_trips() {
    for precision in ALL_DATE_PRECISIONS {
        let token = precision.as_str();
        assert_eq!(
            DatePrecision::from_token(token),
            Some(*precision),
            "'{token}' did not round-trip"
        );
    }
}

#[test]
fn the_tokens_are_the_ones_the_migration_checks() {
    // The CHECK constraint in 20260816143722_add_document_date_and_precision.sql
    // lists these four. If this vocabulary grows without the migration, the
    // insert fails on a live upload.
    let tokens: Vec<&str> = ALL_DATE_PRECISIONS.iter().map(|p| p.as_str()).collect();
    assert_eq!(tokens, vec!["day", "month", "year", "unknown"]);
}

#[test]
fn an_unrecognised_token_is_refused_rather_than_defaulted() {
    assert_eq!(DatePrecision::from_token("moth"), None);
    assert_eq!(DatePrecision::from_token(""), None);
    assert_eq!(
        validate(Some("2009-11-05"), "moth").unwrap_err(),
        DatePrecisionError::UnknownToken {
            token: "moth".to_string()
        }
    );
}

#[test]
fn a_dated_document_is_accepted_at_every_real_precision() {
    for token in ["day", "month", "year"] {
        let (date, precision) = validate(Some("2009-11-05"), token).expect("accepted");
        assert_eq!(date.as_deref(), Some("2009-11-05"));
        assert_eq!(precision.as_str(), token);
        assert!(precision.expects_a_date());
    }
}

#[test]
fn a_silent_blank_is_refused_and_the_error_says_what_to_send_instead() {
    // The whole point of mandatory-with-override. A blank date with a real
    // precision is somebody skipping the field, not somebody answering it.
    let err = validate(None, "day").unwrap_err();
    assert_eq!(err, DatePrecisionError::DateMissing { precision: "day" });
    assert!(
        err.to_string().contains("send precision 'unknown'"),
        "the refusal must name the override, or a user just fights the form: {err}"
    );
}

#[test]
fn an_explicit_unknown_is_accepted_with_no_date() {
    let (date, precision) = validate(None, "unknown").expect("accepted");
    assert_eq!(date, None);
    assert_eq!(precision, DatePrecision::Unknown);
    assert!(!precision.expects_a_date());
}

#[test]
fn unknown_with_a_date_is_refused_as_a_contradiction() {
    // The two halves disagree, and guessing which the user meant would either
    // discard a real date or record a date the user said did not exist.
    assert_eq!(
        validate(Some("2009-11-05"), "unknown").unwrap_err(),
        DatePrecisionError::DateUnexpected {
            date: "2009-11-05".to_string()
        }
    );
}

#[test]
fn unknown_is_an_answer_and_not_the_absence_of_one() {
    // Domain note asserted: `Unknown` is a stored value. The state "nobody has
    // been asked yet" is the ABSENCE of a precision, which this type cannot
    // represent — it is `Option<DatePrecision>::None` at the call site and NULL
    // in the column, and the migration's CHECK keeps the two in step.
    assert!(ALL_DATE_PRECISIONS.contains(&DatePrecision::Unknown));
    assert_eq!(DatePrecision::Unknown.as_str(), "unknown");
}

#[test]
fn every_precision_has_a_label_a_human_can_choose_from() {
    for precision in ALL_DATE_PRECISIONS {
        let label = precision.label();
        assert!(!label.is_empty(), "{precision:?} has no label");
        assert!(
            label != precision.as_str(),
            "{precision:?}'s label is just its token; the intake control would \
             read 'day / month / year / unknown' with no explanation"
        );
    }
}

#[test]
fn the_lookup_version_is_pinned() {
    // Mirrors ACTOR_ROLE_LOOKUP_V: adding or removing a precision is a code
    // change with a matching version bump.
    assert_eq!(DATE_PRECISION_LOOKUP_V, 1);
}
