//! Attaching a story to a scenario, and the button's read (T1.3).
//!
//! ```text
//! GET    /api/cases/:slug/scenarios/:scenario_id/subsets              the button's data
//! POST   /api/cases/:slug/scenarios/:scenario_id/subsets              attach
//! DELETE /api/cases/:slug/scenarios/:scenario_id/subsets/:subset_id   detach
//! ```
//!
//! ## Why these live under the scenario path
//!
//! The path IS the fence. `ensure_scenario_in_case` is the house pattern every
//! `/cases/:slug/scenarios/:id/…` route already applies: a scenario id is
//! globally unique, so reaching a real scenario through the wrong case's path
//! must answer 404 (not 403) — the response never confirms the row exists under
//! some other case. Hanging these three off `/timeline` instead would have meant
//! inventing a second fence, and two fences is one too many.
//!
//! ## ⚑ The read is open; both writes are not
//!
//! `GET` takes `Option<AuthUser>` like every scenario read beside it. `POST` and
//! `DELETE` take `AuthUser`, and the attach stamps `attached_by` from the login.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::auth::AuthUser;
use crate::dto::chronology_subset::{AttachSubsetRequest, ScenarioSubsetDto, ScenarioSubsetsDto};
use crate::dto::chronology_wording::ChronologyWordingDto;
use crate::error::AppError;
use crate::repositories::pipeline_repository::chronology_subsets::{
    is_subset_attached, list_scenario_subsets, next_scenario_subset_position,
};
use crate::services::chronology_guard::open_write;
use crate::services::chronology_subset_read::build_scenario_subsets;
use crate::services::chronology_subset_write as subset_write;
use crate::state::AppState;

use super::support::{require_subset, subset_failure, write_error};
use crate::api::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// `GET /api/cases/:slug/scenarios/:scenario_id/subsets` — the View Timeline
/// button's data.
///
/// Returns `[]` for a scenario carrying none, which is what hides the button. An
/// empty list and a 404 are deliberately different answers: the first says "this
/// scenario has no stories yet", the second says "there is no such scenario",
/// and a surface that collapsed them would draw a working page for a scenario
/// that does not exist.
///
/// # Errors
/// 400 when `scenario_id` is not a UUID; 404 when the scenario does not exist in
/// this case; 500 for a database failure.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn get_scenario_subsets(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<ScenarioSubsetsDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET scenario subsets", u.username);
    }
    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let rows = list_scenario_subsets(&state.pipeline_pool, id)
        .await
        .map_err(|e| subset_failure(e, "the subsets attached to this scenario"))?;
    let subsets = build_scenario_subsets(&rows);
    // ONE snapshot read, the same discipline `/api/timeline` keeps: the words
    // the dock draws with must describe the same configuration as each other.
    // `From<&ChronologyWording>` is the timeline's own conversion, so this is
    // the same block and not a copy of it.
    let wording = ChronologyWordingDto::from(&state.settings.current().chronology_wording);
    tracing::info!(attached = subsets.len(), "scenario subsets read");
    Ok(Json(ScenarioSubsetsDto { subsets, wording }))
}

/// `POST /api/cases/:slug/scenarios/:scenario_id/subsets` — attach one subset.
///
/// Appended at the next position, so the order a scenario carries its stories in
/// is the order somebody attached them until they say otherwise.
///
/// ## Why a DELETED subset is refused
///
/// Attaching a story that is in the bin would create a link the scenario reads
/// as nothing: `list_scenario_subsets` excludes deleted subsets, so the button
/// would stay hidden and the person who pressed Attach would have no way to tell
/// whether it worked. A 409 naming the state is the answer they can act on.
///
/// # Errors
/// 400 for a malformed id; 404 when the scenario is not in this case or the
/// subset does not exist; 409 when it is already attached, or the subset is
/// deleted; 500 for a database failure.
#[tracing::instrument(skip(state, user, body), fields(by = %user.username, slug = %slug, scenario_id = %scenario_id))]
pub async fn post_scenario_subset(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(body): Json<AttachSubsetRequest>,
) -> Result<Json<Vec<ScenarioSubsetDto>>, AppError> {
    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;
    let writer = open_write(&user);

    let subset = require_subset(&state, body.subset_id, "the subset being attached").await?;
    if subset.deleted_at.is_some() {
        return Err(AppError::Conflict {
            message: format!(
                "timeline subset {} is deleted; restore it with Undo before attaching it",
                subset.id
            ),
            details: serde_json::json!({ "subset_id": subset.id, "deleted_at": subset.deleted_at }),
        });
    }
    if is_subset_attached(&state.pipeline_pool, id, subset.id)
        .await
        .map_err(|e| subset_failure(e, "whether this subset is already attached"))?
    {
        return Err(AppError::Conflict {
            message: format!("this scenario already carries '{}'", subset.name),
            details: serde_json::json!({ "subset_id": subset.id }),
        });
    }

    let position = next_scenario_subset_position(&state.pipeline_pool, id)
        .await
        .map_err(|e| subset_failure(e, "the next attachment position"))?;
    subset_write::attach(&state.pipeline_pool, id, subset.id, position, &writer)
        .await
        .map_err(|e| write_error(e, "attaching a subset to a scenario"))?;

    tracing::info!(subset_id = %subset.id, position, by = %writer.by, "scenario: a subset was attached");
    read_back(&state, id).await
}

/// `DELETE /api/cases/:slug/scenarios/:scenario_id/subsets/:subset_id` — detach.
///
/// ## ⚑ THE ONE HARD DELETE IN THIS FEATURE
///
/// The link row is removed outright rather than soft-deleted, and the reason is
/// that a link is not content: the story, its events, its notes, its attribution
/// and its whole history are untouched and one click from being re-attached.
/// There is nothing here a `deleted_at` would preserve — no words somebody
/// wrote, no authorship that would be lost — so the column would only ever make
/// the reads harder to write. Stated in the T1 report as the one exception to
/// this feature's soft-delete discipline.
///
/// Detaching something that was not attached is a 404 naming the subset, not a
/// silent success: the caller is looking at a list that is out of date, and
/// telling them so is what stops them pressing it again.
///
/// # Errors
/// 400 for a malformed id; 404 when the scenario is not in this case or the link
/// does not exist; 500 for a database failure.
#[tracing::instrument(skip(state, user), fields(by = %user.username, slug = %slug, scenario_id = %scenario_id, subset_id = %subset_id))]
pub async fn delete_scenario_subset(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, subset_id)): Path<(String, String, String)>,
) -> Result<Json<Vec<ScenarioSubsetDto>>, AppError> {
    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;
    let target = super::support::parse_subset_id(&subset_id)?;

    let removed = subset_write::detach(&state.pipeline_pool, id, target)
        .await
        .map_err(|e| write_error(e, "detaching a subset from a scenario"))?;
    if removed == 0 {
        return Err(AppError::NotFound {
            message: format!("this scenario does not carry timeline subset {target}"),
        });
    }

    tracing::info!(subset_id = %target, by = %user.username, "scenario: a subset was detached");
    read_back(&state, id).await
}

/// The scenario's attached subsets, after a write.
///
/// Both writes answer with the whole list rather than a status code, for the
/// reason every chronology write answers with the composed event: the surface
/// then reflects the SERVER's account of what it carries instead of applying its
/// own guess, and a page cannot drift from the store it just changed.
async fn read_back(
    state: &AppState,
    scenario_id: uuid::Uuid,
) -> Result<Json<Vec<ScenarioSubsetDto>>, AppError> {
    let rows = list_scenario_subsets(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| subset_failure(e, "this scenario's subsets after the write"))?;
    Ok(Json(build_scenario_subsets(&rows)))
}
