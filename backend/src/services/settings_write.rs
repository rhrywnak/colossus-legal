//! Writing ONE setting, and proving the store still stands afterwards.
//!
//! Split out of [`super::settings_store`] on 2026-08-25, when that file stood at
//! 294 non-comment lines and Phase C was about to add to it. Nothing here was
//! rewritten — the functions moved verbatim, with their doc comments, and the
//! seam is where the file already divided.
//!
//! ## Where the seam actually falls, and why it is not the one first proposed
//!
//! The report that asked for this split named "numeric readers vs wording
//! assembly". That seam no longer exists: the wording assembly moved to
//! [`super::settings_wording`] when it grew its own eleven blocks. What the file
//! still held was two different acts:
//!
//!   · READING the store and building the snapshot — the key list, the
//!     row readers, `build_settings`. It stays next door.
//!   · WRITING one parameter and proving the whole store survives it — the
//!     transaction, the ledger row, the snapshot swap, and the trial rebuild.
//!     That is this file.
//!
//! They share only `AppSettingRecord` and the error type. One is called at boot
//! and on every read of a snapshot; the other only when a human edits a row on
//! the Settings page.

use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;

use crate::domain::settings::{
    parse_count, parse_float, parse_ratio, SettingError, Settings, ValueKind,
};
use crate::domain::wording_templates::validate_wording_candidate;
use crate::repositories::pipeline_repository::{
    get_setting, insert_setting_change, list_settings, update_setting_value, AppSettingRecord,
};
use crate::services::settings_handle::SettingsHandle;
use crate::services::settings_row_readers::bounds_of;
use crate::services::settings_store::{build_settings, by_key, SettingsError};
use crate::services::settings_template_file::{check_named_file, TemplateDir};

/// Returns [`SettingsError`] if the parameter vanished mid-change or a write fails.
async fn commit_change(
    pool: &PgPool,
    key: &str,
    old_value: &str,
    new_value: &str,
    actor: &str,
    at: chrono::DateTime<Utc>,
) -> Result<(), SettingsError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| SettingsError::Write { source: e.into() })?;

    let changed = update_setting_value(&mut *tx, key, new_value, actor, at)
        .await
        .map_err(|source| SettingsError::Write { source })?;

    // Zero rows means the parameter vanished between the caller's read and this
    // write. Recording a change to a row that no longer exists would put a
    // fiction in the ledger, so the transaction is dropped without a commit —
    // which rolls it back.
    if changed == 0 {
        tracing::error!(%key, %actor, "the parameter disappeared mid-change; nothing recorded");
        return Err(SettingsError::UnknownKey {
            key: key.to_string(),
        });
    }

    insert_setting_change(&mut *tx, key, old_value, new_value, actor, at)
        .await
        .map_err(|source| SettingsError::Write { source })?;

    tx.commit()
        .await
        .map_err(|e| SettingsError::Write { source: e.into() })
}

/// Change ONE parameter, and return the snapshot the change is already live in.
///
/// The order is deliberate and each step is a different refusal: the row must
/// exist, the value must actually differ, it must satisfy its own declared kind
/// and bounds, a filename must resolve, and the WHOLE store must still build
/// with it in place — all before anything is written. Only then does the
/// transaction record the value and the ledger row together, and only then does
/// the running snapshot swap.
///
/// # Errors
/// Returns [`SettingsError::UnknownKey`] when no such parameter exists,
/// [`SettingsError::Unchanged`] when the change is a no-op (refused rather than
/// recorded, because a ledger row saying nothing happened is a fiction),
/// the validation error when the candidate fails its own row or the whole-store
/// rebuild, [`SettingsError::Write`] when the transaction fails, and
/// [`SettingsError::SavedButStale`] when the value committed but the snapshot
/// could not be refreshed — see that variant for why it is not a plain read
/// error.
///
/// (The doc comment this replaces was a naked fragment — "change is a no-op, or
/// a write fails." — whose opening sentence had been lost some time before the
/// 2026-08-25 split moved it here. Completed rather than moved verbatim again.)
pub async fn set_setting(
    pool: &PgPool,
    handle: &SettingsHandle,
    key: &str,
    new_value: &str,
    actor: &str,
    templates: &TemplateDir,
) -> Result<Arc<Settings>, SettingsError> {
    let record = get_setting(pool, key)
        .await
        .map_err(|source| SettingsError::Read { source })?
        .ok_or_else(|| SettingsError::UnknownKey {
            key: key.to_string(),
        })?;

    let new_value = new_value.trim();

    // A no-op is refused rather than recorded. The ledger's CHECK would reject it
    // anyway, as an opaque 500 instead of this sentence.
    if record.value == new_value {
        return Err(SettingsError::Unchanged {
            key: key.to_string(),
            value: record.value.clone(),
        });
    }

    validate_candidate(&record, new_value)?;
    check_named_file(key, new_value, templates)?;
    trial_snapshot(pool, key, new_value).await?;

    let now = Utc::now();
    commit_change(pool, key, &record.value, new_value, actor, now).await?;

    tracing::info!(
        %key,
        from = %record.value,
        to = %new_value,
        %actor,
        "configuration changed"
    );

    swap_snapshot(pool, handle, key, new_value).await
}

/// Re-read the whole store and swap the running snapshot.
///
/// Split from [`set_setting`] for the function-size limit; the seam is the write
/// boundary — everything before it decides and records, this makes the change
/// LIVE.
///
/// Re-reads the WHOLE store, not just this row: the snapshot must stay internally
/// consistent, and this is also where a change that broke a cross-row invariant
/// surfaces — before anything serves with it.
///
/// THE FRESHNESS LAW, in one function: the swap happens before the response is
/// sent, so the next read of `AppState` already sees the new value.
///
/// # Errors
/// Returns [`SettingsError::SavedButStale`] — and NOT a plain read error —
/// because the value is already committed by the time this runs. The two are
/// opposite states with opposite remedies: retry, versus restart. See the
/// variant's doc for why it earns its own branch.
async fn swap_snapshot(
    pool: &PgPool,
    handle: &SettingsHandle,
    key: &str,
    new_value: &str,
) -> Result<Arc<Settings>, SettingsError> {
    let fresh = crate::services::settings_boot::load_settings(pool)
        .await
        .map_err(|source| {
            tracing::error!(
                %key,
                %new_value,
                error = %source,
                "the change WAS committed but the snapshot could not be refreshed; \
                 the process is still serving the previous value until it restarts"
            );
            SettingsError::SavedButStale {
                key: key.to_string(),
                value: new_value.to_string(),
                source: Box::new(source),
            }
        })?;

    let fresh = Arc::new(fresh);
    handle.replace(Arc::clone(&fresh));
    Ok(fresh)
}

/// Prove the whole store still builds with the candidate in place.
///
/// ## Why a trial snapshot, and not just the single row's bounds
///
/// `validate_candidate` checks ONE row against its own declared kind and bounds.
/// It cannot see a sibling — so it cannot catch `confidence_band_high = 0.40`
/// while medium sits at 0.50, because 0.40 is perfectly valid for its own row.
///
/// That gap had teeth. Without this function the sequence was: validate (passes),
/// COMMIT, reload, discover `BandsCrossed`, return an error — leaving the stored
/// value crossed. Nothing served with it, because the snapshot swap never
/// happened, but the next restart would read that row, refuse, and `exit(1)`. A
/// 400 on the Settings page would have left the database unable to boot, with
/// nothing on screen saying so.
///
/// Building the trial snapshot BEFORE the write closes it, and closes it for any
/// future cross-row rule too — this runs the real `build_settings`, so a new
/// invariant added there is pre-checked here automatically rather than needing
/// someone to remember this function exists.
///
/// The extra read is on the coldest path in the system (a human editing a
/// parameter), and it buys the guarantee that a change accepted here can never
/// produce a store that refuses to boot.
///
/// # Errors
/// Returns [`SettingsError`] if the read fails, or if the store would not build
/// with this value in place.
async fn trial_snapshot(pool: &PgPool, key: &str, candidate: &str) -> Result<(), SettingsError> {
    let rows = list_settings(pool)
        .await
        .map_err(|source| SettingsError::Read { source })?;

    let mut trial = by_key(rows);
    if let Some(row) = trial.get_mut(key) {
        row.value = candidate.to_string();
    }

    build_settings(&trial)?;
    Ok(())
}

/// Check a proposed value against its row's declared kind and bounds.
///
/// Pure and separate from the write, so the rule is testable without a database
/// and so the API can refuse before opening a transaction.
///
/// # Errors
/// Returns [`SettingError`] naming the parameter and what is wrong with the value.
pub fn validate_candidate(record: &AppSettingRecord, candidate: &str) -> Result<(), SettingError> {
    let kind = ValueKind::try_from(record.value_kind.as_str())?;
    match kind {
        ValueKind::Float => {
            parse_float(&record.key, candidate, bounds_of(record))?;
        }
        ValueKind::Count => {
            parse_count(&record.key, candidate, bounds_of(record))?;
        }
        ValueKind::Ratio => {
            parse_ratio(&record.key, candidate)?;
        }
        // Two rules, both in `domain::wording`: non-blank, and (for a template)
        // still carrying the placeholders that put the facts in the sentence.
        ValueKind::Text => validate_wording_candidate(&record.key, candidate)?,
    }
    Ok(())
}
