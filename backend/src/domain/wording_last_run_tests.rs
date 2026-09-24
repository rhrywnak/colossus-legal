// Tests for `domain::wording_last_run`, and the fixture every other suite uses.
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
    (KEY_LAST_RUN_FINISHED_VALUE, "{done} of {total}"),
    (KEY_LAST_RUN_FINISHED_LABEL, "documents finished"),
    (KEY_LAST_RUN_QUOTES_LABEL, "of quotes found in their source"),
    (KEY_LAST_RUN_QUOTES_NONE, "—"),
    (KEY_LAST_RUN_WHEN_LABEL, "last run, {clock}"),
    (KEY_LAST_RUN_IDLE, "No document run in progress. Time estimates appear here while one is running."),
    (KEY_LAST_RUN_RUNNING, "A document run is in progress: {count} documents left, about {time} to go."),
    (KEY_LAST_RUN_RUNNING_NO_ESTIMATE, "A document run is in progress: {count} documents left. No finished document yet to time it by."),
    (KEY_LAST_RUN_COL_STEP, "Step, in order"),
    (KEY_LAST_RUN_COL_AVG, "Avg time"),
    (KEY_LAST_RUN_COL_RUNS, "Runs"),
    (KEY_LAST_RUN_COL_FAILED, "Failed"),
    (KEY_LAST_RUN_SUMMARY, "{runs} step runs, {failed} failed ({detail})."),
    (KEY_LAST_RUN_SUMMARY_CLEAN, "{runs} step runs, none failed."),
    (KEY_LAST_RUN_FAILURE_DETAIL, "{step}, {day}"),
    (KEY_LAST_RUN_UNKNOWN_STEP, "This page has no plain name for the step \"{step}\", so it is shown by its code name."),
    (KEY_LAST_RUN_EMPTY, "No document has been run yet."),
    (KEY_LAST_RUN_STEP_UPLOAD, "Upload"),
    (KEY_LAST_RUN_STEP_EXTRACT_TEXT, "Read the text"),
    (KEY_LAST_RUN_STEP_LLM_EXTRACT_PASS1, "First extraction pass"),
    (KEY_LAST_RUN_STEP_LLM_EXTRACT_PASS2, "Second extraction pass"),
    (KEY_LAST_RUN_STEP_VERIFY, "Check the quotes"),
    (KEY_LAST_RUN_STEP_AUTO_APPROVE, "Approve automatically"),
    (KEY_LAST_RUN_STEP_INGEST, "Load into the case graph"),
    (KEY_LAST_RUN_STEP_INGEST_DELTA, "Graph update"),
    (KEY_LAST_RUN_STEP_INDEX, "Search index"),
    (KEY_LAST_RUN_STEP_COMPLETENESS, "Completeness check"),
];

impl LastRunWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_last_run_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in LAST_RUN_WORDING_KEYS is in TEST_SEED")
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

    assert_eq!(TEST_SEED.len(), LAST_RUN_WORDING_KEYS.len());
    for key in LAST_RUN_WORDING_KEYS {
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
    let refused = build_last_run_wording::<String>(|key| {
        if key == KEY_LAST_RUN_STEP_COMPLETENESS {
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
    for key in LAST_RUN_WORDING_KEYS {
        assert!(key.starts_with("last_run_"), "{key} lacks the block prefix");
        assert!(seen.insert(*key), "{key} is declared twice");
    }
}
