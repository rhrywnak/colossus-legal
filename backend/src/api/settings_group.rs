//! The coupled-group write route (CC_TASK_REVIEWER_PAIR_EDITOR_v1).
//!
//! Split from [`super::settings`] the moment it was written: that module reached
//! 316 non-comment lines with this handler in it, sixteen over Rule 17. The seam
//! is a real one rather than an arithmetic convenience — every other route there
//! reads or writes ONE row, and this one writes a set of them as a unit.
//!
//! ## What this route is for (Law 23)
//!
//! `practice_reviewer_usernames` and `practice_reviewer_display_names` are read
//! index-aligned. Saved one at a time they cannot both grow: whichever goes
//! first makes the lists different lengths, and the boot check refuses it — in
//! either order. v2.1.14 shipped with 5,540 green tests and no way to add a
//! second reviewer through its own settings page.
//!
//! The generic `PUT /settings/:key` is unchanged for every uncoupled row, and
//! now REFUSES a coupled one with a sentence naming this editor.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    auth::{require_admin, AuthUser},
    dto::settings::{SetSettingPairRequest, SettingChangedDto},
    error::AppError,
    services::settings_groups::{group_by_id, CoupledGroup},
    services::settings_write::set_setting_pair,
    state::AppState,
};

use super::settings::settings_error_to_app_error;

/// `PUT /settings/group/:group_id` — write a whole coupled group at once.
///
/// ## Why a dedicated route and not a coupled shape on `PUT /settings/:key`
///
/// Ruled 2026-09-20. The generic route must be able to REFUSE a coupled row and
/// say where to go instead; a route that sometimes accepts a pair and sometimes
/// refuses one row cannot say that cleanly, and every uncoupled setting would
/// carry pair-shaped optional state it never uses.
#[tracing::instrument(skip(state, user, payload), fields(group = %group_id))]
pub async fn put_setting_group(
    user: AuthUser,
    State(state): State<AppState>,
    Path(group_id): Path<String>,
    Json(payload): Json<SetSettingPairRequest>,
) -> Result<Json<SettingChangedDto>, AppError> {
    require_admin(&user)?;

    let group = group_by_id(&group_id).ok_or_else(|| {
        tracing::warn!(%group_id, "no such coupled group in this build");
        AppError::NotFound {
            message: format!("no coupled group named '{group_id}' exists in this build"),
        }
    })?;

    set_setting_pair(
        &state.pipeline_pool,
        &state.settings,
        group,
        &payload.entries,
        &user.username,
    )
    .await
    .map_err(settings_error_to_app_error)?;

    let entries = payload.entries.len();
    tracing::info!(
        %group_id,
        entries,
        actor = %user.username,
        "a coupled group was changed; the snapshot is live for the next read"
    );

    Ok(Json(SettingChangedDto {
        key: group.id.to_string(),
        value: entries.to_string(),
        // Says the same two things the single-row confirmation says — what it is
        // now, and that no rebuild happened — in the group's own noun.
        message: confirmation(group, entries),
    }))
}

/// What the page says after a group is saved.
///
/// Pure, so the sentence can be tested without a pool — and separate because it
/// carries the one thing §2b promises and operators do not believe until they
/// read it: that nothing was rebuilt.
fn confirmation(group: &CoupledGroup, entries: usize) -> String {
    let noun = if entries == 1 {
        group.entry_noun.to_string()
    } else {
        format!("{}s", group.entry_noun)
    };
    // The label is NOT wrapped in quotes: several already contain them — the
    // reviewer bench is literally `Who may press “Done reviewing”` — and
    // wrapping produced `““Done reviewing”” now lists`, seen in the browser.
    format!(
        "{} now lists {entries} {noun}. It takes effect on the next read — no \
         rebuild, no redeploy.",
        group.label
    )
}

#[cfg(test)]
#[path = "settings_group_tests.rs"]
mod tests;
