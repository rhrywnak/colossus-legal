//! HTTP routes for the admin Settings page (task 1.6, v2 §2b).
//!
//! - `GET /settings`      → every parameter, composed for display
//! - `PUT /settings/:key` → change one, with typed validation and an actor
//!
//! ## Admin-gated, both ways
//!
//! A configuration change moves numbers that decide how cards are banded and how
//! many talking points a witness may carry. `require_admin` on both routes — the
//! READ too, because the page shows the shape of the system's judgment and there
//! is no reason for it to be broader than the edit. (Role policy proper is task
//! 1.R's; this uses the guard that already exists.)
//!
//! ## The freshness law lives in the service, not here
//!
//! `set_setting` writes, records, re-reads and swaps the snapshot before this
//! handler returns — so the next request already sees the new value with no
//! database read on any hot path. See `services::settings_store`.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `app_settings` lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde_json::json;

use crate::{
    auth::{require_admin, AuthUser},
    domain::settings::ValueKind,
    dto::settings::{SetSettingRequest, SettingChangedDto, SettingDto, SettingsPageDto},
    error::AppError,
    repositories::pipeline_repository::{list_settings, AppSettingRecord},
    services::settings_store::SettingsError,
    services::settings_template_file::TemplateDir,
    services::settings_write::set_setting,
    state::AppState,
};

/// This module's routes (declared beside their handlers — see
/// `scenario_augmentation::routes` for why the central table stopped growing).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/settings", get(get_settings))
        .route("/settings/:key", put(put_setting))
}

/// Phrase the declared bounds, or `None` when a parameter is unbounded.
///
/// ## Why the sentence is built here
///
/// The browser would otherwise have to decide how to word a one-sided bound, and
/// whether "min 1, max null" reads as "at least 1" or "1 to ∞". That phrasing is
/// part of what the parameter means, so it belongs on this side of the wire with
/// every other user-visible string.
fn bounds_label(min: Option<f64>, max: Option<f64>) -> Option<String> {
    match (min, max) {
        (Some(min), Some(max)) => Some(format!("Between {min} and {max}")),
        (Some(min), None) => Some(format!("At least {min}")),
        (None, Some(max)) => Some(format!("At most {max}")),
        (None, None) => None,
    }
}

/// Phrase the dormancy note, or `None` when a live path reads this parameter.
///
/// Says both halves out loud: which task will consume it, AND that changing it
/// does nothing today. Naming only the task would leave a human to infer the
/// second half, and the inference most people make is the optimistic one.
fn dormant_note(consumed_by: Option<&str>) -> Option<String> {
    consumed_by.map(|task| {
        format!("Read by {task}, which is not built yet — changing this has no effect today.")
    })
}

/// "Last changed by Roman" / "Never changed since it shipped".
fn last_changed(record: &AppSettingRecord) -> String {
    // The seed writes this marker, so a parameter nobody has touched is visibly
    // different from one a human deliberately set to the same value.
    if record.updated_by == "system (seed)" {
        return "Never changed since it shipped".to_string();
    }
    format!(
        "Last changed by {} on {}",
        record.updated_by,
        record.updated_at.format("%Y-%m-%d")
    )
}

/// Compose one parameter for display.
fn to_dto(record: &AppSettingRecord) -> SettingDto {
    SettingDto {
        key: record.key.clone(),
        value: record.value.clone(),
        default_value: record.default_value.clone(),
        meaning: record.meaning.clone(),
        // A kind this build cannot read still renders, with an honest hint: the
        // page is where someone would go to FIX such a row, so refusing to show
        // it would hide the one screen that can repair it.
        input_hint: ValueKind::try_from(record.value_kind.as_str())
            // The example comes from THIS row's default, so the hint under a cap
            // of 3 does not suggest 240 (task 1.7A, D4).
            .map(|kind| kind.hint(&record.default_value))
            .unwrap_or_else(|_| {
                format!(
                    "this build does not recognise the stored kind '{}'",
                    record.value_kind
                )
            }),
        bounds_label: bounds_label(record.min_value, record.max_value),
        dormant_note: dormant_note(record.consumed_by.as_deref()),
        last_changed: last_changed(record),
    }
}

/// `GET /settings` — every parameter, live ones first.
#[tracing::instrument(skip(state, user))]
pub async fn get_settings(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<SettingsPageDto>, AppError> {
    require_admin(&user)?;

    let records = list_settings(&state.pipeline_pool).await.map_err(|e| {
        tracing::error!(error = %e, "failed to read the configuration store for the settings page");
        AppError::Internal {
            message: "failed to load the settings".to_string(),
        }
    })?;

    tracing::info!(parameters = records.len(), "served the settings page");
    Ok(Json(SettingsPageDto {
        settings: records.iter().map(to_dto).collect(),
    }))
}

/// `PUT /settings/:key` — change one parameter.
///
/// ## Domain note: the confirmation says a rebuild did not happen
///
/// That is the whole promise of §2b — "changing a parameter NEVER requires a
/// rebuild, redeploy, or code change" — and it is the thing a human coming from
/// months of rebuild-to-change-a-number will not believe until the screen says
/// it. Saying it costs one clause and is the difference between a page people
/// trust and a page they follow up with a deploy just in case.
#[tracing::instrument(skip(state, user, payload), fields(key = %key))]
pub async fn put_setting(
    user: AuthUser,
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<SetSettingRequest>,
) -> Result<Json<SettingChangedDto>, AppError> {
    require_admin(&user)?;

    // The template directory travels with the change so a row that names a FILE
    // (`theme_scan_prompt_file`) can be refused here, while the human is still
    // looking at the field — see `settings_template_file` for why the check cannot
    // wait for boot.
    let settings = set_setting(
        &state.pipeline_pool,
        &state.settings,
        &key,
        &payload.value,
        &user.username,
        &TemplateDir::new(state.registry.template_dir()),
    )
    .await
    .map_err(settings_error_to_app_error)?;

    tracing::info!(
        %key,
        actor = %user.username,
        talking_points_cap = settings.talking_points_cap,
        "configuration change applied; the snapshot is live for the next read"
    );

    Ok(Json(SettingChangedDto {
        key: key.clone(),
        value: payload.value.trim().to_string(),
        message: format!(
            "{key} is now {}. It takes effect on the next read — no rebuild, no \
             redeploy.",
            payload.value.trim()
        ),
    }))
}

/// Map a [`SettingsError`] onto its HTTP status.
///
/// The refusals a HUMAN caused reach them verbatim — an out-of-bounds value, a
/// misspelled key, a no-op — because they are the only one who can act on any of
/// them, and each message already names the parameter and the limit. The two
/// server faults stay opaque and are logged with their cause.
fn settings_error_to_app_error(error: SettingsError) -> AppError {
    match error {
        // `FileNotFound` joins the human-caused refusals: the value is the
        // human's, the remedy is the human's (deploy the file, or fix the name),
        // and the message already names both the value and the path it looked in.
        SettingsError::Invalid { .. }
        | SettingsError::Unchanged { .. }
        | SettingsError::FileNotFound { .. } => AppError::BadRequest {
            message: error.to_string(),
            details: json!({ "reason": "invalid_setting" }),
        },
        // A 404, not a 400: the key names a resource that does not exist. A page
        // showing a key this build has never heard of is a page out of date with
        // the deployment, and "no such parameter" is the accurate report.
        SettingsError::UnknownKey { .. } => AppError::NotFound {
            message: error.to_string(),
        },
        SettingsError::Read { .. } => {
            tracing::error!(error = %error, "failed to read the configuration store");
            AppError::Internal {
                message: "failed to load the settings".to_string(),
            }
        }
        // The change LANDED. Reported verbatim rather than as a generic 500,
        // because the operator's next move differs from every other failure here:
        // not retry (a retry answers "already that value — nothing to change",
        // which would read as a contradiction), but reload to confirm and restart
        // to pick it up. Collapsing this into `Read` made a saved change look like
        // a change that never happened.
        SettingsError::SavedButStale { .. } => {
            tracing::error!(error = %error, "a configuration change is stored but not live");
            AppError::Internal {
                message: error.to_string(),
            }
        }
        SettingsError::Write { .. } => {
            tracing::error!(error = %error, "failed to record a configuration change");
            AppError::Internal {
                message: "failed to save the setting".to_string(),
            }
        }
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
