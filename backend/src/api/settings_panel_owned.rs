//! The Settings page's rows that Admin → Overview's jobs panel owns
//! (CC_TASK_MODEL_JOBS_PANEL_v1, item 7 / B5).
//!
//! Every job setting — a row whose `ai_job` is set — is changed on ONE page. On
//! Settings it is still found by search, but it is read-only and says where to
//! go; a direct `PUT /settings/:key` is refused with the same sentence. The
//! refusal lives on the server (Standing Rule 12): a read-only field in the
//! browser alone is a suggestion, not a rule.

use std::collections::HashSet;

use crate::error::AppError;
use crate::repositories::pipeline_repository::ai_job_settings::list_ai_job_settings;
use crate::state::AppState;

/// The keys the panel owns.
///
/// # Errors
/// [`AppError::Internal`] when the job settings cannot be read.
pub(super) async fn panel_keys(state: &AppState) -> Result<HashSet<String>, AppError> {
    let rows = list_ai_job_settings(&state.pipeline_pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "settings: the jobs panel's rows could not be read");
            AppError::Internal {
                message: format!("failed to read which settings the jobs panel owns: {e}"),
            }
        })?;
    Ok(rows.into_iter().map(|r| r.key).collect())
}

/// The pointer sentence shown under — and refusing — a panel-owned row.
pub(super) fn pointer(state: &AppState) -> String {
    state
        .settings
        .current()
        .admin_wording
        .ai_jobs
        .settings_pointer
        .clone()
}

/// Refuse (409) a single-row Settings write to a key the panel owns.
///
/// # Errors
/// [`AppError::Conflict`] carrying the pointer sentence, or
/// [`AppError::Internal`] when the ownership cannot be read.
pub(super) async fn refuse_if_panel_owned(state: &AppState, key: &str) -> Result<(), AppError> {
    if !panel_keys(state).await?.contains(key) {
        return Ok(());
    }
    let message = pointer(state);
    tracing::warn!(%key, "settings: write refused, the jobs panel owns this row");
    Err(AppError::Conflict {
        message,
        details: serde_json::json!({ "reason": "owned_by_panel" }),
    })
}
