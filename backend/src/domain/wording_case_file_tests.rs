// Tests for `domain::wording_case_file`, and the fixture every other suite uses.
//
// Same law as its siblings: a key declared to the boot loader with no row in the
// migration makes the backend REFUSE TO START, and reading the migration off
// disk is the only thing that catches it before a deploy does (Rule 21).

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The one migration that seeds this block.
const SEED_MIGRATION: &str = "pipeline_migrations/20260925081941_chat_keepwarm_settings.sql";

/// The seeded values, for TESTS ONLY.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_CASE_FILE_AUTOMATIC_ON, "Automatic: on, {start} – {end}"),
    (
        KEY_CASE_FILE_AUTOMATIC_PAUSED,
        "Automatic: on, paused until tomorrow",
    ),
    (KEY_CASE_FILE_AUTOMATIC_OFF, "Automatic: off"),
];

impl CaseFileWording {
    /// The fixture, built through the PRODUCTION builder.
    pub fn for_test() -> Self {
        build_case_file_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in CASE_FILE_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(k, v)| (*k, (*v).to_string()))
            .collect()
    }
}

#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the keep-warm migration is on disk");
    assert_eq!(TEST_SEED.len(), CASE_FILE_WORDING_KEYS.len());
    for key in CASE_FILE_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key).unwrap_or_else(|| panic!("{key} is not seeded"));
        let fixture = TEST_SEED
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| *v)
            .unwrap_or_else(|| panic!("{key} is in no fixture"));
        assert_eq!(seeded, fixture, "{key}: migration and fixture disagree");
    }
}

#[test]
fn no_line_names_an_internal_word() {
    for (_, value) in TEST_SEED {
        let lower = value.to_lowercase();
        for word in ["cache", "prefix", "token"] {
            assert!(!lower.contains(word), "{value:?} says {word:?}");
        }
    }
}
