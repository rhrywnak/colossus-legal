// Tests for `domain::wording_model_params`.
//
// Same shape, and same single justification, as the eight sibling wording test
// files: a key declared to the boot loader with no row in the migration makes the
// backend REFUSE TO START. That is a deploy taking DEV down, and reading the
// migration off disk is the only thing that catches it before it happens
// (Rule 21, the disk/code consistency pattern). Nothing here restates the code.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds all seven rows.
const SEED_MIGRATION: &str = "pipeline_migrations/\
                              20260809153630_seed_opus_5_temperature_mode_and_failed_honesty_wording.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_TEMPERATURE_MODE_LABEL, "Temperature parameter"),
    (
        KEY_TEMPERATURE_MODE_HELP,
        "What this model does with a temperature setting. This is not a \
         preference: a model that has retired the parameter rejects every call \
         that sends one.",
    ),
    (
        KEY_TEMPERATURE_MODE_OMIT,
        "Send no temperature — this model rejects it",
    ),
    (KEY_TEMPERATURE_MODE_ZERO_OK, "Send an exact temperature"),
    (
        KEY_TEMPERATURE_MODE_UNSET,
        "Not recorded yet — nothing is sent",
    ),
    (KEY_TEMPERATURE_VALUE_LABEL, "Temperature value"),
    (
        KEY_TEMPERATURE_VALUE_DISABLED,
        "Available once this model is set to send a temperature.",
    ),
];

impl ModelParamsWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_model_params_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in MODEL_PARAMS_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

/// Every declared key is seeded, with the value this build expects.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the model-params migration is on disk");

    let fixture = ModelParamsWording::for_test_values();

    for key in MODEL_PARAMS_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key).unwrap_or_else(|| {
            panic!("{key} is declared to the boot loader but no migration seeds a row for it")
        });
        let in_fixture = fixture
            .get(*key)
            .unwrap_or_else(|| panic!("{key} is missing from TEST_SEED"));

        assert_eq!(
            in_fixture, &seeded,
            "the fixture has {key} = '{in_fixture}' but the migration seeds '{seeded}'."
        );
    }
}

/// No key collides with a sibling block's.
///
/// `app_settings` is keyed by `key` alone, so a collision would make two surfaces
/// read ONE row — renaming a dropdown option on the models admin would silently
/// re-word something in the curation queue.
#[test]
fn no_key_collides_with_another_surface_s_key() {
    for key in MODEL_PARAMS_WORDING_KEYS {
        for (name, list) in [
            ("curation", crate::domain::wording::WORDING_KEYS),
            (
                "working-view accusation",
                crate::domain::wording_accusation::ACCUSATION_WORDING_KEYS,
            ),
            (
                "rehearsal prose",
                crate::domain::wording_rehearsal::REHEARSAL_WORDING_KEYS,
            ),
            (
                "rehearsal chrome",
                crate::domain::wording_rehearsal_chrome::REHEARSAL_CHROME_KEYS,
            ),
            (
                "shared authoring-section",
                crate::domain::wording_authoring::AUTHORING_WORDING_KEYS,
            ),
            (
                "scenario-authoring",
                crate::domain::wording_scenario_authoring::SCENARIO_AUTHORING_WORDING_KEYS,
            ),
            ("scan", crate::domain::wording_scan::SCAN_WORDING_KEYS),
            (
                "card-grammar",
                crate::domain::wording_card_grammar::CARD_GRAMMAR_WORDING_KEYS,
            ),
        ] {
            assert!(!list.contains(key), "{key} is also a {name} key");
        }
    }
}

/// The dropdown names BOTH capabilities, and says what an unrecorded row does.
///
/// Not a restatement of the seed table: this asserts the property that makes the
/// control usable by the person who actually operates it. The stored column holds
/// `zero-ok` / `omit` / NULL, and an operator picking between two tokens he did
/// not choose the spelling of is the state this block exists to end. Every option
/// has to say what SENDING looks like, in words, without naming a token.
#[test]
fn every_option_says_what_the_call_will_carry_without_naming_a_token() {
    let words = ModelParamsWording::for_test();

    for (label, text) in [
        ("omit", &words.temperature_mode_omit_label),
        ("zero-ok", &words.temperature_mode_zero_ok_label),
        ("unset", &words.temperature_mode_unset_label),
    ] {
        assert!(
            text.to_lowercase().contains("send") || text.to_lowercase().contains("sent"),
            "the {label} option reads '{text}', which does not say what goes out on the call"
        );
        // The tokens themselves must not reach the screen: they are the DB's
        // vocabulary, and "zero-ok" tells a reader nothing about whether his next
        // scan will 400.
        for token in ["zero-ok", "omit", "temperature_mode"] {
            assert!(
                !text.contains(token),
                "the {label} option leaks the stored token '{token}' onto the screen: '{text}'"
            );
        }
    }
}
