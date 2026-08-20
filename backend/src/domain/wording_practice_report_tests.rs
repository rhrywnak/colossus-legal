// Tests for `domain::wording_practice_report`.
//
// The sibling of `wording_practice_tests`, and the same law: a declared key with
// no migration row is a backend that refuses to start. See that file's header.
//
// The extra tests below are about the two split sentences and about the words
// that appear on PAPER, where there is no tooltip and no second click.

use super::*;
use crate::domain::wording::tests::{corrected_value_in, seeded_value_in};
use std::collections::HashMap;

/// The migration that seeds every row this module reads.
const SEED_MIGRATION: &str = "pipeline_migrations/20260817213319_practice_session_v0.sql";

/// The migration that CORRECTS one of those rows.
///
/// `practice_stronger_summary` shipped carrying a `▸` of its own, and
/// `<details><summary>` draws a disclosure marker whether or not the label has
/// one — so the drawer rendered two arrows. The v1 migration edits the row.
///
/// It is read here for the reason the card-grammar block reads its own
/// corrections file: a fixture pinned only to the ORIGINAL insert would go green
/// while the store holds something else, which is the drift these tests exist to
/// catch.
const CORRECTION_MIGRATION: &str =
    "pipeline_migrations/20260819100411_practice_v1_chuck_review_deck_keys_kinds_and_points_to.sql";

/// The migration that seeds T1's two rows (2026-08-20).
///
/// A SECOND seed file and not an edit to the first, because where a row was
/// seeded is a fact about migration history and only its VALUE is what these
/// tests pin — the same reasoning `settings_store_tests` gives for concatenating
/// its own list. The abstain line and the don't-recall line arrived with the
/// three-part read; v0 could not have seeded them.
const T1_SEED_MIGRATION: &str =
    "pipeline_migrations/20260820165501_practice_read_t1_per_part_storage.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_WHAT_YOU_SAID_KICKER, "What you said"),
    (KEY_READ_TAG, "system read"),
    (
        KEY_READ_FOOTNOTE,
        "one sentence, against your points, the watch-for and the ALWAYS card. It names the tactic. The boxes below are yours.",
    ),
    (KEY_READ_UNAVAILABLE, "no system read this time"),
    (KEY_READ_ABSTAIN_LINE, "I can't read this one."),
    (
        KEY_READ_DONT_RECALL_LINE,
        "Fine. \"I don't recall\" is a complete answer.",
    ),
    (KEY_POINTS_KICKER, "Your points — in your own words"),
    (KEY_RECEIPT_PREFIX, "Backed by:"),
    (KEY_POINT_NO_RECEIPT, "No receipt recorded for this point."),
    (KEY_PAIR_KICKER, "Where the question came from — and their own sworn answer"),
    (KEY_PAIR_SAID_LABEL, "What they said"),
    (KEY_PAIR_ADMITTED_LABEL, "What they admitted under oath"),
    (KEY_CHECK_KICKER, "Check yourself"),
    (KEY_CHECK_ONLY_ASKED, "I answered only the question that was asked"),
    (KEY_CHECK_ACCEPTED_PREMISE, "I accepted a word or premise I shouldn't have"),
    (KEY_CHECK_EXPLAINED_UNASKED, "I explained something nobody asked about"),
    (KEY_CHECK_GUESSED, "I guessed at a date, a number, or a name"),
    (KEY_STRONGER_SUMMARY, "Show a stronger answer"),
    (KEY_STRONGER_NOTE_PREFIX, "An example of"),
    (KEY_STRONGER_NOTE_EMPHASIS, "how"),
    (
        KEY_STRONGER_NOTE_SUFFIX,
        ", built only from your own points — not a script. Say it your way.",
    ),
    (KEY_STRONGER_NO_RECEIPT, "No receipt for this one — that's a Chuck question."),
    (
        KEY_MARK_NOT_RECORDED,
        "That did not save. Your answer is recorded; the mark for it is not — press the button again.",
    ),
    (
        KEY_HELP_NOT_RECORDED,
        "Chuck's sheet will not show that you opened this — the note did not save.",
    ),
    (KEY_NEXT_BUTTON, "Got it — next question"),
    (KEY_AGAIN_BUTTON, "Ask me this one again later"),
    (KEY_SHEET_KICKER_TEMPLATE, "Session done · {code} · {when}"),
    (KEY_SHEET_HEADING_TEMPLATE, "{count} questions. {repeat}"),
    (KEY_SHEET_REPEAT_CLAUSE_TEMPLATE, "{n} to repeat."),
    (KEY_SHEET_NOTHING_TO_REPEAT, "Nothing to repeat."),
    (
        KEY_SHEET_SUB_PREFIX,
        "This is the sheet Chuck sees. Your words, as you typed them; the ones marked",
    ),
    (KEY_SHEET_SUB_SUFFIX, "are where he'll run the real mock cross."),
    (KEY_SHEET_COL_NUMBER, "#"),
    (KEY_SHEET_COL_FROM, "From"),
    (KEY_SHEET_COL_TACTIC, "Tactic"),
    (KEY_SHEET_COL_QUESTION, "Question"),
    (KEY_SHEET_COL_ANSWER, "Your answer"),
    (KEY_SHEET_COL_MARK, "Mark"),
    (KEY_SHEET_COL_HELP, "Help"),
    (KEY_SHEET_FROM_GEORGE, "George"),
    (KEY_SHEET_FROM_GEORGE_BRAID, "George · braid"),
    (KEY_SHEET_FROM_CHUCK, "Chuck"),
    (KEY_MARK_FINE, "fine"),
    (KEY_MARK_REPEAT, "repeat"),
    (KEY_HELP_OPENED, "opened"),
    (KEY_HELP_NONE, "—"),
    (KEY_TACTIC_NONE, "—"),
    (KEY_SHEET_AGAIN_BUTTON, "Practice again"),
    (KEY_PRINT_BUTTON, "Print Chuck's sheet"),
    (KEY_HOMELAB_LINE, "Nothing here leaves the homelab. Chuck gets the printed sheet."),
];

impl PracticeReportWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_practice_report_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in PRACTICE_REPORT_WORDING_KEYS is in TEST_SEED")
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
    let corrections = std::fs::read_to_string(root.join(CORRECTION_MIGRATION))
        .expect("the practice v1 correction migration is on disk");
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the practice migration is on disk");
    let t1 = std::fs::read_to_string(root.join(T1_SEED_MIGRATION))
        .expect("the T1 per-part storage migration is on disk");

    for key in PRACTICE_REPORT_WORDING_KEYS {
        let seeded = corrected_value_in(&corrections, key)
            .or_else(|| seeded_value_in(&sql, key))
            .or_else(|| seeded_value_in(&t1, key))
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
/// ANTI-VACUITY, and not a formality: the test above walks PRACTICE_REPORT_WORDING_KEYS, so a
/// fixture entry for a key nobody declares would never be visited. Without this,
/// a key removed from the struct but left in the fixture would look tested
/// forever.
#[test]
fn the_fixture_declares_no_key_the_build_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            PRACTICE_REPORT_WORDING_KEYS.contains(key),
            "{key} is in the fixture but no field reads it"
        );
    }
    assert_eq!(
        PRACTICE_REPORT_WORDING_KEYS.len(),
        TEST_SEED.len(),
        "one key, one fixture row"
    );
}

/// The two split sentences carry no markup, and their halves join cleanly.
///
/// See `wording_practice_tests`' pause-note test for the full argument. The
/// suffix rows here deliberately open with punctuation, because the store trims
/// and the emphasised word precedes them with no space.
#[test]
fn the_split_sentences_carry_no_markup_and_join_into_the_mockups_lines() {
    let w = PracticeReportWording::for_test();

    for part in [
        &w.stronger_note_prefix,
        &w.stronger_note_emphasis,
        &w.stronger_note_suffix,
        &w.sheet_sub_prefix,
        &w.sheet_sub_suffix,
    ] {
        assert!(
            !part.contains('<'),
            "a wording row must carry no markup: {part}"
        );
    }

    assert_eq!(
        format!(
            "{} {}{}",
            w.stronger_note_prefix, w.stronger_note_emphasis, w.stronger_note_suffix
        ),
        "An example of how, built only from your own points — not a script. Say it your way."
    );
    assert_eq!(
        format!(
            "{} {} {}",
            w.sheet_sub_prefix, w.mark_repeat, w.sheet_sub_suffix
        ),
        "This is the sheet Chuck sees. Your words, as you typed them; the ones marked repeat are \
         where he'll run the real mock cross."
    );
}

/// The sheet's seven columns are seven distinct rows.
///
/// A duplicated header is the kind of slip a reviewer's eye slides over and a
/// printed table makes permanent — two columns both headed "Your answer" on the
/// sheet Chuck runs his mock cross from.
#[test]
fn the_sheets_seven_columns_are_seven_distinct_headings() {
    let w = PracticeReportWording::for_test();
    let columns = [
        &w.sheet_col_number,
        &w.sheet_col_from,
        &w.sheet_col_tactic,
        &w.sheet_col_question,
        &w.sheet_col_answer,
        &w.sheet_col_mark,
        &w.sheet_col_help,
    ];
    let mut seen = std::collections::HashSet::new();
    for c in columns {
        assert!(seen.insert(c.clone()), "two columns share the heading {c}");
    }
    assert_eq!(seen.len(), 7);
}

/// The two marks are different words, and so are the two help cells.
///
/// They are the columns Chuck scans to decide where to spend a mock cross. If
/// "fine" and "repeat" ever became the same string the sheet would still print,
/// still align, and say nothing.
#[test]
fn the_marks_and_the_help_cells_are_distinguishable_on_paper() {
    let w = PracticeReportWording::for_test();
    assert_ne!(w.mark_fine, w.mark_repeat);
    assert_ne!(w.help_opened, w.help_none);
}

/// The drawer's label carries NO arrow of its own (task A8).
///
/// `<details><summary>` draws a disclosure marker, and the marker is the arrow
/// that matters — it turns when the drawer opens, which a character in a string
/// cannot do. A `▸` in the label put a second, frozen arrow beside it.
///
/// Pinned on the STRING rather than on the rendered markup because the string is
/// where it went wrong and the store is where it can go wrong again: this row is
/// editable on the Settings page, and nothing else in the stack would notice a
/// `▸` typed back into it.
#[test]
fn the_stronger_drawer_label_carries_no_arrow_of_its_own() {
    let w = PracticeReportWording::for_test();
    for arrow in ['\u{25b8}', '\u{25b6}', '\u{2023}', '\u{203a}'] {
        assert!(
            !w.stronger_summary.contains(arrow),
            "the label draws its own arrow beside the disclosure marker: {}",
            w.stronger_summary
        );
    }
    assert_eq!(w.stronger_summary, "Show a stronger answer");
}
