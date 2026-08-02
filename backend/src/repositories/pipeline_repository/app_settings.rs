//! Repository for `app_settings` and `app_setting_changes` — the configuration
//! store (task 1.6, v2 §2b).
//!
//! Two tables, one act: changing a parameter writes the new value AND appends the
//! record of who changed it. Both statements live here so a reader looking for
//! "how does a parameter change" finds them together, and so the service can
//! commit them in one transaction.
//!
//! ## Reads are COLD here
//!
//! Nothing on a hot path calls this module. The parameters are loaded once at
//! boot into a snapshot (`services::settings_store`) and re-loaded when one is
//! written; every card, cap and cutoff is then read from that snapshot with no
//! database involved. See the store's header for the freshness law.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::PipelineRepoError;

/// One row of `app_settings`, exactly as stored.
///
/// Untyped on purpose: `value` and `value_kind` stay raw text here and are turned
/// into numbers by `domain::settings`. Decoding in the repository would make every
/// read fallible for a reason that has nothing to do with the database, and would
/// put the configuration law's parsing rules in two places.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AppSettingRecord {
    pub key: String,
    pub value: String,
    pub value_kind: String,
    pub default_value: String,
    /// Bounds are `NUMERIC` in Postgres and arrive as `f64`. NULL = unbounded.
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub meaning: String,
    /// `None` when a live code path reads this parameter; otherwise the task that
    /// will, so the page can say changing it does nothing yet.
    pub consumed_by: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

/// One recorded change.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AppSettingChangeRecord {
    pub key: String,
    pub old_value: String,
    pub new_value: String,
    pub actor: String,
    pub at: DateTime<Utc>,
}

/// Shared SELECT projection, so the `FromRow` column set cannot drift between
/// query sites (the house anti-drift discipline).
// CONST: column projection locked to the `app_settings` schema — a structural
// schema-coupling invariant, not a deployment value.
const SETTING_COLUMNS: &str = "key, value, value_kind, default_value, min_value, \
     max_value, meaning, consumed_by, updated_at, updated_by";

/// Every stored parameter, ordered so live ones come before dormant ones.
///
/// ## Why the ordering is server-owned
///
/// The Settings page groups live parameters above the ones nothing reads yet, and
/// that grouping is a statement about the SYSTEM, not a display preference. A
/// client sorting it would be a client deciding which parameters currently matter.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_settings(pool: &PgPool) -> Result<Vec<AppSettingRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {SETTING_COLUMNS} FROM app_settings \
         ORDER BY (consumed_by IS NOT NULL), key"
    );
    let rows = sqlx::query_as::<_, AppSettingRecord>(&sql)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// One parameter by key, or `None` when no such key is stored.
///
/// `None` is a REAL answer here, not an error: it is what the write path checks
/// before refusing an unknown key with a message naming it.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn get_setting(
    pool: &PgPool,
    key: &str,
) -> Result<Option<AppSettingRecord>, PipelineRepoError> {
    let sql = format!("SELECT {SETTING_COLUMNS} FROM app_settings WHERE key = $1");
    let row = sqlx::query_as::<_, AppSettingRecord>(&sql)
        .bind(key)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

// CONST: the value update. Held as a `const` so a SQL-shape test can pin it
// without a live database (house pattern).
//
// Sets `value` and the two last-change columns and NOTHING else: the kind,
// bounds, default and meaning are schema-level declarations about what the
// parameter IS, and an edit to its value must never be able to widen its own
// bounds as a side effect.
const UPDATE_SETTING_SQL: &str = "UPDATE app_settings \
     SET value = $2, updated_at = $3, updated_by = $4 \
     WHERE key = $1";

/// Write one parameter's value. Returns how many rows changed (0 = no such key).
///
/// Takes an executor so the caller can commit it in the SAME transaction as its
/// ledger entry — a value that changed with no record of who changed it, or a
/// record of a change that did not land, are both worse than either alone.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the update fails.
pub async fn update_setting_value(
    executor: impl sqlx::PgExecutor<'_>,
    key: &str,
    value: &str,
    actor: &str,
    at: DateTime<Utc>,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(UPDATE_SETTING_SQL)
        .bind(key)
        .bind(value)
        .bind(at)
        .bind(actor)
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
}

// CONST: the append. No `ON CONFLICT`: the surrogate id means two changes to the
// same key never collide, so every edit appends. An upsert here would collapse
// the history into the latest-value column pair this ledger exists to replace.
const INSERT_CHANGE_SQL: &str = "INSERT INTO app_setting_changes \
     (key, old_value, new_value, actor, at) \
     VALUES ($1, $2, $3, $4, $5)";

/// Record one configuration change.
///
/// Both sides are stored so a ledger row reads on its own, without the setting's
/// current state or the previous row.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the insert fails — notably the
/// `old_value <> new_value` CHECK, which fires if a caller records a no-op.
pub async fn insert_setting_change(
    executor: impl sqlx::PgExecutor<'_>,
    key: &str,
    old_value: &str,
    new_value: &str,
    actor: &str,
    at: DateTime<Utc>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(INSERT_CHANGE_SQL)
        .bind(key)
        .bind(old_value)
        .bind(new_value)
        .bind(actor)
        .bind(at)
        .execute(executor)
        .await?;
    Ok(())
}

/// One parameter's change history, most recent first.
///
/// Most-recent-first because the question is nearly always about the LAST edit,
/// and the index is ordered to match.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_setting_changes(
    pool: &PgPool,
    key: &str,
) -> Result<Vec<AppSettingChangeRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, AppSettingChangeRecord>(
        "SELECT key, old_value, new_value, actor, at FROM app_setting_changes \
         WHERE key = $1 ORDER BY at DESC",
    )
    .bind(key)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[cfg(test)]
#[path = "app_settings_tests.rs"]
mod tests;
