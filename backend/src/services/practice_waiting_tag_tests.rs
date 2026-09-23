// Tests for `services::practice_waiting_tag` — the board-4 mark.
//
// What a reader of a deck must be able to trust about the mark on a row:
// that it is about THAT question, that it says which KIND of thing arrived,
// that it names a person rather than a login, that it says how many others are
// under the one it names, and that a row with nothing unread wears nothing at
// all. Each of those is a way the mark could lie quietly.

use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

use super::*;

fn settings() -> Settings {
    Settings::for_test()
}

fn at(day: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, day, 15, 0, 0)
        .single()
        .expect("a real instant")
}

/// One unread item on `question`, of `kind`, written by `author` on `day`.
fn item(kind: &str, author: Option<&str>, question: Uuid, day: u32) -> WaitingItemRow {
    WaitingItemRow {
        kind: kind.to_string(),
        item_id: Uuid::new_v4(),
        scenario_id: Uuid::from_u128(1),
        code_ordinal: Some(11),
        scenario_name: "The $50,000".to_string(),
        question_id: Some(question),
        question_text: Some("Whose money?".to_string()),
        at: at(day),
        author: author.map(str::to_string),
        seen_at: None,
        body: Some("some words".to_string()),
        subject_at: None,
        reply_to: None,
    }
}

/// A note waiting on the row says who left it — by NAME, never by login.
#[test]
fn a_waiting_note_names_the_person_who_left_it() {
    let q = Uuid::from_u128(7);
    let tag = waiting_tag(&settings(), &[item("note", Some("cpenzien"), q, 21)], q);
    // `cpenzien` is the stored login; `Chuck` is the display name beside it.
    assert_eq!(tag.as_deref(), Some("Chuck left a note"));
}

/// An unread ANSWER wears the answer mark — the reviewer's side of board 4.
#[test]
fn a_waiting_answer_wears_the_answer_mark() {
    let q = Uuid::from_u128(7);
    let tag = waiting_tag(&settings(), &[item("answer", Some("docmarie"), q, 21)], q);
    assert_eq!(tag.as_deref(), Some("Marie answered"));
}

/// A rewording names nobody. (M) If it used the note template it would read
/// "Chuck left a note" about a question he EDITED — the wrong fact, stated
/// confidently, which is the failure this whole layer is written against.
#[test]
fn a_rewording_says_the_question_changed_and_names_nobody() {
    let q = Uuid::from_u128(7);
    let tag = waiting_tag(&settings(), &[item("change", Some("cpenzien"), q, 21)], q);
    assert_eq!(tag.as_deref(), Some("the question changed"));
    assert!(!tag.expect("a mark").contains("Chuck"));
}

/// Several unread items: the NEWEST names the mark, and the rest are counted.
#[test]
fn several_items_name_the_newest_and_count_the_rest() {
    let q = Uuid::from_u128(7);
    // Newest first, as the query returns them.
    let rows = vec![
        item("note", Some("cpenzien"), q, 22),
        item("note", Some("cpenzien"), q, 21),
        item("change", None, q, 20),
    ];
    // +2, not +3: the count is the items BESIDES the one named.
    assert_eq!(
        waiting_tag(&settings(), &rows, q).as_deref(),
        Some("Chuck left a note +2 more")
    );
}

/// One item wears no count clause — "+0 more" never ships.
#[test]
fn one_item_wears_no_count_clause() {
    let q = Uuid::from_u128(7);
    let tag = waiting_tag(&settings(), &[item("note", Some("cpenzien"), q, 21)], q);
    assert!(!tag.expect("a mark").contains("more"));
}

/// The mark is about THIS question. (M) Without the filter every row of the
/// deck would wear the same mark, which is a deck that says everything is
/// waiting everywhere.
#[test]
fn an_item_on_another_question_marks_nothing_here() {
    let mine = Uuid::from_u128(7);
    let other = Uuid::from_u128(8);
    let rows = vec![item("note", Some("cpenzien"), other, 21)];
    assert_eq!(waiting_tag(&settings(), &rows, mine), None);
    assert!(waiting_tag(&settings(), &rows, other).is_some());
}

/// Nothing unread, no mark — absent, not an empty string.
#[test]
fn a_question_with_nothing_unread_wears_no_mark() {
    assert_eq!(waiting_tag(&settings(), &[], Uuid::from_u128(7)), None);
}

/// An item whose author is not on record still marks the row, with the stored
/// stand-in name. A blank where a name goes reads as a tag that failed to load.
#[test]
fn an_unattributed_item_still_marks_the_row() {
    let q = Uuid::from_u128(7);
    let tag = waiting_tag(&settings(), &[item("note", None, q, 21)], q);
    assert_eq!(tag.as_deref(), Some("Someone left a note"));
}

/// A note about the whole SCENARIO (no question) marks no row.
///
/// It is on the For you list and it is swept by Done reviewing, but there is no
/// question row for it to sit on — and attaching it to an arbitrary one would
/// send her to a question the note is not about.
#[test]
fn a_scenario_note_marks_no_question_row() {
    let q = Uuid::from_u128(7);
    let mut row = item("note", Some("cpenzien"), q, 21);
    row.question_id = None;
    assert_eq!(waiting_tag(&settings(), &[row], q), None);
}
