// Tests for `domain::wording_env_banner`, and the fixture every other suite uses.
//
// Same law as its siblings: a key declared to the boot loader with no row in the
// migration makes the backend REFUSE TO START, and reading the migration off
// disk is the only thing that catches it before a deploy does (Rule 21).

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The one migration that seeds this block.
const SEED_MIGRATION: &str = "pipeline_migrations/20260922142803_env_banner_wording.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (
        KEY_ENV_BANNER_TEXT,
        "TEST SYSTEM \u{2014} practice here is not saved for trial.",
    ),
    (KEY_ENV_BANNER_LINK_LABEL, "Go to the real system"),
    (
        KEY_ENV_BANNER_PRINT_LINE,
        "TEST SYSTEM \u{2014} NOT FOR TRIAL",
    ),
    (KEY_ENV_BANNER_REAL_URL, "https://colossus-legal.cogmai.com"),
];

impl EnvBannerWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_env_banner_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in ENV_BANNER_WORDING_KEYS is in TEST_SEED")
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
        .expect("the env banner migration is on disk");

    assert_eq!(TEST_SEED.len(), ENV_BANNER_WORDING_KEYS.len());
    for key in ENV_BANNER_WORDING_KEYS {
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

/// ANTI-VACUITY: the fixture holds nothing the boot loader does not read.
#[test]
fn the_fixture_declares_no_key_the_build_does_not_read() {
    for (key, _) in TEST_SEED {
        assert!(
            ENV_BANNER_WORDING_KEYS.contains(key),
            "{key} is in the fixture but nothing reads it"
        );
    }
}

/// A missing row is named, not defaulted.
#[test]
fn a_missing_row_is_refused_by_name() {
    let refused = build_env_banner_wording::<String>(|key| {
        if key == KEY_ENV_BANNER_PRINT_LINE {
            Err("missing".to_string())
        } else {
            Ok("x".to_string())
        }
    });
    assert_eq!(refused, Err("missing".to_string()));
}

/// The warning never names the machine in the words a person reads.
///
/// The fatal-flaw ruling: "DEV", "environment", "instance", a hostname or a
/// version string on screen would tell a witness what this build knows about
/// itself instead of what she needs to do.
#[test]
fn the_visible_words_carry_no_internal_vocabulary() {
    let w = EnvBannerWording::for_test();
    for line in [&w.text, &w.link_label, &w.print_line] {
        let lower = line.to_lowercase();
        for forbidden in ["dev", "environment", "instance", "staging", "cogmai", "v2."] {
            assert!(!lower.contains(forbidden), "{line:?} names {forbidden:?}");
        }
    }
}
