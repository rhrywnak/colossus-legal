//! Repository for `scenario_status_transitions` — the ready gate's record
//! (task 1.5, v2 §5/§6).
//!
//! ## What this answers that the status column cannot
//!
//! `scenarios.status` says whether a scenario is ready NOW. This says who made it
//! so, and — the question that actually gets asked — who took it back out and
//! when. A scenario Marie expected to rehearse being absent the night before
//! trial is a question with a name attached, and a column pair holding only the
//! latest act would have forgotten the demotion on the next promote.
//!
//! Append-only, like the ruling ledger, and deliberately not that ledger: a
//! readiness declaration is a statement about a whole scenario and carries no
//! anchor.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// One recorded transition.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScenarioStatusTransitionRecord {
    pub id: Uuid,
    pub scenario_id: Uuid,
    pub from_status: String,
    pub to_status: String,
    pub actor: String,
    pub at: DateTime<Utc>,
}

// CONST: the append. Held as a `const` so a SQL-shape test can pin its
// append-only semantics without a live database (house pattern).
//
// No `ON CONFLICT`: the surrogate id means two transitions on the same scenario
// never collide, so every act appends. An upsert here would convert the history
// into a latest-value column pair — the design this table exists to replace.
const INSERT_TRANSITION_SQL: &str = r#"INSERT INTO scenario_status_transitions
        (scenario_id, from_status, to_status, actor, at)
    VALUES ($1, $2, $3, $4, $5)
    RETURNING id"#;

/// Record one status transition.
///
/// Takes an executor so the caller can commit it in the SAME transaction as the
/// status change itself — a scenario that became ready with no record of who did
/// it, or a record of a change that did not land, are both worse than either
/// alone.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the insert fails — notably the
/// `from_status <> to_status` CHECK, which fires if a caller records a
/// transition that goes nowhere.
pub async fn insert_status_transition(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: Uuid,
    from_status: &str,
    to_status: &str,
    actor: &str,
    at: DateTime<Utc>,
) -> Result<Uuid, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, Uuid>(INSERT_TRANSITION_SQL)
        .bind(scenario_id)
        .bind(from_status)
        .bind(to_status)
        .bind(actor)
        .bind(at)
        .fetch_one(executor)
        .await?;
    Ok(id)
}

// CONST: the status change that PAIRS with the append above. Held here, beside
// its record, rather than in `scenario_store`: the two statements are one act
// and must stay in one transaction, and a reader looking for "how does a
// scenario become ready" finds both without opening a second file.
//
// Deliberately narrower than `update_scenario`: it sets `status` and nothing
// else, so the readiness path cannot carry a rename or a definition edit as a
// side effect any more than a rename can carry a readiness change.
const UPDATE_STATUS_SQL: &str =
    "UPDATE scenarios SET status = $2, updated_at = now() WHERE scenario_id = $1";

/// Set a scenario's status. Returns how many rows changed (0 = no such scenario).
///
/// Takes an executor for the same reason as [`insert_status_transition`] — the
/// two are written together or not at all.
///
/// ## Rust Learning: `impl sqlx::PgExecutor<'_>` as a parameter
///
/// This is an argument-position `impl Trait`: the function accepts ANYTHING that
/// can run a query — a `&PgPool` (its own connection, auto-committed) or a
/// `&mut PgConnection` borrowed from an open transaction. Writing it this way is
/// what lets one repository function serve both the standalone case and the
/// in-transaction case without a second copy or a runtime flag.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the update fails — notably the column's
/// CHECK, if a caller supplies a status outside the vocabulary.
pub async fn update_scenario_status(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: Uuid,
    status: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(UPDATE_STATUS_SQL)
        .bind(scenario_id)
        .bind(status)
        .execute(executor)
        .await?;
    Ok(result.rows_affected())
}

/// A scenario's transition history, most recent first.
///
/// Most-recent-first because the question is nearly always about the LAST act
/// ("who took this out of rehearsal?"), and the index is ordered to match.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_status_transitions(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<ScenarioStatusTransitionRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ScenarioStatusTransitionRecord>(
        "SELECT id, scenario_id, from_status, to_status, actor, at \
         FROM scenario_status_transitions WHERE scenario_id = $1 ORDER BY at DESC",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The record APPENDS. An upsert would make it a column pair with extra steps.
    #[test]
    fn the_transition_record_is_append_only() {
        assert!(
            !INSERT_TRANSITION_SQL.contains("ON CONFLICT"),
            "every transition is its own row — the history is the point: \
             {INSERT_TRANSITION_SQL}"
        );
        assert!(!INSERT_TRANSITION_SQL.contains("UPDATE"));
        assert!(!INSERT_TRANSITION_SQL.contains("DELETE"));
    }

    /// Every column the table requires is bound.
    #[test]
    fn the_insert_records_who_and_when() {
        for column in ["scenario_id", "from_status", "to_status", "actor", "at"] {
            assert!(
                INSERT_TRANSITION_SQL.contains(column),
                "a transition record without {column} cannot answer who or when"
            );
        }
        assert!(INSERT_TRANSITION_SQL.contains("$5"));
        assert!(!INSERT_TRANSITION_SQL.contains("$6"));
    }

    /// The status change touches the status and nothing else.
    ///
    /// A readiness change that also wrote `name` or `definition` would be the
    /// mirror of the bug task 1.5 closed on the other side (a rename carrying a
    /// readiness change). Both directions must stay narrow.
    #[test]
    fn the_status_update_changes_only_the_status() {
        for column in ["name", "definition", "theme_statement", "motivation"] {
            assert!(
                !UPDATE_STATUS_SQL.contains(column),
                "the readiness update touches {column}; it must set status alone: \
                 {UPDATE_STATUS_SQL}"
            );
        }
        assert!(
            UPDATE_STATUS_SQL.contains("status = $2"),
            "{UPDATE_STATUS_SQL}"
        );
    }

    /// The update is fenced to one scenario.
    #[test]
    fn the_status_update_names_its_scenario() {
        assert!(
            UPDATE_STATUS_SQL.contains("WHERE scenario_id = $1"),
            "an unfenced UPDATE would change every scenario's status: \
             {UPDATE_STATUS_SQL}"
        );
    }
}
