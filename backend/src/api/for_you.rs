//! "For you" — one page that answers "what is waiting for me?" across the case.
//!
//! CC_TASK_FOR_YOU_v1 layer L1. Three addresses:
//!
//! - `GET  /cases/:slug/for-you`          → the page, in one request
//! - `GET  /cases/:slug/for-you/summary`  → the menu badge's one number
//! - `POST /practice/questions/:id/seen`  → opening a question clears it
//!
//! ## Why the badge has its own address
//!
//! The count rides every page in the app. Asking the whole list for it would
//! make every screen pay for rows nobody is looking at; asking a second query
//! for it would risk a badge that disagrees with the page. So it is the SAME
//! predicate, counted (`waiting_total`) instead of listed.
//!
//! ## Why the read-clear is NOT case-scoped
//!
//! `question_id` is a server-minted handle the browser only learns by having
//! been served a row about it — the same argument `practice.rs` makes for the
//! answer and note routes. The write it performs is per-person read-state and
//! touches nothing anybody else can see.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table read here lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::for_you::{ForYouPayload, ForYouSide, ForYouSummaryDto, QuestionSeenResponse},
    error::AppError,
    repositories::pipeline_repository::{
        item_seen::mark_question_seen,
        scenario_store::list_scenarios_for_case,
        waiting_items::{waiting_items, waiting_total, WaitingItemRow, WaitingQuery, WaitingScope},
    },
    services::{
        for_you::{assemble_page, query_side, side_for},
        for_you_rows::RowVoice,
        practice_clock::local_day,
        practice_notes::attribution,
        war_room_progress::{practice_witness, review_queue_reviewer, reviewer_display_line},
    },
    state::AppState,
};

/// This route's own id parser.
///
/// ## Why not `scenario_facts::parse_scenario_id`
///
/// It did use it, and the 400 it produces carries `details: {"field":
/// "scenario_id"}` — a parameter that does not appear in this route at all. A
/// caller who sent a malformed QUESTION id was told to look at a scenario id,
/// which is the WHERE half of an error message pointing at the wrong place.
fn parse_question_id(raw: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(raw).map_err(|_| AppError::BadRequest {
        message: "question_id must be a valid UUID".to_string(),
        details: serde_json::json!({ "field": "question_id" }),
    })
}

/// Turn a repository failure into a 500 that says nothing, having logged
/// everything.
///
/// ## Why this page does not reuse `practice::repo_error`
///
/// It did, and the body it returns reads "the practice drill could not reach
/// its store" — the wrong feature. A person whose waiting list failed to load
/// is not in the drill, and a message naming a screen they are not on sends
/// them looking in the wrong place. The LOG half is identical in shape: the
/// operation, the inputs, and the underlying cause, which is what an operator
/// needs and what the caller must never be told.
fn store_error(operation: &'static str, error: impl std::fmt::Display) -> AppError {
    tracing::error!(operation, error = %error, "for you: database call failed");
    AppError::Internal {
        message: "your waiting list could not be read".to_string(),
    }
}

/// This module's routes, declared beside their handlers.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/cases/:slug/for-you", get(get_for_you))
        .route("/cases/:slug/for-you/summary", get(get_for_you_summary))
        .route(
            "/practice/questions/:question_id/seen",
            post(post_question_seen),
        )
}

/// Narrow the page to ONE deck (CC_TASK_FOR_YOU_v1 L2).
///
/// ## Domain note: where this arrives from
///
/// The war room's counts link here. "2 notes for Marie" on a card used to be a
/// dead number; it now opens this page filtered to that deck, so the count and
/// the rows behind it are one click apart and cannot be read as disagreeing.
///
/// A deck that does not belong to this case simply matches nothing and the page
/// is empty — the case's own scenario list is what the filter runs over, so a
/// guessed id cannot reach another case's items.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForYouQuery {
    /// A scenario id. Absent means the whole case, which is the page's own address.
    #[serde(default)]
    pub deck: Option<Uuid>,
}

/// Every scenario of this case, as ids — or just the one the query names.
async fn case_scenarios(
    state: &AppState,
    slug: &str,
    deck: Option<Uuid>,
) -> Result<Vec<Uuid>, AppError> {
    let records = list_scenarios_for_case(&state.pipeline_pool, slug)
        .await
        .map_err(|e| store_error("list_scenarios_for_case", format!("{slug}: {e}")))?;
    Ok(records
        .iter()
        .map(|r| r.scenario_id)
        .filter(|id| deck.is_none_or(|only| only == *id))
        .collect())
}

/// The page: the unread list, the everything list, and the words for both.
///
/// ## Domain note: a person who is neither side gets a PAGE, not a 403
///
/// Nothing was refused — this simply is not their list. The payload says so
/// (`side: "none"`) and carries the sentence that explains it, and no query is
/// run at all.
///
/// # Errors
/// A logged 500 naming the read that failed. There is no 404 and no 403: an
/// unknown case yields no scenarios, which is an empty list rather than an
/// error, exactly as an empty case would be.
pub async fn get_for_you(
    user: AuthUser,
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<ForYouQuery>,
) -> Result<Json<ForYouPayload>, AppError> {
    let settings = state.settings.current();
    let (user_id, _) = attribution(&user);
    let reviewers = review_queue_reviewer(&settings);
    let witness = practice_witness(&settings);
    let side = side_for(&user_id, user.is_admin(), reviewers, witness);
    let wording = &settings.for_you_wording;
    let timezone = &settings.practice_read.case_timezone;

    let (unread, everything) = match query_side(side) {
        Some(query_side) => {
            let ids = case_scenarios(&state, &slug, query.deck).await?;
            read_both(&state, &slug, &ids, &user_id, reviewers, query_side).await?
        }
        None => (Vec::new(), Vec::new()),
    };

    let voice = RowVoice {
        wording,
        timezone,
        side,
        reviewer_logins: reviewers,
        reviewer_names: &settings.practice_read.reviewer_display_names,
        witness_login: witness,
        today: local_day(chrono::Utc::now(), timezone),
    };
    // WHO the reader is waiting on: the bench for the witness, the witness for
    // a reviewer. It fills `{who}` in the subtitle and in the empty state.
    let other_side = match side {
        ForYouSide::Witness => reviewer_display_line(&settings),
        _ => wording.witness_name.clone(),
    };
    tracing::info!(
        %slug, user = %user_id, side = ?side, deck = ?query.deck,
        unread = unread.len(), everything = everything.len(),
        "for you: served the page"
    );
    Ok(Json(assemble_page(
        &voice,
        &other_side,
        &unread,
        &everything,
        settings.practice_read.for_you_deck_threshold,
    )))
}

/// Both tabs, as two readings of one predicate.
///
/// ## Rust Learning: `tokio::try_join!`
///
/// The two reads do not depend on one another, so they run concurrently and the
/// request pays roughly the slower of the two rather than their sum. `try_join!`
/// also short-circuits: if either fails, that error is returned and the other is
/// dropped.
async fn read_both(
    state: &AppState,
    slug: &str,
    ids: &[Uuid],
    user_id: &str,
    reviewers: &[String],
    side: crate::repositories::pipeline_repository::waiting_items::WaitingSide,
) -> Result<(Vec<WaitingItemRow>, Vec<WaitingItemRow>), AppError> {
    // ## Rust Learning: two bindings, not two temporaries
    //
    // `&ask(scope)` inside the `try_join!` would borrow a value created and
    // dropped within the same statement, and the futures outlive it. Naming the
    // two queries gives each a home for the whole await — which the compiler
    // insists on, and which also makes the two readings visible side by side.
    let unseen = WaitingQuery {
        scenario_ids: ids,
        viewer: user_id,
        reviewers,
        side,
        scope: WaitingScope::Unseen,
    };
    let everything_query = WaitingQuery {
        scope: WaitingScope::Everything,
        ..unseen
    };
    let pool = &state.pipeline_pool;
    // No LIMIT: this is one person's inbox across the whole case, and silently
    // truncating it would hide work — see `waiting_items`.
    let (unread, everything) = tokio::try_join!(
        waiting_items(pool, &unseen, None),
        waiting_items(pool, &everything_query, None),
    )
    .map_err(|e| store_error("waiting_items", format!("{slug} user {user_id}: {e}")))?;
    Ok((unread, everything))
}

/// The menu badge's one number.
///
/// # Errors
/// A logged 500 naming the read that failed.
pub async fn get_for_you_summary(
    user: AuthUser,
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<ForYouSummaryDto>, AppError> {
    let settings = state.settings.current();
    let (user_id, _) = attribution(&user);
    let reviewers = review_queue_reviewer(&settings);
    let side = side_for(
        &user_id,
        user.is_admin(),
        reviewers,
        practice_witness(&settings),
    );
    let Some(query_side) = query_side(side) else {
        return Ok(Json(ForYouSummaryDto {
            side,
            unread_count: 0,
        }));
    };
    let ids = case_scenarios(&state, &slug, None).await?;
    let total = waiting_total(
        &state.pipeline_pool,
        &WaitingQuery {
            scenario_ids: &ids,
            viewer: &user_id,
            reviewers,
            side: query_side,
            scope: WaitingScope::Unseen,
        },
    )
    .await
    .map_err(|e| store_error("waiting_total", format!("{slug} user {user_id}: {e}")))?;
    // A count that does not fit a u32 is a query defect, not a badge: refused
    // loudly rather than wrapped into a small number that looks plausible.
    let unread_count = u32::try_from(total).map_err(|_| {
        store_error(
            "waiting_total",
            format!("{slug} user {user_id}: the unread count {total} is not a u32"),
        )
    })?;
    Ok(Json(ForYouSummaryDto { side, unread_count }))
}

/// Opening a question marks everything on it read, for this person (ruling Q4).
///
/// Idempotent: a second visit writes nothing and says so with a 0.
///
/// # Errors
/// 400 for an unparseable question id; a logged 500 for a failed write.
pub async fn post_question_seen(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<String>,
) -> Result<Json<QuestionSeenResponse>, AppError> {
    let question_id = parse_question_id(&question_id)?;
    let (user_id, _) = attribution(&user);
    let marked = mark_question_seen(&state.pipeline_pool, &user_id, question_id)
        .await
        .map_err(|e| {
            store_error(
                "mark_question_seen",
                format!("question {question_id} user {user_id}: {e}"),
            )
        })?;
    // The SAME refusal `get_for_you_summary` makes of the same conversion: a
    // count that does not fit a `u32` is a query defect, and `u32::MAX` is a
    // plausible-looking number on a 200 that no log would ever explain.
    let marked = u32::try_from(marked).map_err(|_| {
        store_error(
            "mark_question_seen",
            format!("question {question_id} user {user_id}: the row count {marked} is not a u32"),
        )
    })?;
    tracing::info!(%question_id, user = %user_id, marked, "for you: a question was read");
    Ok(Json(QuestionSeenResponse {
        question_id,
        marked,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A malformed question id is a 400, not a 500 and not a silent empty write.
    #[test]
    fn parse_question_id_rejects_non_uuid() {
        let result = parse_question_id("not-a-uuid");
        assert!(matches!(result, Err(AppError::BadRequest { .. })));
    }

    /// (M) AND THE 400 NAMES THE RIGHT PARAMETER.
    ///
    /// This is the whole reason the function exists. The route used to reuse
    /// `scenario_facts::parse_scenario_id`, whose refusal carries
    /// `details: {"field": "scenario_id"}` — a parameter this route does not
    /// have. A caller who mistyped a QUESTION id was sent to look at a scenario
    /// id. Nothing failed, nothing logged, and the message pointed at the wrong
    /// place; a regression to it would be just as quiet, which is why the field
    /// name is asserted rather than only the status.
    #[test]
    fn the_refusal_names_the_question_id_and_not_a_scenario() {
        let Err(AppError::BadRequest { message, details }) = parse_question_id("nope") else {
            panic!("a malformed question id is a 400");
        };
        assert!(message.contains("question_id"), "message: {message}");
        assert_eq!(details["field"], "question_id");
    }

    /// A real uuid comes back unchanged.
    #[test]
    fn parse_question_id_accepts_a_uuid() {
        let id = Uuid::new_v4();
        let parsed = parse_question_id(&id.to_string()).expect("a valid uuid string parses");
        assert_eq!(parsed, id);
    }
}
