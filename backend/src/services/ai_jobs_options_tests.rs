// Tests for `services::ai_jobs_options` — which models and files each job's
// dropdown offers, and what is wrong with the value in force.

use super::*;
use crate::domain::wording_ai_jobs::AiJobsWording;

/// A model row with the three facts the rules read.
pub(crate) fn model(
    id: &str,
    name: &str,
    provider: &str,
    active: bool,
    grounded: bool,
) -> LlmModelRecord {
    LlmModelRecord {
        id: id.into(),
        display_name: name.into(),
        provider: provider.into(),
        api_endpoint: None,
        max_context_tokens: None,
        max_output_tokens: None,
        cost_per_input_token: None,
        cost_per_output_token: None,
        is_active: active,
        created_at: chrono::Utc::now(),
        notes: None,
        default_temperature: None,
        temperature_mode: None,
        timeout_secs: None,
        structured_output_mode: None,
        max_concurrency: None,
        billing_class: "billed".into(),
        grounded,
    }
}

/// PROD's model list on 2026-09-24: two active grounded Anthropic models, one
/// active local model, and a switched-off Anthropic one.
pub(crate) fn prod_models() -> Vec<LlmModelRecord> {
    vec![
        model("claude-opus-5", "Claude Opus 5", "anthropic", true, true),
        model(
            "claude-opus-5-5",
            "Claude Opus 5.5",
            "anthropic",
            true,
            true,
        ),
        model("qwen", "Qwen3.8 27B", "vllm", true, false),
        model(
            "claude-sonnet-5",
            "Claude Sonnet 5",
            "anthropic",
            false,
            true,
        ),
    ]
}

fn all_scan() -> HashSet<String> {
    ["claude-opus-5", "claude-opus-5-5", "qwen"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn labels(options: &[AiJobOptionDto]) -> Vec<&str> {
    options.iter().map(|o| o.label.as_str()).collect()
}

#[test]
fn each_job_gets_the_rule_its_code_path_enforces() {
    assert_eq!(ModelRule::for_job(JOB_DISCUSS_CHAT), ModelRule::Grounded);
    assert_eq!(ModelRule::for_job(JOB_ANSWER_ANALYSIS), ModelRule::Active);
    assert_eq!(ModelRule::for_job(JOB_CHAT_PAGE), ModelRule::Anthropic);
    assert_eq!(ModelRule::for_job(JOB_THEME_SCAN), ModelRule::Scan);
    assert_eq!(ModelRule::for_job("a_job_added_later"), ModelRule::Active);
}

#[test]
fn the_discuss_chat_offers_only_active_models_with_quotes_checked() {
    let got = model_options(
        ModelRule::Grounded,
        &prod_models(),
        &all_scan(),
        "claude-opus-5-5",
    );
    assert_eq!(labels(&got), ["Claude Opus 5", "Claude Opus 5.5"]);
    assert_eq!(got.iter().filter(|o| o.current).count(), 1);
    assert!(got[1].current);
}

#[test]
fn the_answer_analysis_offers_every_active_model_local_included() {
    let got = model_options(
        ModelRule::Active,
        &prod_models(),
        &all_scan(),
        "claude-opus-5",
    );
    assert_eq!(
        labels(&got),
        ["Claude Opus 5", "Claude Opus 5.5", "Qwen3.8 27B"]
    );
}

#[test]
fn the_chat_page_offers_only_active_anthropic_models() {
    let got = model_options(
        ModelRule::Anthropic,
        &prod_models(),
        &all_scan(),
        "claude-opus-5",
    );
    assert_eq!(labels(&got), ["Claude Opus 5", "Claude Opus 5.5"]);
}

#[test]
fn the_theme_scan_offers_only_models_offered_for_scans() {
    let only_qwen: HashSet<String> = ["qwen".to_string()].into_iter().collect();
    let got = model_options(ModelRule::Scan, &prod_models(), &only_qwen, "qwen");
    assert_eq!(labels(&got), ["Qwen3.8 27B"]);
}

#[test]
fn a_value_in_force_the_rule_excludes_is_still_listed_as_current() {
    // Switched off, so the rule drops it — but it is what the job is set to.
    let got = model_options(
        ModelRule::Active,
        &prod_models(),
        &all_scan(),
        "claude-sonnet-5",
    );
    assert!(got
        .iter()
        .any(|o| o.value == "claude-sonnet-5" && o.current));
    // Not in the table at all: listed by its id, still current.
    let got = model_options(ModelRule::Active, &prod_models(), &all_scan(), "gone-model");
    assert_eq!(
        got.last().map(|o| (o.label.as_str(), o.current)),
        Some(("gone-model", true))
    );
}

#[test]
fn each_problem_is_named_distinctly() {
    let models = prod_models();
    let find = |id: &str| models.iter().find(|m| m.id == id);
    let scan = all_scan();
    assert_eq!(
        ModelRule::Active.problem(None, &scan),
        Some(ModelProblem::Missing)
    );
    assert_eq!(
        ModelRule::Active.problem(find("claude-sonnet-5"), &scan),
        Some(ModelProblem::Off)
    );
    assert_eq!(
        ModelRule::Grounded.problem(find("qwen"), &scan),
        Some(ModelProblem::Unquoted)
    );
    assert_eq!(
        ModelRule::Anthropic.problem(find("qwen"), &scan),
        Some(ModelProblem::Local)
    );
    let none: HashSet<String> = HashSet::new();
    assert_eq!(
        ModelRule::Scan.problem(find("qwen"), &none),
        Some(ModelProblem::NotScan)
    );
    assert_eq!(
        ModelRule::Grounded.problem(find("claude-opus-5-5"), &scan),
        None
    );
}

#[test]
fn a_version_is_read_from_the_file_name() {
    assert_eq!(
        split_version("question_chat_prompt_v1.md"),
        Some(("question_chat_prompt_v", 1))
    );
    assert_eq!(
        split_version("practice_read_prompt_v12.md"),
        Some(("practice_read_prompt_v", 12))
    );
    assert_eq!(split_version("case_narrative.md"), None);
    assert_eq!(split_version("notes_v2.txt"), None);
    assert_eq!(split_version("odd_v.md"), None);
    assert_eq!(split_version("odd_vx1.md"), None);
}

#[test]
fn the_instructions_list_is_the_family_in_version_order_with_its_markers() {
    let w = AiJobsWording::for_test();
    let files: Vec<String> = [
        "practice_read_prompt_v1.md",
        "practice_read_prompt_v10.md",
        "practice_read_prompt_v2.md",
        "practice_read_prompt_v3.md",
        "question_chat_prompt_v1.md",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let used: HashSet<String> = ["practice_read_prompt_v1.md".to_string()]
        .into_iter()
        .collect();
    let got = instruction_options("practice_read_prompt_v3.md", &files, &used, &w);
    let seen: Vec<(&str, Option<&str>, bool)> = got
        .iter()
        .map(|o| (o.label.as_str(), o.marker.as_deref(), o.current))
        .collect();
    assert_eq!(
        seen,
        [
            ("practice_read_prompt_v1.md", Some("used before"), false),
            ("practice_read_prompt_v2.md", Some("earlier version"), false),
            ("practice_read_prompt_v3.md", Some("in use"), true),
            (
                "practice_read_prompt_v10.md",
                Some("new, never used"),
                false
            ),
        ]
    );
}

#[test]
fn a_file_in_force_that_is_missing_from_the_folder_is_still_listed_in_use() {
    let w = AiJobsWording::for_test();
    let files = vec!["question_chat_prompt_v2.md".to_string()];
    let got = instruction_options("question_chat_prompt_v1.md", &files, &HashSet::new(), &w);
    assert!(got
        .iter()
        .any(|o| o.value == "question_chat_prompt_v1.md" && o.current));
    assert!(got
        .iter()
        .any(|o| o.value == "question_chat_prompt_v2.md" && !o.current));
}

#[test]
fn a_file_without_a_version_lists_only_itself() {
    let w = AiJobsWording::for_test();
    let files = vec!["narrative.md".to_string(), "narrative_v2.md".to_string()];
    let got = instruction_options("narrative.md", &files, &HashSet::new(), &w);
    assert_eq!(labels(&got), ["narrative.md"]);
}

#[test]
fn the_thinking_list_is_all_five_levels_in_plain_words() {
    let w = AiJobsWording::for_test();
    let got = effort_options("medium", &w);
    assert_eq!(labels(&got), ["Low", "Medium", "High", "Extra high", "Max"]);
    assert_eq!(got.iter().position(|o| o.current), Some(1));
    // "absent" is listed only when it is the value in force.
    let absent = effort_options("absent", &w);
    assert_eq!(absent.len(), 6);
    assert_eq!(
        absent.last().map(|o| o.label.as_str()),
        Some("The model's own default")
    );
}
