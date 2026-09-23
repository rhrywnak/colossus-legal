// Tests for `domain::wording_practice_row`.
//
// Same shape, and the same single justification, as every sibling wording test
// file: a key declared to the boot loader with no row in the migration makes the
// backend REFUSE TO START. That is a deploy taking DEV down, and reading the
// migration off disk is the only thing that catches it before it happens
// (Rule 21, the disk/code consistency pattern).
//
// This block carries a second reason of its own. Two of its rows are the only
// thing on screen that distinguishes states a witness would otherwise read as
// the same: `skipped today` versus `answered today · fine` on a deck row, and
// `Tell it — this is Chuck's time.` versus the drill's ordinary "no receipt for
// this one" line. A wrong value there is not a cosmetic slip — it tells her the
// opposite of what happened.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// Every migration that seeds a row of this block, oldest first.
///
/// ## Why a LIST and not two named constants
///
/// A block's rows arrive over time — the v1 seed, the attribution hotfix that
/// evening, and now the one-page work. Reading only the ORIGINAL seed makes
/// every later key look un-seeded and fails this test for a row that is on disk
/// two files along. It was two named constants and an `.or_else` chain; the
/// third addition is where that shape stops paying, because the chain has to be
/// edited in two places and one of them is easy to miss.
///
/// Order matters only for a key seeded twice — the FIRST file that carries it
/// wins, which is the oldest. No key here is seeded twice today; if one ever is,
/// the value that must win is the newest, and this becomes a `rev()` plus a
/// comment saying why. (The `wording_practice` block already learned that lesson
/// the hard way: its `corrected_value_in` searched forwards and returned the
/// FIRST of two corrections.)
const SEED_MIGRATIONS: &[&str] = &[
    "pipeline_migrations/20260819100411_practice_v1_chuck_review_deck_keys_kinds_and_points_to.sql",
    // The hints the attribution / case-timezone hotfix added, 2026-08-19.
    "pipeline_migrations/20260819135156_practice_hotfix_attribution_from_login_and_case_timezone.sql",
    // `practice_row_answered_on_template` — the one status a one-page row carries.
    "pipeline_migrations/20260823123657_practice_one_page_l1_answered_on.sql",
    // The review bar and the notes (CC_TASK_REVIEW_LOOP_v1).
    "pipeline_migrations/20260917080207_review_loop_cursor_and_wording.sql",
    // SIMPLE_COUNTS: the bar's owned-count pair.
    "pipeline_migrations/20260917104454_simple_counts_reviewer_and_summary_wording.sql",
    // REVIEW_PAGE: the bar's oldest-waiting clause (ruling STOP-A).
    "pipeline_migrations/20260919100133_review_page_reviewer_list_and_wording.sql",
    // REVIEW_COUNTS_HONEST: the confirmation Done reviewing now asks first.
    "pipeline_migrations/20260920161341_practice_witness_row_and_done_confirm_wording.sql",
    // PRACTICE_FIXES_v2.2.1: the answer box says it is saved.
    "pipeline_migrations/20260922072151_practice_fixes_v2_2_1.sql",
    // FOR_YOU L1: the one line the question page shows when the read-clear the
    // list asked for did not land.
    "pipeline_migrations/20260922165053_for_you_page_wording.sql",
];

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_PRACTICE_THIS_LABEL, "Practice this one ▸"),
    (KEY_ANSWERED_TODAY_TEMPLATE, "answered today · {mark}"),
    (KEY_SKIPPED_TODAY, "skipped today"),
    (KEY_EARLIER_TEMPLATE, "last: {when} · {mark}"),
    (KEY_ATTEMPT_SUFFIX_TEMPLATE, "· attempt {n}"),
    (KEY_REDIRECT_TAG, "redirect"),
    (
        KEY_REDIRECT_STRONGER_LINE,
        "Tell it — this is Chuck's time.",
    ),
    (KEY_POINTS_TO_LABEL, "I'd point to…"),
    (KEY_POINTS_TO_DONE_LABEL, "Done"),
    (KEY_POINTS_TO_REVEAL_PREFIX, "You'd point to:"),
    (KEY_POINTS_TO_SHEET_PREFIX, "would point to:"),
    (KEY_UNFINISHED_TODAY_WORD, "today"),
    (KEY_ANSWER_EMPTY_HINT, "Type your answer first \u{2014} or press \"I don't recall.\""),
    (KEY_ANSWER_ALREADY_RECORDED, "That question is already answered in this sitting \u{2014} this tab is behind. Reload to see it."),
    // The one status a one-page deck row carries. `{when}` is filled by
    // `practice_clock::local_day_month` — no weekday, deliberately.
    (KEY_ANSWERED_ON_TEMPLATE, "Answered on {when}"),
    // `{when}` here is `practice_clock::local_stamp` — the day AND the time.
    (KEY_ANSWER_SAVED_TEMPLATE, "Your answer \u{2014} saved {when}"),
    (
        KEY_ANSWER_SAVED_OFF_LINE,
        "Saved. Answer analysis is off, so no analysis was requested.",
    ),
    // As SEEDED. This file pins what the seeding migration INSERTs; the defect
    // sweep CORRECTED both to "questions" (an unstruck note puts a question in
    // the queue too), and the corrected value is pinned by the correction pass
    // in `services::settings_store_tests`.
    (
        KEY_DECK_REVIEW_AWAITING_TEMPLATE,
        "{count} answers awaiting {reviewer}'s review",
    ),
    (
        KEY_DECK_REVIEW_AWAITING_ONE,
        "{count} answer awaiting {reviewer}'s review",
    ),
    (
        KEY_DECK_REVIEW_OLDEST_TEMPLATE,
        "\u{b7} oldest waiting since {date}",
    ),
    (KEY_DECK_REVIEW_DONE_LABEL, "Done reviewing"),
    (KEY_DECK_REVIEW_FAILED, "Could not mark this deck reviewed \u{2014} nothing was changed."),
    (
        KEY_DECK_REVIEW_CONFIRM_TEMPLATE,
        "Mark all {count} answers in {code} as reviewed?",
    ),
    (
        KEY_DECK_REVIEW_CONFIRM_ONE,
        "Mark the {count} answer in {code} as reviewed?",
    ),
    (KEY_DECK_REVIEW_CONFIRM_YES_LABEL, "Yes"),
    (KEY_DECK_REVIEW_CONFIRM_CANCEL_LABEL, "Cancel"),
    (KEY_NOTE_ADD_LABEL, "Add a note"),
    (KEY_NOTE_SAVE_LABEL, "Save note"),
    (KEY_NOTE_CANCEL_LABEL, "Cancel"),
    (KEY_NOTE_STRIKE_LABEL, "Strike"),
    (KEY_NOTE_STRUCK_TEMPLATE, "struck {when}"),
    (KEY_NOTE_FAILED, "The note could not be saved \u{2014} nothing was written."),
    (
        KEY_ROW_READ_FAILED,
        "This question could not be marked as read \u{2014} it is still on your For you list.",
    ),
];

impl PracticeRowWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_practice_row_wording::<String>(|key| {
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
        .expect("every key in PRACTICE_ROW_WORDING_KEYS is in TEST_SEED")
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
    let sources: Vec<String> = SEED_MIGRATIONS
        .iter()
        .map(|file| {
            std::fs::read_to_string(root.join(file))
                .unwrap_or_else(|cause| panic!("{file} is not on disk: {cause}"))
        })
        .collect();

    for key in PRACTICE_ROW_WORDING_KEYS {
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
            .unwrap_or_else(|| {
                panic!(
                    "{key} is declared to the boot loader but missing from this file's \
                     TEST_SEED fixture — add the value the migration seeds"
                )
            });
        assert_eq!(
            seeded, expected,
            "the migration and the fixture disagree about {key}"
        );
    }
}

/// The fixture holds nothing the boot loader does not read.
///
/// ANTI-VACUITY: the test above walks `PRACTICE_ROW_WORDING_KEYS`, so a key
/// dropped from that list stops being checked by it and nothing complains. This
/// one fails instead, naming the orphan.
#[test]
fn the_fixture_holds_no_key_the_boot_loader_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            PRACTICE_ROW_WORDING_KEYS.contains(key),
            "{key} is in TEST_SEED but declared to nothing — either the boot \
             loader stopped reading it or the fixture was never cleaned up"
        );
    }
    assert_eq!(
        TEST_SEED.len(),
        PRACTICE_ROW_WORDING_KEYS.len(),
        "the fixture and the declared list must be the same size"
    );
}

/// The templates carry the placeholders their callers fill.
///
/// A status template that lost `{mark}` renders `answered today ·` with nothing
/// after it — not a crash, not a failure anywhere else, and exactly the kind of
/// small wrongness a witness stops trusting a screen over.
#[test]
fn every_template_carries_its_placeholders() {
    let w = PracticeRowWording::for_test();
    for (name, value, needed) in [
        (
            "answered_today_template",
            &w.answered_today_template,
            vec!["{mark}"],
        ),
        (
            "earlier_template",
            &w.earlier_template,
            vec!["{when}", "{mark}"],
        ),
        (
            "attempt_suffix_template",
            &w.attempt_suffix_template,
            vec!["{n}"],
        ),
        (
            "deck_review_awaiting_template",
            &w.deck_review_awaiting_template,
            vec!["{count}", "{reviewer}"],
        ),
        (
            "deck_review_awaiting_one",
            &w.deck_review_awaiting_one,
            vec!["{count}", "{reviewer}"],
        ),
        (
            "deck_review_oldest_template",
            &w.deck_review_oldest_template,
            vec!["{date}"],
        ),
        (
            "deck_review_confirm_template",
            &w.deck_review_confirm_template,
            vec!["{count}", "{code}"],
        ),
        (
            "deck_review_confirm_one",
            &w.deck_review_confirm_one,
            vec!["{count}", "{code}"],
        ),
        (
            "note_struck_template",
            &w.note_struck_template,
            vec!["{when}"],
        ),
    ] {
        for placeholder in needed {
            assert!(
                value.contains(placeholder),
                "{name} lost {placeholder}: {value}"
            );
        }
    }
}

/// The labels that are NOT templates carry no placeholder.
///
/// The mirror image of the test above, and the one that catches a paste: a
/// `{mark}` left in `skipped_today` would ship a raw brace to Marie's screen,
/// which this repo has done before with `{when}` and which nothing in the build
/// can warn about because the string is well-typed either way.
#[test]
fn the_plain_labels_carry_no_placeholder() {
    let w = PracticeRowWording::for_test();
    for (name, value) in [
        ("practice_this_label", &w.practice_this_label),
        ("skipped_today", &w.skipped_today),
        ("redirect_tag", &w.redirect_tag),
        ("redirect_stronger_line", &w.redirect_stronger_line),
        ("points_to_label", &w.points_to_label),
        ("points_to_done_label", &w.points_to_done_label),
        ("points_to_reveal_prefix", &w.points_to_reveal_prefix),
        ("points_to_sheet_prefix", &w.points_to_sheet_prefix),
        ("unfinished_today_word", &w.unfinished_today_word),
        ("deck_review_done_label", &w.deck_review_done_label),
        (
            "deck_review_confirm_yes_label",
            &w.deck_review_confirm_yes_label,
        ),
        (
            "deck_review_confirm_cancel_label",
            &w.deck_review_confirm_cancel_label,
        ),
        ("deck_review_failed", &w.deck_review_failed),
        ("note_add_label", &w.note_add_label),
        ("note_save_label", &w.note_save_label),
        ("note_cancel_label", &w.note_cancel_label),
        ("note_strike_label", &w.note_strike_label),
        ("note_failed", &w.note_failed),
    ] {
        assert!(
            !value.contains('{'),
            "{name} is a plain label and must carry no placeholder: {value}"
        );
    }
}

/// The defect sweep CORRECTED both review sentences to count questions. (M)
///
/// ## Why this needs a test of its own
///
/// `every_declared_key_is_seeded_with_the_value_this_build_expects` above pins
/// what the SEEDING migration inserts, and the correction pass in
/// `services::settings_store_tests` walks parameters rather than wording — so
/// between them, nothing in `cargo test --lib` would notice these two sentences
/// going back to "answers". The migration's own rule-25a assertion catches it at
/// deploy time, which is later than it needs to be caught.
///
/// The count includes questions waiting on an unstruck NOTE as well as on an
/// answer (`review_cursor::NOTE_WAITING`). A bar that says "answers" over that
/// number is wrong on exactly the rows the defect sweep existed to surface.
#[test]
fn the_review_bar_counts_questions_rather_than_answers() {
    let sql = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "pipeline_migrations/20260917153315_defect_sweep_sittings_deck_keys_and_review_wording.sql",
    ))
    .expect("the defect sweep migration is on disk");

    for (key, expected) in [
        (
            KEY_DECK_REVIEW_AWAITING_TEMPLATE,
            "{count} questions awaiting {reviewer}'s review",
        ),
        (
            KEY_DECK_REVIEW_AWAITING_ONE,
            "{count} question awaiting {reviewer}'s review",
        ),
    ] {
        let corrected = crate::domain::wording::tests::corrected_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is not corrected by the defect sweep migration"));
        assert_eq!(corrected, expected, "{key}");
        assert!(
            !corrected.contains("answer"),
            "{key} still names answers over a count that includes notes: {corrected}"
        );
    }
}
