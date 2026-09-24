//! Admin → Overview's jobs panel: read everything it is composed from
//! (CC_TASK_MODEL_JOBS_PANEL_v1, Stage A — the read side).
//!
//! The choices live in [`super::ai_jobs_compose`]; this module only gathers the
//! inputs: the job rows and their history, the models, the scan list, the
//! instruction files, the server's fixed scan model and the reload price.

use std::collections::HashSet;

use chrono::Utc;

use crate::dto::ai_jobs::AiJobsPanelDto;
use crate::repositories::pipeline_repository::ai_job_settings::{
    list_ai_job_settings, list_changes_for_keys, list_loose_model_settings,
};
use crate::repositories::pipeline_repository::models::{
    list_all_models, list_scan_eligible_models,
};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::ai_jobs_compose::{compose_panel, ComposeError, PanelInputs};
use crate::services::chat_keepwarm_button::{reload_estimate, when};
use crate::services::chat_question_gather::display_name;
use crate::state::AppState;

/// Why the panel could not be read. Each names what failed.
#[derive(Debug, thiserror::Error)]
pub enum AiJobsError {
    #[error("the job settings or their history could not be read: {0}")]
    Store(#[from] PipelineRepoError),
    #[error("the model list could not be read: {0}")]
    Models(#[from] sqlx::Error),
    #[error("the instructions folder {dir} could not be listed: {source}")]
    Folder { dir: String, source: std::io::Error },
    #[error("{0}")]
    Compose(#[from] ComposeError),
}

/// The panel, read now.
///
/// ## Rust Learning: `?` with several error types
///
/// Each `?` below converts its own error into [`AiJobsError`] through the
/// `#[from]` impls `thiserror` generates, so one function can call a repository
/// (`PipelineRepoError`), sqlx directly (`sqlx::Error`) and the filesystem, and
/// the caller still sees one type that says which of them failed.
///
/// # Errors
/// [`AiJobsError`], naming the read that failed.
pub async fn panel(state: &AppState) -> Result<AiJobsPanelDto, AiJobsError> {
    let settings = state.settings.current();
    let pool = &state.pipeline_pool;
    let rows = list_ai_job_settings(pool).await?;
    let keys: Vec<String> = rows.iter().map(|r| r.key.clone()).collect();
    let changes = list_changes_for_keys(pool, &keys).await?;
    let models = list_all_models(pool).await?;
    let scan_ids: HashSet<String> = list_scan_eligible_models(pool)
        .await?
        .into_iter()
        .map(|m| m.id)
        .collect();
    let files = instruction_files(state.registry.template_dir()).await?;
    let loose = list_loose_model_settings(pool).await?;

    // Why: the reload price is a courtesy on two rows. When the case file cannot
    // be assembled (a missing narrative file, say), the rows still render — the
    // missing-file warning names the cause — and the price line falls back to its
    // no-price wording. The failure itself is logged, never swallowed.
    let reload_cost = match reload_estimate(state).await {
        Ok(cost) => cost,
        Err(e) => {
            tracing::error!(error = %e, "jobs panel: the case file reload price could not be worked out");
            None
        }
    };

    let now = Utc::now();
    let timezone = settings.practice_read.case_timezone.clone();
    let when_fn = move |at| when(at, now, &timezone);
    let who_fn = |login: &str| display_name(&settings, login);
    let inputs = PanelInputs {
        rows: &rows,
        changes: &changes,
        models: &models,
        scan_ids: &scan_ids,
        files: &files,
        fixed_scan_model: state.config.theme_scan_model.as_deref(),
        reload_cost,
        wording: &settings.admin_wording,
        loose: &loose,
        when: &when_fn,
        who: &who_fn,
    };
    Ok(compose_panel(&inputs)?)
}

/// The `.md` file names in the instructions folder, sorted.
///
/// # Errors
/// [`AiJobsError::Folder`] naming the folder when it cannot be listed — an
/// unreadable folder is not the same answer as an empty one (Rule 1).
pub(crate) async fn instruction_files(dir: &str) -> Result<Vec<String>, AiJobsError> {
    let folder_error = |source| AiJobsError::Folder {
        dir: dir.to_string(),
        source,
    };
    let mut entries = tokio::fs::read_dir(dir).await.map_err(folder_error)?;
    let mut files = Vec::new();
    while let Some(entry) = entries.next_entry().await.map_err(folder_error)? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".md") {
            files.push(name);
        }
    }
    files.sort();
    Ok(files)
}
