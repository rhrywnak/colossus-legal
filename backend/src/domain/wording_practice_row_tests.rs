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

/// The migration that seeds every row this module reads.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260819100411_practice_v1_chuck_review_deck_keys_kinds_and_points_to.sql";

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
    let sql =
        std::fs::read_to_string(root.join(SEED_MIGRATION)).expect("the v1 migration is on disk");

    for key in PRACTICE_ROW_WORDING_KEYS {
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
    ] {
        assert!(
            !value.contains('{'),
            "{name} is a plain label and must carry no placeholder: {value}"
        );
    }
}
