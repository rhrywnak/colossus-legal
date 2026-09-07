// Tests for `domain::wording_fact_card`.
//
// Same shape, and same single justification, as the eleven sibling wording test
// files: a key declared to the boot loader with no row in the migration makes the
// backend REFUSE TO START. That is a deploy taking DEV down, and reading the
// migration off disk is the only thing that catches it before it happens
// (Rule 21, the disk/code consistency pattern). Nothing here restates the code.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use crate::domain::wording_templates::missing_placeholders;
use std::collections::HashMap;

/// The migration that seeds all twenty-three rows.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260906161201_fact_card_v2_tables_and_wording.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_PROOF_LABEL, "Proof"),
    (KEY_BACKS_LABEL, "Backs"),
    (KEY_SUPPORTS_LABEL, "Supports"),
    (KEY_WATCH_OUT_LABEL, "Watch out"),
    (KEY_ANSWER_LABEL, "Answer"),
    (KEY_EMPTY_VALUE, "\u{2014}"),
    (KEY_DRAFT_MARK, "draft"),
    (KEY_CONTEXT_LABEL, "Show context"),
    (KEY_CONTEXT_HIDE_LABEL, "Hide context"),
    (KEY_BACKS_TEMPLATE, "Point {position} \u{2014} {text}"),
    (KEY_SUPPORTS_TEMPLATE, "{verb} {code} \u{2014} {text}"),
    (KEY_STANCE_SUPPORTS_VERB, "Supports"),
    (KEY_STANCE_REBUTS_VERB, "Disputes"),
    (KEY_RFA_TEMPLATE, "RFA {number} \u{2014} {request} \u{2014} {answer}"),
    (KEY_RFA_UNNUMBERED_TEMPLATE, "{request} \u{2014} {answer}"),
    (KEY_DECK_TEMPLATE, "{shown} of {total} shown \u{b7} {collapsed} more collapsed"),
    (KEY_EDIT_LABEL, "Edit"),
    (KEY_SAVE_LABEL, "Save"),
    (KEY_CANCEL_LABEL, "Cancel"),
    (KEY_SAVE_FAILED_TEMPLATE, "That edit did not save: {detail} Reopen the field \u{2014} what you see now may not be what is stored."),
    (KEY_NO_ANSWER_NOTICE, "No answer written yet."),
    (KEY_ACCUSATION_HEADING, "The Accusation"),
    (KEY_NO_CARDS_NOTICE, "No cards have been written for this scenario yet."),
];

impl FactCardWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_fact_card_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in FACT_CARD_WORDING_KEYS is in TEST_SEED")
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
        .expect("the fact-card migration is on disk");

    let fixture = FactCardWording::for_test_values();

    for key in FACT_CARD_WORDING_KEYS {
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

/// Every template still carries every placeholder the store requires of it.
///
/// The values live in `TEST_SEED`, which the test above pins to the migration —
/// so this asserts the SHIPPED strings, not a fixture written to pass. A template
/// that reached the migration with a placeholder missing would render a sentence
/// with its facts silently removed: "Point  — " with no point, or
/// "Supports  — " with no accusation, on a card a witness reads.
#[test]
fn every_template_carries_the_placeholders_the_store_requires() {
    let words = FactCardWording::for_test();
    let checked = [
        (KEY_BACKS_TEMPLATE, &words.backs_template),
        (KEY_SUPPORTS_TEMPLATE, &words.supports_template),
        (KEY_RFA_TEMPLATE, &words.rfa_template),
        (KEY_RFA_UNNUMBERED_TEMPLATE, &words.rfa_unnumbered_template),
        (KEY_DECK_TEMPLATE, &words.deck_template),
        (KEY_SAVE_FAILED_TEMPLATE, &words.save_failed_template),
    ];
    for (key, template) in checked {
        let missing = missing_placeholders(key, template);
        assert!(
            missing.is_empty(),
            "{key} seeds '{template}', which is missing {missing:?}",
        );
    }

    // Anti-vacuity: `missing_placeholders` returns an empty Vec for a key it does
    // not know, so the loop above would pass for six keys nobody registered.
    // Prove the table actually constrains one of them.
    assert_eq!(
        missing_placeholders(KEY_BACKS_TEMPLATE, "Point two, about the money"),
        vec!["{position}", "{text}"],
        "the store does not constrain the Backs line — the loop above is vacuous",
    );
}

/// The two stance verbs are DIFFERENT words.
///
/// A reader skimming five cards must not have to parse a negation, and a build
/// that seeded both as "Supports" would render every card as helping the
/// accusation — including the ones that destroy it.
#[test]
fn the_two_stance_verbs_are_different_words() {
    let w = FactCardWording::for_test();
    assert_ne!(w.stance_supports_verb, w.stance_rebuts_verb);
    assert!(!w.stance_rebuts_verb.is_empty());
}

/// No key collides with a sibling block's.
///
/// `app_settings` is keyed by `key` alone, so a collision would make two surfaces
/// read ONE row — renaming a witness's label would silently re-word a curator's
/// filter chip.
#[test]
fn no_key_collides_with_another_surface_s_key() {
    for key in FACT_CARD_WORDING_KEYS {
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
