//! Which AI job uses which model and which instructions file — the "Used for"
//! column on Prompt Management → Models and → Prompts, and the refusals that
//! keep a job from losing what it runs on (CC_TASK_MODEL_JOBS_PANEL_v1, B2).
//!
//! ## Domain note: why switching off a model a job uses is refused
//!
//! Switching off the Chat default or the Discuss chat's model does not just stop
//! that job: the next restart REFUSES TO BOOT (`assert_chat_default_is_live`,
//! `chat_model_check::assert_chat_model_usable`). A click on the Models list must
//! not be able to take the whole application down at its next deploy. The
//! message names the job so the person knows where to move it first.

use std::collections::BTreeMap;

use crate::domain::wording_ai_job_rows::AiJobRowsWording;
use crate::domain::wording_ai_jobs::AiJobsWording;
use crate::domain::wording_templates::render;
use crate::repositories::pipeline_repository::ai_job_settings::{
    list_ai_job_settings, AiJobSettingRow,
};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::ai_jobs_compose::{JOIN, ROLE_INSTRUCTIONS, ROLE_MODEL};
use crate::services::ai_jobs_meta::job_words;
use crate::services::ai_jobs_options::{JOB_ORDER, JOB_THEME_SCAN};
use crate::state::AppState;

/// Job names by model id and by file name, in panel words.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Usage {
    pub models: BTreeMap<String, Vec<String>>,
    pub files: BTreeMap<String, Vec<String>>,
}

impl Usage {
    /// "Answer analysis · Chat page" for a model, or `None` when no job uses it.
    pub fn model(&self, id: &str) -> Option<String> {
        self.models.get(id).map(|jobs| jobs.join(JOIN))
    }

    /// The same for an instructions file.
    pub fn file(&self, name: &str) -> Option<String> {
        self.files.get(name).map(|jobs| jobs.join(JOIN))
    }
}

/// Who uses what, read now.
///
/// A job whose model the server's configuration fixes (THEME_SCAN_MODEL) is
/// counted against THAT model, not its settings row: the row is not what runs.
///
/// # Errors
/// A database failure reading the job settings.
pub async fn usage(state: &AppState) -> Result<Usage, PipelineRepoError> {
    let rows = list_ai_job_settings(&state.pipeline_pool).await?;
    let settings = state.settings.current();
    Ok(usage_from(
        rows,
        state.config.theme_scan_model.as_deref(),
        &settings.admin_wording.job_rows,
    ))
}

/// [`usage`] from rows already read — pure, so the rule is tested directly.
pub fn usage_from(
    mut rows: Vec<AiJobSettingRow>,
    fixed: Option<&str>,
    words: &AiJobRowsWording,
) -> Usage {
    // Panel order, so "Used for" reads the jobs as Overview lists them.
    rows.sort_by_key(|r| {
        JOB_ORDER
            .iter()
            .position(|j| *j == r.ai_job)
            .unwrap_or(usize::MAX)
    });
    let mut out = Usage::default();
    for row in &rows {
        let title = job_words(&row.ai_job, words).title;
        let (map, key) = match (row.ai_role.as_str(), row.ai_job.as_str(), fixed) {
            (ROLE_MODEL, JOB_THEME_SCAN, Some(model)) => (&mut out.models, model.to_string()),
            (ROLE_MODEL, _, _) => (&mut out.models, row.value.clone()),
            (ROLE_INSTRUCTIONS, _, _) => (&mut out.files, row.value.clone()),
            _ => continue,
        };
        let target = map.entry(key).or_default();
        if !target.contains(&title) {
            target.push(title);
        }
    }
    out
}

/// The refusal for switching off or deleting `model` (display name `name`), or
/// `None` when no job uses it.
pub fn model_refusal(w: &AiJobsWording, usage: &Usage, id: &str, name: &str) -> Option<String> {
    let jobs = usage.model(id)?;
    Some(render(
        &w.refuse_model_in_use,
        &[("model", name), ("jobs", jobs.as_str())],
    ))
}

/// The refusal for creating, editing or deleting an instructions file: it names
/// the jobs when a job uses the file, and says the folder is read-only otherwise
/// (ruling Q3 — new versions arrive with a release).
pub fn file_refusal(w: &AiJobsWording, usage: &Usage, file: &str) -> String {
    match usage.file(file) {
        Some(jobs) => render(
            &w.refuse_file_in_use,
            &[("file", file), ("jobs", jobs.as_str())],
        ),
        None => w.refuse_files_read_only.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::wording_admin::AdminWording;
    use crate::services::ai_jobs_compose::tests::prod_rows;

    #[test]
    fn prods_store_says_who_uses_what_in_panel_words() {
        let w = AdminWording::for_test();
        let used = usage_from(prod_rows(), Some("qwen"), &w.job_rows);
        assert_eq!(
            used.model("claude-opus-5-5").as_deref(),
            Some("Discuss chat on an answer")
        );
        assert_eq!(
            used.model("claude-opus-5").as_deref(),
            Some("Answer analysis · Chat page")
        );
        // The scan's row names Opus 5, but the server fixes Qwen — Qwen is what runs.
        assert_eq!(used.model("qwen").as_deref(), Some("Theme scan"));
        assert_eq!(
            used.file("case_narrative_v1.md").as_deref(),
            Some("Case story given to every chat")
        );
        assert_eq!(used.file("practice_read_prompt_v4.md"), None);
    }

    #[test]
    fn without_the_server_override_the_scan_row_counts() {
        let w = AdminWording::for_test();
        let used = usage_from(prod_rows(), None, &w.job_rows);
        assert_eq!(
            used.model("claude-opus-5").as_deref(),
            Some("Answer analysis · Chat page · Theme scan")
        );
        assert_eq!(used.model("qwen"), None);
    }

    #[test]
    fn a_used_model_is_refused_by_job_and_an_unused_one_is_not() {
        let w = AdminWording::for_test();
        let used = usage_from(prod_rows(), Some("qwen"), &w.job_rows);
        assert_eq!(
            model_refusal(&w.ai_jobs, &used, "claude-opus-5-5", "Claude Opus 5.5").as_deref(),
            Some(
                "Claude Opus 5.5 is used by Discuss chat on an answer. Pick another model for \
                  that job on Admin → Overview first."
            )
        );
        assert_eq!(
            model_refusal(&w.ai_jobs, &used, "claude-sonnet-5", "Claude Sonnet 5"),
            None
        );
    }

    #[test]
    fn a_file_write_names_the_job_or_says_the_folder_is_read_only() {
        let w = AdminWording::for_test();
        let used = usage_from(prod_rows(), Some("qwen"), &w.job_rows);
        assert_eq!(
            file_refusal(&w.ai_jobs, &used, "question_chat_prompt_v1.md"),
            "question_chat_prompt_v1.md is used by Discuss chat on an answer, so it can't be \
             changed or deleted."
        );
        assert_eq!(
            file_refusal(&w.ai_jobs, &used, "anything_else.md"),
            "Instruction files can't be changed here. New versions of these files arrive with a release."
        );
    }

    #[test]
    fn a_model_used_by_two_jobs_names_both_in_panel_order() {
        let mut usage = Usage::default();
        usage.models.insert(
            "claude-opus-5".into(),
            vec!["Answer analysis".into(), "Chat page".into()],
        );
        assert_eq!(
            usage.model("claude-opus-5").as_deref(),
            Some("Answer analysis · Chat page")
        );
        assert_eq!(usage.model("unused"), None);
    }
}

#[cfg(test)]
#[path = "ai_jobs_disk_tests.rs"]
mod disk_tests;
