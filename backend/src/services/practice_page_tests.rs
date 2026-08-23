// Tests for `services::practice_page`.
//
// Everything here composes a sentence a witness reads. The failure mode this
// file exists to catch is not a crash — it is a screen that renders perfectly
// while saying `{when}`, or "0 to repeat", or nothing at all.

use super::*;
use crate::repositories::pipeline_repository::practice::{
    PracticePointReceipt, PracticePointRecord, PracticeQuestionRecord,
};
use chrono::TimeZone;

pub(super) fn settings() -> Settings {
    Settings::for_test()
}

fn point(position: i32, exhibit: Option<&str>) -> PracticePointRecord {
    PracticePointRecord {
        position,
        text: format!("point {position}"),
        exhibit: exhibit.map(str::to_string),
    }
}

pub(super) fn record(tactic: Option<i16>, braid: Option<&str>) -> PracticeQuestionRecord {
    PracticeQuestionRecord {
        id: Uuid::nil(),
        scenario_id: Uuid::nil(),
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
        flag_note: None,
        deck_key: Some("g1".to_string()),
        kind: "cross".to_string(),
        follows_key: None,
        source_line: Some("the hearing, p. 34".to_string()),
        hidden_at: None,
        draft_by: None,
        // A fixed instant, not `Utc::now()`: the print sheets' "deck as of"
        // line reads the MAX of this, and a fixture that moved with the
        // clock would make any assertion about that line unwritable.
        updated_at: chrono::DateTime::from_timestamp(1_755_000_000, 0)
            .expect("a fixed, valid instant"),
    }
}

/// A fixed instant, so a payload's "today" means the same thing on every run.
pub(super) fn now() -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono::Utc
        .with_ymd_and_hms(2026, 8, 19, 10, 0, 0)
        .single()
        .expect("a real instant")
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
    let dto = question_dto(
        &s,
        &[],
        &[],
        &[],
        record(Some(5), Some("Barrage rows 1 · 2 · 5")),
    );

    assert_eq!(dto.tactic.as_deref(), Some("compound · braid"));
    assert!(dto.braid, "the pill must change, not only the tag");

    let plain = question_dto(&s, &[], &[], &[], record(Some(5), None));
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

    let line = last_session_line(
        &s.practice_wording,
        Some(&record),
        &s.practice_read.case_timezone,
    );
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
    let line = last_session_line(&s.practice_wording, None, &s.practice_read.case_timezone);

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
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-6".to_string(),
            title: "Too many attorneys".to_string(),
            deck: vec![],
            points: vec![],
            receipts: &[],
            last: None,
            statuses: &[],
            current: &[],
            open: None,
            badged: &[],
            notes: vec![],
            changed: None,
            attach_options: vec![],
        },
    );

    assert!(payload.questions.is_empty());
    assert_eq!(payload.code, "S-6");
    // No questions, so no date on which the deck last changed. `None` withdraws
    // the print sheets' "deck as of" line rather than inventing a day.
    assert_eq!(payload.deck_as_of, None);
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
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-5".to_string(),
            title: "Marie refused to divide property amicably".to_string(),
            deck: vec![record(Some(4), None)],
            points: vec![],
            receipts: &[],
            last: None,
            statuses: &[],
            current: &[],
            open: None,
            badged: &[],
            notes: vec![],
            changed: None,
            attach_options: vec![],
        },
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

/// The seeded receipt shows under a point nobody has paired yet.
///
/// Roman's ruling of 2026-08-17: the reveal prints "Backed by: your certified
/// letter, 16 Nov 2009" rather than a named absence, because the editor that
/// would author that pairing properly is v1 and Marie sits down on Tuesday.
#[test]
fn a_point_with_no_pairing_shows_the_seeded_receipt() {
    let s = settings();
    let payload = deck_payload(
        &s,
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-5".to_string(),
            title: "Marie refused to divide property amicably".to_string(),
            deck: vec![],
            points: vec![point(1, None), point(2, None)],
            receipts: &[
                PracticePointReceipt {
                    position: 1,
                    text: "your certified letter, 16 Nov 2009".to_string(),
                },
                PracticePointReceipt {
                    position: 2,
                    text: "CFS Interrogatory Response, p. 10".to_string(),
                },
            ],
            last: None,
            statuses: &[],
            current: &[],
            open: None,
            badged: &[],
            notes: vec![],
            changed: None,
            attach_options: vec![],
        },
    );

    assert_eq!(
        payload.points[0].exhibit.as_deref(),
        Some("your certified letter, 16 Nov 2009")
    );
    assert_eq!(
        payload.points[1].exhibit.as_deref(),
        Some("CFS Interrogatory Response, p. 10")
    );
}

/// A REAL pairing beats the seeded stand-in.
///
/// ## Why this precedence and not the other one
///
/// `response_item_fact_refs.note` is the record of which exhibit backs a point,
/// authored by a human in the v1 editor. The seeded receipt exists only because
/// that editor does not yet. If the stand-in won, the editor would ship and
/// change nothing on screen — and the deck row would be a second truth speaking
/// over a human's own words. This way v1 takes over by being used, with nothing
/// to migrate.
#[test]
fn a_real_pairing_supersedes_the_seeded_stand_in() {
    let s = settings();
    let payload = deck_payload(
        &s,
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-5".to_string(),
            title: "x".to_string(),
            deck: vec![],
            points: vec![point(1, Some("My certified letter"))],
            receipts: &[PracticePointReceipt {
                position: 1,
                text: "your certified letter, 16 Nov 2009".to_string(),
            }],
            last: None,
            statuses: &[],
            current: &[],
            open: None,
            badged: &[],
            notes: vec![],
            changed: None,
            attach_options: vec![],
        },
    );

    assert_eq!(
        payload.points[0].exhibit.as_deref(),
        Some("My certified letter")
    );
}

/// A point no receipt names still shows the named absence.
///
/// The honest-gap law survives the ruling: seeding SOME receipts must not make an
/// unseeded point silently borrow another point's.
#[test]
fn a_point_with_neither_still_names_its_absence() {
    let s = settings();
    let payload = deck_payload(
        &s,
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-5".to_string(),
            title: "x".to_string(),
            deck: vec![],
            points: vec![point(3, None)],
            receipts: &[PracticePointReceipt {
                position: 1,
                text: "your certified letter, 16 Nov 2009".to_string(),
            }],
            last: None,
            statuses: &[],
            current: &[],
            open: None,
            badged: &[],
            notes: vec![],
            changed: None,
            attach_options: vec![],
        },
    );

    assert_eq!(payload.points[0].exhibit, None);
    assert_eq!(
        s.practice_report_wording.point_no_receipt, "No receipt recorded for this point.",
        "and the screen has the sentence to print in its place"
    );
}

/// The deck's own date is the NEWEST change across it, not the first row's.
///
/// ## Why the print sheets need this, and why the MAX is the whole of it
///
/// Paper outlives the deck it was taken from. A sheet carrying no date cannot
/// tell a reader how stale it is, and a sheet carrying the FIRST row's date would
/// say a deck edited this morning was last touched in August. Every editor write
/// sets `updated_at = NOW()` on the row it touched, so the deck's date is the
/// maximum — and reading it from the deck already in hand costs no second query.
#[test]
fn the_deck_carries_the_date_of_its_newest_change() {
    let s = settings();
    let old = chrono::DateTime::from_timestamp(1_700_000_000, 0).expect("a valid instant");
    let newest = chrono::DateTime::from_timestamp(1_755_000_000, 0).expect("a valid instant");

    // Deliberately NOT in date order: `max` must find the newest wherever it
    // sits, and a deck is ordered by `sort_order`, never by when it was edited.
    let mut first = record(Some(5), None);
    first.updated_at = newest;
    let mut second = record(None, None);
    second.id = Uuid::from_u128(2);
    second.updated_at = old;

    let payload = deck_payload(
        &s,
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-5".to_string(),
            title: "Marie refused to divide the property".to_string(),
            deck: vec![first, second],
            points: vec![],
            receipts: &[],
            last: None,
            statuses: &[],
            current: &[],
            open: None,
            badged: &[],
            notes: vec![],
            changed: None,
            attach_options: vec![],
        },
    );

    assert_eq!(payload.deck_as_of, Some(newest));
    assert_ne!(
        payload.deck_as_of,
        Some(old),
        "the OLDEST change would tell Chuck a deck edited today is months stale"
    );
}

// -----------------------------------------------------------------------------
// `Answered on 22 Aug` — the one status a one-page deck row carries
// -----------------------------------------------------------------------------
//
// The failure this section exists to catch is the quiet one: a row that says
// nothing when there IS an answer behind it. Marie then re-answers a question she
// already answered, and Chuck prints a sheet that claims she answered none of
// them. Nothing crashes and no log records it.

/// One current-answer record, at a fixed instant.
fn current(question_id: Uuid, at: chrono::DateTime<Utc>) -> CurrentAnswerRecord {
    CurrentAnswerRecord {
        question_id,
        answer_text: "her words".to_string(),
        answered_at: at,
    }
}

/// 22 Aug 2026, 01:30 UTC — which is still 21 Aug in Michigan. The zone is the
/// point of the fixture, not decoration.
fn late_night_utc() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 22, 1, 30, 0)
        .single()
        .expect("a real instant")
}

#[test]
fn a_row_with_an_answer_says_when_it_was_answered() {
    let s = settings();
    let id = Uuid::from_u128(7);
    let mut rec = record(Some(5), None);
    rec.id = id;

    let dto = question_dto(
        &s,
        &[],
        &[current(
            id,
            Utc.with_ymd_and_hms(2026, 8, 22, 16, 0, 0)
                .single()
                .expect("an instant"),
        )],
        &[],
        rec,
    );

    let line = dto.answered_on.expect("an answered row carries the line");
    assert!(
        line.contains("22 Aug"),
        "the day must reach the row, got {line:?}"
    );
    assert!(
        !line.contains('{'),
        "a raw template slot reached the screen: {line:?}"
    );
}

#[test]
fn a_row_with_no_answer_says_nothing_at_all() {
    // NOT an empty string. An empty status line under a question reads as a
    // status that failed to load, which is a different fact from "not answered
    // yet" and the wrong one to show the person least able to diagnose it.
    let s = settings();
    let dto = question_dto(&s, &[], &[], &[], record(Some(5), None));

    assert_eq!(dto.answered_on, None);
}

#[test]
fn the_line_carries_no_weekday() {
    // Its siblings say `last: Wed 19 Aug`, where the weekday earns its place.
    // This one is on every row of a list, and Roman's mockup shows `22 Aug`.
    let s = settings();
    let id = Uuid::from_u128(7);
    let mut rec = record(Some(5), None);
    rec.id = id;

    let line = question_dto(&s, &[], &[current(id, late_night_utc())], &[], rec)
        .answered_on
        .expect("an answered row carries the line");

    for weekday in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] {
        assert!(
            !line.contains(weekday),
            "{weekday} reached a line that should carry none: {line:?}"
        );
    }
}

#[test]
fn the_day_is_the_case_s_day_and_not_utc_s() {
    // 22 Aug 01:30 UTC is 21 Aug 21:30 in Michigan. Marie practises in the
    // evening; a row composed in UTC would tell her she answered it TOMORROW.
    // This is the same defect `row_statuses` was fixed for, and it does not
    // announce itself — the line is well-formed and off by one day.
    let mut s = settings();
    s.practice_read.case_timezone = "America/Detroit".to_string();
    let id = Uuid::from_u128(7);
    let mut rec = record(Some(5), None);
    rec.id = id;

    let line = question_dto(&s, &[], &[current(id, late_night_utc())], &[], rec)
        .answered_on
        .expect("an answered row carries the line");

    assert!(
        line.contains("21 Aug"),
        "the case's own day must win, got {line:?}"
    );
}

#[test]
fn one_question_s_answer_never_lands_on_another_s_row() {
    // The match is by id, and this is the assertion that says so. A `first()`
    // where a `find()` belongs would stamp every row with the same date and
    // still pass every test above.
    let s = settings();
    let mine = Uuid::from_u128(7);
    let other = Uuid::from_u128(8);
    let mut rec = record(Some(5), None);
    rec.id = mine;

    let dto = question_dto(&s, &[], &[current(other, late_night_utc())], &[], rec);

    assert_eq!(
        dto.answered_on, None,
        "another question's answer reached this row"
    );
}
