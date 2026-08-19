// Tests for `domain::wording_practice_review`.
//
// Same law as every sibling wording test file — a declared key with no migration
// row is a backend that refuses to start (Rule 21).
//
// The second reason here is the notes panel. Two of its rows are the whole of
// why anybody writes honestly in it: who reads a note, and that a note cannot be
// made to disappear. A wrong value there changes what people are willing to say
// to each other about a witness's testimony.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds every row this module reads.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260819113610_practice_v1_part_b_deck_editor_notes_and_review.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_NOTES_HEADING_TEMPLATE, "Notes ({n})"),
    (KEY_NOTES_SCENARIO_TITLE, "Notes on this scenario"),
    (KEY_NOTES_QUESTION_TITLE, "Notes on this question"),
    (
        KEY_NOTES_HINT,
        "Chuck, Marie and Roman see all of these. Nothing is deleted; a note can be struck.",
    ),
    (KEY_NOTES_PLACEHOLDER, "Add a note…"),
    (KEY_NOTES_ATTEMPT_PLACEHOLDER, "Add a note on this attempt…"),
    (KEY_NOTES_SAVE_LABEL, "Save"),
    (KEY_NOTES_STRIKE_LABEL, "Strike"),
    (KEY_NOTES_STRUCK_TEMPLATE, "struck {when}"),
    (KEY_NOTES_EMPTY, "No notes on this yet."),
    (
        KEY_NOTES_FAILED,
        "That note was not saved. Nothing was written; try again.",
    ),
    (KEY_NOTES_AUTHOR_UNSET, "Who is writing?"),
    (KEY_ROW_REVIEW_LINK, "review"),
    (KEY_REVIEW_PROGRESS_TEMPLATE, "Question {n} · review"),
    (KEY_REVIEW_ATTEMPTS_KICKER, "Your attempts — newest first"),
    (KEY_REVIEW_ATTEMPT_TEMPLATE, "attempt {n} · {when}"),
    (KEY_REVIEW_DETAIL_TEMPLATE, "help: {help} · boxes: {boxes}"),
    (KEY_REVIEW_BOXES_NONE, "none ticked"),
    (
        KEY_REVIEW_NO_ATTEMPTS,
        "You have not answered this question yet.",
    ),
    (KEY_REVIEW_PRACTICE_AGAIN, "Practice this one again ▸"),
    (KEY_REVIEW_STRONGER_HEADING, "A stronger answer"),
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
                .ok_or_else(|| {
                    format!(
                        "{key} is declared to the boot loader but missing from this file's \
                         TEST_SEED fixture — add the value the migration seeds"
                    )
                })
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

/// Every declared key is seeded by the migration, with the value this build
/// expects.
///
/// The equality half is what makes this more than an existence check: a row
/// whose wording someone edited in the migration without editing the fixture
/// would pass a "the key is present" test and ship a screen saying something
/// this build never read.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the Part B migration is on disk");

    for key in PRACTICE_REVIEW_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key).unwrap_or_else(|| {
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
/// ANTI-VACUITY: the test above walks the declared list, so a key dropped from
/// it stops being checked and nothing complains. This one fails instead, naming
/// the orphan.
#[test]
fn the_fixture_holds_no_key_the_boot_loader_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            PRACTICE_REVIEW_WORDING_KEYS.contains(key),
            "{key} is in TEST_SEED but declared to nothing — either the boot \
             loader stopped reading it or the fixture was never cleaned up"
        );
    }
    assert_eq!(
        TEST_SEED.len(),
        PRACTICE_REVIEW_WORDING_KEYS.len(),
        "the fixture and the declared list must be the same size"
    );
}
