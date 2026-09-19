// Tests for `domain::wording_practice_review`.
//
// Same law as every sibling wording test file: a key declared to the boot loader
// with no row in the migration makes the backend REFUSE TO START, and reading the
// migration off disk is the only thing that catches it before a deploy takes DEV
// down (Rule 21, the disk/code consistency pattern).
//
// This block carries a second reason of its own. Two of its rows are the only
// thing on screen that tells a reader WHERE A NOTE WILL LAND — on the answer that
// stands now, or on the question itself. Those two writes go to different rows,
// reach different readers and cannot be undone by re-typing; and the screens are
// otherwise identical. A blank or a wrong value there does not look broken, it
// looks like the other one.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds every row this module reads.
const SEED_MIGRATIONS: &[&str] =
    &["pipeline_migrations/20260919100133_review_page_reviewer_list_and_wording.sql"];

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_REVIEW_TITLE, "Review answers"),
    (
        KEY_REVIEW_UNANSWERED,
        "Not answered yet \u{2014} a note here lands on the question and Marie sees it before she answers.",
    ),
    (
        KEY_REVIEW_NOTE_PLACEHOLDER_ANSWER,
        "Add a note on this answer\u{2026}",
    ),
    (
        KEY_REVIEW_NOTE_PLACEHOLDER_QUESTION,
        "Add a note on this question\u{2026}",
    ),
    (KEY_REVIEW_ANSWERED_TEMPLATE, "Answered {when} \u{b7} {author}"),
    (KEY_REVIEW_AUTHOR_UNKNOWN, "author not recorded"),
    (
        KEY_REVIEW_LOAD_FAILED,
        "The answers for this deck could not be loaded.",
    ),
    (
        KEY_REVIEW_EMPTY_DECK,
        "This scenario has no questions yet, so there is nothing to review.",
    ),
    (KEY_REVIEW_NAME_JOINER, "\u{b7}"),
];

impl PracticeReviewWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_practice_review_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in PRACTICE_REVIEW_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

/// Read every named migration off disk, failing loudly if one has moved.
fn read_all(files: &[&str]) -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    files
        .iter()
        .map(|file| {
            std::fs::read_to_string(root.join(file))
                .unwrap_or_else(|cause| panic!("{file} is not on disk: {cause}"))
        })
        .collect()
}

#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let sources = read_all(SEED_MIGRATIONS);

    for key in PRACTICE_REVIEW_WORDING_KEYS {
        let seeded = sources
            .iter()
            .find_map(|sql| seeded_value_in(sql, key))
            .unwrap_or_else(|| {
                panic!(
                    "{key} is declared to the boot loader but seeded by no migration \
                     — the backend would refuse to start"
                )
            });
        let expected = TEST_SEED
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| (*v).to_string())
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));
        assert_eq!(
            seeded, expected,
            "the migration and the fixture disagree about {key}"
        );
    }
}

/// The fixture holds nothing the boot loader does not read.
///
/// ANTI-VACUITY, and not a formality: the test above walks
/// `PRACTICE_REVIEW_WORDING_KEYS`, so a fixture entry for a key nobody declares
/// would never be visited. Without this, a key removed from the struct but left
/// in the fixture would look tested forever.
#[test]
fn the_fixture_declares_no_key_the_build_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            PRACTICE_REVIEW_WORDING_KEYS.contains(key),
            "{key} is in TEST_SEED but declared nowhere — it is tested by nothing"
        );
    }
}

/// The one template carries both of its placeholders.
///
/// A paste that dropped `{author}` would render "Answered 14 Sep" under every
/// answer and lose the fact this page was built to show: WHO wrote it. The
/// string stays well-typed either way, which is why this is a test and not a
/// compile error.
#[test]
fn the_answered_template_carries_when_and_author() {
    let w = PracticeReviewWording::for_test();
    for placeholder in ["{when}", "{author}"] {
        assert!(
            w.answered_template.contains(placeholder),
            "answered_template lost {placeholder}: {}",
            w.answered_template
        );
    }
}

/// The plain sentences carry no placeholder.
///
/// The mirror image of the test above, and the one that catches a paste: a
/// `{when}` left in `unanswered` ships a raw brace to Chuck's screen, which this
/// repo has done before and which nothing in the build can warn about.
#[test]
fn the_plain_sentences_carry_no_placeholder() {
    let w = PracticeReviewWording::for_test();
    for (name, value) in [
        ("title", &w.title),
        ("unanswered", &w.unanswered),
        ("note_placeholder_answer", &w.note_placeholder_answer),
        ("note_placeholder_question", &w.note_placeholder_question),
        ("author_unknown", &w.author_unknown),
        ("load_failed", &w.load_failed),
        ("empty_deck", &w.empty_deck),
        ("name_joiner", &w.name_joiner),
    ] {
        assert!(
            !value.contains('{'),
            "{name} carries a placeholder: {value}"
        );
    }
}

/// The two note placeholders say which THING the note lands on.
///
/// Not a style check: these two boxes write to two different rows — one to the
/// answer that stands now, one to the question — and the wording is the only
/// warning a person gets before they type. Two rows reading the same words would
/// be a silent failure of exactly the kind Standing Rule 1 forbids.
#[test]
fn the_two_note_placeholders_differ_and_name_their_target() {
    let w = PracticeReviewWording::for_test();
    assert_ne!(
        w.note_placeholder_answer, w.note_placeholder_question,
        "the two note boxes write to different rows and must not read alike"
    );
    assert!(
        w.note_placeholder_answer.contains("answer"),
        "the answer box does not say 'answer': {}",
        w.note_placeholder_answer
    );
    assert!(
        w.note_placeholder_question.contains("question"),
        "the question box does not say 'question': {}",
        w.note_placeholder_question
    );
}

/// The joiner carries no space of its own.
///
/// The store TRIMS every value (`settings_store::text_of`), so a joiner seeded
/// as `" · "` arrives as `"·"` and the names run together. The server supplies
/// the spaces; this pins the row to that contract rather than leaving it to be
/// rediscovered when two reviewers' names first print.
#[test]
fn the_name_joiner_is_stored_without_its_spaces() {
    let w = PracticeReviewWording::for_test();
    assert_eq!(w.name_joiner.trim(), w.name_joiner);
    assert!(!w.name_joiner.is_empty());
}
