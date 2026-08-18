// Tests for `services::practice_page`.
//
// Everything here composes a sentence a witness reads. The failure mode this
// file exists to catch is not a crash — it is a screen that renders perfectly
// while saying `{when}`, or "0 to repeat", or nothing at all.

use super::*;
use crate::repositories::pipeline_repository::practice::PracticeQuestionRecord;
use chrono::TimeZone;

fn settings() -> Settings {
    Settings::for_test()
}

fn record(tactic: Option<i16>, braid: Option<&str>) -> PracticeQuestionRecord {
    PracticeQuestionRecord {
        id: Uuid::nil(),
        side: "george".to_string(),
        text: "a question".to_string(),
        tactic,
        braid_rows: braid.map(str::to_string),
        source_kind: "instance".to_string(),
        source_ref: Some("doc:evidence:aaa".to_string()),
        receipt: Some("Built from: the hearing, p. 34".to_string()),
        watch_for: None,
        stronger: None,
        stronger_lean: None,
        pair_said: None,
        pair_admitted: None,
        sort_order: 1,
    }
}

/// A card number becomes the card's own name, from the settings row.
///
/// The mapping is 1-based, and the off-by-one is the whole risk: card 4 is the
/// false premise, and a slip would tag the "at each other's throats" question
/// "character jab" — a wrong lesson taught confidently.
#[test]
fn a_card_number_becomes_the_card_name_from_the_stored_vocabulary() {
    let s = settings();
    assert_eq!(
        tactic_name(&s, Some(1)).as_deref(),
        Some("broad generalization")
    );
    assert_eq!(tactic_name(&s, Some(4)).as_deref(), Some("false premise"));
    assert_eq!(tactic_name(&s, Some(5)).as_deref(), Some("compound"));
    assert_eq!(
        tactic_name(&s, Some(6)).as_deref(),
        Some("authority borrow")
    );
    assert_eq!(tactic_name(&s, Some(7)).as_deref(), Some("echo"));
}

/// No card, no tag — and a card the vocabulary cannot name gets no tag either.
///
/// A Chuck question has no trap in it, so a tag would be a lie. A number the row
/// is too short to name is a store that drifted from the deck; printing "card 8"
/// would tell Marie something meaningless, and inventing a name would be worse.
#[test]
fn a_question_with_no_card_and_a_card_with_no_name_both_render_no_tag() {
    let s = settings();
    assert_eq!(tactic_name(&s, None), None);
    assert_eq!(tactic_name(&s, Some(8)), None);
    assert_eq!(tactic_name(&s, Some(0)), None);
}

/// A braid's tag carries its suffix, joined by the space the store cannot hold.
#[test]
fn a_braid_wears_the_card_name_and_the_stored_suffix() {
    let s = settings();
    let dto = question_dto(&s, record(Some(5), Some("Barrage rows 1 · 2 · 5")));

    assert_eq!(dto.tactic.as_deref(), Some("compound · braid"));
    assert!(dto.braid, "the pill must change, not only the tag");

    let plain = question_dto(&s, record(Some(5), None));
    assert_eq!(plain.tactic.as_deref(), Some("compound"));
    assert!(!plain.braid);
}

/// The last-session line is composed, with every slot filled.
///
/// The `{` assertion is the one that matters: this repo has shipped a raw
/// `{when}` token to a screen before, because a template's key and its fill
/// disagreed. Nothing else in the stack notices — the string is well-typed.
#[test]
fn the_last_session_line_fills_every_slot_and_leaves_no_token_behind() {
    let s = settings();
    let record = LastSessionRecord {
        ended_at: Utc.with_ymd_and_hms(2026, 8, 16, 19, 30, 0).unwrap(),
        answered: 5,
        repeats: 2,
    };

    let line = last_session_line(&s.practice_wording, Some(&record));
    assert_eq!(line, "Last session: Sun 16 Aug · 5 questions · 2 to repeat");
    assert!(
        !line.contains('{'),
        "an unfilled slot reached the screen: {line}"
    );
}

/// A witness who has never sat down is TOLD so, in words.
#[test]
fn no_previous_session_yields_the_stored_sentence_and_never_a_blank() {
    let s = settings();
    let line = last_session_line(&s.practice_wording, None);

    assert_eq!(line, s.practice_wording.no_last_session);
    assert!(!line.trim().is_empty());
}

/// An EMPTY deck is a payload, not an error.
///
/// This is the S-6 case the task names: the page must render and say "no practice
/// deck yet — seed it". A function that returned an error here would put a red
/// failure notice in front of a scenario that is simply not seeded yet.
#[test]
fn a_scenario_with_no_deck_still_yields_a_payload_with_its_words() {
    let s = settings();
    let payload = deck_payload(
        &s,
        Uuid::nil(),
        "S-6".to_string(),
        "Too many attorneys".to_string(),
        vec![],
        vec![],
        None,
    );

    assert!(payload.questions.is_empty());
    assert_eq!(payload.code, "S-6");
    assert_eq!(
        payload.wording.empty_deck, "no practice deck yet — seed it",
        "the page needs the sentence even when it has no questions"
    );
}

/// The payload carries no score, no streak and no timer.
///
/// ## Why this is asserted on the SERIALIZED bytes
///
/// The design's "not in v1" list is scoring, streaks, timers and anything that
/// makes it feel like a test. A field cannot leak past the DTO without appearing
/// in this file — but a value smuggled inside a STRING could, and scanning the
/// JSON is what catches that. Same technique the rehearsal payload's exclusion
/// test uses.
#[test]
fn the_payload_carries_nothing_that_would_make_it_feel_like_a_test() {
    let s = settings();
    let payload = deck_payload(
        &s,
        Uuid::nil(),
        "S-5".to_string(),
        "Marie refused to divide property amicably".to_string(),
        vec![record(Some(4), None)],
        vec![],
        None,
    );
    let json = serde_json::to_string(&payload).expect("the payload serializes");

    for banned in [
        "\"score\"",
        "\"streak\"",
        "\"seconds\"",
        "\"elapsed\"",
        "\"grade\"",
    ] {
        assert!(!json.contains(banned), "{banned} reached the wire");
    }
}
