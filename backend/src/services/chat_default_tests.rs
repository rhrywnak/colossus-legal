// Tests for `services::chat_default` (CC_TASK_CHAT_DEFAULT_MODEL_v1).
//
// ## Why the rule is pure, and why that matters here
//
// This predicate decides whether the process STARTS. A check exercisable only by
// booting a server against a database is a check nobody runs — and the whole
// point of the task is that the previous version of this decision was a
// `tracing::error!` in a log nobody was tailing while PROD answered 400 to every
// unqualified `/ask`. Split out, all four faults are a unit test, and the async
// half in `main.rs` does nothing but fetch a row and exit.
//
// ## The four faults are four REMEDIES
//
// They are not decoration. `NoSuchModel` means check the id for a typo;
// `Inactive` means press the Admin toggle; `NotAnthropic` means the row is fine
// and the CHOICE is wrong; `ProviderFailed` means the row is right and the
// credentials are not. Collapsing them into one "bad default" would hand an
// operator a refused deploy and no first move (Standing Rule 1).

use super::{verify_chat_default, ChatDefaultFault};
use crate::repositories::pipeline_repository::models::LlmModelRecord;

/// One `llm_models` row. Only the three columns this rule reads are meaningful;
/// the rest are the struct's shape, not the test's subject.
fn row(id: &str, provider: &str, is_active: bool) -> LlmModelRecord {
    LlmModelRecord {
        id: id.to_string(),
        display_name: id.to_string(),
        provider: provider.to_string(),
        api_endpoint: None,
        max_context_tokens: None,
        max_output_tokens: Some(8000),
        cost_per_input_token: None,
        cost_per_output_token: None,
        is_active,
        created_at: chrono::Utc::now(),
        notes: None,
        default_temperature: None,
        temperature_mode: None,
        timeout_secs: None,
        structured_output_mode: None,
        max_concurrency: None,
        billing_class: "billed".to_string(),
    }
}

/// The happy path: a live Anthropic row that built a provider.
#[test]
fn a_live_anthropic_default_is_accepted() {
    let model = row("claude-opus-5", "anthropic", true);
    assert_eq!(verify_chat_default(Some(&model), true), Ok(()));
}

/// A DEACTIVATED row is refused — and says so, rather than "no such model".
///
/// This is the PROD failure of 2026-09-19 in one assertion: the row was there
/// the whole time, `is_active = false`, and the service came up anyway.
#[test]
fn an_inactive_default_is_refused_as_inactive() {
    let model = row("claude-sonnet-4-6", "anthropic", false);
    assert_eq!(
        verify_chat_default(Some(&model), false),
        Err(ChatDefaultFault::Inactive)
    );
}

/// A MISSING row is a different fault from a deactivated one.
///
/// Different remedy — add the row or fix the typo, rather than press a toggle —
/// so the two must not be one state (Standing Rule 1). The assertion that they
/// are `!=` is the point; a rule returning the same fault for both would pass
/// each test above on its own.
#[test]
fn a_missing_default_is_refused_as_no_such_model() {
    assert_eq!(
        verify_chat_default(None, false),
        Err(ChatDefaultFault::NoSuchModel)
    );
    let deactivated = row("claude-sonnet-4-6", "anthropic", false);
    assert_ne!(
        verify_chat_default(None, false),
        verify_chat_default(Some(&deactivated), false),
        "a missing row and a deactivated one need different first moves"
    );
}

/// A live vLLM row is a legal `llm_models` row and an illegal Chat default.
///
/// `build_chat_providers` skips every non-Anthropic provider, so this id could
/// never be in the map — and without this arm it would be reported as
/// `ProviderFailed`, sending an operator to look at credentials for a model
/// whose only problem is that Chat cannot dispatch to it at all.
#[test]
fn a_vllm_default_is_refused_as_not_anthropic() {
    let model = row("unsloth/Qwen3.8-27B-NVFP4", "vllm", true);
    assert_eq!(
        verify_chat_default(Some(&model), false),
        Err(ChatDefaultFault::NotAnthropic)
    );
}

/// Live, Anthropic, and STILL not in the map: the row is right and this process
/// could not honour it.
///
/// `AnthropicProvider::new` can refuse a row (`main::build_chat_providers`, the
/// `Err` arm of the provider loop). The row needs no edit; the deployment does.
#[test]
fn a_live_row_that_built_no_provider_is_refused_as_provider_failed() {
    let model = row("claude-opus-5", "anthropic", true);
    assert_eq!(
        verify_chat_default(Some(&model), false),
        Err(ChatDefaultFault::ProviderFailed)
    );
}

/// The four faults are checked in the order an operator would want them.
///
/// A row that is BOTH deactivated and non-Anthropic reports `Inactive`, because
/// reactivating it is the first thing anyone would try and the second fault is
/// only interesting afterwards. Pinned so a reordering of the guards is a
/// visible change rather than a silent one.
#[test]
fn a_deactivated_non_anthropic_row_reports_inactive_first() {
    let model = row("Qwen/Qwen2.5-14B-Instruct-AWQ", "vllm", false);
    assert_eq!(
        verify_chat_default(Some(&model), false),
        Err(ChatDefaultFault::Inactive)
    );
}

/// Every fault carries a remedy, and no two remedies read alike.
///
/// The remedy is the whole reason the enum has four variants rather than being
/// a `bool`. Two variants sharing a sentence would be two variants nobody can
/// act on differently.
#[test]
fn every_fault_carries_a_distinct_remedy() {
    let faults = [
        ChatDefaultFault::NoSuchModel,
        ChatDefaultFault::Inactive,
        ChatDefaultFault::NotAnthropic,
        ChatDefaultFault::ProviderFailed,
    ];
    let mut seen: Vec<&str> = Vec::new();
    for fault in &faults {
        let remedy = fault.remedy();
        assert!(!remedy.trim().is_empty(), "{fault:?} has no remedy");
        assert!(
            !seen.contains(&remedy),
            "{fault:?} repeats another fault's remedy: {remedy}"
        );
        seen.push(remedy);
    }
    assert_eq!(
        seen.len(),
        4,
        "a fault was added without a remedy of its own"
    );
}
