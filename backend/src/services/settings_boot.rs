//! Loading the configuration snapshot at boot (task 1.6, v2 §2b).
//!
//! Split from `settings_store` for Rule 17, and the seam is a real one: that
//! module PARSES — pure functions over stored rows, every refusal unit-testable
//! without a database — while this one owns the process LIFECYCLE, up to and
//! including ending it. A module that can call `std::process::exit` should not
//! also be the module a hundred unit tests import.
//!
//! ## The failure law, which is the whole reason this file exists
//!
//! A parameter missing, unreadable, out of bounds, or self-contradictory is a
//! REFUSAL at boot. There is deliberately no fallback: after task 1.6 no
//! compiled-in default exists to fall back TO — by design — so there is nothing
//! to serve with. Refusing names the key in the log and stops; starting would
//! hide it and band cards by a number nobody chose.

use sqlx::PgPool;

use crate::domain::practice_params::PRACTICE_PARAM_KEYS;
use crate::domain::settings::Settings;
use crate::domain::wording::WORDING_KEYS;
use crate::domain::wording_accusation::ACCUSATION_WORDING_KEYS;
use crate::domain::wording_authoring::AUTHORING_WORDING_KEYS;
use crate::domain::wording_matrix::MATRIX_WORDING_KEYS;
use crate::domain::wording_model_params::MODEL_PARAMS_WORDING_KEYS;
use crate::domain::wording_practice::PRACTICE_WORDING_KEYS;
use crate::domain::wording_practice_editor::PRACTICE_EDITOR_WORDING_KEYS;
use crate::domain::wording_practice_flow::PRACTICE_FLOW_WORDING_KEYS;
use crate::domain::wording_practice_report::PRACTICE_REPORT_WORDING_KEYS;
use crate::domain::wording_practice_review::PRACTICE_REVIEW_WORDING_KEYS;
use crate::domain::wording_practice_row::PRACTICE_ROW_WORDING_KEYS;
use crate::domain::wording_rehearsal::REHEARSAL_WORDING_KEYS;
use crate::domain::wording_rehearsal_chrome::REHEARSAL_CHROME_KEYS;
use crate::domain::wording_scan::SCAN_WORDING_KEYS;
use crate::domain::wording_scenario_authoring::SCENARIO_AUTHORING_WORDING_KEYS;
use crate::domain::wording_war_room::WAR_ROOM_WORDING_KEYS;
use crate::repositories::pipeline_repository::list_settings;
use crate::services::settings_handle::SettingsHandle;
use crate::services::settings_store::{build_settings, by_key, SettingsError, REQUIRED_KEYS};

/// Read the whole store and build the snapshot.
///
/// Called at boot and again after every write. Reads all rows in one query rather
/// than seven, so the snapshot is internally consistent — a per-key read could
/// interleave with a concurrent write and produce a snapshot that never existed.
///
/// # Errors
/// Returns [`SettingsError`] if the read fails or any parameter is unusable.
pub async fn load_settings(pool: &PgPool) -> Result<Settings, SettingsError> {
    let rows = list_settings(pool)
        .await
        .map_err(|source| SettingsError::Read { source })?;

    // What the store ACTUALLY held, before anything is parsed. The
    // "configuration store loaded" line below reports `REQUIRED_KEYS.len()`,
    // which is a compiled-in constant — it says what this build needs, not what
    // it found, so it can never confirm that the seed ran. This line can: a boot
    // log showing `rows=0 required=7` names a store that was never seeded, and a
    // count that drifts above `required` is a parameter seeded ahead of its code
    // (which is legal, and worth seeing).
    tracing::info!(
        rows = rows.len(),
        required = REQUIRED_KEYS.len(),
        wording = WORDING_KEYS.len(),
        // Counted apart from `wording` (task 2.11) so a half-run seed names which
        // half is missing: `rows=56 wording=48 accusation=25` says the accusation
        // migration has not been applied, which one summed number could not.
        accusation_wording = ACCUSATION_WORDING_KEYS.len(),
        rehearsal_wording = REHEARSAL_WORDING_KEYS.len(),
        rehearsal_chrome = REHEARSAL_CHROME_KEYS.len(),
        authoring_wording = AUTHORING_WORDING_KEYS.len(),
        scenario_authoring_wording = SCENARIO_AUTHORING_WORDING_KEYS.len(),
        scan_wording = SCAN_WORDING_KEYS.len(),
        model_params_wording = MODEL_PARAMS_WORDING_KEYS.len(),
        matrix_wording = MATRIX_WORDING_KEYS.len(),
        war_room_wording = WAR_ROOM_WORDING_KEYS.len(),
        practice_params = PRACTICE_PARAM_KEYS.len(),
        practice_wording = PRACTICE_WORDING_KEYS.len(),
        practice_flow_wording = PRACTICE_FLOW_WORDING_KEYS.len(),
        practice_row_wording = PRACTICE_ROW_WORDING_KEYS.len(),
        practice_editor_wording = PRACTICE_EDITOR_WORDING_KEYS.len(),
        practice_review_wording = PRACTICE_REVIEW_WORDING_KEYS.len(),
        practice_report_wording = PRACTICE_REPORT_WORDING_KEYS.len(),
        "configuration store read"
    );

    Ok(build_settings(&by_key(rows))?)
}

/// Load the snapshot at boot, or refuse to start.
///
/// ## Why this is a hard failure and not a warning
///
/// v2 §16's boot-precondition discipline, applied to configuration: serving with
/// a parameter this build cannot read would mean serving cards banded by a number
/// nobody chose. After task 1.6 there is no compiled-in default to fall back to —
/// by design — so there is nothing to serve WITH. Refusing names the key in the
/// log and stops; starting would hide it.
///
/// # Errors
/// Returns [`SettingsError`] with a message naming the offending parameter.
pub async fn load_at_boot(pool: &PgPool) -> Result<Settings, SettingsError> {
    match load_settings(pool).await {
        Ok(settings) => {
            tracing::info!(
                parameters = REQUIRED_KEYS.len(),
                wording_strings = WORDING_KEYS.len(),
                accusation_strings = ACCUSATION_WORDING_KEYS.len(),
                rehearsal_strings = REHEARSAL_WORDING_KEYS.len(),
                rehearsal_chrome_strings = REHEARSAL_CHROME_KEYS.len(),
                authoring_strings = AUTHORING_WORDING_KEYS.len(),
                scenario_authoring_strings = SCENARIO_AUTHORING_WORDING_KEYS.len(),
                matrix_strings = MATRIX_WORDING_KEYS.len(),
                war_room_strings = WAR_ROOM_WORDING_KEYS.len(),
                practice_parameters = PRACTICE_PARAM_KEYS.len(),
                practice_strings = PRACTICE_WORDING_KEYS.len(),
                practice_flow_strings = PRACTICE_FLOW_WORDING_KEYS.len(),
                practice_row_strings = PRACTICE_ROW_WORDING_KEYS.len(),
                practice_editor_strings = PRACTICE_EDITOR_WORDING_KEYS.len(),
                practice_review_strings = PRACTICE_REVIEW_WORDING_KEYS.len(),
                practice_report_strings = PRACTICE_REPORT_WORDING_KEYS.len(),
                // The tier map is not a string count — it is how many
                // (statement_type, evidence_strength) pairs the Proof Matrix can
                // rank. A boot log showing `evidence_tier_pairs=0` names a store
                // whose three map rows are present but empty, which would render
                // every item unranked with nothing else failing.
                evidence_tier_pairs = settings.evidence_tier_map.len(),
                talking_points_cap = settings.talking_points_cap,
                rehearsal_instance_rows_expand_max = settings.rehearsal_instance_rows_expand_max,
                confidence_band_high = settings.confidence_band_high,
                link_short_list_max = settings.link_short_list_max,
                "startup: configuration store loaded"
            );
            Ok(settings)
        }
        Err(error) => {
            tracing::error!(
                error = %error,
                "startup: REFUSING to serve — the configuration store is unusable. \
                 Every parameter must be present and valid in app_settings; there \
                 are no compiled-in defaults to fall back to (v2 §2b)."
            );
            Err(error)
        }
    }
}

/// Write the new value and its ledger entry, in ONE transaction.
///
/// Split out of [`set_setting`] so that function stays a readable sequence of
/// decisions (look up, refuse a no-op, validate, write, swap) and this one is the
/// single place the atomicity rule lives.
///
/// # Errors
/// Load the snapshot at boot, or END THE PROCESS.
///
/// Wraps [`load_at_boot`] with the operator instruction and the exit, so `main`
/// carries one call rather than twenty lines of failure handling — and so the
/// refusal's wording lives beside the law it enforces.
///
/// ## Why this exits rather than returning an error to `main`
///
/// There is no degraded mode to return to. Serving with a parameter this build
/// cannot read means banding cards by a number nobody chose, and after task 1.6
/// there is no compiled-in default to substitute (v2 §2b). The only honest
/// options are "start correctly" and "do not start".
pub async fn load_at_boot_or_exit(pool: &PgPool) -> SettingsHandle {
    match load_at_boot(pool).await {
        Ok(settings) => SettingsHandle::new(settings),
        Err(error) => {
            // `load_at_boot` already logged the offending parameter; this is the
            // operator's instruction for what to do about it, on stderr where a
            // failed container start will show it.
            eprintln!(
                "FATAL: the configuration store is unusable — {error}\n\
                 Fix the row in app_settings (colossus_legal_v2) and restart. \
                 Every parameter must be present and valid; there are no \
                 compiled-in defaults (v2 §2b)."
            );
            std::process::exit(1);
        }
    }
}
