//! "Done reviewing" — the one act that moves a reader's cursor on a deck.
//!
//! CC_TASK_REVIEW_LOOP_v1 §1. `PUT /cases/:slug/scenarios/:scenario_id/practice/review-cursor`
//! upserts `now()` for the signed-in user. There is no delete route, no per-item
//! mark and no stored count: the War Room pill and the deck's review bar are both
//! DERIVED from this one timestamp at read time (`review_cursor::new_answers_for_viewer`).
//!
//! ## Why PUT
//!
//! Pressing it twice leaves the same one row — the mark simply moves to the later
//! press. That is idempotent in the sense HTTP means, which is what PUT promises
//! and POST does not.
//!
//! ## Why this route IS case- and scenario-scoped
//!
//! Unlike the answer and note routes, its target is a scenario the caller names
//! in the path, not a server-minted row id; the case fence
//! (`ensure_scenario_in_case`) is what stops a mark landing on another case's deck.

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::practice_review::ReviewCursorResponse,
    error::AppError,
    repositories::pipeline_repository::review_cursor::{
        mark_reviewed, new_answers_for_viewer, ViewerNewRow,
    },
    services::practice_notes::attribution,
    state::AppState,
};

use super::practice::repo_error;
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// Move the signed-in user's review mark on this deck to now.
///
/// ## Domain note: per person, never shared
///
/// The cursor is keyed by `username` — the id `attribution` stamps on every
/// practice write. Chuck pressing it zeroes Chuck's pill and bar; Roman's are
/// untouched, because Roman has not read those answers.
///
/// # Errors
/// 400 for an unparseable scenario id; 404 for a scenario not in this case;
/// 500 (logged) for a failed write.
pub async fn put_review_cursor(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<ReviewCursorResponse>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;
    let (user_id, _) = attribution(&user);

    let looked_at = mark_reviewed(&state.pipeline_pool, &user_id, scenario_id)
        .await
        .map_err(|e| {
            repo_error(
                "mark_reviewed",
                format!("{slug} scenario {scenario_id} user {user_id}: {e}"),
            )
        })?;

    tracing::info!(%slug, %scenario_id, user = %user_id, %looked_at, "practice: deck marked reviewed");
    Ok(Json(ReviewCursorResponse {
        scenario_id,
        looked_at,
    }))
}

/// The review bar's number: this viewer's unreviewed answers on one deck.
///
/// The same query as the War Room pill, asked about one scenario, so the bar and
/// the pill cannot disagree. A missing row or a count that is not a `u32` is a
/// logged 500 — never a quiet 0 that would hide the bar.
pub(super) async fn viewer_new_count(
    state: &AppState,
    scenario_id: Uuid,
    user_id: &str,
) -> Result<u32, AppError> {
    let rows = new_answers_for_viewer(&state.pipeline_pool, &[scenario_id], Some(user_id))
        .await
        .map_err(|e| {
            repo_error(
                "new_answers_for_viewer",
                format!("scenario {scenario_id} user {user_id}: {e}"),
            )
        })?;
    usable_count(&rows, scenario_id).ok_or_else(|| {
        repo_error(
            "new_answers_for_viewer",
            format!("no usable count for {scenario_id} in {rows:?}"),
        )
    })
}

/// The count for `scenario_id`, if the read returned one that fits a `u32`.
///
/// `None` for a missing row or a negative/overflowing count — both are query
/// defects, and the caller turns them into a logged 500 rather than a quiet 0.
fn usable_count(rows: &[ViewerNewRow], scenario_id: Uuid) -> Option<u32> {
    rows.iter()
        .find(|r| r.scenario_id == scenario_id)
        .and_then(|r| u32::try_from(r.new_answers).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(n: u128, new_answers: i64) -> ViewerNewRow {
        ViewerNewRow {
            scenario_id: Uuid::from_u128(n),
            new_answers,
        }
    }

    /// The asked-for scenario's count comes back, not a neighbour's.
    #[test]
    fn the_count_is_the_asked_scenarios() {
        let rows = [row(1, 7), row(2, 4)];
        assert_eq!(usable_count(&rows, Uuid::from_u128(2)), Some(4));
    }

    /// A read that forgot the scenario is `None` — never a quiet 0.
    #[test]
    fn a_missing_scenario_is_none() {
        assert_eq!(usable_count(&[row(1, 7)], Uuid::from_u128(9)), None);
    }

    /// A negative count is refused rather than wrapped.
    #[test]
    fn a_negative_count_is_none() {
        assert_eq!(usable_count(&[row(1, -1)], Uuid::from_u128(1)), None);
    }
}
