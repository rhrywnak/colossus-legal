//! Executing the chronology seed against Postgres, in one transaction.
//!
//! Three modes, one code path. The ONLY difference between them is what happens
//! at the end — commit, roll back, or never open the write at all — so the thing
//! `--prove` exercises is genuinely the thing `--apply` will do, rather than a
//! rehearsal of a different function.
//!
//! ## Every refusal happens BEFORE the first insert
//!
//! A case that already holds events, or a plan whose targets are not all real,
//! is refused with nothing written. That ordering is deliberate: a one-shot that
//! discovers its problem half way through leaves an operator asking "how much of
//! it landed?", and the answer to that question should always be "none of it, or
//! all of it".

use sqlx::PgPool;

use super::seed::{SeedPlan, SEED_PRECISION, SEED_TARGET_TYPE};
use crate::repositories::pipeline_repository::chronology::{count_events, count_phases};
use crate::repositories::pipeline_repository::chronology_links::existing_document_ids;
use crate::repositories::pipeline_repository::chronology_write::{
    insert_event, insert_link, NewChronologyEvent, NewChronologyLink,
};

/// What the caller asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedMode {
    /// Read-only. Plans, checks the targets, writes nothing and opens no write.
    DryRun,
    /// Executes every insert and its verification, then ROLLS BACK. The proof
    /// that `--apply` will work, taken without changing anything.
    ProveInTransaction,
    /// Executes and commits.
    Apply,
}

/// What actually happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedOutcome {
    pub events_written: usize,
    pub links_written: usize,
    pub phases_present: i64,
    /// True when the writes ran and were then deliberately discarded.
    pub rolled_back: bool,
}

/// Why the seed refused, or failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SeedExecError {
    #[error(
        "case '{case_slug}' already holds {existing} chronology event(s). This tool \
         seeds a case ONCE; re-running it would double the chronology. If the \
         seed must be redone, the existing rows are removed deliberately first"
    )]
    AlreadySeeded { case_slug: String, existing: i64 },

    #[error(
        "the plan links to {missing_count} document(s) that do not exist: {missing}. \
         Nothing was written. Either the corpus moved or the re-point map in \
         `chronology::seed` is wrong — both need a human, not a retry"
    )]
    MissingTargets {
        missing: String,
        missing_count: usize,
    },

    #[error(
        "verification failed after writing: expected {expected} {what}, counted \
         {counted}. The transaction was rolled back and nothing was written"
    )]
    Verification {
        what: &'static str,
        expected: i64,
        counted: i64,
    },

    #[error("the chronology seed failed against the database: {0}")]
    Database(String),
}

/// Check every target the plan names, and report the ones that are not there.
///
/// Run in every mode, including the read-only one: knowing the six re-points are
/// still correct is most of what a dry run is FOR.
pub async fn check_targets(pool: &PgPool, plan: &SeedPlan) -> Result<(), SeedExecError> {
    let wanted = plan.target_ids();
    let found = existing_document_ids(pool, &wanted)
        .await
        .map_err(|e| SeedExecError::Database(e.to_string()))?;

    let missing: Vec<&str> = wanted
        .iter()
        .map(String::as_str)
        .filter(|id| !found.contains(*id))
        .collect();

    if missing.is_empty() {
        return Ok(());
    }
    Err(SeedExecError::MissingTargets {
        missing_count: missing.len(),
        missing: missing.join(", "),
    })
}

/// Run the seed in the requested mode.
///
/// The refusals happen here, before `write_and_verify` opens anything, so a
/// refused run and a successful one differ only in what they wrote — never in
/// how far through the work they got.
pub async fn run(
    pool: &PgPool,
    plan: &SeedPlan,
    case_slug: &str,
    created_by: &str,
    expected_phases: i64,
    mode: SeedMode,
) -> Result<SeedOutcome, SeedExecError> {
    let existing = count_events(pool, case_slug)
        .await
        .map_err(|e| SeedExecError::Database(e.to_string()))?;
    if existing > 0 {
        return Err(SeedExecError::AlreadySeeded {
            case_slug: case_slug.to_string(),
            existing,
        });
    }
    check_targets(pool, plan).await?;

    let phases_present = count_phases(pool)
        .await
        .map_err(|e| SeedExecError::Database(e.to_string()))?;

    if mode == SeedMode::DryRun {
        return Ok(SeedOutcome {
            events_written: plan.events.len(),
            links_written: plan.link_count(),
            phases_present,
            rolled_back: false,
        });
    }

    write_and_verify(pool, plan, case_slug, created_by, expected_phases, mode).await
}

/// The writing half: one transaction, every insert, then the verification, then
/// commit or roll back.
async fn write_and_verify(
    pool: &PgPool,
    plan: &SeedPlan,
    case_slug: &str,
    created_by: &str,
    expected_phases: i64,
    mode: SeedMode,
) -> Result<SeedOutcome, SeedExecError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| SeedExecError::Database(e.to_string()))?;

    insert_all(&mut tx, plan, case_slug, created_by).await?;
    let outcome = verify(&mut tx, plan, case_slug, expected_phases).await?;

    if mode == SeedMode::Apply {
        tx.commit()
            .await
            .map_err(|e| SeedExecError::Database(e.to_string()))?;
        return Ok(SeedOutcome {
            rolled_back: false,
            ..outcome
        });
    }
    // ProveInTransaction: rolled back explicitly rather than by letting the
    // transaction drop, so a reader does not have to know sqlx's Drop impl to
    // know that nothing survives this branch.
    tx.rollback()
        .await
        .map_err(|e| SeedExecError::Database(e.to_string()))?;
    Ok(SeedOutcome {
        rolled_back: true,
        ..outcome
    })
}

/// Write every event, and the link belonging to each event that has one.
///
/// ## Rust Learning: `&mut *tx` as an executor
///
/// `sqlx::Transaction` derefs to the underlying connection, and the repository
/// functions take `impl PgExecutor`. `&mut *tx` reborrows that connection for
/// one statement and gives it straight back, which is how a loop can issue many
/// statements on the same transaction without moving it.
async fn insert_all(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    plan: &SeedPlan,
    case_slug: &str,
    created_by: &str,
) -> Result<(), SeedExecError> {
    for event in &plan.events {
        let id = insert_event(
            &mut **tx,
            &NewChronologyEvent {
                case_slug,
                event_date: event.event_date,
                date_precision: SEED_PRECISION,
                approximate: event.approximate,
                phase: &event.phase,
                title: &event.title,
                fact: event.fact.as_deref(),
                attributes: &event.attributes,
                created_by,
            },
        )
        .await
        // The event's own source id, so a constraint that fires mid-loop names
        // WHICH of the 22 legacy events triggered it. Without it an operator
        // gets a table name and has to correlate insert order against the plan
        // report by hand.
        .map_err(|e| SeedExecError::Database(format!("event {}: {e}", event.source_id)))?;

        let Some(link) = &event.link else { continue };
        insert_link(
            &mut **tx,
            &NewChronologyLink {
                event_id: id,
                target_type: SEED_TARGET_TYPE,
                target_id: &link.target_id,
                label: link.label.as_deref(),
                // Honest: the legacy JSON carried no pinpoints, and inventing
                // one would be the fabrication this project keeps dead.
                pinpoint: None,
                created_by,
            },
        )
        .await
        .map_err(|e| {
            SeedExecError::Database(format!(
                "event {} link -> {}: {e}",
                event.source_id, link.target_id
            ))
        })?;
    }
    Ok(())
}

/// Count what is really there, inside the transaction, and compare to the plan.
///
/// The expectations come from the PLAN and from the source file, never from
/// literals: "22" and "7" are facts about today's corpus, and a tool that
/// asserted them as constants would start lying the day Roman adds an event.
async fn verify(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    plan: &SeedPlan,
    case_slug: &str,
    expected_phases: i64,
) -> Result<SeedOutcome, SeedExecError> {
    let events = count_events(&mut **tx, case_slug)
        .await
        .map_err(|e| SeedExecError::Database(e.to_string()))?;
    expect("events", plan.events.len() as i64, events)?;

    let links = crate::repositories::pipeline_repository::chronology_write::count_links(
        &mut **tx, case_slug,
    )
    .await
    .map_err(|e| SeedExecError::Database(e.to_string()))?;
    expect("link rows", plan.link_count() as i64, links)?;

    let phases = count_phases(&mut **tx)
        .await
        .map_err(|e| SeedExecError::Database(e.to_string()))?;
    expect("phase rows", expected_phases, phases)?;

    Ok(SeedOutcome {
        events_written: events as usize,
        links_written: links as usize,
        phases_present: phases,
        rolled_back: false,
    })
}

/// One comparison, one error shape. Keeps `verify` to three readable lines each.
fn expect(what: &'static str, expected: i64, counted: i64) -> Result<(), SeedExecError> {
    if expected == counted {
        return Ok(());
    }
    Err(SeedExecError::Verification {
        what,
        expected,
        counted,
    })
}

#[cfg(test)]
#[path = "seed_execute_tests.rs"]
mod tests;
