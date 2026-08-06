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
//! by any client that did not implement it, including a curl. The cap arrives in
//! the `Settings` snapshot the handler took (task 1.6, v2 §2b) and is checked on
//! this side; the refusal names the limit so the human knows what happened.

use chrono::{NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::human_authored::{DateType, HumanFactKind};
use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::{
    delete_human_fact, delete_responses_for_scenario, insert_human_fact, insert_response_item,
    insert_scenario_response, list_human_facts_for_scenario, list_items_for_response,
    list_responses_for_scenario, update_human_fact_text, update_response_item_text, HumanFactWrite,
    PipelineRepoError, ResponseItemRecord, ScenarioHumanFactRecord,
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

    /// The edit named a row that is not there.
    ///
    /// Its own variant rather than a `bool` returned to the handler: "the write
    /// failed" and "you edited something that no longer exists" send a human to
    /// opposite remedies, and the second is the normal outcome of two people
    /// having the page open when one of them removes a point.
    #[error(
        "there is no {what} {which} on this scenario any more — somebody may have \
         removed it since this page loaded. Reload to see what is there now."
    )]
    NoSuchRow { what: String, which: String },

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
    /// `fact` or `watch_list` (task 1.5). Both travel this one write path so the
    /// §8 invariants are enforced once — see `HumanFactKind`.
    pub kind: HumanFactKind,
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
            kind: fact.kind.code(),
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
pub fn check_talking_points(
    points: &[String],
    settings: &Settings,
) -> Result<Vec<String>, AugmentationError> {
    let kept: Vec<String> = points
        .iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();

    let cap = settings.talking_points_cap;
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
    settings: &Settings,
) -> Result<Vec<String>, AugmentationError> {
    let kept = check_talking_points(points, settings)?;

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

/// Rewrite ONE talking point, addressed by its printed position.
///
/// ## Why a blank is REFUSED rather than treated as a deletion
///
/// Emptying the box and saving is indistinguishable from a slip of the keyboard,
/// and the two intentions have opposite consequences for a list a witness
/// rehearses from. Removing a point is the whole-list write's job, where the
/// human can see what the list becomes. Same discipline as the accusation
/// sentence's Withdraw control.
///
/// ## Rust Learning: `i32::try_from` at the boundary between a count and a column
///
/// `position` is a `usize` because it counts; `item_index` is `i32` because
/// Postgres `int` is signed. The conversion is fallible in principle (a `usize`
/// can exceed `i32::MAX`), and rather than clamp — which would silently edit the
/// wrong row — an out-of-range position is refused as "no such row", which is
/// exactly what it is.
///
/// # Errors
/// Returns [`AugmentationError`] if the text is blank, the position names no
/// point, or a read/write fails.
pub async fn edit_talking_point(
    pool: &PgPool,
    scenario_id: Uuid,
    position: usize,
    text: &str,
) -> Result<(), AugmentationError> {
    let trimmed = checked_line(text)?;
    let index = talking_point_index(position)?;

    let responses = list_responses_for_scenario(pool, scenario_id)
        .await
        .map_err(|source| AugmentationError::Read { source })?;

    let Some(response) = responses.first() else {
        return Err(no_such_point(position));
    };

    let changed = update_response_item_text(pool, response.id, index, trimmed)
        .await
        .map_err(|source| AugmentationError::Write { source })?;

    if changed == 0 {
        return Err(no_such_point(position));
    }
    Ok(())
}

/// The refusal for a position that names no point, composed in one place.
fn no_such_point(position: usize) -> AugmentationError {
    AugmentationError::NoSuchRow {
        what: "talking point".to_string(),
        which: position.to_string(),
    }
}

/// One authored line, trimmed — or the refusal for an empty one.
///
/// Pure and separate from the writes so the rule is testable without a database,
/// exactly as [`check_talking_points`] is. Both edit routes share it, because
/// "an empty note records nothing" is one rule and not two.
///
/// # Errors
/// Returns [`AugmentationError::EmptyText`] when the text is blank or whitespace.
pub fn checked_line(text: &str) -> Result<&str, AugmentationError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AugmentationError::EmptyText);
    }
    Ok(trimmed)
}

/// The stored `item_index` a printed position addresses.
///
/// ## Rust Learning: `checked_sub` on a `usize`
///
/// `position - 1` on a `usize` of 0 does not go negative — it WRAPS to
/// `usize::MAX`, silently, in release builds. `checked_sub` returns `None`
/// instead, which is the difference between "position 0 is not a row" and an
/// update aimed at index 9,223,372,036,854,775,807.
///
/// # Errors
/// Returns [`AugmentationError::NoSuchRow`] for position 0 or a position past
/// what an `i32` column can hold.
pub fn talking_point_index(position: usize) -> Result<i32, AugmentationError> {
    // Position 0 is not a row. Written as a `let ... else` rather than folded
    // into the chain below so the two failures stay visibly separate: one is a
    // position nobody could have meant, the other is a position past what the
    // column can hold, and both end at the same refusal for the same reason.
    let Some(zero_based) = position.checked_sub(1) else {
        return Err(no_such_point(position));
    };

    // `map_err` rather than `.ok()`: the discarded `TryFromIntError` says only
    // "out of range", which the refusal replacing it already says WITH the
    // position the caller actually sent. Nothing observable is lost, and the
    // conversion is never clamped — clamping would edit the LAST point instead
    // of refusing, which is a silent write to the wrong row.
    i32::try_from(zero_based).map_err(|_| no_such_point(position))
}

/// Rewrite ONE watch-list note, in place.
///
/// The row keeps its `authored_by` and `created_at`; only `updated_at` moves, so
/// the panel's "edited since written" tag stays true. See
/// `update_human_fact_text` for why this is not a delete-then-insert.
///
/// # Errors
/// Returns [`AugmentationError`] if the text is blank, no such note exists on
/// this scenario, or the write fails.
pub async fn edit_watch_item(
    pool: &PgPool,
    scenario_id: Uuid,
    fact_id: Uuid,
    text: &str,
) -> Result<(), AugmentationError> {
    let trimmed = checked_line(text)?;

    let changed = update_human_fact_text(
        pool,
        scenario_id,
        fact_id,
        trimmed,
        HumanFactKind::WatchList.code(),
    )
    .await
    .map_err(|source| AugmentationError::Write { source })?;

    if changed == 0 {
        return Err(AugmentationError::NoSuchRow {
            what: "watch item".to_string(),
            which: fact_id.to_string(),
        });
    }
    Ok(())
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
