//! Save and Switch back on Admin → Overview's jobs panel
//! (CC_TASK_MODEL_JOBS_PANEL_v1, board 3 — Stage B).
//!
//! ## One write path
//!
//! Every value is written by `settings_write::set_setting` — the SAME guarded
//! path the Settings page uses: the row must exist, the value must differ, it
//! must fit its own kind and bounds, a named file must be in the folder, the
//! Discuss chat's model must be active and "Quotes checked", and the whole store
//! must still build with it. The panel adds only the checks that belong to a JOB:
//! the setting must be one of the job's, a model must be one the job's code can
//! call (the dropdown's own rule), and a model the server's configuration fixes
//! cannot be saved here.
//!
//! ## Domain note: Switch back is an ordinary Save of the previous value
//!
//! It writes the history's last `old_value` through the same path, so it is
//! recorded like any change and a second Switch back returns to where the first
//! began. Nothing is overwritten that the history cannot restore.

use std::collections::HashSet;

use crate::domain::wording_ai_jobs::AiJobsWording;
use crate::domain::wording_templates::render;
use crate::dto::ai_jobs::{AiJobChangeDto, AiJobSavedDto};
use crate::repositories::pipeline_repository::ai_job_settings::{
    list_ai_job_settings, list_changes_for_keys, AiJobSettingRow,
};
use crate::repositories::pipeline_repository::models::{
    list_all_models, list_scan_eligible_models, LlmModelRecord,
};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::ai_jobs::{panel, AiJobsError};
use crate::services::ai_jobs_compose::{dollars, ROLE_MODEL};
use crate::services::ai_jobs_meta::{job_words, label_for, model_problem, JobWords};
use crate::services::ai_jobs_options::{JOB_CASE_STORY, JOB_DISCUSS_CHAT, JOB_THEME_SCAN};
use crate::services::chat_keepwarm_button::reload_estimate;
use crate::services::settings_store::SettingsError;
use crate::services::settings_template_file::TemplateDir;
use crate::services::settings_write::set_setting;
use crate::state::AppState;

/// Why a Save or Switch back did not happen — each a different answer.
#[derive(Debug, thiserror::Error)]
pub enum AiJobsWriteError {
    /// No setting carries this job token.
    #[error("there is no AI job called {0}")]
    UnknownJob(String),
    /// A refusal a person caused, already in plain words for the page.
    #[error("{0}")]
    Refused(String),
    /// The settings store's own guard refused the value.
    #[error("{0}")]
    Settings(#[from] SettingsError),
    /// The first of two settings saved and the second did not.
    #[error("{0}")]
    Partial(String),
    #[error("the job settings or the models could not be read: {0}")]
    Read(String),
    #[error("{0}")]
    Panel(#[from] AiJobsError),
}

impl From<PipelineRepoError> for AiJobsWriteError {
    fn from(e: PipelineRepoError) -> Self {
        Self::Read(e.to_string())
    }
}

impl From<sqlx::Error> for AiJobsWriteError {
    fn from(e: sqlx::Error) -> Self {
        Self::Read(e.to_string())
    }
}

/// Save `changes` for `job`, as `actor`.
///
/// # Errors
/// [`AiJobsWriteError`], naming what refused the change.
pub async fn save(
    state: &AppState,
    job: &str,
    changes: &[AiJobChangeDto],
    actor: &str,
) -> Result<AiJobSavedDto, AiJobsWriteError> {
    let settings = state.settings.current();
    let w = &settings.admin_wording.ai_jobs;
    let words = job_words(job, &settings.admin_wording.job_rows);
    let rows = job_rows(state, job).await?;
    let models = list_all_models(&state.pipeline_pool).await?;
    let scan_ids: HashSet<String> = list_scan_eligible_models(&state.pipeline_pool)
        .await?
        .into_iter()
        .map(|m| m.id)
        .collect();

    let fixed = job == JOB_THEME_SCAN && state.config.theme_scan_model.is_some();
    let check = WriteCheck {
        job,
        words: &words,
        w,
        models: &models,
        scan_ids: &scan_ids,
        fixed,
    };
    let writes = plan_writes(&rows, changes, &check).map_err(AiJobsWriteError::Refused)?;

    let templates = TemplateDir::new(state.registry.template_dir());
    let mut saved: Vec<String> = Vec::new();
    for (row, value) in &writes {
        let label = label_for(&models, w, &row.ai_role, value);
        if let Err(e) = set_setting(
            &state.pipeline_pool,
            &state.settings,
            &row.key,
            value,
            actor,
            &templates,
        )
        .await
        {
            if saved.is_empty() {
                return Err(e.into());
            }
            tracing::error!(job, %actor, error = %e, "jobs panel: a two-setting save stopped half-way");
            return Err(partial_refusal(w, &saved, &e.to_string()));
        }
        tracing::info!(job, key = %row.key, to = %value, %actor, "jobs panel: saved");
        saved.push(label);
    }

    let confirmation = confirmation(state, job, &words, &saved).await;
    Ok(AiJobSavedDto {
        job: job.to_string(),
        confirmation,
        switch_back: w.switch_back.clone(),
        panel: panel(state).await?,
    })
}

/// Restore what `job` used before its last change in the app.
///
/// # Errors
/// [`AiJobsWriteError::Refused`] when the history holds no change for the job;
/// otherwise whatever [`save`] refuses.
pub async fn switch_back(
    state: &AppState,
    job: &str,
    actor: &str,
) -> Result<AiJobSavedDto, AiJobsWriteError> {
    let settings = state.settings.current();
    let rows = job_rows(state, job).await?;
    let keys: Vec<String> = rows.iter().map(|r| r.key.clone()).collect();
    let history = list_changes_for_keys(&state.pipeline_pool, &keys).await?;
    let Some(last) = history.first() else {
        let words = job_words(job, &settings.admin_wording.job_rows);
        let text = render(
            &settings.admin_wording.ai_jobs.refuse_no_history,
            &[("job", words.title.as_str())],
        );
        return Err(AiJobsWriteError::Refused(text));
    };
    let change = AiJobChangeDto {
        key: last.key.clone(),
        value: last.old_value.clone(),
    };
    save(state, job, &[change], actor).await
}

/// The job's settings rows, or [`AiJobsWriteError::UnknownJob`].
async fn job_rows(state: &AppState, job: &str) -> Result<Vec<AiJobSettingRow>, AiJobsWriteError> {
    rows_of_job(list_ai_job_settings(&state.pipeline_pool).await?, job)
}

/// The rows belonging to `job`, or [`AiJobsWriteError::UnknownJob`] when none
/// does — a job token no setting carries. Pure, so the refusal is tested.
pub(crate) fn rows_of_job(
    rows: Vec<AiJobSettingRow>,
    job: &str,
) -> Result<Vec<AiJobSettingRow>, AiJobsWriteError> {
    let mine: Vec<AiJobSettingRow> = rows.into_iter().filter(|r| r.ai_job == job).collect();
    if mine.is_empty() {
        return Err(AiJobsWriteError::UnknownJob(job.to_string()));
    }
    Ok(mine)
}

/// The refusal after the first of two settings saved and the second did not:
/// it names what WAS saved, so nobody believes the whole change failed.
pub(crate) fn partial_refusal(
    w: &AiJobsWording,
    saved: &[String],
    reason: &str,
) -> AiJobsWriteError {
    let saved = saved.join(", ");
    let values = [("saved", saved.as_str()), ("reason", reason)];
    AiJobsWriteError::Partial(render(&w.refuse_partial, &values))
}

/// What a Save is checked against, already read.
pub(crate) struct WriteCheck<'a> {
    pub job: &'a str,
    pub words: &'a JobWords,
    pub w: &'a AiJobsWording,
    pub models: &'a [LlmModelRecord],
    pub scan_ids: &'a HashSet<String>,
    /// The server's configuration fixes this job's model.
    pub fixed: bool,
}

/// The writes a Save will make, checked before ANY is made — so a refusal leaves
/// nothing half-done. Pure: every job-level refusal is tested with plain values.
///
/// # Errors
/// The refusal sentence: a setting not of this job, a model the job cannot call
/// or one the server fixes, or nothing to change.
pub(crate) fn plan_writes<'r>(
    rows: &'r [AiJobSettingRow],
    changes: &[AiJobChangeDto],
    c: &WriteCheck<'_>,
) -> Result<Vec<(&'r AiJobSettingRow, String)>, String> {
    let title = c.words.title.as_str();
    let mut writes = Vec::new();
    for change in changes {
        let Some(row) = rows.iter().find(|r| r.key == change.key) else {
            let values = [("setting", change.key.as_str()), ("job", title)];
            return Err(render(&c.w.refuse_not_job, &values));
        };
        let value = change.value.trim().to_string();
        if row.value == value {
            continue;
        }
        if row.ai_role == ROLE_MODEL {
            if c.fixed {
                return Err(render(&c.w.refuse_fixed, &[("job", title)]));
            }
            let subject = c.words.subject.as_str();
            if let Some(refusal) = model_problem(c.models, c.scan_ids, c.w, c.job, &value, subject)
            {
                return Err(refusal);
            }
        }
        writes.push((row, value));
    }
    if writes.is_empty() {
        return Err(render(&c.w.refuse_nothing, &[("job", title)]));
    }
    Ok(writes)
}

/// Board 3's confirmation: what the job now uses and, for a change that reloads
/// the case file, what that will cost.
async fn confirmation(state: &AppState, job: &str, words: &JobWords, saved: &[String]) -> String {
    let settings = state.settings.current();
    let w = &settings.admin_wording.ai_jobs;
    let values = saved.join(" · ");
    let head = render(
        &w.saved,
        &[("job", words.title.as_str()), ("value", values.as_str())],
    );
    if job != JOB_DISCUSS_CHAT && job != JOB_CASE_STORY {
        return head;
    }
    // Why: the price is a courtesy, as on the panel. A failure to work it out
    // is logged and the no-price sentence is used; the save itself stands.
    let reload = match reload_estimate(state).await {
        Ok(Some(cost)) => render(&w.saved_reload, &[("cost", dollars(cost).as_str())]),
        Ok(None) => w.saved_reload_unpriced.clone(),
        Err(e) => {
            tracing::error!(error = %e, "jobs panel: the reload price after a save could not be worked out");
            w.saved_reload_unpriced.clone()
        }
    };
    format!("{head} {reload}")
}

#[cfg(test)]
#[path = "ai_jobs_write_tests.rs"]
mod tests;
