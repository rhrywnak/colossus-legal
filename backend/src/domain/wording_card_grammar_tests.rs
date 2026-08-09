// Tests for `domain::wording_card_grammar`.
//
// Same shape, and same single justification, as the sibling wording test files:
// a key declared to the boot loader with no row in the migration makes the
// backend REFUSE TO START. That is a deploy taking DEV down, and reading the
// migration off disk is the only thing that catches it before it happens
// (Rule 21, the disk/code consistency pattern). Nothing here restates the code.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds every row in this block.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260809121531_one_card_grammar_wording_and_settings.sql";

/// The seeded values, for TESTS ONLY — kept beside the test that pins them to
/// the migration file, so a fixture and its proof cannot drift apart.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_FILTER_PROPOSED, "Proposed"),
    (KEY_FILTER_DEFERRED, "Deferred"),
    (KEY_FILTER_INCLUDED, "Included"),
    (KEY_FILTER_EXCLUDED, "Excluded"),
    (KEY_FILTER_FULL_POOL, "Full pool"),
    (
        KEY_FULL_POOL_EXPLAINER,
        "Everything the system ever gathered for this scenario, across all scans. \
         The other filters are slices of this. Nothing in the full pool is lost \
         when you filter.",
    ),
    (KEY_FILTER_PROGRESS, "{ruled} of {total} {filter} ruled"),
    (KEY_QUESTION_EXPAND, "show full question"),
    (KEY_QUESTION_COLLAPSE, "hide full question"),
    (
        KEY_QUESTION_MACHINE_AUTHORSHIP,
        "Question as transcribed from the document",
    ),
    (KEY_SPEAKER_EXTRACTED, "extracted"),
    (KEY_SPEAKER_ABSENT, "speaker not extracted"),
    (KEY_ELEMENTS_MORE, "+{count} more"),
    (KEY_ELEMENTS_FEWER, "show fewer"),
    (KEY_CONTEXT_SHOW, "Show context"),
    (KEY_CONTEXT_HIDE, "Hide context"),
    (KEY_SCAN_REASON, "Scan:"),
    (
        KEY_LINK_TYPEAHEAD_PLACEHOLDER,
        "Type A-41, or a word from any allegation…",
    ),
    (
        KEY_LINK_TYPEAHEAD_INTRO,
        "This statement is not linked to anything they have accused you of. Link \
         it and the ruling buttons wake up. A link belongs to the statement — it \
         follows this item everywhere it appears.",
    ),
    (
        KEY_LINK_TYPEAHEAD_NO_MATCH,
        "No allegation matches what you typed.",
    ),
    (
        KEY_LINK_WOKE_RULING,
        "Linked. {code} can be ruled now — the link follows this statement \
         everywhere it appears.",
    ),
    (KEY_WEIGHT_PICKER, "Weight"),
    (KEY_WEIGHT_CHANGED, "Weight set: {code} now reads {tier}."),
    (KEY_WEIGHT_UNDO, "undo"),
    (KEY_RESET_ORDER, "Reset order"),
    (
        KEY_RESET_ORDER_CONFIRM,
        "Forget where you have placed every fact in this scenario? The sequence \
         is the argument, and this cannot be undone.",
    ),
    (KEY_RESET_ORDER_YES, "Reset the order"),
    (KEY_RESET_ORDER_CANCEL, "Keep my order"),
    (
        KEY_RESET_ORDER_DONE,
        "Order reset — {count} facts returned to their natural order.",
    ),
    (
        KEY_RESET_ORDER_FAILED,
        "The order could not be reset: {reason}",
    ),
    (KEY_CHIP_FILTER_HINT, "Show only: {value}"),
    (
        KEY_CHIP_FILTER_CLEAR,
        "Showing only {value} — show everything",
    ),
];

impl CardGrammarWording {
    /// The fixture, built through the PRODUCTION builder — so a fixture the real
    /// builder would reject cannot exist.
    pub fn for_test() -> Self {
        build_card_grammar_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in CARD_GRAMMAR_WORDING_KEYS is in TEST_SEED")
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
        .expect("the one-card-grammar migration is on disk");

    let fixture = CardGrammarWording::for_test_values();

    for key in CARD_GRAMMAR_WORDING_KEYS {
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
/// read ONE row — renaming a filter chip would silently re-word something on the
/// rehearsal page.
#[test]
fn no_key_collides_with_another_surface_s_key() {
    for key in CARD_GRAMMAR_WORDING_KEYS {
        assert!(
            !crate::domain::wording::WORDING_KEYS.contains(key),
            "{key} is also a curation-surface key"
        );
        assert!(
            !crate::domain::wording_accusation::ACCUSATION_WORDING_KEYS.contains(key),
            "{key} is also a working-view accusation key"
        );
        assert!(
            !crate::domain::wording_rehearsal::REHEARSAL_WORDING_KEYS.contains(key),
            "{key} is also a rehearsal prose key"
        );
        assert!(
            !crate::domain::wording_rehearsal_chrome::REHEARSAL_CHROME_KEYS.contains(key),
            "{key} is also a rehearsal chrome key"
        );
        assert!(
            !crate::domain::wording_authoring::AUTHORING_WORDING_KEYS.contains(key),
            "{key} is also a shared authoring-section key"
        );
        assert!(
            !crate::domain::wording_scenario_authoring::SCENARIO_AUTHORING_WORDING_KEYS
                .contains(key),
            "{key} is also a scenario-authoring key"
        );
        assert!(
            !crate::domain::wording_scan::SCAN_WORDING_KEYS.contains(key),
            "{key} is also a scan-surface key"
        );
    }
}

/// Every shipped template keeps the slot its consumer fills.
///
/// Not a restatement of the key table: it asserts that the SEEDED value — the
/// one that ships — still carries the placeholder the code substitutes into. A
/// template edited to drop its slot renders a sentence with the fact missing and
/// still looks like a complete sentence, which is the worst shape a refusal or
/// an acknowledgment can take.
#[test]
fn the_shipped_templates_keep_the_slots_their_callers_fill() {
    let w = CardGrammarWording::for_test();

    for (name, template, slots) in [
        (
            KEY_FILTER_PROGRESS,
            &w.filter_progress_template,
            &["{ruled}", "{total}", "{filter}"][..],
        ),
        (KEY_ELEMENTS_MORE, &w.elements_more_template, &["{count}"]),
        (
            KEY_LINK_WOKE_RULING,
            &w.link_woke_ruling_template,
            &["{code}"],
        ),
        (
            KEY_WEIGHT_CHANGED,
            &w.weight_changed_template,
            &["{code}", "{tier}"],
        ),
        (
            KEY_RESET_ORDER_DONE,
            &w.reset_order_done_template,
            &["{count}"],
        ),
        (
            KEY_RESET_ORDER_FAILED,
            &w.reset_order_failed_template,
            &["{reason}"],
        ),
        (
            KEY_CHIP_FILTER_HINT,
            &w.chip_filter_hint_template,
            &["{value}"],
        ),
        (
            KEY_CHIP_FILTER_CLEAR,
            &w.chip_filter_clear_template,
            &["{value}"],
        ),
    ] {
        for slot in slots {
            assert!(
                template.contains(slot),
                "the shipped {name} must contain {slot}, or the sentence renders \
                 with that fact missing and still reads as complete"
            );
        }
    }
}

/// The badge that replaced "System" does not read as a speaker.
///
/// ## Why this is a real test and not a restatement of a string
///
/// The whole of ruling R2 is that this row's PREDECESSOR was misread as the
/// speaker of the answer beneath it. The seeded sentence has to say what it is
/// about — the question, and where its text came from — and it must not be the
/// bare word that caused the confusion. A future edit back to "System" is
/// exactly the regression this guards.
#[test]
fn the_question_authorship_badge_names_the_question_not_a_speaker() {
    let label = CardGrammarWording::for_test().question_machine_authorship_label;

    assert_ne!(
        label, "System",
        "the badge must not go back to the bare word Roman read as a speaker"
    );
    assert!(
        label.to_lowercase().contains("question"),
        "the badge must name what it is about — it says where the QUESTION's \
         text came from, which is a different claim from who gave the answer"
    );
}
