//! The augmentation service — writing the three human-authored components
//! (task 1.4, v2 §8).
//!
//! C1 identity, C4 human facts, C5 talking points. Everything a human writes
//! about a scenario that no system process may touch.
//!
//! ## The two §8 invariants, and where each lives
//!
//! *"Re-gathering never edits human content."* Structural: this module is the
//! only writer of `scenario_human_facts` and of the talking-points tables, and
//! `scenario_human_facts_tests` scans the source tree to pin the allowlist of
//! tables the scan/merge paths may write. A scan that grew a write into human
//! content fails the build.
//!
//! *"Editing human content never triggers re-gathering."* Behavioural, and just
//! as checkable: nothing in this module calls the gather path, and
//! `augmentation_never_gathers` asserts it by scanning this file. The invariant
//! matters because gather MEMOIZES candidate ordinals — a stray gather triggered
//! by an edit would mint identity as a side effect of typing a sentence.
//!
//! ## Why the cap is enforced here rather than in the browser
//!
//! A cap the client applies is a suggestion: the fourth talking point is written
//! by any client that did not implement it, including a curl. The seam
//! (`domain::human_authored::talking_points_cap`) is read on this side, and the
//! refusal names the limit so the human knows what happened.

use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::human_authored::{talking_points_cap, DateType};
use crate::repositories::pipeline_repository::{
    delete_human_fact, delete_responses_for_scenario, insert_human_fact, insert_response_item,
    insert_scenario_response, list_human_facts_for_scenario, list_items_for_response,
    list_responses_for_scenario, HumanFactWrite, PipelineRepoError, ResponseItemRecord,
    ScenarioHumanFactRecord,
};

/// Why an augmentation write was refused.
///
/// Each variant's message is written for the HUMAN who hit it — they are the only
/// one who can act on any of these — in the same spirit as `RulingError`.
#[derive(Debug, thiserror::Error)]
pub enum AugmentationError {
    #[error("a human fact needs some text — an empty note records nothing")]
    EmptyText,

    #[error("a date type ({date_type}) needs a date to qualify; add the date or drop the type")]
    DateTypeWithoutDate { date_type: String },

    #[error("'{token}' is not a date type this build understands (exact, around, range, ordered)")]
    UnknownDateType { token: String },

    #[error(
        "that would be talking point {attempted} of at most {cap}. Marie holds a \
         few points under pressure, not a list — shorten or replace one instead."
    )]
    TooManyTalkingPoints { attempted: usize, cap: usize },

    #[error("failed to store the human content: {source}")]
    Write {
        #[source]
        source: PipelineRepoError,
    },

    /// A READ failed.
    ///
    /// Separate from [`AugmentationError::Write`] because collapsing the two made
    /// a failed panel LOAD tell the human "failed to save" — a message about an
    /// action they never took, and a diagnostic trail pointing an operator at the
    /// write path when the fault was in a query. Different operations, different
    /// observables (Standing Rule 1).
    #[error("failed to read the human content: {source}")]
    Read {
        #[source]
        source: PipelineRepoError,
    },
}

/// One human fact as the caller supplies it.
#[derive(Debug)]
pub struct NewHumanFact<'a> {
    pub scenario_id: Uuid,
    pub text: &'a str,
    pub occurred_on: Option<NaiveDate>,
    pub date_type: Option<&'a str>,
    pub person_refs: &'a [String],
    pub authored_by: &'a str,
}

/// Validate a human fact before it reaches the database.
///
/// Pure, so every branch of the law is unit-testable. The table CHECKs the same
/// two rules, but a constraint violation surfaces as an opaque 500; validating
/// here turns each into a precise message naming what to fix.
///
/// ## Rust Learning: returning `Result<(), E>` from a validator
///
/// The function produces no value — its whole output is "this is acceptable, or
/// here is precisely why not". `Result<(), E>` says exactly that in the type, and
/// lets the caller use `?` to bail on the first problem.
pub fn validate_human_fact(fact: &NewHumanFact<'_>) -> Result<(), AugmentationError> {
    if fact.text.trim().is_empty() {
        return Err(AugmentationError::EmptyText);
    }

    if let Some(token) = fact.date_type {
        // Parse-don't-validate: an unknown token is refused here rather than
        // stored and mis-read later.
        DateType::try_from(token).map_err(|_| AugmentationError::UnknownDateType {
            token: token.to_string(),
        })?;

        if fact.occurred_on.is_none() {
            return Err(AugmentationError::DateTypeWithoutDate {
                date_type: token.to_string(),
            });
        }
    }
    Ok(())
}

/// Write one human fact. Returns its id.
///
/// # Errors
/// Returns [`AugmentationError`] if the fact is invalid or the write fails.
pub async fn add_human_fact(
    pool: &PgPool,
    fact: &NewHumanFact<'_>,
) -> Result<Uuid, AugmentationError> {
    validate_human_fact(fact)?;

    let now = Utc::now();
    insert_human_fact(
        pool,
        &HumanFactWrite {
            scenario_id: fact.scenario_id,
            text: fact.text.trim(),
            occurred_on: fact.occurred_on,
            date_type: fact.date_type,
            person_refs: fact.person_refs,
            authored_by: fact.authored_by,
            written_at: now,
        },
    )
    .await
    .map_err(|source| AugmentationError::Write { source })
}

/// Read a scenario's human facts, oldest first.
///
/// # Errors
/// Returns [`AugmentationError`] if the read fails.
pub async fn human_facts(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<ScenarioHumanFactRecord>, AugmentationError> {
    list_human_facts_for_scenario(pool, scenario_id)
        .await
        .map_err(|source| AugmentationError::Read { source })
}

/// Remove one human fact. `false` when the scenario has no such fact.
///
/// # Errors
/// Returns [`AugmentationError`] if the delete fails.
pub async fn remove_human_fact(
    pool: &PgPool,
    scenario_id: Uuid,
    fact_id: Uuid,
) -> Result<bool, AugmentationError> {
    let removed = delete_human_fact(pool, scenario_id, fact_id)
        .await
        .map_err(|source| AugmentationError::Write { source })?;
    Ok(removed > 0)
}

/// Check a proposed talking-point list against the cap.
///
/// Pure and separate from the write so the rule is testable without a database,
/// and so the API can reject before opening a transaction.
///
/// ## Domain note: blank points are dropped, not counted
///
/// A trailing empty field in the editor is not a talking point. Counting it would
/// refuse a legitimate third point because the human left a blank fourth box open.
pub fn check_talking_points(points: &[String]) -> Result<Vec<String>, AugmentationError> {
    let kept: Vec<String> = points
        .iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();

    let cap = talking_points_cap();
    if kept.len() > cap {
        return Err(AugmentationError::TooManyTalkingPoints {
            attempted: kept.len(),
            cap,
        });
    }
    Ok(kept)
}

/// Replace a scenario's talking points with the supplied list.
///
/// ## Why REPLACE rather than append
///
/// C5 is a short ordered list a human curates as a whole — reordering the three
/// points, or dropping the middle one, is the normal edit. An append-only API
/// would make every such edit a delete-then-add dance in the client, and the
/// ordering would be the client's job. Replacing keeps `item_index` server-owned.
///
/// The whole replacement runs in ONE transaction: a partial write would leave the
/// human with some of their old points and some of their new ones, which is worse
/// than either.
///
/// # Errors
/// Returns [`AugmentationError`] if the list exceeds the cap or a write fails.
pub async fn set_talking_points(
    pool: &PgPool,
    scenario_id: Uuid,
    points: &[String],
    authored_by: &str,
) -> Result<Vec<String>, AugmentationError> {
    let kept = check_talking_points(points)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| AugmentationError::Write { source: e.into() })?;

    // Clear the scenario's existing points. The FK cascade from
    // `scenario_responses` removes the items, so deleting the response row is
    // the whole reset — one statement rather than a per-item loop.
    delete_responses_for_scenario(&mut *tx, scenario_id)
        .await
        .map_err(|source| AugmentationError::Write { source })?;

    if !kept.is_empty() {
        // One response row per scenario (the ratified v1 reading of "per attack"):
        // a scenario IS one attack, and its ordered items are the points.
        //
        // `origin` is 'human', always. C5 is human-authored; the schema's
        // 'suggested' value exists for a future the system does not have.
        let response_id = insert_scenario_response(
            &mut *tx,
            scenario_id,
            None,
            "",
            "draft",
            "human",
            Some(authored_by),
        )
        .await
        .map_err(|source| AugmentationError::Write { source })?;

        for (index, text) in kept.iter().enumerate() {
            // `index` is server-owned, so the order the human arranged is the
            // order stored — the client never sends an index to be trusted.
            let item_index = i32::try_from(index).unwrap_or(i32::MAX);
            insert_response_item(&mut *tx, response_id, item_index, text, Some(authored_by))
                .await
                .map_err(|source| AugmentationError::Write { source })?;
        }
    }

    tx.commit()
        .await
        .map_err(|e| AugmentationError::Write { source: e.into() })?;

    Ok(kept)
}

/// Read a scenario's talking points in order.
///
/// Returns an empty list when none are authored — a real state (nobody has
/// written Marie's answer yet), not a missing one.
///
/// # Errors
/// Returns [`AugmentationError`] if a read fails.
pub async fn talking_points(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<ResponseItemRecord>, AugmentationError> {
    let responses = list_responses_for_scenario(pool, scenario_id)
        .await
        .map_err(|source| AugmentationError::Read { source })?;

    // One response per scenario by the ratified reading. More than one is
    // structurally impossible (`set_talking_points` deletes before it inserts), so
    // if it happens something wrote around this service — which is exactly the
    // kind of thing that must not pass silently.
    if responses.len() > 1 {
        tracing::warn!(
            %scenario_id,
            count = responses.len(),
            "more than one scenario_responses row for this scenario; C5 is one row \
             per scenario (v2 §2) — using the first and ignoring the rest"
        );
    }

    let Some(response) = responses.first() else {
        return Ok(Vec::new());
    };

    list_items_for_response(pool, response.id)
        .await
        .map_err(|source| AugmentationError::Read { source })
}

#[cfg(test)]
#[path = "scenario_augmentation_tests.rs"]
mod tests;
