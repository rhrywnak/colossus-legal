// Tests for `domain::wording_practice_editor`.
//
// Same law as every sibling wording test file: a key declared to the boot loader
// with no row in the migration makes the backend REFUSE TO START, and reading
// the migration off disk is the only thing that catches it before a deploy takes
// DEV down (Rule 21).
//
// This block carries a second reason of its own. Several of these strings are
// the only thing telling the person editing that a change is SIGNED and VISIBLE
// ("Saved as a change by {who} — Marie sees a changed badge on this row"). A
// wrong one there does not merely mislead; it makes somebody edit a witness's
// deck believing nobody will know who did it.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds every row this module reads.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260819113610_practice_v1_part_b_deck_editor_notes_and_review.sql";

/// The nav cleanup added one row to this block: the deck editor's drag grip.
const NAV_MIGRATION: &str =
    "pipeline_migrations/20260819152958_nav_cleanup_scenario_header_buttons.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_NOTE_AUTHORS, "Chuck,Marie,Roman"),
    (KEY_EDITOR_AUTHORS, "Chuck,Roman"),
    (KEY_EDITOR_SWITCH_LABEL, "Edit the deck"),
    (KEY_EDITOR_DONE_LABEL, "Done editing"),
    (KEY_EDITOR_AS_LABEL, "Editing as"),
    (KEY_EDITOR_AS_UNSET, "Who is editing?"),
    (KEY_EDITOR_EDIT_LABEL, "Edit"),
    (KEY_EDITOR_HIDE_LABEL, "Hide"),
    (KEY_EDITOR_DRAG_HINT, "Drag to re-order within this side"),
    (KEY_EDITOR_UNHIDE_LABEL, "Unhide"),
    (KEY_EDITOR_HIDDEN_BADGE, "hidden"),
    (KEY_EDITOR_UP_LABEL, "Move up"),
    (KEY_EDITOR_DOWN_LABEL, "Move down"),
    (KEY_EDITOR_SAVE_LABEL, "Save"),
    (KEY_EDITOR_CANCEL_LABEL, "Cancel"),
    (
        KEY_EDITOR_SAVED_HINT_TEMPLATE,
        "Saved as a change by {who} — Marie sees a changed badge on this row.",
    ),
    (KEY_EDITOR_FIELD_QUESTION, "Question"),
    (KEY_EDITOR_FIELD_TACTIC, "Tactic"),
    (KEY_EDITOR_FIELD_FOLLOWS, "Follows (George question)"),
    (KEY_EDITOR_FIELD_WATCH_FOR, "Watch for"),
    (KEY_EDITOR_FIELD_STRONGER, "Stronger answer"),
    (KEY_EDITOR_FIELD_SIDE, "Side"),
    (KEY_EDITOR_FIELD_ATTACH, "Attach to"),
    (KEY_EDITOR_SIDE_CROSS, "George's side (cross)"),
    (KEY_EDITOR_SIDE_DIRECT, "Chuck (direct)"),
    (
        KEY_EDITOR_SIDE_REDIRECT,
        "Chuck (redirect — follows a George question)",
    ),
    (KEY_EDITOR_ATTACH_NONE, "no receipt"),
    (KEY_EDITOR_ATTACH_INSTANCE_TEMPLATE, "instance {n} — {text}"),
    (KEY_EDITOR_ATTACH_POINT_TEMPLATE, "point {n} — {text}"),
    (KEY_EDITOR_ADD_LABEL, "+ Add a question"),
    (KEY_EDITOR_ADD_HEADING, "Add a question"),
    (KEY_EDITOR_ADD_BUTTON, "Add"),
    (
        KEY_EDITOR_ADD_HINT,
        "A new question shows a changed badge to Marie until she has answered it once.",
    ),
    (
        KEY_EDITOR_QUESTION_PLACEHOLDER,
        "One sentence, in the voice of the side asking it.",
    ),
    (
        KEY_EDITOR_FAILED,
        "That change was not saved. Nothing on the deck has moved; try again.",
    ),
    (
        KEY_CHANGED_HEADING_TEMPLATE,
        "Changed since your last sitting: {n} questions — {who}, {when}",
    ),
    (KEY_CHANGED_NOTES_TEMPLATE, "{n} new notes — {who}"),
    (KEY_CHANGED_SUMMARY, "what changed"),
    (KEY_CHANGE_ADDED_TEMPLATE, "new: Q{n} ({side})"),
    (KEY_CHANGE_REWORDED_TEMPLATE, "Q{n} re-worded"),
    (KEY_CHANGE_EDITED_TEMPLATE, "Q{n} — {field} changed"),
    (KEY_CHANGE_MOVED_TEMPLATE, "Q{n} moved"),
    (KEY_CHANGE_HIDDEN_TEMPLATE, "Q{n} hidden"),
    (KEY_CHANGE_UNHIDDEN_TEMPLATE, "Q{n} put back"),
    (KEY_BADGE_CHANGED, "changed"),
    (KEY_BADGE_DRAFT, "draft — Chuck to edit"),
    (KEY_SHEET_CHANGES_HEADING, "Changed today"),
    (KEY_SHEET_CHANGE_ITEM_TEMPLATE, "{what} — {who}"),
];

impl PracticeEditorWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_practice_editor_wording::<String>(|key| {
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
        .expect("every key in PRACTICE_EDITOR_WORDING_KEYS is in TEST_SEED")
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
    // Two files: Part B seeded the block, and the nav cleanup added the drag
    // grip's hint when the deck gained drag re-ordering. Concatenated because
    // WHICH migration seeded a row is migration history — only the VALUE is what
    // this test pins.
    let nav = std::fs::read_to_string(root.join(NAV_MIGRATION))
        .expect("the nav cleanup migration is on disk");
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the Part B migration is on disk");

    let sql = format!("{sql}\n{nav}");

    for key in PRACTICE_EDITOR_WORDING_KEYS {
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
            PRACTICE_EDITOR_WORDING_KEYS.contains(key),
            "{key} is in TEST_SEED but declared to nothing — either the boot \
             loader stopped reading it or the fixture was never cleaned up"
        );
    }
    assert_eq!(
        TEST_SEED.len(),
        PRACTICE_EDITOR_WORDING_KEYS.len(),
        "the fixture and the declared list must be the same size"
    );
}
