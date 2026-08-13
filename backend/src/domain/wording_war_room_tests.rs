// Tests for `domain::wording_war_room`.
//
// Same shape and same justification as the ten sibling wording test files: a key
// declared to the boot loader with no row in the migration makes the backend
// REFUSE TO START (Rule 21, the disk/code consistency pattern).
//
// This block carries one extra burden the siblings do not. Its rows were RULED on
// 2026-08-10 and never migrated — the R2 batch shipped a different part of its
// own instruction and nobody noticed for three days, because the literals it was
// meant to replace kept rendering perfectly well. So the tests below check not
// only that the rows exist but that the words actually CHANGED: a row seeded with
// the sentence it was supposed to replace would satisfy every structural test and
// fix nothing.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds all four rows.
const SEED_MIGRATION: &str = "pipeline_migrations/\
                              20260813152536_tuesday_batch_396_matrix_strength_war_room_and_human_fact_completeness.sql";

/// The seeded values, for TESTS ONLY.
const TEST_SEED: &[(&str, &str)] = &[
    (
        KEY_SUBTITLE,
        "The attacks and what we answer them with — built by you, gathered by \
         the system, rehearsed by Marie.",
    ),
    (KEY_METRIC_SCENARIOS, "Scenarios"),
    (KEY_METRIC_READY, "Ready"),
    (KEY_METRIC_DRAFT, "Draft"),
];

impl WarRoomWording {
    /// The fixture, built through the PRODUCTION builder.
    pub fn for_test() -> Self {
        build_war_room_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in WAR_ROOM_WORDING_KEYS is in TEST_SEED")
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
        .expect("the .396 batch migration is on disk");

    let fixture = WarRoomWording::for_test_values();

    for key in WAR_ROOM_WORDING_KEYS {
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

/// The subtitle no longer credits the machine for a human's judgment.
///
/// This is the R2 §3 ruling itself, as an assertion. "System-generated" said the
/// scenarios were produced by the system; they are not — a human writes the
/// attack, the scan proposes, and a human rules every candidate. A row reseeded
/// with the old sentence would pass every other test in this file.
#[test]
fn the_subtitle_no_longer_claims_the_scenarios_are_system_generated() {
    let words = WarRoomWording::for_test();
    assert!(
        !words.subtitle.to_lowercase().contains("system-generated"),
        "the subtitle still reads '{}'",
        words.subtitle,
    );
    assert!(
        words.subtitle.to_lowercase().contains("by you"),
        "the subtitle must credit the human who builds the scenarios: '{}'",
        words.subtitle,
    );
}

/// The not-yet-ready tile carries ONE word for ONE number.
///
/// "Drafted / in review" invited a reader to look for two figures in one tile.
#[test]
fn the_draft_tile_names_one_state() {
    let words = WarRoomWording::for_test();
    assert!(
        !words.metric_draft_label.contains('/'),
        "the draft tile reads '{}', which names two states for one number",
        words.metric_draft_label,
    );
}

/// The three tile labels are distinct.
///
/// They are three short strings of the same shape read into three adjacent
/// fields; two tiles wearing one label is the failure mode, and it would look
/// like a rendering bug rather than a wording one.
#[test]
fn the_three_tile_labels_are_distinct() {
    let words = WarRoomWording::for_test();
    let labels = [
        &words.metric_scenarios_label,
        &words.metric_ready_label,
        &words.metric_draft_label,
    ];
    for (i, a) in labels.iter().enumerate() {
        for b in labels.iter().skip(i + 1) {
            assert_ne!(a, b, "two metric tiles carry the same label");
        }
    }
}

/// No key collides with a sibling block's.
#[test]
fn no_key_collides_with_another_surface_s_key() {
    for key in WAR_ROOM_WORDING_KEYS {
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
            (
                "model-params",
                crate::domain::wording_model_params::MODEL_PARAMS_WORDING_KEYS,
            ),
            ("matrix", crate::domain::wording_matrix::MATRIX_WORDING_KEYS),
        ] {
            assert!(!list.contains(key), "{key} is also a {name} key");
        }
    }
}
