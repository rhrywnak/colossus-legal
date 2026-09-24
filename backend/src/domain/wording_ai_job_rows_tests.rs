// Tests for `domain::wording_ai_job_rows`, and the fixture every other suite uses.
//
// Same law as its siblings: a key declared to the boot loader with no row in the
// migration makes the backend REFUSE TO START, and reading the migration off
// disk is the only thing that catches it before a deploy does (Rule 21).

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The one migration that seeds this block.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260924145151_ai_jobs_panel_and_last_run_wording.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_AI_JOB_DISCUSS_CHAT_TITLE, "Discuss chat on an answer"),
    (KEY_AI_JOB_DISCUSS_CHAT_DESC, "Reads the whole record and quotes it."),
    (KEY_AI_JOB_DISCUSS_CHAT_SUBJECT, "the Discuss chat"),
    (KEY_AI_JOB_DISCUSS_CHAT_NOTE, "Changing the model or the instructions reloads the case file on the next question (about {cost})."),
    (KEY_AI_JOB_DISCUSS_CHAT_NOTE_UNPRICED, "Changing the model or the instructions reloads the case file on the next question."),
    (KEY_AI_JOB_DISCUSS_EFFORT_TITLE, "How hard the Discuss chat thinks"),
    (KEY_AI_JOB_DISCUSS_EFFORT_DESC, "Medium answers sooner; High thinks longer."),
    (KEY_AI_JOB_DISCUSS_EFFORT_SUBJECT, "the Discuss chat"),
    (KEY_AI_JOB_DISCUSS_EFFORT_NOTE, "No reload."),
    (KEY_AI_JOB_CASE_STORY_TITLE, "Case story given to every chat"),
    (KEY_AI_JOB_CASE_STORY_DESC, "Parties, timeline, claims, who is who."),
    (KEY_AI_JOB_CASE_STORY_SUBJECT, "the Discuss chat"),
    (KEY_AI_JOB_CASE_STORY_NOTE, "Changing it reloads the case file (about {cost})."),
    (KEY_AI_JOB_CASE_STORY_NOTE_UNPRICED, "Changing it reloads the case file."),
    (KEY_AI_JOB_ANSWER_ANALYSIS_TITLE, "Answer analysis"),
    (KEY_AI_JOB_ANSWER_ANALYSIS_DESC, "The verdict shown under each practice answer."),
    (KEY_AI_JOB_ANSWER_ANALYSIS_SUBJECT, "the answer analysis"),
    (KEY_AI_JOB_CHAT_PAGE_TITLE, "Chat page"),
    (KEY_AI_JOB_CHAT_PAGE_DESC, "The model the Chat page in the top menu starts on. Anyone can switch it there."),
    (KEY_AI_JOB_CHAT_PAGE_SUBJECT, "the Chat page"),
    (KEY_AI_JOB_THEME_SCAN_TITLE, "Theme scan"),
    (KEY_AI_JOB_THEME_SCAN_DESC, "The model and instructions the scan screen starts with."),
    (KEY_AI_JOB_THEME_SCAN_SUBJECT, "the theme scan"),
];

impl AiJobRowsWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_ai_job_rows_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in AI_JOB_ROWS_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(k, v)| (*k, (*v).to_string()))
            .collect()
    }
}

/// Every declared key is seeded by the migration, with the value this build
/// expects.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the ai jobs wording migration is on disk");

    assert_eq!(TEST_SEED.len(), AI_JOB_ROWS_WORDING_KEYS.len());
    for key in AI_JOB_ROWS_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is declared to the boot loader but not seeded"));
        let fixture = TEST_SEED
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| (*v).to_string())
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));
        assert_eq!(
            seeded, fixture,
            "the migration and the fixture disagree about {key}"
        );
    }
}

/// A missing row is named, not defaulted.
#[test]
fn a_missing_row_is_refused_by_name() {
    let refused = build_ai_job_rows_wording::<String>(|key| {
        if key == KEY_AI_JOB_THEME_SCAN_SUBJECT {
            Err("missing".to_string())
        } else {
            Ok("x".to_string())
        }
    });
    assert_eq!(refused, Err("missing".to_string()));
}

/// Every key carries this block's prefix, and none is declared twice.
#[test]
fn every_key_is_prefixed_and_unique() {
    let mut seen = std::collections::HashSet::new();
    for key in AI_JOB_ROWS_WORDING_KEYS {
        assert!(key.starts_with("ai_job_"), "{key} lacks the block prefix");
        assert!(seen.insert(*key), "{key} is declared twice");
    }
}
