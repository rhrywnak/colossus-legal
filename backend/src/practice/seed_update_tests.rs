// Tests for `practice::seed_update`.
//
// Every function tested here is pure: the plan is decided before a transaction
// opens, which is what lets the refusals below be unit tests rather than a
// deployment. What they guard is the one thing this path must never do —
// silently point a file's edit at the wrong stored question.

use super::*;
use crate::practice::deck_file::{DeckQuestion, DeckSide, DeckSourceKind};

fn question(key: Option<&str>, text: &str) -> DeckQuestion {
    DeckQuestion {
        key: key.map(str::to_string),
        side: DeckSide::George,
        kind: None,
        follows: None,
        source_line: None,
        draft_by: None,
        source_kind: DeckSourceKind::Manual,
        source_index: None,
        tactic: None,
        braid_rows: None,
        text: text.to_string(),
        receipt: None,
        pair_said: None,
        pair_admitted: None,
        watch_for: None,
        stronger: None,
        stronger_lean: None,
    }
}

fn deck(questions: Vec<DeckQuestion>) -> DeckFile {
    DeckFile {
        scenario_code: "S-5".to_string(),
        points: vec![],
        questions,
    }
}

fn stored(deck_key: Option<&str>, text: &str) -> StoredQuestion {
    StoredQuestion {
        id: Uuid::nil(),
        deck_key: deck_key.map(str::to_string),
        text: text.to_string(),
    }
}

/// A file with no keys cannot drive `--update`, and says so by position.
///
/// The refusal is the whole feature: matching on text is what this path exists
/// to stop doing, and a file with no keys would leave it nothing else to match
/// on. The position is what an operator needs to fix it.
#[test]
fn a_file_question_without_a_key_refuses_the_run_by_position() {
    let file = deck(vec![
        question(Some("g1"), "first"),
        question(None, "second"),
    ]);
    let error = file_keys(&file).expect_err("a keyless question must refuse");
    assert!(
        matches!(error, UpdateError::FileQuestionHasNoKey { position: 2 }),
        "{error}"
    );
}

/// The one-time pass gives each pre-key stored row the key of the file question
/// with its exact text.
#[test]
fn an_unkeyed_stored_row_takes_the_key_of_the_question_with_its_text() {
    let file = deck(vec![
        question(Some("g1"), "first"),
        question(Some("g2"), "second"),
    ]);
    let keys = file_keys(&file).expect("both questions are keyed");
    let rows = vec![stored(None, "second"), stored(Some("g1"), "first")];

    let assigned = match_unkeyed(&rows, &file, &keys).expect("the text matches exactly");
    assert_eq!(assigned.len(), 1, "only the un-keyed row needs a key");
    assert_eq!(assigned[0].1, "g2");
}

/// A stored row whose text is in no file question refuses the WHOLE run.
///
/// Task A4 in its own words: "refuse if any existing row cannot be matched". A
/// partial keying would leave the deck in a state where a second run behaved
/// differently, which is the worst thing a re-runnable tool can do.
#[test]
fn a_stored_row_the_file_no_longer_contains_refuses_the_whole_run() {
    let file = deck(vec![question(Some("g1"), "first")]);
    let keys = file_keys(&file).expect("keyed");
    let rows = vec![stored(None, "a question nobody kept")];

    let error = match_unkeyed(&rows, &file, &keys).expect_err("an unmatched row must refuse");
    assert!(
        matches!(error, UpdateError::StoredRowUnmatched { ref text } if text.contains("nobody kept")),
        "{error}"
    );
}

/// Two file questions with the same text make the match a coin toss, so it
/// refuses rather than picking one.
#[test]
fn two_questions_with_the_same_text_refuse_rather_than_guess() {
    let file = deck(vec![
        question(Some("g1"), "the same words"),
        question(Some("g2"), "the same words"),
    ]);
    let keys = file_keys(&file).expect("keyed");
    let rows = vec![stored(None, "the same words")];

    let error = match_unkeyed(&rows, &file, &keys).expect_err("an ambiguous text must refuse");
    assert!(
        matches!(error, UpdateError::AmbiguousText { .. }),
        "{error}"
    );
}

/// Whitespace around a stored text does not stop it matching.
///
/// The seed TRIMS on the way in, and a YAML block scalar can leave a trailing
/// newline. A match that failed on that would refuse a run for a difference
/// nobody can see in an editor.
#[test]
fn the_text_match_ignores_surrounding_whitespace() {
    let file = deck(vec![question(Some("g1"), "  first  ")]);
    let keys = file_keys(&file).expect("keyed");
    let rows = vec![stored(None, "first\n")];

    let assigned = match_unkeyed(&rows, &file, &keys).expect("trimmed texts match");
    assert_eq!(assigned[0].1, "g1");
}

/// The plan splits the file's keys into updates and inserts, and names what the
/// file no longer mentions.
///
/// The whole of `--update`'s decision, and it touches no database — which is why
/// it is a unit test rather than a run against DEV.
#[test]
fn the_plan_separates_updates_inserts_and_the_rows_left_alone() {
    let stored = Uuid::nil();
    let key_of = vec![(stored, "g1".to_string()), (stored, "g9".to_string())];
    let keys = vec!["g1", "r1"];

    let report = plan("S-5", stored, &keys, &key_of, &[]);

    assert_eq!(
        report.updated,
        vec!["g1".to_string()],
        "g1 is already stored"
    );
    assert_eq!(report.inserted, vec!["r1".to_string()], "r1 is new");
    assert_eq!(
        report.untouched,
        vec!["g9".to_string()],
        "a stored row the file no longer names is LEFT, and listed"
    );
    assert!(!report.written, "planning writes nothing");
}

/// The rendered report names what was updated, what was inserted and what was
/// left alone — and says which of the three it did nothing to.
///
/// "(none)" rather than a blank line: an empty value in a proof reads as a
/// number that failed to print, which is the one thing a count proof may not do.
#[test]
fn the_report_names_all_three_lists_even_when_empty() {
    let rendered = render_update_report(&UpdateReport {
        scenario_code: "S-5".to_string(),
        scenario_id: Uuid::nil(),
        keyed_by_text: vec![(
            "g1".to_string(),
            "Isn't it true that you and your sisters were".to_string(),
        )],
        updated: vec!["g1".to_string()],
        inserted: vec!["r1".to_string()],
        untouched: vec![],
        written: false,
    });
    assert!(rendered.contains("keyed by text"), "{rendered}");
    assert!(rendered.contains("g1 ←"), "{rendered}");
    assert!(rendered.contains("updated"), "{rendered}");
    assert!(rendered.contains("inserted"), "{rendered}");
    assert!(
        rendered.contains("left as they are      (none)"),
        "{rendered}"
    );
    assert!(rendered.contains("DRY RUN"), "{rendered}");
}
