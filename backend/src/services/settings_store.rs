//! The settings store — one snapshot, one read path (task 1.6, v2 §2b).
//!
//! The configuration law says every tunable is "read from stored configuration at
//! runtime" and that "edits take effect on next read". This module is how both
//! are true at once without putting a database query on a hot path.
//!
//! ## The snapshot, and why it is THREADED rather than global
//!
//! Boot reads all seven parameters once and builds a [`Settings`] — a small,
//! immutable, `Copy`-cheap struct of already-parsed numbers. It lives in
//! `AppState`, and handlers hand `&Settings` to the pure functions that need it.
//!
//! The alternative was a process-global (`static SETTINGS: OnceLock<…>`) so the
//! existing seams could keep their signatures. It was rejected, and the deciding
//! argument is worth keeping: the seams are called from functions this codebase
//! documents as PURE — `build_card` says "Pure — no I/O" and the §7 completeness
//! test asserts against it. A global read inside a pure function is hidden state,
//! and worse, a unit test calling that function without booting would find the
//! global empty, leaving only two ways out: panic (forbidden), or fall back to a
//! compiled-in default — *the exact defect this task exists to delete*. Threading
//! one parameter has no such corner. "Pure — takes one more input" is still pure.
//!
//! ## The freshness law
//!
//! A write updates the row, appends the ledger entry, and re-reads the whole
//! store into a new snapshot, all before the response is sent. The next read of
//! `AppState` therefore sees the new value: "edits take effect on next read",
//! literally, with zero database reads on the card path.
//!
//! **This assumes a single-process backend.** One process holds one snapshot, so
//! the swap is total. If a second process ever serves this API, the swap needs a
//! cross-process story — a notification channel, or a short TTL re-read. Not
//! today's problem; recorded so tomorrow's reader knows it was seen and not
//! missed.
//!
//! ## The failure law
//!
//! A parameter missing, unreadable, out of bounds, or self-contradictory is a
//! REFUSAL, at boot and on write. There is deliberately no fallback: after this
//! task no compiled-in default exists to fall back to, and inventing one at the
//! moment of failure would silently reinstate the defect §2b bans.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use sqlx::PgPool;

use crate::domain::settings::{
    parse_count, parse_float, parse_ratio, parse_text, Bounds, Ratio, SettingError, Settings,
    ValueKind,
};
use crate::domain::wording::{build_wording, validate_wording_candidate, WORDING_KEYS};
use crate::repositories::pipeline_repository::{
    get_setting, insert_setting_change, list_settings, update_setting_value, AppSettingRecord,
    PipelineRepoError,
};
use crate::services::settings_handle::SettingsHandle;

// KEYS: the stable identifiers of the seven stored parameters. These are not
// tunables — they are the NAMES of tunables, the join key between this code and
// the `app_settings` rows the migration seeded, in the same category as a column
// name or a serde tag. Renaming one is a migration, and until that migration runs
// the boot loader refuses to start rather than guessing.
const KEY_BAND_HIGH: &str = "confidence_band_high";
const KEY_BAND_MEDIUM: &str = "confidence_band_medium";
const KEY_CONTEXT_WINDOW: &str = "quote_context_window_chars";
const KEY_TALKING_POINTS_CAP: &str = "talking_points_cap";
const KEY_READINESS_N: &str = "readiness_item_threshold_n";
const KEY_CARD_TEST_RATIO: &str = "card_test_ratio";
const KEY_REANCHOR_TOLERANCE: &str = "reanchor_close_match_tolerance";
const KEY_LINK_SHORT_LIST_MAX: &str = "link_short_list_max";

/// Every NUMERIC key this build reads, so a missing one is caught at boot by name.
///
/// The anti-drift half of the configuration law: the store may hold parameters
/// this build does not know about (a future task's, seeded early), but this build
/// must never START without every parameter it does know about.
///
/// The twenty stored STRINGS live in `wording::WORDING_KEYS` rather than here.
/// Two lists rather than one flat twenty-eight because they answer different
/// questions — these decide how the system judges, those are the words it speaks
/// — and because a boot log reporting both counts says more than one number does.
pub const REQUIRED_KEYS: &[&str] = &[
    KEY_BAND_HIGH,
    KEY_BAND_MEDIUM,
    KEY_CONTEXT_WINDOW,
    KEY_TALKING_POINTS_CAP,
    KEY_READINESS_N,
    KEY_CARD_TEST_RATIO,
    KEY_REANCHOR_TOLERANCE,
    KEY_LINK_SHORT_LIST_MAX,
];

/// Why the store could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// A stored parameter is missing, unreadable, or out of bounds.
    ///
    /// Carries the domain error verbatim: it already names the key, the value and
    /// the expectation, and rewrapping it in vaguer words would lose exactly the
    /// detail a human needs to fix the row.
    #[error("{source}")]
    Invalid {
        #[source]
        source: SettingError,
    },

    #[error("no parameter named '{key}' can be changed — this build stores no such key")]
    UnknownKey { key: String },

    #[error("{key} is already '{value}' — nothing to change")]
    Unchanged { key: String, value: String },

    #[error("failed to read the configuration store: {source}")]
    Read {
        #[source]
        source: PipelineRepoError,
    },

    /// The change WAS saved, but the in-memory snapshot could not be refreshed.
    ///
    /// ## Why this is its own variant and not a `Read`
    ///
    /// It was a `Read` at first, which made it indistinguishable from a failure
    /// BEFORE anything was written — both reached the operator as "failed to
    /// load the settings". Those are opposite states: in one, nothing happened
    /// and the fix is to retry; in the other the value is stored and a retry
    /// returns "already {value} — nothing to change", which reads as a
    /// contradiction of the error they just saw.
    ///
    /// It also has a different remedy. The stored value is correct and the page
    /// will show it, but the RUNNING process is serving the old snapshot until it
    /// is restarted or the next successful write swaps it — so the message says
    /// restart, not retry.
    #[error(
        "{key} was saved as '{value}', but the running configuration could not be          refreshed: {source}. The stored value is correct — reload this page to          confirm it. The service keeps using the previous value until it is          restarted."
    )]
    SavedButStale {
        key: String,
        value: String,
        #[source]
        source: Box<SettingsError>,
    },

    #[error("failed to record the configuration change: {source}")]
    Write {
        #[source]
        source: PipelineRepoError,
    },
}

impl From<SettingError> for SettingsError {
    /// ## Rust Learning: `From` for error conversion
    ///
    /// Implementing `From<SettingError>` is what lets `?` promote a parse failure
    /// into a `SettingsError` with no `map_err` at the call site. The trait pair
    /// `From`/`Into` is Rust's standard conversion idiom: implement `From`, get
    /// `Into` for free, and `?` uses it automatically.
    fn from(source: SettingError) -> Self {
        SettingsError::Invalid { source }
    }
}

/// Bounds as declared on one row.
fn bounds_of(record: &AppSettingRecord) -> Bounds {
    Bounds {
        min: record.min_value,
        max: record.max_value,
    }
}

/// Find a required row, or report it missing by name.
fn require<'a>(
    rows: &'a HashMap<String, AppSettingRecord>,
    key: &str,
) -> Result<&'a AppSettingRecord, SettingError> {
    rows.get(key).ok_or_else(|| SettingError::Missing {
        key: key.to_string(),
    })
}

/// Read one row as a float, checking that its declared kind agrees.
///
/// The kind check is not redundant with the parse: a row whose `value_kind` says
/// `ratio` while this build reads it as a float is a store that has drifted from
/// the code, and it must be reported as that rather than as a parse failure that
/// sends a human hunting through the value.
fn float_of(record: &AppSettingRecord) -> Result<f32, SettingError> {
    expect_kind(record, ValueKind::Float)?;
    parse_float(&record.key, &record.value, bounds_of(record))
}

/// Read one row as a count, checking its declared kind.
fn count_of(record: &AppSettingRecord) -> Result<usize, SettingError> {
    expect_kind(record, ValueKind::Count)?;
    parse_count(&record.key, &record.value, bounds_of(record))
}

/// Read one row as a ratio, checking its declared kind.
fn ratio_of(record: &AppSettingRecord) -> Result<Ratio, SettingError> {
    expect_kind(record, ValueKind::Ratio)?;
    parse_ratio(&record.key, &record.value)
}

/// Read one row as stored text, checking its declared kind (task 2.10).
fn text_of(record: &AppSettingRecord) -> Result<String, SettingError> {
    expect_kind(record, ValueKind::Text)?;
    parse_text(&record.key, &record.value)
}

/// Confirm a row's stored kind is the one this build expects to read.
fn expect_kind(record: &AppSettingRecord, expected: ValueKind) -> Result<(), SettingError> {
    let stored = ValueKind::try_from(record.value_kind.as_str())?;
    if stored != expected {
        return Err(SettingError::Unreadable {
            key: record.key.clone(),
            value: record.value.clone(),
            expected: match expected {
                ValueKind::Float => "a number (the store declares a different kind)",
                ValueKind::Count => "a whole number (the store declares a different kind)",
                ValueKind::Ratio => "a ratio (the store declares a different kind)",
                ValueKind::Text => "words (the store declares a different kind)",
            },
        });
    }
    Ok(())
}

/// Build a [`Settings`] from the stored rows, or say precisely what is wrong.
///
/// Pure — takes rows, returns parsed values — so every refusal branch is
/// unit-testable without a database, which matters because these branches are the
/// configuration law's teeth.
///
/// # Errors
/// Returns [`SettingError`] naming the first parameter that is missing,
/// unreadable, out of bounds, or contradicts another.
pub fn build_settings(rows: &HashMap<String, AppSettingRecord>) -> Result<Settings, SettingError> {
    let confidence_band_high = float_of(require(rows, KEY_BAND_HIGH)?)?;
    let confidence_band_medium = float_of(require(rows, KEY_BAND_MEDIUM)?)?;

    // The one invariant that spans two rows, and therefore the one a column CHECK
    // cannot express. Checked HERE as well as in the write path because a `psql`
    // edit bypasses the write path entirely — and a store where high <= medium
    // makes the medium band unreachable, which would look like a banding bug
    // rather than a configuration mistake.
    if confidence_band_high <= confidence_band_medium {
        return Err(SettingError::BandsCrossed {
            high: confidence_band_high,
            medium: confidence_band_medium,
        });
    }

    // The twenty stored strings, read by the same rules as the numbers: the row
    // must exist, declare `text`, and carry something non-blank. `build_wording`
    // knows the KEYS and the shape; this closure is the only thing that knows
    // where a value comes from and what a refusal looks like (task 2.10).
    let wording = build_wording(|key| text_of(require(rows, key)?))?;

    Ok(Settings {
        confidence_band_high,
        confidence_band_medium,
        quote_context_window_chars: count_of(require(rows, KEY_CONTEXT_WINDOW)?)?,
        talking_points_cap: count_of(require(rows, KEY_TALKING_POINTS_CAP)?)?,
        readiness_item_threshold_n: count_of(require(rows, KEY_READINESS_N)?)?,
        card_test_ratio: ratio_of(require(rows, KEY_CARD_TEST_RATIO)?)?,
        reanchor_close_match_tolerance: float_of(require(rows, KEY_REANCHOR_TOLERANCE)?)?,
        link_short_list_max: count_of(require(rows, KEY_LINK_SHORT_LIST_MAX)?)?,
        wording,
    })
}

/// Index rows by key.
fn by_key(rows: Vec<AppSettingRecord>) -> HashMap<String, AppSettingRecord> {
    rows.into_iter().map(|r| (r.key.clone(), r)).collect()
}

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
                talking_points_cap = settings.talking_points_cap,
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

/// Change one parameter's value and return the fresh snapshot.
///
/// The row update and its ledger entry commit in ONE transaction; the snapshot is
/// rebuilt from the store afterwards, so the value the caller is told about is the
/// value that is now stored — not the value they asked for.
///
/// ## Why the new value is validated BEFORE the write, in TWO steps
///
/// Writing first and validating at reload would leave the store holding a value
/// that makes the next boot refuse to start. So nothing is written until both
/// checks pass:
///
/// 1. [`validate_candidate`] — the row's own declared kind and bounds, so a
///    refusal names the parameter and the limit it crossed.
/// 2. [`trial_snapshot`] — the WHOLE store rebuilt with the candidate in place,
///    which is the only way to catch a rule that spans two rows (high vs. medium
///    band). Step 1 alone accepted a crossed pair, because each half is valid on
///    its own.
///
/// # Errors
/// Returns [`SettingsError`] if the key is unknown, the value is invalid, the
/// change is a no-op, or a write fails.
pub async fn set_setting(
    pool: &PgPool,
    handle: &SettingsHandle,
    key: &str,
    new_value: &str,
    actor: &str,
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

    // Re-read the WHOLE store, not just this row: the snapshot must stay
    // internally consistent, and this is also where a change that broke a
    // cross-row invariant would surface — before anything serves with it.
    //
    // THE FRESHNESS LAW, in two lines: the swap happens here, before the response
    // is sent, so the next read of `AppState` already sees the new value.
    //
    // A failure HERE is not the same as a failure before the write, and must not
    // report as one — the value is already stored. `SavedButStale` carries that
    // distinction to the operator (see the variant's doc for why it earns its own
    // branch).
    let fresh = load_settings(pool).await.map_err(|source| {
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

#[cfg(test)]
#[path = "settings_store_tests.rs"]
mod tests;
