// Tests for `domain::text`.

use super::*;

#[test]
fn an_absent_value_stays_absent() {
    assert_eq!(non_blank(None), None);
}

#[test]
fn an_empty_string_reads_as_absent() {
    assert_eq!(non_blank(Some("")), None);
}

#[test]
fn a_whitespace_only_string_reads_as_absent() {
    // Four DEV documents carry `document_date = '   '`-shaped values; a row of
    // spaces renders as a value that is not there.
    assert_eq!(non_blank(Some("   \t\n")), None);
}

#[test]
fn a_real_value_comes_back_trimmed() {
    assert_eq!(
        non_blank(Some("  Judge Tighe  ")),
        Some("Judge Tighe".into())
    );
}

#[test]
fn inner_whitespace_is_left_alone() {
    // Only the ENDS are trimmed. A quote's internal spacing is the record's.
    assert_eq!(
        non_blank(Some(" no  authority  was given ")),
        Some("no  authority  was given".into())
    );
}
