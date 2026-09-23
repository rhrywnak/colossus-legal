// Tests for `services::for_you::group_unread` — the deck threshold (L3).
//
// Ruling 2, as code: a deck holding at least `practice_for_you_deck_threshold`
// UNREAD items shows as ONE row; fewer shows the items. The properties worth
// holding down are the ones a screenshot cannot show — that the threshold
// counts unread and not everything, that the witness's list is never grouped,
// that no item is LOST by grouping, and that the list keeps one order either
// way.

use super::*;
use crate::domain::wording_for_you::ForYouWording;
use crate::services::for_you_rows::RowVoice;
use chrono::{TimeZone, Utc};
use uuid::Uuid;

const TZ: &str = "America/Detroit";

fn bench() -> Vec<String> {
    vec!["cpenzien".to_string()]
}

/// One unread item on `deck`, written on `day` of September 2026.
fn item(deck: u128, day: u32, ordinal: i32) -> WaitingItemRow {
    WaitingItemRow {
        kind: "answer".to_string(),
        item_id: Uuid::new_v4(),
        scenario_id: Uuid::from_u128(deck),
        code_ordinal: Some(ordinal),
        scenario_name: format!("Deck {deck}"),
        question_id: Some(Uuid::new_v4()),
        question_text: Some("Whose money?".to_string()),
        at: Utc
            .with_ymd_and_hms(2026, 9, day, 15, 30, 0)
            .single()
            .expect("a real moment"),
        author: Some("docmarie".to_string()),
        seen_at: None,
        body: Some("her answer".to_string()),
        subject_at: None,
        reply_to: None,
    }
}

fn voice<'a>(
    w: &'a ForYouWording,
    logins: &'a [String],
    names: &'a [String],
    side: ForYouSide,
) -> RowVoice<'a> {
    RowVoice {
        wording: w,
        timezone: TZ,
        side,
        reviewer_logins: logins,
        reviewer_names: names,
        witness_login: "docmarie",
        today: chrono::NaiveDate::from_ymd_opt(2026, 9, 22).expect("a real day"),
    }
}

/// Newest first, which is the order the query returns and this code assumes.
fn deck_of(count: usize, deck: u128) -> Vec<WaitingItemRow> {
    (0..count)
        // day 21, 20, 19 … so the first element is the newest.
        .map(|i| item(deck, 21 - u32::try_from(i).unwrap_or(0), 11))
        .collect()
}

/// Below the threshold: every item keeps its own row.
#[test]
fn a_deck_under_the_threshold_shows_its_items() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let rows = group_unread(&voice(&w, &l, &n, ForYouSide::Reviewers), &deck_of(2, 1), 5);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.kind == "answer"));
}

/// At the threshold: ONE row, which says how many and how old.
///
/// Journey (h) of the instruction, as a unit test: two unread items show two
/// rows at a threshold of 5 and ONE row at a threshold of 1 — the same list,
/// the same data, one settings row apart.
#[test]
fn a_deck_at_the_threshold_becomes_one_row() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let v = voice(&w, &l, &n, ForYouSide::Reviewers);
    let deck = deck_of(2, 1);

    assert_eq!(group_unread(&v, &deck, 5).len(), 2);

    let grouped = group_unread(&v, &deck, 1);
    assert_eq!(grouped.len(), 1);
    let row = &grouped[0];
    assert_eq!(row.kind, "deck");
    assert_eq!(row.body, "2 answers waiting");
    // The OLDEST of the two, not the newest: how long the deck has waited.
    assert_eq!(row.byline, "oldest Sun 20 Sep");
    // A deck row opens the deck, never a question.
    assert_eq!(row.question_id, None);
    // And it is never drawn as already read — it is built from unread items.
    assert!(!row.read);
}

/// A deck row's first line names the deck and no question.
#[test]
fn a_deck_row_names_the_deck_without_a_question() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let rows = group_unread(&voice(&w, &l, &n, ForYouSide::Reviewers), &deck_of(5, 1), 5);
    assert_eq!(rows[0].deck_line, "S-11 · Deck 1");
    assert!(!rows[0].deck_line.contains("Whose money?"));
}

/// Exactly one unread item, threshold 1: the singular ships.
///
/// Reachable the moment Roman sets the row to 1 — which is exactly when
/// "1 answers waiting" would be on screen.
#[test]
fn a_single_item_deck_row_uses_the_singular() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let rows = group_unread(&voice(&w, &l, &n, ForYouSide::Reviewers), &deck_of(1, 1), 1);
    assert_eq!(rows[0].body, "1 answer waiting");
}

/// (M) The WITNESS's list is never grouped, at any threshold.
///
/// She answers one question at a time. Collapsing four notes into "4 answers
/// waiting" would hide the one sentence she has to read before she can answer
/// anything — and the deck row opens the REVIEW page, which is not her screen.
#[test]
fn the_witness_list_is_never_grouped() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let rows = group_unread(&voice(&w, &l, &n, ForYouSide::Witness), &deck_of(9, 1), 1);
    assert_eq!(rows.len(), 9);
    assert!(rows.iter().all(|r| r.kind != "deck"));
}

/// Two decks, one busy and one not: one deck row and the other's items.
///
/// And the ORDER holds — the deck row sits where its newest item sat, so the
/// list reads newest-first whether or not anything grouped.
#[test]
fn grouping_is_per_deck_and_keeps_the_order() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let mut rows = Vec::new();
    // Deck 2's single item is the newest thing in the list.
    rows.push(item(2, 22, 7));
    rows.extend(deck_of(5, 1));

    let out = group_unread(&voice(&w, &l, &n, ForYouSide::Reviewers), &rows, 5);
    assert_eq!(out.len(), 2, "one item row, one deck row");
    assert_eq!(out[0].kind, "answer", "the newest item still leads");
    assert_eq!(out[1].kind, "deck");
    assert_eq!(out[1].body, "5 answers waiting");
}

/// (M) Nothing is lost: the deck row counts every item it replaced.
///
/// The failure this guards is the one that looks like working software — a
/// grouped list that quietly drops rows and still reads as a complete page.
#[test]
fn a_deck_row_accounts_for_every_item_it_replaced() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let rows = group_unread(&voice(&w, &l, &n, ForYouSide::Reviewers), &deck_of(7, 1), 5);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].body, "7 answers waiting");
}

/// A threshold of 0 cannot collapse everything.
///
/// The store's `min_value` is 1, so this is unreachable through Settings — and
/// it is handled anyway, because Standing Rule 1 has no "unreachable in
/// practice" clause and a 0 here would group a deck holding one item.
#[test]
fn a_zero_threshold_is_read_as_one_not_as_group_everything() {
    let w = ForYouWording::for_test();
    let (l, n) = (bench(), vec!["Chuck".to_string()]);
    let rows = group_unread(&voice(&w, &l, &n, ForYouSide::Reviewers), &deck_of(1, 1), 0);
    assert_eq!(rows.len(), 1);
    // Grouped (threshold 1), not multiplied and not dropped.
    assert_eq!(rows[0].kind, "deck");
}
