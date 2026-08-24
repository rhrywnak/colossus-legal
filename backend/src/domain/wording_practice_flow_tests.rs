// Tests for `domain::wording_practice_flow`.
//
// Same shape, and the same single justification, as every sibling wording test
// file: a key declared to the boot loader with no row in the migration makes the
// backend REFUSE TO START. That is a deploy taking DEV down, and reading the
// migration off disk is the only thing that catches it before it happens
// (Rule 21, the disk/code consistency pattern).
//
// This block carries the same second reason its two siblings do: Marie reads
// these sentences alone, and a wrong one is a witness being coached by a typo.
// It carries a third of its own — several of these strings are the only thing
// telling her that an exit is SAFE ("your answers so far are kept", "Starting
// over keeps what you answered on Chuck's sheet"). A wrong one there does not
// merely mislead; it stops her using a control that would have helped.

use super::*;
use crate::domain::wording::tests::{corrected_value_in, seeded_value_in};
use std::collections::HashMap;

/// The migration that seeds every row this module reads.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260818093139_practice_flow_v1_deck_controls_and_session_queue.sql";

/// The hotfix that seeds the rows added on 2026-08-19 evening.
///
/// A block's rows can arrive in more than one migration — this one carries the
/// hints the attribution/edit-mode hotfix added. Reading only the ORIGINAL seed
/// would make every new key look un-seeded and fail this test for a row that is
/// on disk two files along.
const HOTFIX_MIGRATION: &str =
    "pipeline_migrations/20260819135156_practice_hotfix_attribution_from_login_and_case_timezone.sql";

/// Migrations that CORRECT a value this block's seed already wrote.
///
/// ## ⚑ Why this exists, and what its absence was doing
///
/// It did not exist until 2026-08-23, and the parity test below read the seed
/// migration ALONE. A row corrected by a later `UPDATE` therefore kept its
/// ORIGINAL value in the fixture, the test went green, and the live store held
/// something else — the exact drift this file exists to catch, reported as a
/// pass. The base block (`wording_practice`) had learned this on 08-19 with
/// `scenario_practice_link_label`; three of its siblings had not.
///
/// `corrected_value_in` uses `rfind`, so a key corrected TWICE ends up pinned to
/// the LAST correction, which is what the store actually holds.
const CORRECTION_MIGRATIONS: &[&str] =
    &["pipeline_migrations/20260823134349_practice_one_page_l2_list_and_print_answers.sql"];

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_DECK_HEADING, "The questions"),
    (KEY_DECK_COUNT_TEMPLATE, "· {george} from the defense · {chuck} from Chuck"),
    (KEY_DECK_SKIPPED_SUFFIX_TEMPLATE, "· {k} skipped today"),
    (KEY_DECK_HIDE_LINK, "Hide the questions"),
    (KEY_DECK_SHOW_LINK, "Show the questions"),
    (KEY_DECK_INSTRUCTION_TEMPLATE, "Read them first. If one doesn't fit, {skip} keeps it out of this sitting; {flag} tells Roman and Chuck what's wrong with it — it stays in the deck until they change it."),
    (KEY_SKIP_TODAY_LABEL, "Skip today"),
    (KEY_SKIPPED_TODAY_LABEL, "Skipped today ✓"),
    (KEY_FLAG_LABEL, "Flag"),
    (KEY_FLAG_EDIT_LABEL, "Edit flag"),
    (KEY_FLAG_PLACEHOLDER, "What's wrong with it? One line — Roman and Chuck read this."),
    (KEY_FLAG_SAVE_LABEL, "Save flag"),
    (KEY_FLAG_CANCEL_LABEL, "Cancel"),
    (KEY_FLAG_SHOWN_TEMPLATE, "flagged: “{note}”"),
    (KEY_NOTHING_LEFT_LABEL, "Nothing left to ask"),
    (KEY_UNFINISHED_LABEL, "Unfinished session"),
    (KEY_UNFINISHED_DETAIL_TEMPLATE, "· {when} · {who} · {answered} of {total} answered."),
    (KEY_RESUME_LABEL, "Resume"),
    (KEY_START_OVER_LABEL, "Start over"),
    (KEY_START_OVER_HINT, "Starting over keeps what you answered on Chuck's sheet."),
    (KEY_BACK_LABEL, "◂ Back to start"),
    (KEY_BACK_HINT_QUESTION, "your answers so far are kept"),
    (KEY_BACK_HINT_REVEAL, "this answer is already on Chuck's sheet"),
    (KEY_SKIP_QUESTION_LABEL, "Skip this one — doesn't fit"),
    (KEY_END_SESSION_LABEL, "End session ▸"),
    (KEY_SKIPPED_ANSWER_TEXT, "(skipped — doesn't fit)"),
    (KEY_MARK_SKIPPED, "skipped"),
    (KEY_SHEET_SKIPPED_CLAUSE_TEMPLATE, "{s} skipped."),
    (KEY_SHEET_ENDED_EARLY_CLAUSE, "Ended early."),
    (KEY_FLAG_SUMMARY_HEADING, "Flagged before the session"),
    (KEY_FLAG_SUMMARY_HINT, "— questions Marie said don't fit; Roman/Chuck decide what to do with them:"),
    (KEY_FLAG_SUMMARY_ITEM_TEMPLATE, "{id} — “{question}” → {note}"),    (KEY_MARK_HIDDEN_BEFORE_ASKED, "hidden before asked"),
];

impl PracticeFlowWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_practice_flow_wording::<String>(|key| {
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
        .expect("every key in PRACTICE_FLOW_WORDING_KEYS is in TEST_SEED")
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
        .expect("the practice flow migration is on disk");
    let corrections: String = CORRECTION_MIGRATIONS
        .iter()
        .map(|relative| {
            std::fs::read_to_string(root.join(relative))
                .unwrap_or_else(|_| panic!("{relative} is on disk"))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let hotfix = std::fs::read_to_string(root.join(HOTFIX_MIGRATION))
        .expect("the attribution hotfix migration is on disk");

    for key in PRACTICE_FLOW_WORDING_KEYS {
        // Corrections first: a value UPDATEd after its INSERT is the one the
        // store actually holds, and searching the seed first pins the
        // superseded string while looking perfectly green.
        let seeded = corrected_value_in(&corrections, key)
            .or_else(|| seeded_value_in(&corrections, key))
            .or_else(|| seeded_value_in(&sql, key))
            .or_else(|| seeded_value_in(&hotfix, key))
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
/// ANTI-VACUITY: the test above walks `PRACTICE_FLOW_WORDING_KEYS`, so a key
/// dropped from that list stops being checked by it and nothing complains. This
/// one fails instead, naming the orphan.
#[test]
fn the_fixture_holds_no_key_the_boot_loader_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            PRACTICE_FLOW_WORDING_KEYS.contains(key),
            "{key} is in TEST_SEED but declared to nothing — either the boot \
             loader stopped reading it or the fixture was never cleaned up"
        );
    }
    assert_eq!(
        TEST_SEED.len(),
        PRACTICE_FLOW_WORDING_KEYS.len(),
        "the fixture and the declared list must be the same size"
    );
}

/// The templates carry the placeholders their callers fill.
///
/// A template that lost `{n}` renders a count with no number in it — which is
/// not a crash, not a test failure anywhere else, and exactly the kind of small
/// wrongness a witness stops trusting a screen over.
#[test]
fn every_template_carries_its_placeholders() {
    let w = PracticeFlowWording::for_test();
    for (name, value, needed) in [
        (
            "deck_count_template",
            &w.deck_count_template,
            vec!["{george}", "{chuck}"],
        ),
        (
            "deck_skipped_suffix_template",
            &w.deck_skipped_suffix_template,
            vec!["{k}"],
        ),
        (
            "deck_instruction_template",
            &w.deck_instruction_template,
            vec!["{skip}", "{flag}"],
        ),
        (
            "flag_shown_template",
            &w.flag_shown_template,
            vec!["{note}"],
        ),
        (
            "unfinished_detail_template",
            &w.unfinished_detail_template,
            vec!["{when}", "{who}", "{answered}", "{total}"],
        ),
        (
            "sheet_skipped_clause_template",
            &w.sheet_skipped_clause_template,
            vec!["{s}"],
        ),
        (
            "flag_summary_item_template",
            &w.flag_summary_item_template,
            vec!["{id}", "{question}", "{note}"],
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
