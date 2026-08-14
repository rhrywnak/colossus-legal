// Tests for `domain::wording_matrix`.
//
// Same shape, and same single justification, as the ten sibling wording test
// files: a key declared to the boot loader with no row in the migration makes the
// backend REFUSE TO START. That is a deploy taking DEV down, and reading the
// migration off disk is the only thing that catches it before it happens
// (Rule 21, the disk/code consistency pattern). Nothing here restates the code.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use crate::domain::wording_templates::missing_placeholders;
use std::collections::HashMap;

/// The migration that seeds all eight rows.
const SEED_MIGRATION: &str = "pipeline_migrations/\
                              20260813152536_tuesday_batch_396_matrix_strength_war_room_and_human_fact_completeness.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_STRONG_COLUMN_LABEL, "Strong support"),
    (KEY_RAW_APPROVED_TEMPLATE, "· {count} approved"),
    (
        KEY_STRONG_HINT,
        "Sworn admissions by the other side, and the court's own findings. \
         The number beside it is every approved item, however qualified.",
    ),
    (KEY_TIER_STRONG_CHIP, "Their own words"),
    (KEY_TIER_HEDGED_CHIP, "Qualified"),
    (KEY_TIER_OTHER_CHIP, "Our sworn word"),
    (KEY_DUPLICATE_TEMPLATE, "×{count}"),
    (KEY_RANKED_LIST_NOTE, "Strongest first"),
];

impl MatrixWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_matrix_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in MATRIX_WORDING_KEYS is in TEST_SEED")
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

    let fixture = MatrixWording::for_test_values();

    for key in MATRIX_WORDING_KEYS {
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

/// The three tier-MAP rows are seeded too.
///
/// They are not wording — they carry extraction vocabulary, not sentences — so
/// they live in `REQUIRED_KEYS` rather than in `MATRIX_WORDING_KEYS`, and no
/// wording test would notice their absence. They are checked HERE because they
/// ship in the same migration and answer the same question: does the row this
/// build refuses to boot without actually exist on disk?
#[test]
fn the_three_tier_map_rows_are_seeded_by_the_same_migration() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the .396 batch migration is on disk");

    for key in [
        "matrix_tier_strong_pairs",
        "matrix_tier_hedged_pairs",
        "matrix_tier_other_pairs",
    ] {
        let seeded = seeded_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is required at boot but no migration seeds it"));
        assert!(
            seeded.contains('+'),
            "{key} seeds '{seeded}', which carries no statement_type+evidence_strength pair",
        );
    }
}

/// The seeded map parses, and maps the six pairs measured on DEV as ruled.
///
/// This is the end-to-end of the configuration half: the exact text in the
/// migration, through the store's token splitter, through the domain's pair
/// parser, to a tier. A typo in the migration that the parser rejects would
/// otherwise only surface as a boot refusal on DEV after a deploy.
#[test]
fn the_seeded_map_parses_and_ranks_the_measured_dev_pairs_as_ruled() {
    use crate::domain::evidence_tier::{EvidenceTier, EvidenceTierMap};
    use crate::domain::settings::parse_token_list;

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .expect("the .396 batch migration is on disk");

    let read = |key: &str| {
        let value = seeded_value_in(&sql, key).unwrap_or_else(|| panic!("{key} is seeded"));
        parse_token_list(key, &value).unwrap_or_else(|e| panic!("{key} splits into tokens: {e}"))
    };
    let strong = read("matrix_tier_strong_pairs");
    let hedged = read("matrix_tier_hedged_pairs");
    let other = read("matrix_tier_other_pairs");

    let map = EvidenceTierMap::from_entries(&[
        (EvidenceTier::Strong, "matrix_tier_strong_pairs", &strong),
        (EvidenceTier::Hedged, "matrix_tier_hedged_pairs", &hedged),
        (EvidenceTier::Other, "matrix_tier_other_pairs", &other),
    ])
    .expect("the seeded map parses");

    // Roman's ruling of 2026-08-13, pinned against the rows that implement it.
    assert_eq!(
        map.tier_for(Some("admission"), Some("sworn_party_admission")),
        Some(EvidenceTier::Strong),
        "an opposing party's firm sworn admission is the headline",
    );
    assert_eq!(
        map.tier_for(Some("court_finding"), Some("court_finding")),
        Some(EvidenceTier::Strong),
    );
    assert_eq!(
        map.tier_for(Some("partial_admission"), Some("sworn_party_admission")),
        Some(EvidenceTier::Hedged),
        "the SAME strength under a partial admission must not reach the headline",
    );
    assert_eq!(
        map.tier_for(Some("factual_assertion"), Some("sworn_testimony")),
        Some(EvidenceTier::Other),
        "our own sworn affidavit word is ranked, not headlined",
    );
    assert_eq!(map.len(), 6, "six measured pairs, all mapped");
}

/// Both count-bearing templates keep their placeholder.
///
/// A depth line that lost `{count}` would read "· approved" — a claim with no
/// number — and a duplicate marker that lost it would read "×". The store's write
/// path refuses an edit that drops a required placeholder; this asserts the
/// SEEDED values satisfy the same rule, which the write path never sees.
#[test]
fn the_count_templates_carry_their_placeholder() {
    let words = MatrixWording::for_test();
    for (name, template) in [
        ("matrix_raw_approved_template", &words.raw_approved_template),
        ("matrix_duplicate_template", &words.duplicate_template),
    ] {
        assert!(
            template.contains("{count}"),
            "{name} reads '{template}', which cannot carry a number",
        );
        assert!(
            missing_placeholders(name, template).is_empty(),
            "{name} is missing a placeholder the store requires",
        );
    }
}

/// No key collides with a sibling block's.
///
/// `app_settings` is keyed by `key` alone, so a collision would make two surfaces
/// read ONE row — renaming a matrix chip would silently re-word something in the
/// curation queue.
#[test]
fn no_key_collides_with_another_surface_s_key() {
    for key in MATRIX_WORDING_KEYS {
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
            (
                "war-room",
                crate::domain::wording_war_room::WAR_ROOM_WORDING_KEYS,
            ),
        ] {
            assert!(!list.contains(key), "{key} is also a {name} key");
        }
    }
}
