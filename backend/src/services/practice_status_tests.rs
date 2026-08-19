// Tests for `services::practice_status`.
//
// Both functions here compose a sentence a witness reads about her own past
// work, and both have a failure mode nothing else in the stack can see: the
// string is well-typed whatever it says. A status reading `answered today ·
// fine` on a question she set aside, or `1 of 0 answered.` on a sitting that
// predates the stored queue, is not a crash, not a type error and not a lint —
// it is a screen quietly telling her something untrue.
//
// The clock is a PARAMETER, which is what makes every case below deterministic:
// there is no "run this before midnight" test in this file.

use super::*;
use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::practice_flow::{OpenSessionRecord, RowStatusRecord};
use chrono::TimeZone;
use uuid::Uuid;

/// A fixed instant, so "today" means the same thing on every run.
fn at(day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, day, hour, minute, 0)
        .single()
        .expect("a real instant")
}

fn status(mark: &str, answered_at: DateTime<Utc>, attempts: i64) -> RowStatusRecord {
    RowStatusRecord {
        question_id: Uuid::nil(),
        mark: mark.to_string(),
        answered_at,
        attempts,
    }
}

#[test]
fn an_answer_today_reads_as_today_and_names_the_mark() {
    let settings = Settings::for_test();
    let line = row_status(&settings, at(19, 21, 0), &status("fine", at(19, 20, 0), 1));
    assert_eq!(line, "answered today · fine");
}

#[test]
fn a_repeat_today_names_the_repeat_and_not_the_fine() {
    let settings = Settings::for_test();
    let line = row_status(
        &settings,
        at(19, 21, 0),
        &status("repeat", at(19, 20, 0), 1),
    );
    assert_eq!(line, "answered today · repeat");
}

/// A question she was DEALT and set aside has its own sentence.
///
/// The defect this pins is the one the sheet's mark cell had before flow v1: an
/// `else` arm that turned every non-repeat into "fine" printed `answered today ·
/// fine` against a question she never answered.
#[test]
fn a_skip_today_is_its_own_sentence_and_never_answered() {
    let settings = Settings::for_test();
    let line = row_status(
        &settings,
        at(19, 21, 0),
        &status("skipped", at(19, 20, 0), 1),
    );
    assert_eq!(line, "skipped today");
    assert!(!line.contains("answered"));
}

#[test]
fn an_answer_on_an_earlier_day_is_dated_rather_than_called_today() {
    let settings = Settings::for_test();
    let line = row_status(&settings, at(19, 9, 0), &status("repeat", at(18, 21, 0), 1));
    assert_eq!(line, "last: Tue 18 Aug · repeat");
}

/// The attempt suffix appears above one attempt and never at one.
///
/// "attempt 1" on every row is noise; the number only means something once it is
/// above one, and the task says so in those words.
#[test]
fn the_attempt_count_appears_only_above_one() {
    let settings = Settings::for_test();
    let once = row_status(&settings, at(19, 21, 0), &status("fine", at(19, 20, 0), 1));
    let twice = row_status(&settings, at(19, 21, 0), &status("fine", at(19, 20, 0), 2));
    assert_eq!(once, "answered today · fine");
    assert_eq!(twice, "answered today · fine · attempt 2");
}

/// No composed status ships a raw placeholder.
///
/// `render` takes UNBRACED keys and supplies the braces itself; a braced key at
/// a call site matches nothing and ships `{mark}` to Marie's screen. This repo
/// has done exactly that with `{when}`, and nothing in the build can warn about
/// it because the string is well-typed either way.
#[test]
fn no_status_ships_a_raw_placeholder() {
    let settings = Settings::for_test();
    for record in [
        status("fine", at(19, 20, 0), 3),
        status("repeat", at(18, 20, 0), 2),
        status("skipped", at(19, 20, 0), 1),
    ] {
        let line = row_status(&settings, at(19, 21, 0), &record);
        assert!(!line.contains('{'), "a placeholder survived: {line}");
    }
}

fn open(
    started_at: DateTime<Utc>,
    who: &str,
    answered: i64,
    queue_len: Option<i32>,
) -> OpenSessionRecord {
    OpenSessionRecord {
        id: Uuid::nil(),
        who: who.to_string(),
        started_at,
        answered,
        queue_len,
    }
}

#[test]
fn a_sitting_started_today_says_today_and_the_clock() {
    let settings = Settings::for_test();
    let line = open_session_detail(
        &settings,
        at(19, 10, 30),
        &open(at(19, 9, 57), "george", 1, Some(5)),
    );
    assert_eq!(line, "· today 09:57 · George's side · 1 of 5 answered.");
}

#[test]
fn a_sitting_left_on_another_day_is_dated() {
    let settings = Settings::for_test();
    let line = open_session_detail(
        &settings,
        at(19, 10, 30),
        &open(at(18, 21, 5), "mixed", 3, Some(10)),
    );
    assert_eq!(line, "· Tue 18 Aug 21:05 · Mixed · 3 of 10 answered.");
}

/// A sitting with no stored queue reports the dash, not a zero.
///
/// Sessions opened before flow v1 carry no queue. `1 of 0 answered.` reads as a
/// bug AND is a lie about her evening; the stored dash says the number is not
/// known, which is the same position the sheet's `Ended early.` clause takes.
#[test]
fn a_sitting_with_no_stored_queue_refuses_to_invent_a_total() {
    let settings = Settings::for_test();
    let line = open_session_detail(
        &settings,
        at(19, 10, 30),
        &open(at(19, 9, 0), "chuck", 2, None),
    );
    assert_eq!(line, "· today 09:00 · Chuck · 2 of — answered.");
    assert!(!line.contains("of 0"));
}

/// An unrecognised side is SHOWN, never disguised as George's.
///
/// The column's CHECK permits three values, so this is unreachable through the
/// database — which is the point: a fourth side added to a migration and not to
/// the match must appear on the screen rather than being silently renamed.
#[test]
fn an_unknown_side_renders_itself() {
    let settings = Settings::for_test();
    let line = open_session_detail(
        &settings,
        at(19, 10, 0),
        &open(at(19, 9, 0), "someone_else", 0, Some(1)),
    );
    assert!(
        line.contains("someone_else"),
        "the unknown side vanished: {line}"
    );
}
