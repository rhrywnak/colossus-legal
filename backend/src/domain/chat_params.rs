//! The question chat's parameters (CC_TASK_CHAT_ENGINE_v1).
//!
//! Twelve stored values that decide which model answers, what it is told, and the
//! bounds on one reply. They live in their own block — not as fields on
//! `Settings` and not inside `PracticeReadParams` — because they configure a
//! different conversation from the one-shot read, and because `settings.rs` and
//! `practice_params.rs` are both near Rule 17's limit (memory: the settings
//! registry size ceiling).
//!
//! ## Domain note: the engine crate never sees these keys
//!
//! `colossus-chat` takes a `ChatRequest` of plain values. This block is where
//! colossus-legal's CONFIGURATION becomes those values; a second consumer of the
//! crate would have its own block and its own keys (Standing Rule 11).

use colossus_chat::CacheTtl;

use crate::domain::llm_effort::Effort;

/// The chat's configuration, read from the settings store.
#[derive(Debug, Clone, PartialEq)]
pub struct QuestionChatParams {
    /// Which `llm_models` row answers. Must be active and grounded — refused at
    /// save (`settings_write`) and at boot (`main.rs`) otherwise.
    pub model: String,
    /// The system prompt file, in the template directory.
    pub prompt_file: String,
    /// The case narrative file, in the template directory. Boot refuses without it
    /// (GO ruling on STOP 3).
    pub narrative_file: String,
    /// Output cap per call, thinking included.
    pub max_tokens: u32,
    /// Thinking effort; `None` sends no effort key.
    pub effort: Option<Effort>,
    /// Most model calls one reply may take.
    pub max_tool_rounds: u32,
    /// Room kept free in the context window for history and reply.
    pub context_headroom_tokens: u32,
    /// Cache lifetime for every breakpoint.
    pub cache_ttl: CacheTtl,
    /// Server-side compaction trigger; `None` = off (the stored 0).
    pub compaction_trigger_tokens: Option<u32>,
    /// Most model replies one person's thread on one question may hold.
    pub max_turns: u32,
    /// How long the browser waits with no event before giving up.
    pub client_idle_timeout_secs: u32,
    /// The witness's name on threads and in the package (case data).
    pub witness_display_name: String,
}

// KEYS: the stable identifiers. Renaming one is a migration.
pub const KEY_QUESTION_CHAT_MODEL: &str = "question_chat_model";
pub const KEY_QUESTION_CHAT_PROMPT_FILE: &str = "question_chat_prompt_file";
pub const KEY_CHAT_CASE_NARRATIVE_FILE: &str = "chat_case_narrative_file";
pub const KEY_QUESTION_CHAT_MAX_TOKENS: &str = "question_chat_max_tokens";
pub const KEY_QUESTION_CHAT_EFFORT: &str = "question_chat_effort";
pub const KEY_QUESTION_CHAT_MAX_TOOL_ROUNDS: &str = "question_chat_max_tool_rounds";
pub const KEY_QUESTION_CHAT_CONTEXT_HEADROOM_TOKENS: &str = "question_chat_context_headroom_tokens";
pub const KEY_QUESTION_CHAT_CACHE_TTL: &str = "question_chat_cache_ttl";
pub const KEY_QUESTION_CHAT_COMPACTION_TRIGGER_TOKENS: &str =
    "question_chat_compaction_trigger_tokens";
pub const KEY_QUESTION_CHAT_MAX_TURNS: &str = "question_chat_max_turns";
pub const KEY_QUESTION_CHAT_CLIENT_IDLE_TIMEOUT_SECS: &str =
    "question_chat_client_idle_timeout_secs";
pub const KEY_CHAT_WITNESS_DISPLAY_NAME: &str = "chat_witness_display_name";

/// The smallest compaction trigger the API accepts (its documented minimum).
///
/// STRUCTURAL: the provider's own floor, not a tunable — a stored value below it
/// would be refused by the API mid-call, so the reader refuses it at boot instead.
pub const COMPACTION_TRIGGER_MIN: u32 = 50_000;

/// Every key this block reads — counted at boot and walked by the store tests.
pub const QUESTION_CHAT_PARAM_KEYS: &[&str] = &[
    KEY_QUESTION_CHAT_MODEL,
    KEY_QUESTION_CHAT_PROMPT_FILE,
    KEY_CHAT_CASE_NARRATIVE_FILE,
    KEY_QUESTION_CHAT_MAX_TOKENS,
    KEY_QUESTION_CHAT_EFFORT,
    KEY_QUESTION_CHAT_MAX_TOOL_ROUNDS,
    KEY_QUESTION_CHAT_CONTEXT_HEADROOM_TOKENS,
    KEY_QUESTION_CHAT_CACHE_TTL,
    KEY_QUESTION_CHAT_COMPACTION_TRIGGER_TOKENS,
    KEY_QUESTION_CHAT_MAX_TURNS,
    KEY_QUESTION_CHAT_CLIENT_IDLE_TIMEOUT_SECS,
    KEY_CHAT_WITNESS_DISPLAY_NAME,
];

#[cfg(test)]
impl QuestionChatParams {
    /// The seeded TEXT rows, as the migration writes them — for tests only. The
    /// count rows carry bounds and are in [`Self::for_test_count_rows`].
    pub fn for_test_values() -> std::collections::HashMap<&'static str, String> {
        [
            (KEY_QUESTION_CHAT_MODEL, "claude-opus-5"),
            (KEY_QUESTION_CHAT_PROMPT_FILE, "question_chat_prompt_v1.md"),
            (KEY_CHAT_CASE_NARRATIVE_FILE, "case_narrative_v1.md"),
            (KEY_QUESTION_CHAT_EFFORT, "high"),
            (KEY_QUESTION_CHAT_CACHE_TTL, "1h"),
            (KEY_CHAT_WITNESS_DISPLAY_NAME, "Marie"),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect()
    }

    /// The seeded COUNT rows with their bounds: `(key, value, min, max)`.
    pub fn for_test_count_rows() -> [(&'static str, &'static str, f64, f64); 6] {
        [
            (KEY_QUESTION_CHAT_MAX_TOKENS, "16000", 256.0, 64000.0),
            (KEY_QUESTION_CHAT_MAX_TOOL_ROUNDS, "6", 1.0, 20.0),
            (
                KEY_QUESTION_CHAT_CONTEXT_HEADROOM_TOKENS,
                "80000",
                0.0,
                500000.0,
            ),
            (
                KEY_QUESTION_CHAT_COMPACTION_TRIGGER_TOKENS,
                "700000",
                0.0,
                2000000.0,
            ),
            (KEY_QUESTION_CHAT_MAX_TURNS, "200", 1.0, 2000.0),
            (
                KEY_QUESTION_CHAT_CLIENT_IDLE_TIMEOUT_SECS,
                "150",
                30.0,
                900.0,
            ),
        ]
    }

    /// The block as the seeded store would build it.
    pub fn for_test() -> Self {
        Self {
            model: "claude-opus-5".into(),
            prompt_file: "question_chat_prompt_v1.md".into(),
            narrative_file: "case_narrative_v1.md".into(),
            max_tokens: 16_000,
            effort: Some(Effort::High),
            max_tool_rounds: 6,
            context_headroom_tokens: 80_000,
            cache_ttl: CacheTtl::OneHour,
            compaction_trigger_tokens: Some(700_000),
            max_turns: 200,
            client_idle_timeout_secs: 150,
            witness_display_name: "Marie".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording::tests::seeded_value_in;

    const SEED_MIGRATION: &str =
        "pipeline_migrations/20260921140958_chat_discussions_and_grounded_flag.sql";

    /// Every key is seeded by the migration, with the fixture's value — so the test
    /// snapshot cannot drift from the store the migration builds.
    #[test]
    fn every_parameter_is_seeded_with_the_fixture_value() {
        let sql = std::fs::read_to_string(SEED_MIGRATION).expect("the seed migration is readable");
        let texts = QuestionChatParams::for_test_values();
        let counts = QuestionChatParams::for_test_count_rows();
        assert_eq!(texts.len() + counts.len(), QUESTION_CHAT_PARAM_KEYS.len());
        for key in QUESTION_CHAT_PARAM_KEYS {
            let seeded =
                seeded_value_in(&sql, key).unwrap_or_else(|| panic!("{key} is not seeded"));
            let fixture = texts
                .get(key)
                .cloned()
                .or_else(|| counts.iter().find(|c| c.0 == *key).map(|c| c.1.to_string()))
                .unwrap_or_else(|| panic!("{key} is in no fixture"));
            assert_eq!(seeded, fixture, "{key}: migration and fixture disagree");
        }
    }
}
