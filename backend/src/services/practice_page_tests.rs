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

fn settings() -> Settings {
    Settings::for_test()
}

fn point(position: i32, exhibit: Option<&str>) -> PracticePointRecord {
    PracticePointRecord {
        position,
        text: format!("point {position}"),
        exhibit: exhibit.map(str::to_string),
    }
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
        flag_note: None,
        deck_key: Some("g1".to_string()),
        kind: "cross".to_string(),
        follows_key: None,
        source_line: Some("the hearing, p. 34".to_string()),
    }
}

/// A fixed instant, so a payload's "today" means the same thing on every run.
fn now() -> chrono::DateTime<chrono::Utc> {
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
        now(),
        &[],
        record(Some(5), Some("Barrage rows 1 · 2 · 5")),
    );

    assert_eq!(dto.tactic.as_deref(), Some("compound · braid"));
    assert!(dto.braid, "the pill must change, not only the tag");

    let plain = question_dto(&s, now(), &[], record(Some(5), None));
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
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-6".to_string(),
            title: "Too many attorneys".to_string(),
            deck: vec![],
            points: vec![],
            receipts: &[],
            last: None,
            statuses: &[],
            open: None,
            now: now(),
        },
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
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-5".to_string(),
            title: "Marie refused to divide property amicably".to_string(),
            deck: vec![record(Some(4), None)],
            points: vec![],
            receipts: &[],
            last: None,
            statuses: &[],
            open: None,
            now: now(),
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
            open: None,
            now: now(),
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
            open: None,
            now: now(),
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
            open: None,
            now: now(),
        },
    );

    assert_eq!(payload.points[0].exhibit, None);
    assert_eq!(
        s.practice_report_wording.point_no_receipt, "No receipt recorded for this point.",
        "and the screen has the sentence to print in its place"
    );
}

/// A deck row with a `source_line`, for the picker tests.
fn sourced(source_line: Option<&str>) -> PracticeQuestionRecord {
    let mut row = record(Some(4), None);
    row.source_line = source_line.map(str::to_string);
    row
}

/// One seeded point receipt.
fn seeded(position: i32, text: &str) -> PracticePointReceipt {
    PracticePointReceipt {
        position,
        text: text.to_string(),
    }
}

/// Build a payload with nothing but the two things the picker reads.
fn picker(deck: Vec<PracticeQuestionRecord>, receipts: &[PracticePointReceipt]) -> Vec<String> {
    deck_payload(
        &settings(),
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-5".to_string(),
            title: "an accusation".to_string(),
            deck,
            points: vec![],
            receipts,
            last: None,
            statuses: &[],
            open: None,
            now: now(),
        },
    )
    .receipts
}

/// Her POINTS' receipts come first, then the exhibits her questions stand on.
///
/// The order is the argument: the three receipts under her own points are what
/// her case rests on, and a picker that buried them below five hearing pages
/// would have her scrolling past her own letter to find it.
#[test]
fn the_picker_lists_her_points_receipts_before_the_questions_exhibits() {
    let list = picker(
        vec![
            sourced(Some("Hearing, 15 Dec 2009, p. 34")),
            sourced(Some("Court of Appeals, 12 Jan 2012, p. 11")),
        ],
        &[
            seeded(1, "your certified letter, 16 Nov 2009"),
            seeded(2, "CFS Interrogatory Response, p. 10"),
        ],
    );

    assert_eq!(
        list,
        vec![
            "your certified letter, 16 Nov 2009".to_string(),
            "CFS Interrogatory Response, p. 10".to_string(),
            "Hearing, 15 Dec 2009, p. 34".to_string(),
            "Court of Appeals, 12 Jan 2012, p. 11".to_string(),
        ]
    );
}

/// The same exhibit behind two questions is offered ONCE.
///
/// The S-5 case exactly: the hearing backs more than one question. A list
/// offering the same page twice is a list she stops reading.
#[test]
fn the_picker_offers_a_shared_exhibit_only_once() {
    let list = picker(
        vec![
            sourced(Some("Hearing, 15 Dec 2009, p. 34")),
            sourced(Some("Hearing, 15 Dec 2009, p. 34")),
            sourced(Some("Phillips Response to Discovery, p. 5")),
        ],
        &[],
    );

    assert_eq!(
        list,
        vec![
            "Hearing, 15 Dec 2009, p. 34".to_string(),
            "Phillips Response to Discovery, p. 5".to_string(),
        ]
    );
}

/// A question that stands on no document of its own contributes nothing.
///
/// Every Chuck question is in this state: it stands on her POINTS, which the
/// picker already lists from the seeded receipts. An entry for it would list the
/// same exhibit twice under two names.
#[test]
fn a_question_with_no_source_line_adds_nothing_to_the_picker() {
    let list = picker(
        vec![
            sourced(None),
            sourced(Some("Court of Appeals, p. 11")),
            sourced(None),
        ],
        &[],
    );

    assert_eq!(list, vec!["Court of Appeals, p. 11".to_string()]);
}

/// A scenario with neither receipts nor source lines offers an EMPTY list.
///
/// Not a failure, and not a placeholder: the control withdraws itself entirely
/// rather than opening onto an empty box, which would read as a list that failed
/// to load.
#[test]
fn a_scenario_with_nothing_to_point_at_yields_an_empty_picker() {
    assert!(picker(vec![sourced(None)], &[]).is_empty());
}

/// The status on a row is COMPOSED, and absent on a question nobody answered.
#[test]
fn a_row_carries_its_composed_status_or_none_at_all() {
    use crate::repositories::pipeline_repository::practice_flow::RowStatusRecord;

    let answered = record(Some(4), None);
    let untouched = {
        let mut row = record(Some(4), None);
        row.id = Uuid::from_u128(9);
        row
    };
    let statuses = vec![RowStatusRecord {
        question_id: answered.id,
        mark: "repeat".to_string(),
        answered_at: now(),
        attempts: 2,
    }];

    let payload = deck_payload(
        &settings(),
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-5".to_string(),
            title: "an accusation".to_string(),
            deck: vec![answered, untouched],
            points: vec![],
            receipts: &[],
            last: None,
            statuses: &statuses,
            open: None,
            now: now(),
        },
    );

    assert_eq!(
        payload.questions[0].status.as_deref(),
        Some("answered today · repeat · attempt 2")
    );
    assert!(
        payload.questions[1].status.is_none(),
        "a question nobody has answered renders NOTHING, not an empty line"
    );
}
