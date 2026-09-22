// Tests for `domain::wording_question_chat`. Generated from the migration.
//
// A key declared to the boot loader with no row in the migration makes the
// backend REFUSE TO START, so every key is pinned to the migration.

use super::*;
use crate::domain::wording::tests::seeded_value_in;
use std::collections::HashMap;

/// The migration that seeds this block.
const SEED_MIGRATION: &str =
    "pipeline_migrations/20260921140958_chat_discussions_and_grounded_flag.sql";

/// The v2.2.1 fixes migration, which seeds the side panel's close label. A second
/// file rather than an edit to the first: where a row was seeded is history, and
/// only its VALUE is what this test pins.
const V221_SEED_MIGRATION: &str = "pipeline_migrations/20260922072151_practice_fixes_v2_2_1.sql";

/// The seeded values, for TESTS ONLY.
const TEST_SEED: &[(&str, &str)] = &[
    (KEY_OPEN_LABEL, "Discuss this answer"),
    (KEY_OPEN_HINT, "The read is the quick verdict. The discussion is where the strategy lives — ask why, argue back, or work out a better shape."),
    (KEY_YOUR_THREAD, "Your thread"),
    (KEY_THREAD_OF_TEMPLATE, "{name}'s thread"),
    (KEY_SWITCH_LABEL, "Switch thread"),
    (KEY_VISIBILITY_TEMPLATE, "{others} can read this thread"),
    (KEY_RESUMED_TEMPLATE, "resumed from {date}"),
    (KEY_READONLY_LINE, "Read-only — you can read this thread, and the AI reads it too, but only its owner writes in it."),
    (KEY_EARLIER_READONLY_LINE, "Read-only — the team's discussion of this question from before each person had a thread. The AI reads it too."),
    (KEY_AND, "and"),
    (KEY_GROUNDED_CHIP_TEMPLATE, "{model} · grounded"),
    (KEY_EXPAND_LABEL, "Expand discussion to full screen"),
    (KEY_COLLAPSE_LABEL, "Collapse to side panel"),
    (KEY_CLOSE_LABEL, "Close discussion"),
    (KEY_BACK_LABEL, "Back"),
    (KEY_BACK_ARIA, "Back to question"),
    (KEY_RESIZE_LABEL, "Drag to resize"),
    (KEY_STRIP_ANSWER_TEMPLATE, "your answer: “{answer}”"),
    (KEY_INPUT_PLACEHOLDER, "Ask about this question, the documents behind it, or the strategy…"),
    (KEY_MESSAGE_LABEL, "Message"),
    (KEY_SEND_LABEL, "Send"),
    (KEY_SWITCHER_OWN_TEMPLATE, "Your thread · {count} messages"),
    (KEY_SWITCHER_OWN_ONE, "Your thread · 1 message"),
    (KEY_SWITCHER_OWN_EMPTY, "Your thread · no messages yet"),
    (KEY_SWITCHER_OTHER_TEMPLATE, "{name} · {count} messages"),
    (KEY_SWITCHER_OTHER_ONE, "{name} · 1 message"),
    (KEY_SWITCHER_OTHER_EMPTY, "{name} · no messages yet"),
    (KEY_UNREAD_TEMPLATE, "{count} new"),
    (KEY_READONLY_MARK, "Read-only"),
    (KEY_SWITCHER_FOOTER, "Everyone on the case can read all threads. The AI reads them too, so insight crosses between you."),
    (KEY_EARLIER_LABEL, "Earlier team discussion"),
    (KEY_EARLIER_META_TEMPLATE, "{count} messages · last {date}"),
    (KEY_EARLIER_META_ONE, "1 message · {date}"),
    (KEY_EMPTY, "No messages yet — ask the first question."),
    (KEY_WAITING, "Thinking…"),
    (KEY_TOOL_LINE, "Checking the record…"),
    (KEY_SEND_FAILED, "No reply came back. Your message is saved above — send again to retry."),
    (KEY_STALLED, "The reply stopped arriving. Your message is saved above — send again to retry."),
    (KEY_REFUSED, "The model declined to answer that. Your message is saved above — try putting it another way."),
    (KEY_TRUNCATED, "The reply ran past its length limit and is not shown. Your message is saved above — send again to retry."),
    (KEY_LOAD_FAILED, "The discussion could not be loaded."),
    (KEY_READ_MARK_FAILED, "Your place in this thread could not be saved — its messages may show as new again."),
    (KEY_CAP_REACHED_TEMPLATE, "This thread has reached its limit of {max} replies."),
];

impl QuestionChatWording {
    /// The fixture, built through the PRODUCTION builder.
    pub fn for_test() -> Self {
        build_question_chat_wording::<String>(|key| {
            TEST_SEED
                .iter()
                .find(|(k, _)| *k == key)
                .map(|(_, v)| (*v).to_string())
                .ok_or_else(|| format!("{key} is missing from TEST_SEED"))
        })
        .expect("every key in QUESTION_CHAT_WORDING_KEYS is in TEST_SEED")
    }

    /// The fixture as a key→value map, in the shape the store reads.
    pub fn for_test_values() -> HashMap<&'static str, String> {
        TEST_SEED
            .iter()
            .map(|(key, value)| (*key, (*value).to_string()))
            .collect()
    }
}

#[test]
fn every_key_is_in_the_fixture_once_and_seeded_by_the_migration() {
    assert_eq!(TEST_SEED.len(), QUESTION_CHAT_WORDING_KEYS.len());
    let migration =
        std::fs::read_to_string(SEED_MIGRATION).expect("the seed migration is readable");
    let v221 = std::fs::read_to_string(V221_SEED_MIGRATION)
        .expect("the v2.2.1 fixes migration is readable");
    for key in QUESTION_CHAT_WORDING_KEYS {
        let seeded = seeded_value_in(&migration, key)
            .or_else(|| seeded_value_in(&v221, key))
            .unwrap_or_else(|| {
                panic!("{key} is not seeded by {SEED_MIGRATION} or {V221_SEED_MIGRATION}")
            });
        let fixture = TEST_SEED.iter().find(|(k, _)| k == key).map(|(_, v)| *v);
        assert_eq!(
            fixture,
            Some(seeded.as_str()),
            "{key}: fixture and migration disagree"
        );
    }
}

#[test]
fn the_builder_reads_every_key() {
    let wording = QuestionChatWording::for_test();
    assert_eq!(wording.open_label, "Discuss this answer");
    assert_eq!(wording.earlier_label, "Earlier team discussion");
}
