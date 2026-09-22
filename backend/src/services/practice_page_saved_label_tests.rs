// Tests for `services::practice_page::answer_saved_label` (v2.2.1, Fix 2).
//
// A sibling of `practice_page_tests.rs`, which was already over Rule 17's 300
// lines before this task; it borrows that file's `settings()` fixture.

use super::tests::settings;
use super::*;
use chrono::TimeZone;

/// The answer box's label names the DAY AND THE TIME, in the case's zone.
///
/// v2.2.1, Fix 2. The instant is DEV's 02:54 UTC press of 2026-09-22 — which
/// was 10:54 pm the evening BEFORE in Michigan. A UTC rendering would tell
/// Marie she answered "tomorrow".
#[test]
fn the_saved_label_composes_day_and_time_in_the_case_zone() {
    let mut s = settings();
    s.practice_read.case_timezone = "America/Detroit".to_string();
    s.practice_wording.row.answer_saved_template = "Your answer — saved {when}".to_string();
    let at = Utc
        .with_ymd_and_hms(2026, 9, 22, 2, 54, 0)
        .single()
        .expect("a valid instant");

    assert_eq!(
        answer_saved_label(&s, at),
        "Your answer — saved Mon 21 Sep · 10:54 pm"
    );
}

/// The template's token is filled from the stored row, never shipped raw.
#[test]
fn the_saved_label_leaves_no_token_unfilled() {
    let s = settings();
    let line = answer_saved_label(
        &s,
        Utc.with_ymd_and_hms(2026, 8, 21, 3, 30, 0)
            .single()
            .expect("a valid instant"),
    );
    assert!(!line.contains("{when}"), "{line:?}");
    assert!(
        line.starts_with("Your answer"),
        "the stored template, not a literal: {line:?}"
    );
}
