//! The subset READS. Open, like the timeline's own (T1.3).
//!
//! ```text
//! GET /api/timeline/subsets      the home section's list
//! GET /api/timeline/subsets/:id  the subset with its events joined
//! ```
//!
//! Both take `Option<AuthUser>`: looking at a story is not privileged, exactly
//! as looking at the chronology is not. Nothing in this file writes anything.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::auth::AuthUser;
use crate::dto::chronology_subset::{SubsetDetailDto, SubsetSummaryDto};
use crate::error::AppError;
use crate::repositories::pipeline_repository::chronology_subsets::{
    carriers_for_subsets, list_subsets, subset_counts,
};
use crate::state::AppState;

use super::support::{parse_subset_id, require_subset, subset_failure, subset_response};
use crate::api::timeline::case_slug;
use crate::services::chronology_subset_read::{
    build_subset_list, carriers_by_subset, counts_by_subset,
};

/// `GET /api/timeline/subsets` — every live subset for the configured case.
///
/// Three queries, not three-per-subset: the subsets, their two counts in one
/// grouped read, and every carrying scenario in one more.
///
/// # Errors
/// 503 when `CASE_SLUG` is unset — an unconfigured deployment and a case with no
/// subsets must not look the same (Standing Rule 1); 500 for a database failure.
#[tracing::instrument(skip(state, user), fields(case_slug = tracing::field::Empty))]
pub async fn get_subsets(
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SubsetSummaryDto>>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /timeline/subsets", u.username);
    }
    let case = case_slug(&state)?;
    tracing::Span::current().record("case_slug", tracing::field::display(&case));

    let subsets = list_subsets(&state.pipeline_pool, &case)
        .await
        .map_err(|e| subset_failure(e, "the subset list"))?;
    let ids: Vec<uuid::Uuid> = subsets.iter().map(|s| s.id).collect();

    let counts = subset_counts(&state.pipeline_pool, &ids)
        .await
        .map_err(|e| subset_failure(e, "the per-subset event and gap counts"))?;
    let carriers = carriers_for_subsets(&state.pipeline_pool, &ids)
        .await
        .map_err(|e| subset_failure(e, "the scenarios carrying these subsets"))?;

    let payload = build_subset_list(
        &subsets,
        &counts_by_subset(&counts),
        &carriers_by_subset(&carriers),
    );
    tracing::info!(subsets = payload.len(), "timeline subsets read");
    Ok(Json(payload))
}

/// `GET /api/timeline/subsets/:id` — the subset, its events in story order.
///
/// A DELETED subset is returned rather than hidden, with its `deleted_at` set:
/// the Undo line has to be able to show what it would restore, and a 404 for a
/// row that is one press from live would be the wrong answer to "what was this?".
///
/// # Errors
/// 400 when the id is not a UUID; 404 when there is no such subset; 500 for a
/// database failure.
#[tracing::instrument(skip(state, user), fields(subset_id = %id))]
pub async fn get_subset(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SubsetDetailDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /timeline/subsets/{}", u.username, id);
    }
    let subset_id = parse_subset_id(&id)?;
    let subset = require_subset(&state, subset_id, "the subset being read").await?;
    let payload = subset_response(&state, &subset).await?;
    tracing::info!(
        subset_id = %subset.id,
        events = payload.event_count,
        gaps = payload.gap_count,
        "timeline subset read"
    );
    Ok(Json(payload))
}
