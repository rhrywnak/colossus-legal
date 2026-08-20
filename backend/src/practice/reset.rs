//! `reset_practice` — put one scenario's drill back to "never practised".
//!
//! ## What it clears, and what it must not touch
//!
//! CLEARS, for one scenario:
//!   · `practice_sessions`  — every sitting, finished or not
//!   · `practice_answers`   — every answer, every mark, every model read
//!   · `practice_notes`     — every note on the scenario, its questions and its
//!                            attempts
//!
//! KEEPS, untouched:
//!   · `practice_questions`     — the deck itself: text, order, flags, hidden
//!                                state, draft badges
//!   · `practice_deck_changes`  — the record of who edited the deck and when
//!
//! ## Domain note: why the deck and its history survive a reset
//!
//! This tool exists so Marie can practise a deck again from nothing — a rehearsal
//! before trial, not an undo of Chuck's work. The questions are a WRITTEN
//! ARTEFACT that Chuck and the architect authored and edited; the answers are a
//! record of one person's practice. Clearing the second is a rehearsal reset.
//! Clearing the first would silently throw away authored work, and clearing
//! `practice_deck_changes` would erase the audit trail of who changed what — the
//! one table in this family that exists to be evidence.
//!
//! ## Why the deletes are ordered, and in one transaction
//!
//! `practice_answers` references its session, and `practice_notes` may reference
//! an answer. Children first, then parents — the same order the foreign keys
//! would force, written explicitly so a reader can see the reason rather than
//! inferring it from a constraint error. All in one transaction: a reset that
//! removed the sittings and failed on the notes would leave notes pointing at
//! sittings that no longer exist, which is a worse state than either end.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// What a reset found, and what it removed.
///
/// The same shape for a dry run and an apply — a dry run reports what IT WOULD
/// remove, which is the count it just measured. Two shapes would mean the proof
/// an operator reads before `--apply` is a different object from the one they
/// read after, and comparing them would be eyeballing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResetCounts {
    pub sessions: i64,
    pub answers: i64,
    pub notes: i64,
}

impl ResetCounts {
    /// True when there is nothing to do — every count is zero.
    pub fn is_empty(&self) -> bool {
        self.sessions == 0 && self.answers == 0 && self.notes == 0
    }
}

/// Why a reset refused. Every variant is a state an operator can act on.
#[derive(Debug, thiserror::Error)]
pub enum ResetError {
    #[error("no scenario carries the code {code} — check the code, or list the scenarios first")]
    UnknownScenario { code: String },

    #[error("the database refused the reset for {code}: {source}")]
    Database {
        code: String,
        #[source]
        source: sqlx::Error,
    },
}

/// The scenario's id, or a refusal naming the code.
///
/// ## Why an unknown code is an ERROR and not an empty result
///
/// A typo'd code would otherwise report "0 sessions, 0 answers, 0 notes" and
/// exit 0 — an operator would read that as "already clean" and move on, when in
/// fact nothing was examined at all. Two operationally distinct states must not
/// print the same thing (Standing Rule 1).
pub async fn scenario_id(pool: &PgPool, code: &str) -> Result<Uuid, ResetError> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM scenarios WHERE code = $1")
        .bind(code)
        .fetch_optional(pool)
        .await
        .map_err(|source| ResetError::Database {
            code: code.to_string(),
            source,
        })?;
    row.map(|r| r.0).ok_or_else(|| ResetError::UnknownScenario {
        code: code.to_string(),
    })
}

/// The three counting statements, named once.
///
/// Both the dry run (against the pool) and the apply (inside the transaction)
/// run exactly these. Two copies would be two things to keep in step, and the
/// SQL-shape guard would scan both while nothing checked they agreed.
const COUNT_SESSIONS: &str = "SELECT count(*) FROM practice_sessions WHERE scenario_id = $1";
const COUNT_ANSWERS: &str = "SELECT count(*) FROM practice_answers a \
     JOIN practice_sessions s ON s.id = a.session_id \
    WHERE s.scenario_id = $1";
const COUNT_NOTES: &str = "SELECT count(*) FROM practice_notes WHERE scenario_id = $1";

/// What this scenario's practice tables hold right now, read from the pool.
///
/// The DRY RUN's counter. The apply uses [`count_tx`] instead, so its proof is
/// part of the same transaction as its deletes.
///
/// Counted through the SESSION for answers, because `practice_answers` carries
/// no `scenario_id` of its own — an answer belongs to a sitting, and the sitting
/// belongs to the scenario. Notes carry the scenario directly.
pub async fn count(pool: &PgPool, scenario: Uuid, code: &str) -> Result<ResetCounts, ResetError> {
    let fail = |source: sqlx::Error| ResetError::Database {
        code: code.to_string(),
        source,
    };
    let (sessions,): (i64,) = sqlx::query_as(COUNT_SESSIONS)
        .bind(scenario)
        .fetch_one(pool)
        .await
        .map_err(fail)?;
    let (answers,): (i64,) = sqlx::query_as(COUNT_ANSWERS)
        .bind(scenario)
        .fetch_one(pool)
        .await
        .map_err(fail)?;
    let (notes,): (i64,) = sqlx::query_as(COUNT_NOTES)
        .bind(scenario)
        .fetch_one(pool)
        .await
        .map_err(fail)?;
    Ok(ResetCounts {
        sessions,
        answers,
        notes,
    })
}

/// The same three counts, INSIDE a transaction.
///
/// ## Why this exists, and the bug it removes
///
/// The apply used to take its after-count from the POOL, after the commit. That
/// left two states no observable could tell apart: a commit that failed (nothing
/// deleted — retry) and a post-commit count that failed (everything deleted —
/// do NOT retry). Both surfaced as "the reset failed; nothing was written",
/// which on a tool that erases a witness's record of her own preparation is not
/// a vague message, it is a false one.
///
/// Counting inside the transaction collapses the second state out of existence:
/// if the count fails, the transaction has not committed, so nothing WAS written
/// and the message is true again.
///
/// ## Rust Learning: `&mut **tx`
///
/// `tx` is `&mut Transaction`, and `Transaction` derefs to `PgConnection`. So
/// `*tx` is the transaction, `**tx` is the connection, and `&mut **tx` is the
/// mutable borrow sqlx's executor trait is implemented for. Each query re-borrows
/// rather than consuming, which is what lets three of them share one transaction.
async fn count_tx(
    tx: &mut Transaction<'_, Postgres>,
    scenario: Uuid,
    code: &str,
) -> Result<ResetCounts, ResetError> {
    let fail = |source: sqlx::Error| ResetError::Database {
        code: code.to_string(),
        source,
    };
    let (sessions,): (i64,) = sqlx::query_as(COUNT_SESSIONS)
        .bind(scenario)
        .fetch_one(&mut **tx)
        .await
        .map_err(fail)?;
    let (answers,): (i64,) = sqlx::query_as(COUNT_ANSWERS)
        .bind(scenario)
        .fetch_one(&mut **tx)
        .await
        .map_err(fail)?;
    let (notes,): (i64,) = sqlx::query_as(COUNT_NOTES)
        .bind(scenario)
        .fetch_one(&mut **tx)
        .await
        .map_err(fail)?;
    Ok(ResetCounts {
        sessions,
        answers,
        notes,
    })
}

/// Delete the three tables' rows for one scenario, children first.
///
/// Inside the caller's transaction. Returns nothing: the proof is the count
/// taken AFTER, by the caller, from the same connection — a delete that reported
/// its own row count would be the tool grading its own homework.
async fn delete_all(
    tx: &mut Transaction<'_, Postgres>,
    scenario: Uuid,
    code: &str,
) -> Result<(), ResetError> {
    let fail = |source: sqlx::Error| ResetError::Database {
        code: code.to_string(),
        source,
    };
    // Notes first: one may reference an answer, which references a session.
    sqlx::query("DELETE FROM practice_notes WHERE scenario_id = $1")
        .bind(scenario)
        .execute(&mut **tx)
        .await
        .map_err(fail)?;
    sqlx::query(
        "DELETE FROM practice_answers WHERE session_id IN \
             (SELECT id FROM practice_sessions WHERE scenario_id = $1)",
    )
    .bind(scenario)
    .execute(&mut **tx)
    .await
    .map_err(fail)?;
    sqlx::query("DELETE FROM practice_sessions WHERE scenario_id = $1")
        .bind(scenario)
        .execute(&mut **tx)
        .await
        .map_err(fail)?;
    Ok(())
}

/// Clear one scenario's practice record. Returns the counts BEFORE and AFTER.
///
/// ## Every count is inside the transaction, and that is the point
///
/// Before-count, deletes, after-count, THEN commit. Both counts therefore
/// describe the same consistent snapshot as the deletes between them, and — the
/// part that matters — a failure anywhere before the commit means nothing was
/// written. That is what lets the binary's error message say "nothing was
/// written" and be telling the truth.
///
/// The commit is the last statement, so the only failure that can follow a
/// successful delete is the commit itself, which rolls back. There is no state
/// where this function returns an error and the rows are gone.
pub async fn apply(
    pool: &PgPool,
    scenario: Uuid,
    code: &str,
) -> Result<(ResetCounts, ResetCounts), ResetError> {
    let mut tx = pool.begin().await.map_err(|source| ResetError::Database {
        code: code.to_string(),
        source,
    })?;
    let before = count_tx(&mut tx, scenario, code).await?;
    delete_all(&mut tx, scenario, code).await?;
    let after = count_tx(&mut tx, scenario, code).await?;
    tx.commit().await.map_err(|source| ResetError::Database {
        code: code.to_string(),
        source,
    })?;
    Ok((before, after))
}

/// The proof an operator reads, and the file the run writes.
///
/// Names every table, including the two it did NOT touch — a report that listed
/// only the deletions would leave a reader wondering what happened to the deck,
/// and "it says nothing about the questions" is not an answer anybody should
/// have to take on trust before running `--apply` on a witness's practice record.
pub fn render_report(code: &str, before: &ResetCounts, after: Option<&ResetCounts>) -> String {
    let mut out = String::new();
    out.push_str(&format!("reset_practice — scenario {code}\n\n"));
    match after {
        None => out.push_str("DRY RUN — nothing was written. Re-run with --apply to clear.\n\n"),
        Some(_) => out.push_str("APPLIED — the rows below were deleted.\n\n"),
    }
    out.push_str("cleared:\n");
    for (label, b, a) in [
        (
            "practice_sessions",
            before.sessions,
            after.map(|a| a.sessions),
        ),
        (
            "practice_answers ",
            before.answers,
            after.map(|a| a.answers),
        ),
        ("practice_notes   ", before.notes, after.map(|a| a.notes)),
    ] {
        match a {
            Some(after) => out.push_str(&format!("  {label}  {b} -> {after}\n")),
            None => out.push_str(&format!("  {label}  {b} -> 0 (would be)\n")),
        }
    }
    out.push_str("\nkept, untouched:\n");
    out.push_str("  practice_questions     the deck: text, order, flags, hidden state\n");
    out.push_str("  practice_deck_changes  who edited the deck, and when\n");
    out
}

#[cfg(test)]
#[path = "reset_tests.rs"]
mod tests;
