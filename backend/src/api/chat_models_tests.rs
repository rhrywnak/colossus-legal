//! Unit tests for [`super`] — the scan catalog's ordering, labels and default.
//!
//! These are the rules that stop a human starting a 148-candidate scan on a
//! metered endpoint by accident, so each is asserted rather than assumed. All
//! pure: `entry_for` and `local_first` take rows and return the payload.

use super::*;
use crate::repositories::pipeline_repository::models::LlmModelRecord;

/// A catalog row, classified. Everything not under test is left empty — the
/// fixture asserts nothing about columns these functions never read.
fn model(id: &str, display: &str, billing_class: &str) -> LlmModelRecord {
    LlmModelRecord {
        id: id.to_string(),
        display_name: display.to_string(),
        provider: "vllm".to_string(),
        api_endpoint: None,
        max_context_tokens: None,
        max_output_tokens: None,
        cost_per_input_token: None,
        cost_per_output_token: None,
        is_active: true,
        created_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
        notes: None,
        default_temperature: None,
        temperature_mode: None,
        timeout_secs: None,
        structured_output_mode: None,
        max_concurrency: None,
        billing_class: billing_class.to_string(),
    }
}

fn entries(rows: Vec<LlmModelRecord>, default: &str) -> Vec<ChatModelEntry> {
    classify(rows, default).0
}

// ── The label ────────────────────────────────────────────────────────────────

/// The cost warning is attached server-side, so no client composes it.
#[test]
fn a_billed_model_carries_its_warning_in_the_label() {
    let listed = entries(vec![model("claude-opus-4-8", "Opus 4.8", "billed")], "x");
    assert_eq!(listed[0].display_label, "Opus 4.8 (API — billed)");
    assert_eq!(listed[0].billing_class, "billed");
    // The bare name survives beside it: a client that wants the name without the
    // warning must not have to strip the suffix back off.
    assert_eq!(listed[0].display_name, "Opus 4.8");
}

/// A local model is undecorated — a label on every row is a label nobody reads.
#[test]
fn a_local_model_is_not_decorated() {
    let listed = entries(vec![model("qwen-32b", "Qwen 32B", "local")], "x");
    assert_eq!(listed[0].display_label, "Qwen 32B");
    assert_eq!(listed[0].billing_class, "local");
}

/// A model this build cannot classify is NOT offered.
///
/// Serving it unlabelled is the one failure this mechanism exists to prevent: a
/// metered model presented as though scanning across it were free. Its absence
/// is visible (and logged by name); a wrong label is not.
#[test]
fn a_model_whose_class_cannot_be_read_is_not_offered() {
    let listed = entries(
        vec![
            model("qwen-32b", "Qwen 32B", "local"),
            model("mystery", "Mystery", "free-tier"),
        ],
        "x",
    );
    assert_eq!(listed.len(), 1, "the unreadable model must not be listed");
    assert_eq!(listed[0].model_id, "qwen-32b");
}

/// …and its absence is REPORTED, not merely logged.
///
/// A picker one row shorter than the database looks exactly like a complete one.
/// The person who can repair the row is the one reading the screen, and a server
/// log they are not tailing does not reach them.
#[test]
fn a_dropped_model_comes_back_as_a_warning_naming_it() {
    let (listed, warnings) = classify(
        vec![
            model("qwen-32b", "Qwen 32B", "local"),
            model("mystery", "Mystery", "free-tier"),
        ],
        "x",
    );

    assert_eq!(listed.len(), 1);
    assert_eq!(warnings.len(), 1, "the drop must be reported");
    assert!(warnings[0].contains("mystery"), "{}", warnings[0]);
    assert!(warnings[0].contains("free-tier"), "{}", warnings[0]);
}

/// A healthy catalog reports nothing — a warnings list that is never empty is a
/// warnings list nobody reads.
#[test]
fn a_healthy_catalog_carries_no_warnings() {
    let (_, warnings) = classify(
        vec![
            model("qwen-32b", "Qwen 32B", "local"),
            model("opus", "Opus", "billed"),
        ],
        "x",
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

// ── The ordering ─────────────────────────────────────────────────────────────

#[test]
fn local_models_come_first() {
    let listed = entries(
        vec![
            model("opus", "Opus", "billed"),
            model("qwen-14b", "Qwen 14B", "local"),
            model("sonnet", "Sonnet", "billed"),
            model("qwen-32b", "Qwen 32B", "local"),
        ],
        "x",
    );
    let (ordered, _) = local_first(listed, "x".to_string());

    let ids: Vec<&str> = ordered.iter().map(|e| e.model_id.as_str()).collect();
    assert_eq!(ids, vec!["qwen-14b", "qwen-32b", "opus", "sonnet"]);
}

/// Within a class the incoming order survives — the rule is "local first, then
/// as listed", not a reshuffle that moves rows for no stated reason.
#[test]
fn the_sort_is_stable_within_a_class() {
    let listed = entries(
        vec![
            model("z-local", "Z", "local"),
            model("a-local", "A", "local"),
        ],
        "x",
    );
    let (ordered, _) = local_first(listed, "x".to_string());

    let ids: Vec<&str> = ordered.iter().map(|e| e.model_id.as_str()).collect();
    assert_eq!(ids, vec!["z-local", "a-local"], "repository order survives");
}

// ── The default ──────────────────────────────────────────────────────────────

#[test]
fn the_configured_default_is_kept_when_it_is_local() {
    let listed = entries(
        vec![
            model("qwen-14b", "Qwen 14B", "local"),
            model("qwen-32b", "Qwen 32B", "local"),
        ],
        "qwen-32b",
    );
    let (ordered, default) = local_first(listed, "qwen-32b".to_string());

    assert_eq!(default, "qwen-32b");
    assert!(
        ordered
            .iter()
            .find(|e| e.model_id == "qwen-32b")
            .unwrap()
            .is_default
    );
    assert!(
        !ordered
            .iter()
            .find(|e| e.model_id == "qwen-14b")
            .unwrap()
            .is_default
    );
}

/// THE RULE THAT COSTS MONEY IF IT IS WRONG. A configured default that is BILLED
/// does not get selected — the page opens on a local model instead.
#[test]
fn a_billed_configured_default_is_overridden_by_the_first_local_model() {
    let listed = entries(
        vec![
            model("opus", "Opus", "billed"),
            model("qwen-14b", "Qwen 14B", "local"),
        ],
        "opus",
    );
    let (ordered, default) = local_first(listed, "opus".to_string());

    assert_eq!(
        default, "qwen-14b",
        "the page must not open on a metered model"
    );
    assert!(
        !ordered
            .iter()
            .find(|e| e.model_id == "opus")
            .unwrap()
            .is_default
    );
    assert!(
        ordered
            .iter()
            .find(|e| e.model_id == "qwen-14b")
            .unwrap()
            .is_default
    );
}

/// With no local model there is nothing free to fall back to, and the configured
/// default stands. The honest floor: selecting nothing would leave the Run button
/// dead with no explanation.
#[test]
fn with_no_local_model_the_configured_default_stands() {
    let listed = entries(
        vec![
            model("opus", "Opus", "billed"),
            model("sonnet", "Sonnet", "billed"),
        ],
        "sonnet",
    );
    let (ordered, default) = local_first(listed, "sonnet".to_string());

    assert_eq!(default, "sonnet");
    assert!(
        ordered
            .iter()
            .find(|e| e.model_id == "sonnet")
            .unwrap()
            .is_default
    );
}

/// `is_default` and `default_model` cannot disagree, or the picker highlights one
/// row and runs another.
#[test]
fn exactly_one_entry_is_marked_default_and_it_is_the_one_named() {
    let listed = entries(
        vec![
            model("opus", "Opus", "billed"),
            model("qwen-14b", "Qwen 14B", "local"),
            model("qwen-32b", "Qwen 32B", "local"),
        ],
        "opus",
    );
    let (ordered, default) = local_first(listed, "opus".to_string());

    let marked: Vec<&str> = ordered
        .iter()
        .filter(|e| e.is_default)
        .map(|e| e.model_id.as_str())
        .collect();
    assert_eq!(marked, vec![default.as_str()]);
}

/// An empty catalog does not panic and does not invent a model.
#[test]
fn an_empty_catalog_keeps_the_configured_default_and_lists_nothing() {
    let (ordered, default) = local_first(Vec::new(), "configured".to_string());
    assert!(ordered.is_empty());
    assert_eq!(default, "configured");
}

// ── The scan picker's default, three layers deep (task R2 / 10e) ────────────

/// The env var wins wherever it is set — no deploy changes with the new row.
#[test]
fn the_env_var_still_decides_the_scan_default_when_it_is_set() {
    assert_eq!(
        scan_default_model(Some("claude-opus-4-8"), "claude-opus-5", "chat-default"),
        "claude-opus-4-8"
    );
    assert_eq!(
        scan_default_source(Some("claude-opus-4-8"), "claude-opus-5"),
        "THEME_SCAN_MODEL env var"
    );
}

/// The step that did not exist before .391.
///
/// Beneath the env var the fallback was the CHAT default, which is scan-ineligible
/// by design — so `is_default` came back false for every listed model and the
/// browser fell through to `catalog.models[0]`. The picker's default was decided
/// by however the registry happened to sort, which is not a decision anybody made.
#[test]
fn the_settings_row_decides_when_the_env_var_is_unset() {
    assert_eq!(
        scan_default_model(None, "claude-opus-5", "chat-default"),
        "claude-opus-5"
    );
    assert_eq!(
        scan_default_source(None, "claude-opus-5"),
        "theme_scan_default_model settings row"
    );
}

/// A blank row is not an answer.
///
/// Guarding whitespace as well as empty: a row edited to a space on the Settings
/// page would otherwise make the picker default to `" "`, which matches no model
/// and renders as nothing selected.
#[test]
fn a_blank_settings_row_falls_through_to_the_chat_default() {
    assert_eq!(scan_default_model(None, "", "chat-default"), "chat-default");
    assert_eq!(
        scan_default_model(None, "   ", "chat-default"),
        "chat-default"
    );
    assert_eq!(
        scan_default_source(None, "   "),
        "chat default (last resort)"
    );
}
