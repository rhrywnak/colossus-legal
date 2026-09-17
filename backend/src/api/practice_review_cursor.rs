//! "Done reviewing" — the one act that moves a reader's cursor on a deck.
//!
//! CC_TASK_REVIEW_LOOP_v1 §1. `PUT /cases/:slug/scenarios/:scenario_id/practice/review-cursor`
//! upserts `now()` for the signed-in user. There is no delete route, no per-item
//! mark and no stored count: the War Room pill and the deck's review bar are both
//! DERIVED at read time (`review_cursor::awaiting_review`) — from the REVIEWER's
//! row only, since CC_TASK_SIMPLE_COUNTS_v1.
//!
//! ## Why the route still accepts anybody's press
//!
//! The route is unchanged by that task: any signed-in user may still write their
//! own row. It simply is not READ unless that user is the reviewer, so a press by
//! anyone else changes nothing on any page — and the page offers the button only
//! to the reviewer (`can_mark_reviewed`).
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
    dto::practice_review::DeckReviewDto,
    dto::practice_review::ReviewCursorResponse,
    error::AppError,
    repositories::pipeline_repository::review_cursor::{
        awaiting_review, mark_reviewed, AwaitingReviewRow,
    },
    services::{practice_notes::attribution, war_room_progress::review_queue_reviewer},
    state::AppState,
};

use super::practice::repo_error;
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// Move the signed-in user's review mark on this deck to now.
///
/// ## Domain note: only the reviewer's row is ever read
///
/// The cursor is keyed by `username` — the id `attribution` stamps on every
/// practice write. The review queue reads the `practice_reviewer_username` row
/// alone, so the reviewer's press clears the queue for everyone and anybody
/// else's press is recorded and ignored.
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

/// The deck's review bar: the reviewer's backlog on this scenario, and whether
/// THIS user may press Done reviewing.
///
/// The same query as the War Room pill, asked about one scenario, so the bar and
/// the pill cannot disagree. A missing row or a count that is not a `u32` is a
/// logged 500 — never a quiet 0 that would hide the bar.
pub(super) async fn deck_review(
    state: &AppState,
    scenario_id: Uuid,
    user_id: &str,
) -> Result<DeckReviewDto, AppError> {
    let settings = state.settings.current();
    let reviewer = review_queue_reviewer(&settings);
    let rows = awaiting_review(&state.pipeline_pool, &[scenario_id], reviewer)
        .await
        .map_err(|e| {
            repo_error(
                "awaiting_review",
                format!("scenario {scenario_id} reviewer {reviewer}: {e}"),
            )
        })?;
    let awaiting = usable_count(&rows, scenario_id).ok_or_else(|| {
        repo_error(
            "awaiting_review",
            format!("no usable count for {scenario_id} in {rows:?}"),
        )
    })?;
    Ok(DeckReviewDto {
        awaiting,
        can_mark_reviewed: can_mark_reviewed(user_id, reviewer),
        reviewer_display_name: settings.practice_read.reviewer_display_name.clone(),
    })
}

/// Whether this signed-in user may press Done reviewing: only the reviewer may.
///
/// `user_id` is `attribution`'s stable id (the Authentik username), the same
/// value the cursor row is keyed by — so "may press" and "whose press is read"
/// are one comparison and cannot drift apart. Exact match: usernames are ids.
pub(super) fn can_mark_reviewed(user_id: &str, reviewer: &str) -> bool {
    user_id == reviewer
}

/// The count for `scenario_id`, if the read returned one that fits a `u32`.
///
/// `None` for a missing row or a negative/overflowing count — both are query
/// defects, and the caller turns them into a logged 500 rather than a quiet 0.
fn usable_count(rows: &[AwaitingReviewRow], scenario_id: Uuid) -> Option<u32> {
    rows.iter()
        .find(|r| r.scenario_id == scenario_id)
        .and_then(|r| u32::try_from(r.awaiting).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(n: u128, awaiting: i64) -> AwaitingReviewRow {
        AwaitingReviewRow {
            scenario_id: Uuid::from_u128(n),
            awaiting,
            oldest: None,
        }
    }

    /// Only the reviewer is offered Done reviewing — the settings row's login,
    /// exactly (mutation-proved in the report: flipping the comparison reds this).
    #[test]
    fn can_mark_reviewed_only_for_the_reviewer() {
        assert!(can_mark_reviewed("cpenzien", "cpenzien"));
        assert!(!can_mark_reviewed("roman", "cpenzien"));
        assert!(!can_mark_reviewed("docmarie", "cpenzien"));
        // The reviewer comes from the row, not a literal: change it, and the
        // answer follows.
        assert!(can_mark_reviewed("roman", "roman"));
        assert!(!can_mark_reviewed("cpenzien", "roman"));
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
