// Tests for `domain::wording_practice_discuss`.
//
// A key declared to the boot loader with no row in the migration makes the
// backend REFUSE TO START (Rule 21), so every key is pinned to the migration.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The one migration that seeds this block.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260917120219_question_chat_threads_prompts_and_wording.sql";

/// The seeded values, for TESTS ONLY.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_BUTTON_LABEL, "Discuss with AI"),
    (KEY_TITLE_TEMPLATE, "Discuss · Question {n}"),
    (KEY_SUBTITLE_TEMPLATE, "{code}"),
    (KEY_SUBTITLE_DRAFT_TEMPLATE, "{code} · your draft answer is visible to the model"),
    (KEY_CONTEXT_LINE, "The model sees: the question · its tactic · your current answer · the talking points and receipts · the sworn pair · the latest analysis · the scenario's attack · the team's notes · for redirects, the question it repairs"),
    (KEY_CONTEXT_LINE_DRAFT, "The model sees: the question · its tactic · your unsaved draft · the talking points and receipts · the sworn pair · the latest analysis · the scenario's attack · the team's notes · for redirects, the question it repairs"),
    (KEY_INPUT_PLACEHOLDER, "Ask about this question or your answer…"),
    (KEY_SEND_LABEL, "Send"),
    (KEY_CLOSE_LABEL, "Close the discussion"),
    (KEY_MODEL_LABEL, "Model"),
    (KEY_FOOTER_TEMPLATE, "Thread is saved on this question · visible to Marie, Chuck and Roman · {cost} on {model}"),
    (KEY_COST_BILLED, "$ per turn"),
    (KEY_COST_LOCAL, "$0 per turn"),
    (KEY_COST_TEMPLATE, "{input} in · {output} out · {seconds} s"),
    (KEY_EMPTY, "No discussion yet — ask the first question."),
    (KEY_SENDING_TEMPLATE, "Waiting for {model}…"),
    (KEY_SEND_FAILED, "No reply came back. Your message is saved above — send again to retry."),
    (KEY_LOAD_FAILED, "The discussion could not be loaded."),
    (KEY_CAP_REACHED_TEMPLATE, "This question has reached its limit of {max} model replies."),
    (KEY_CAP_REACHED_ONE, "This question has reached its limit of {max} model reply."),
];

impl PracticeDiscussWording {
    /// The fixture, built through the PRODUCTION builder.
    pub fn for_test() -> Self {
        build_practice_discuss_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in PRACTICE_DISCUSS_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

fn echo(key: &str) -> Result<String, std::convert::Infallible> {
    Ok(key.to_string())
}

/// Every field reads the key it claims to.
#[test]
fn every_field_reads_its_own_key() {
    let w = build_practice_discuss_wording(echo).expect("infallible read");
    assert_eq!(w.button_label, KEY_BUTTON_LABEL);
    assert_eq!(w.title_template, KEY_TITLE_TEMPLATE);
    assert_eq!(w.subtitle_template, KEY_SUBTITLE_TEMPLATE);
    assert_eq!(w.subtitle_draft_template, KEY_SUBTITLE_DRAFT_TEMPLATE);
    assert_eq!(w.context_line, KEY_CONTEXT_LINE);
    assert_eq!(w.context_line_draft, KEY_CONTEXT_LINE_DRAFT);
    assert_eq!(w.input_placeholder, KEY_INPUT_PLACEHOLDER);
    assert_eq!(w.send_label, KEY_SEND_LABEL);
    assert_eq!(w.close_label, KEY_CLOSE_LABEL);
    assert_eq!(w.model_label, KEY_MODEL_LABEL);
    assert_eq!(w.footer_template, KEY_FOOTER_TEMPLATE);
    assert_eq!(w.cost_billed, KEY_COST_BILLED);
    assert_eq!(w.cost_local, KEY_COST_LOCAL);
    assert_eq!(w.cost_template, KEY_COST_TEMPLATE);
    assert_eq!(w.empty, KEY_EMPTY);
    assert_eq!(w.sending_template, KEY_SENDING_TEMPLATE);
    assert_eq!(w.send_failed, KEY_SEND_FAILED);
    assert_eq!(w.load_failed, KEY_LOAD_FAILED);
    assert_eq!(w.cap_reached_template, KEY_CAP_REACHED_TEMPLATE);
    assert_eq!(w.cap_reached_one, KEY_CAP_REACHED_ONE);
}

/// Every declared key is seeded with the value this build expects, and the
/// fixture holds nothing the boot loader does not read.
#[test]
fn every_declared_key_is_seeded_with_the_value_this_build_expects() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let sql = std::fs::read_to_string(root.join(SEED_MIGRATION))
        .unwrap_or_else(|_| panic!("{SEED_MIGRATION} is on disk"));
    let fixture = PracticeDiscussWording::for_test_values();
    for key in PRACTICE_DISCUSS_WORDING_KEYS {
        let seeded = seeded_value_in(&sql, key)
            .unwrap_or_else(|| panic!("{key} is declared but no migration seeds it"));
        assert_eq!(
            fixture.get(*key),
            Some(&seeded),
            "the migration and the fixture disagree about {key}"
        );
    }
    assert_eq!(TEST_SEED.len(), PRACTICE_DISCUSS_WORDING_KEYS.len());
}

/// The button names no model (ruled amendment 1) — and no stored word here does.
#[test]
fn no_dock_word_names_a_model() {
    for (key, value) in TEST_SEED {
        let lower = value.to_lowercase();
        for name in ["opus", "sonnet", "claude", "qwen"] {
            assert!(!lower.contains(name), "{key} names a model: {value}");
        }
    }
}

/// Templates carry their placeholders; the plain words carry none.
#[test]
fn templates_carry_their_placeholders() {
    let w = PracticeDiscussWording::for_test();
    for (name, value, needed) in [
        ("title_template", &w.title_template, &["{n}"][..]),
        ("subtitle_template", &w.subtitle_template, &["{code}"][..]),
        (
            "subtitle_draft_template",
            &w.subtitle_draft_template,
            &["{code}"][..],
        ),
        (
            "footer_template",
            &w.footer_template,
            &["{cost}", "{model}"][..],
        ),
        ("sending_template", &w.sending_template, &["{model}"][..]),
        (
            "cost_template",
            &w.cost_template,
            &["{input}", "{output}", "{seconds}"][..],
        ),
        (
            "cap_reached_template",
            &w.cap_reached_template,
            &["{max}"][..],
        ),
        ("cap_reached_one", &w.cap_reached_one, &["{max}"][..]),
    ] {
        for placeholder in needed {
            assert!(
                value.contains(placeholder),
                "{name} lost {placeholder}: {value}"
            );
        }
    }
    for (name, value) in [
        ("button_label", &w.button_label),
        ("send_label", &w.send_label),
        ("empty", &w.empty),
        ("send_failed", &w.send_failed),
        ("load_failed", &w.load_failed),
    ] {
        assert!(
            !value.contains('{'),
            "{name} must carry no placeholder: {value}"
        );
    }
}
