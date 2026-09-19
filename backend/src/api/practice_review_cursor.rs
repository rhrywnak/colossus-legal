//! "Done reviewing" — the one act that moves a reader's cursor on a deck.
//!
//! CC_TASK_REVIEW_LOOP_v1 §1. `PUT /cases/:slug/scenarios/:scenario_id/practice/review-cursor`
//! upserts `now()` for the signed-in user. There is no delete route, no per-item
//! mark and no stored count: the War Room pill and the deck's review bar are both
//! DERIVED at read time (`review_cursor::awaiting_review`) — from the REVIEWER
//! BENCH's rows only, since CC_TASK_SIMPLE_COUNTS_v1 (one login until
//! CC_TASK_REVIEW_PAGE_v1 made it a list).
//!
//! ## Why the route still accepts anybody's press
//!
//! The route is unchanged by that task: any signed-in user may still write their
//! own row. It simply is not READ unless that user is on the bench, so a press by
//! anyone else changes nothing on any page — and the page offers the button only
//! to a listed reviewer (`can_mark_reviewed`).
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
    services::{
        practice_notes::attribution,
        war_room_progress::{review_queue_reviewer, reviewer_display_line},
    },
    state::AppState,
};

use super::practice::repo_error;
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// Move the signed-in user's review mark on this deck to now.
///
/// ## Domain note: only the reviewer's row is ever read
///
/// The cursor is keyed by `username` — the id `attribution` stamps on every
/// practice write. The review queue reads the logins in the
/// `practice_reviewer_usernames` row alone, and takes the LATEST of their marks,
/// so any listed reviewer's press clears the queue for everyone and anybody
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
    let reviewers = review_queue_reviewer(&settings);
    let rows = awaiting_review(&state.pipeline_pool, &[scenario_id], reviewers)
        .await
        .map_err(|e| {
            repo_error(
                "awaiting_review",
                format!("scenario {scenario_id} reviewers {reviewers:?}: {e}"),
            )
        })?;
    let row = rows.iter().find(|r| r.scenario_id == scenario_id);
    let awaiting = usable_count(&rows, scenario_id).ok_or_else(|| {
        repo_error(
            "awaiting_review",
            format!("no usable count for {scenario_id} in {rows:?}"),
        )
    })?;
    Ok(DeckReviewDto {
        awaiting,
        can_mark_reviewed: can_mark_reviewed(user_id, reviewers),
        reviewer_display_name: reviewer_display_line(&settings),
        // Formatted HERE, in the case's own timezone, like every other date on
        // this surface — the browser holds no date format and fills only the
        // stored clause's `{date}`. `None` when the read returned no date, which
        // is the honest shape: a deck with nothing waiting has no oldest item.
        oldest: row.and_then(|r| r.oldest).map(|at| {
            crate::services::practice_clock::local_day_month(
                at,
                &settings.practice_read.case_timezone,
            )
        }),
    })
}

/// Whether this signed-in user may press Done reviewing: only a listed reviewer.
///
/// `user_id` is `attribution`'s stable id (the Authentik username), the same
/// value the cursor row is keyed by — so "may press" and "whose press is read"
/// are one comparison and cannot drift apart. Exact match on each entry:
/// usernames are ids, not names, and a case-insensitive or trimmed comparison
/// here would let a mistyped row silently grant the button to somebody.
///
/// ## Domain note: MEMBERSHIP, not equality (ruled 2026-09-19)
///
/// This was `user_id == reviewer` until CC_TASK_REVIEW_PAGE_v1. Chuck reviews
/// Marie's answers; before trial, so does Roman, and one row could not say so
/// without handing the whole queue from one man to the other.
///
/// ## Rust Learning: `iter().any()` over a slice
///
/// `any` short-circuits on the first match and returns a plain `bool` — no
/// allocation, no `HashSet` built per request for a list of two. `r == user_id`
/// compares a `&String` with a `&str` through `PartialEq<str>`, which is why
/// neither side needs a `.as_str()`.
pub(super) fn can_mark_reviewed(user_id: &str, reviewers: &[String]) -> bool {
    // A blank caller is refused before the list is consulted. The reader drops
    // blank entries, so a bench cannot hold one today — but `"" == ""` is true,
    // and this is the one comparison in the build where being wrong hands a
    // write control to somebody the auth layer could not name.
    !user_id.trim().is_empty() && reviewers.iter().any(|r| r == user_id)
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

    /// One name on the bench behaves exactly as the old equality test did.
    ///
    /// The migration seeds the bench FROM the single row it replaces, so on the
    /// day this ships every store has exactly one entry. If this behaviour had
    /// moved, it would have moved for every deployment at once.
    #[test]
    fn one_name_behaves_exactly_as_the_old_equality() {
        let one = bench(&["cpenzien"]);
        assert!(can_mark_reviewed("cpenzien", &one));
        assert!(!can_mark_reviewed("roman", &one));
        assert!(!can_mark_reviewed("docmarie", &one));
        // The bench comes from the row, not a literal: change it, and the
        // answer follows.
        let moved = bench(&["roman"]);
        assert!(can_mark_reviewed("roman", &moved));
        assert!(!can_mark_reviewed("cpenzien", &moved));
    }

    /// Done reviewing is offered to EVERY listed reviewer and to nobody else —
    /// membership, not equality (mutation-proved in the report: negating the
    /// predicate reds this).
    ///
    /// First, last and middle are all asserted on purpose: a predicate that
    /// compared only the first entry would pass a test that checked one name.
    #[test]
    fn can_mark_reviewed_is_membership_over_the_whole_bench() {
        let bench = bench(&["cpenzien", "roman", "jdoe"]);
        assert!(can_mark_reviewed("cpenzien", &bench), "the first entry");
        assert!(can_mark_reviewed("roman", &bench), "a middle entry");
        assert!(can_mark_reviewed("jdoe", &bench), "the last entry");
        assert!(!can_mark_reviewed("docmarie", &bench), "nobody else");
    }

    /// An EMPTY bench offers the button to nobody.
    ///
    /// The boot check refuses a blank row, so this state cannot reach a running
    /// server — but `any` over an empty slice being `false` is what makes the
    /// refusal safe rather than merely early. The opposite (`all`, which is
    /// `true` when empty) would have handed the button to every signed-in user
    /// the moment a row went blank.
    #[test]
    fn an_empty_bench_offers_the_button_to_nobody() {
        assert!(!can_mark_reviewed("cpenzien", &[]));
        assert!(!can_mark_reviewed("", &[]));
    }

    /// A blank entry never matches, and neither does a near-miss.
    ///
    /// A trailing comma in the settings row is the way a blank entry gets in.
    /// Matching one would grant Done reviewing to a caller whose username the
    /// auth layer left empty, which is the worst possible reading of a typo.
    #[test]
    fn a_blank_or_near_miss_username_never_matches() {
        let bench = bench(&["cpenzien", ""]);
        assert!(
            !can_mark_reviewed("", &bench),
            "a blank login is not a member"
        );
        assert!(!can_mark_reviewed("CPENZIEN", &bench), "usernames are ids");
        assert!(
            !can_mark_reviewed("cpenzien ", &bench),
            "and are not trimmed"
        );
    }

    /// The bench as a slice of owned names — the shape the settings row reads as.
    fn bench(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| (*n).to_string()).collect()
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
