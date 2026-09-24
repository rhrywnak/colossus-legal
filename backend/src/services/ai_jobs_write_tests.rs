// Tests for `services::ai_jobs_write::plan_writes` — every job-level refusal a
// Save can meet, checked before anything is written.

use super::*;
use crate::domain::wording_admin::AdminWording;
use crate::services::ai_jobs_compose::tests::prod_rows;
use crate::services::ai_jobs_meta::job_words;
use crate::services::ai_jobs_options::tests::prod_models;

fn change(key: &str, value: &str) -> AiJobChangeDto {
    AiJobChangeDto {
        key: key.into(),
        value: value.into(),
    }
}

fn scan() -> HashSet<String> {
    ["claude-opus-5", "claude-opus-5-5", "qwen"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Run `plan_writes` for `job` against PROD's rows and models.
fn plan(
    job: &str,
    changes: &[AiJobChangeDto],
    fixed: bool,
) -> Result<Vec<(String, String)>, String> {
    let wording = AdminWording::for_test();
    let words = job_words(job, &wording.job_rows);
    let models = prod_models();
    let scan_ids = scan();
    let rows: Vec<AiJobSettingRow> = prod_rows()
        .into_iter()
        .filter(|r| r.ai_job == job)
        .collect();
    let check = WriteCheck {
        job,
        words: &words,
        w: &wording.ai_jobs,
        models: &models,
        scan_ids: &scan_ids,
        fixed,
    };
    plan_writes(&rows, changes, &check)
        .map(|w| w.into_iter().map(|(r, v)| (r.key.clone(), v)).collect())
}

#[test]
fn a_changed_model_is_planned_trimmed() {
    let got = plan(
        "answer_analysis",
        &[change("practice_read_model", " claude-opus-5-5 ")],
        false,
    );
    assert_eq!(
        got,
        Ok(vec![(
            "practice_read_model".into(),
            "claude-opus-5-5".into()
        )])
    );
}

#[test]
fn both_controls_of_a_job_are_planned_together() {
    let got = plan(
        "answer_analysis",
        &[
            change("practice_read_model", "claude-opus-5-5"),
            change("practice_read_prompt_file", "practice_read_prompt_v4.md"),
        ],
        false,
    );
    assert_eq!(got.map(|w| w.len()), Ok(2));
}

#[test]
fn an_unchanged_control_is_skipped_and_nothing_at_all_is_refused() {
    let got = plan(
        "answer_analysis",
        &[
            change("practice_read_model", "claude-opus-5"),
            change("practice_read_prompt_file", "practice_read_prompt_v4.md"),
        ],
        false,
    );
    assert_eq!(got.map(|w| w.len()), Ok(1));
    let got = plan(
        "answer_analysis",
        &[change("practice_read_model", "claude-opus-5")],
        false,
    );
    assert_eq!(
        got,
        Err("Nothing changed: Answer analysis already uses that.".into())
    );
}

#[test]
fn a_setting_of_another_job_is_refused_by_name() {
    let got = plan(
        "answer_analysis",
        &[change("question_chat_model", "claude-opus-5")],
        false,
    );
    assert_eq!(
        got,
        Err("question_chat_model is not one of Answer analysis's settings.".into())
    );
}

#[test]
fn a_model_the_job_cannot_call_is_refused_with_the_rows_own_sentence() {
    let got = plan(
        "discuss_chat",
        &[change("question_chat_model", "qwen")],
        false,
    );
    assert_eq!(
        got,
        Err(
            "Qwen3.8 27B has no \"Quotes checked\" tick on Prompt Management → Models, so the \
             Discuss chat can't run. Pick another model."
                .into()
        )
    );
    let got = plan("chat_page", &[change("chat_default_model", "qwen")], false);
    assert!(got.is_err_and(|e| e.contains("local model")));
    let got = plan(
        "answer_analysis",
        &[change("practice_read_model", "claude-sonnet-5")],
        false,
    );
    assert!(got.is_err_and(|e| e.contains("switched off")));
}

#[test]
fn a_model_the_server_fixes_cannot_be_saved_but_its_instructions_can() {
    let got = plan(
        "theme_scan",
        &[change("theme_scan_default_model", "claude-opus-5-5")],
        true,
    );
    assert!(got.is_err_and(
        |e| e.starts_with("Theme scan takes its model from the server's configuration")
    ));
    let got = plan(
        "theme_scan",
        &[change("theme_scan_prompt_file", "theme_scan_prompt_v2.md")],
        true,
    );
    assert_eq!(got.map(|w| w.len()), Ok(1));
}

#[test]
fn the_thinking_level_is_not_checked_as_a_model() {
    let got = plan(
        "discuss_effort",
        &[change("question_chat_effort", "high")],
        false,
    );
    assert_eq!(
        got,
        Ok(vec![("question_chat_effort".into(), "high".into())])
    );
}

#[test]
fn a_job_no_setting_carries_is_refused_by_its_token() {
    let err = rows_of_job(prod_rows(), "no_such_job").expect_err("no row carries it");
    assert!(matches!(err, AiJobsWriteError::UnknownJob(_)));
    assert_eq!(err.to_string(), "there is no AI job called no_such_job");
    let rows = rows_of_job(prod_rows(), "answer_analysis").expect("two rows carry it");
    assert_eq!(rows.len(), 2);
}

#[test]
fn a_half_saved_change_names_what_was_saved_and_why_the_rest_was_not() {
    let wording = AdminWording::for_test();
    let err = partial_refusal(
        &wording.ai_jobs,
        &["Claude Opus 5.5".to_string()],
        "the file is missing",
    );
    assert!(matches!(err, AiJobsWriteError::Partial(_)));
    assert_eq!(
        err.to_string(),
        "Claude Opus 5.5 was saved; the rest was not: the file is missing"
    );
}
