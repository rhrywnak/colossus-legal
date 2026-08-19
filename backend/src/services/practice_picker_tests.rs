// Tests for the two things the deck payload ASSEMBLES rather than copies: the
// "I'd point to…" picker's list, and the composed status on a row.
//
// Split from `practice_page_tests` on 2026-08-19 when those carried it past Rule
// 17's limit. The seam is honest: the sibling tests what one deck row becomes on
// the wire, one field at a time, while everything here is about a value that
// exists only because the payload was built — neither the picker list nor a row's
// status is a column, and neither can be checked by reading one record.
//
// Both are asserted THROUGH `deck_payload` rather than against the private
// helpers, so what is pinned is what the browser actually receives.

use super::*;
use crate::repositories::pipeline_repository::practice::{
    PracticePointReceipt, PracticePointRecord,
};
use uuid::Uuid;

// The three fixtures the sibling already owns. `pub(super)` there reaches into
// `practice_page` and everything under it, which is exactly this module and
// stops there — the narrowest visibility that works, and the same one
// `practice_sql_shape` uses to lend its helpers to `practice_schema_tests`.
use super::tests::{now, record, settings};

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
            badged: &[],
            notes: vec![],
            changed: None,
            attach_options: vec![],
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
        answered_today: true,
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
            badged: &[],
            notes: vec![],
            changed: None,
            attach_options: vec![],
        },
    );

    assert_eq!(
        payload.questions[0].status.as_deref(),
        Some("answered today · repeat · attempt 2")
    );
    assert_eq!(
        payload.questions[0].status_mark.as_deref(),
        Some("repeat"),
        "the RAW mark rides beside the sentence, so the screen colours without \
         searching a template somebody can re-word"
    );
    assert!(
        payload.questions[1].status.is_none(),
        "a question nobody has answered renders NOTHING, not an empty line"
    );
    assert!(payload.questions[1].status_mark.is_none());
}

/// The review page's mapper drops the status and the badge, and keeps the rest.
///
/// `question_dto_for` exists precisely so ONE question on a page with no list
/// does not inherit two facts that are about a LIST: `answered today · repeat`
/// is the row's report on the start card, and `changed` asks her to re-read the
/// deck. Both would be noise beside the question they describe.
///
/// Everything else must survive: the review page renders the same pills, the
/// same tactic tag and the same redirect badge the drill does, and a second
/// mapper is exactly how those would drift apart.
#[test]
fn the_review_pages_mapper_drops_the_status_and_the_badge_and_keeps_the_rest() {
    let mut record = record(Some(4), Some("Barrage rows 1 · 2"));
    record.kind = "redirect".to_string();
    record.draft_by = Some("architect".to_string());

    let dto = question_dto_for(&settings(), record);

    assert!(
        dto.status.is_none(),
        "a list's report has no meaning on one question"
    );
    assert!(dto.status_mark.is_none());
    assert!(
        !dto.changed,
        "the re-read badge is about the list, not the question"
    );

    assert_eq!(dto.tactic.as_deref(), Some("false premise · braid"));
    assert!(dto.braid);
    assert_eq!(dto.kind, "redirect");
    assert_eq!(dto.draft_by.as_deref(), Some("architect"));
    assert!(!dto.hidden);
}

/// One point, with its receipt, for a caller outside the payload.
///
/// The precedence itself is proved through `deck_payload` above; this pins the
/// FUNCTION `practice_notes` imports, so the review page and the start card
/// cannot end up showing a point two different ways.
#[test]
fn the_point_mapper_carries_the_receipt_and_names_its_absence() {
    let backed = point_dto(
        PracticePointRecord {
            position: 1,
            text: "I asked in writing.".to_string(),
            exhibit: None,
        },
        &[seeded(1, "your certified letter, 16 Nov 2009")],
    );
    assert_eq!(backed.position, 1);
    assert_eq!(backed.text, "I asked in writing.");
    assert_eq!(
        backed.exhibit.as_deref(),
        Some("your certified letter, 16 Nov 2009")
    );

    // A point nobody paired and nobody seeded carries NONE — and the screen
    // then prints the stored named-absence line rather than a blank.
    let bare = point_dto(
        PracticePointRecord {
            position: 2,
            text: "They got my letter.".to_string(),
            exhibit: None,
        },
        &[],
    );
    assert!(bare.exhibit.is_none());
}
