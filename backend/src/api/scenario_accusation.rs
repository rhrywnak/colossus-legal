//! The accusation's three write concerns, on ONE guarded path (task 2.11 B1).
//!
//! ```text
//! GET    …/scenarios/:id/accusation                            the section
//! PUT    …/scenarios/:id/accusation                            set / clear the sentence
//! POST   …/scenarios/:id/accusation/instances/:graph_node_id   mark an instance
//! DELETE …/scenarios/:id/accusation/instances/:graph_node_id   un-mark it
//! PUT    …/scenarios/:id/accusation/instances/:id/answer       pair / re-pair an answer
//! DELETE …/scenarios/:id/accusation/instances/:id/answer       un-pair it
//! ```
//!
//! ## "One write path" means one GUARDED path — recorded 2026-08-06
//!
//! Not one SQL statement. Three human judgments of three different shapes cannot
//! share a statement; what they must share is the FENCE. Every write below goes
//! through the same three lines in the same order — `require_edit`,
//! `parse_scenario_id`, `ensure_scenario_in_case` — in this one module, so there
//! is no second place for a guard to be forgotten. Task 1.4's gate review found an
//! asymmetry of exactly that sort, where C5 lacked a guard C4 had; duplicated
//! guards is how that happens.
//!
//! ## The §8 invariants at this layer
//!
//! No handler here calls gather, and nothing here writes to
//! `scenario_ruling_anchors`. Marking a statement as an instance of the accusation
//! and pairing an answer to it are human AUGMENTATION — they say what included
//! evidence MEANS, not whether it belongs in the scenario — so they never trigger
//! re-gathering and are not rulings. The source scan next door
//! (`augmentation_never_gathers`) asserts the same property for the sibling panel.
//!
//! ## Domain note: the machine asserts neither judgment
//!
//! "This statement IS them making the accusation" and "this is what we said back"
//! are both human. The second especially: asserting that one record item REBUTS
//! another is the contradiction judgment the requirement defers to a layer we have
//! not built, and a rehearsal page that guessed it would put words in a witness's
//! mouth. Every route below records what a human clicked and infers nothing.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `scenarios` and `scenario_human_facts` live in `colossus_legal_v2`, so every
//! write here uses `&state.pipeline_pool`, NOT `state.pg_pool`.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::{
    auth::{require_edit, AuthUser},
    domain::human_authored::HumanFactKind,
    dto::scenario_accusation::{PairAnswerRequest, SetAccusationTextRequest},
    error::AppError,
    repositories::pipeline_repository::{
        delete_anchored_judgment, insert_accusation_instance, set_scenario_accusation,
        upsert_anchored_judgment, AnchoredJudgmentWrite, PipelineRepoError,
    },
    state::AppState,
};

use super::scenario_accusation_read::get_accusation_panel;
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// This module's routes.
///
/// Declared here beside the handlers they name, like `scenario_augmentation` and
/// `scenario_fact_curation`: the central table in `api::mod` had already grown
/// past the module-size limit, and a reader looking at `pair_answer` can see its
/// path without opening another file.
///
/// ## Rust Learning: why the paths are written out rather than built
///
/// A `const PREFIX` plus `format!` would give matchit a `String` per route and
/// would hide the shape of the tree from a reader — and the shape is the point: a
/// static child (`/accusation`) beside a param child (`/instances/:graph_node_id`)
/// beside a static grandchild (`/answer`) is exactly what matchit 0.7.3 accepts,
/// and the same shape `facts/:graph_node_id/tier` already uses.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/cases/:slug/scenarios/:scenario_id/accusation",
            get(get_accusation_panel).put(put_accusation_text),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/accusation/instances/:graph_node_id",
            post(mark_instance).delete(unmark_instance),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/accusation/instances/:graph_node_id/answer",
            put(pair_answer).delete(unpair_answer),
        )
}

/// The fence, in one place.
///
/// Every write in this module calls this first and does nothing before it. Three
/// guards, in an order that matters: refuse a reader before parsing anything,
/// refuse a malformed id before touching the database, and refuse a scenario that
/// is not in this case before writing a row keyed to it.
///
/// ## Why case-fencing is not optional on a scoped route
///
/// The scenario id alone is enough to write the row. Without this check, a
/// well-formed id from another case would be accepted under this case's slug, and
/// the URL would then be a lie about what was edited. The sibling panels enforce
/// the same thing the same way.
async fn fence(
    user: &AuthUser,
    state: &AppState,
    scenario_id: &str,
    slug: &str,
) -> Result<Uuid, AppError> {
    require_edit(user)?;
    let id = parse_scenario_id(scenario_id)?;
    ensure_scenario_in_case(state, id, slug).await?;
    Ok(id)
}

/// `PUT …/accusation` — set or withdraw the plain-words accusation.
///
/// The write that kills the first beta.371 defect at its root: this column is what
/// the rehearsal block reads, and `definition->>'attack_text'` — a verbatim
/// first-person quote from the record — is never allowed to stand in for it.
///
/// `null` clears, which returns the block to its named gap. A blank string is
/// REFUSED rather than treated as a clear: "withdraw this" and a slip of the
/// keyboard are different intentions, and the human is the only one who can tell
/// which they meant.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn put_accusation_text(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(payload): Json<SetAccusationTextRequest>,
) -> Result<StatusCode, AppError> {
    let id = fence(&user, &state, &scenario_id, &slug).await?;

    let trimmed = checked_accusation_text(payload.text.as_deref())?;

    let rows = set_scenario_accusation(&state.pipeline_pool, id, trimmed)
        .await
        .map_err(|e| write_failure(e, id, &user.username, "the accusation sentence"))?;

    require_row_touched(
        rows,
        &user.username,
        id,
        "no scenario matched when writing the accusation sentence; it was deleted \
         between the page load and this write",
    )?;

    tracing::info!(
        author = %user.username,
        scenario_id = %id,
        // The words themselves are NOT logged — they are a human's authored prose
        // about a live case, and a log line is the wrong place for it. Whether one
        // now exists is the operationally useful fact, and it is the one recorded.
        withdrawn = trimmed.is_none(),
        "stored the scenario's plain-words accusation"
    );
    Ok(StatusCode::NO_CONTENT)
}

/// `POST …/accusation/instances/:graph_node_id` — mark a statement as an instance.
///
/// Idempotent by design: marking something already marked answers 204 rather than
/// failing, because the human's intent is satisfied and their page was simply
/// stale. The partial unique index is what makes that true — a second row cannot
/// exist — and the upsert's conflict target names the PAIRING predicate, so an
/// instance's own conflict surfaces here as a unique violation rather than a
/// duplicate row.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id, graph_node_id = %graph_node_id))]
pub async fn mark_instance(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, graph_node_id)): Path<(String, String, String)>,
) -> Result<StatusCode, AppError> {
    let id = fence(&user, &state, &scenario_id, &slug).await?;

    // An instance judges the anchor ALONE — it stores no answer, which is what
    // keeps a mark from being mistaken for a pairing (and what the CHECK
    // `answers_graph_node_id IS NULL OR anchor_graph_node_id IS NOT NULL` allows).
    let recorded = insert_accusation_instance(
        &state.pipeline_pool,
        id,
        &graph_node_id,
        &user.username,
        Utc::now(),
    )
    .await
    .map_err(|e| write_failure(e, id, &user.username, "an accusation instance"))?;

    match recorded {
        Some(fact_id) => tracing::info!(
            %fact_id,
            author = %user.username,
            %graph_node_id,
            scenario_id = %id,
            "marked a statement as an accusation instance"
        ),
        // Their intent is satisfied either way, so this is not an error — but it
        // is a DIFFERENT thing that happened, and `info` is what an operator
        // reading the log needs to see rather than wondering why a click has no
        // write behind it.
        None => tracing::info!(
            author = %user.username,
            %graph_node_id,
            scenario_id = %id,
            "that statement was already marked as an instance; the page was stale"
        ),
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE …/accusation/instances/:graph_node_id` — withdraw a marking.
///
/// The fact stays in the scenario; what is withdrawn is the judgment that it is
/// the accusation being made. The delete is keyed on KIND as well as the anchor,
/// so un-marking cannot also destroy the answer paired to the same anchor — that
/// pairing survives and surfaces as its named gap, which is the Remove law.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id, graph_node_id = %graph_node_id))]
pub async fn unmark_instance(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, graph_node_id)): Path<(String, String, String)>,
) -> Result<StatusCode, AppError> {
    let id = fence(&user, &state, &scenario_id, &slug).await?;

    let rows = delete_anchored_judgment(
        &state.pipeline_pool,
        id,
        &graph_node_id,
        HumanFactKind::AccusationInstance.code(),
    )
    .await
    .map_err(|e| write_failure(e, id, &user.username, "an accusation instance"))?;

    require_anchor_touched(
        rows,
        &user.username,
        &graph_node_id,
        id,
        "no instance matched when un-marking; it was already withdrawn, and this \
         page is out of date",
    )?;

    tracing::info!(
        author = %user.username,
        %graph_node_id,
        scenario_id = %id,
        "withdrew an accusation instance"
    );
    Ok(StatusCode::NO_CONTENT)
}

/// `PUT …/accusation/instances/:graph_node_id/answer` — pair or re-pair an answer.
///
/// `PUT` rather than `POST` because it is idempotent in the way PUT means: one
/// answer per accusation, and sending it twice leaves the same one stored. The
/// partial unique index enforces that, and the upsert REPLACES — two stored
/// answers to one accusation would leave the rehearsal page choosing between them
/// with no basis for choosing.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id, graph_node_id = %graph_node_id))]
pub async fn pair_answer(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, graph_node_id)): Path<(String, String, String)>,
    Json(payload): Json<PairAnswerRequest>,
) -> Result<StatusCode, AppError> {
    let id = fence(&user, &state, &scenario_id, &slug).await?;

    let answer = checked_answer(&payload.answers_graph_node_id, &graph_node_id)?;

    let write = AnchoredJudgmentWrite {
        scenario_id: id,
        anchor_graph_node_id: &graph_node_id,
        answers_graph_node_id: Some(answer),
        authored_by: &user.username,
        kind: HumanFactKind::AnswerPairing.code(),
        written_at: Utc::now(),
    };

    let fact_id = upsert_anchored_judgment(&state.pipeline_pool, &write)
        .await
        .map_err(|e| write_failure(e, id, &user.username, "an answer pairing"))?;

    tracing::info!(
        %fact_id,
        author = %user.username,
        %graph_node_id,
        answers = %answer,
        scenario_id = %id,
        "paired an answer to an accusation instance"
    );
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE …/accusation/instances/:graph_node_id/answer` — un-pair.
///
/// A real act, recorded as one. Un-pairing returns the instance to the prep list,
/// which is a change to what still needs preparing before trial — not a side
/// effect of something else, and not something that should happen implicitly when
/// a human pairs a different answer (that is a re-pair, and it is logged as one).
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id, graph_node_id = %graph_node_id))]
pub async fn unpair_answer(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, graph_node_id)): Path<(String, String, String)>,
) -> Result<StatusCode, AppError> {
    let id = fence(&user, &state, &scenario_id, &slug).await?;

    let rows = delete_anchored_judgment(
        &state.pipeline_pool,
        id,
        &graph_node_id,
        HumanFactKind::AnswerPairing.code(),
    )
    .await
    .map_err(|e| write_failure(e, id, &user.username, "an answer pairing"))?;

    require_anchor_touched(
        rows,
        &user.username,
        &graph_node_id,
        id,
        "no pairing matched when un-pairing; it was already removed, and this page \
         is out of date",
    )?;

    tracing::info!(
        author = %user.username,
        %graph_node_id,
        scenario_id = %id,
        "un-paired an answer from an accusation instance"
    );
    Ok(StatusCode::NO_CONTENT)
}

/// The accusation sentence, trimmed — or `None`, which withdraws it.
///
/// Extracted from the handler for the same reason [`checked_answer`] was: a
/// boundary rule that lives inside a handler body cannot be tested without the
/// whole Axum-and-database stack, and this one decides whether a human's sentence
/// is stored at all.
///
/// Trimmed like every other authored text on this surface — leading whitespace is
/// invisible in psql and visible on screen.
///
/// ## Domain note: blank is a REFUSAL, not a withdrawal
///
/// "Withdraw what I wrote" and "I fat-fingered the box" are different intentions
/// with different consequences, and only the human knows which they meant. Sending
/// no text at all is how the first is said; the column's CHECK refuses a blank
/// anyway, so accepting one here would only move which layer said no.
///
/// # Errors
/// Returns [`AppError::BadRequest`] naming the field when the text is present but
/// has no non-whitespace characters.
fn checked_accusation_text(text: Option<&str>) -> Result<Option<&str>, AppError> {
    let trimmed = text.map(str::trim);
    if trimmed.is_some_and(str::is_empty) {
        return Err(AppError::BadRequest {
            message: "an accusation with no words in it is not an accusation — send \
                      no text at all to withdraw the one that is stored"
                .to_string(),
            details: json!({ "field": "text" }),
        });
    }
    Ok(trimmed)
}

/// The two things a pairing must be before it is stored.
///
/// Pure, so both refusals are unit-testable without a database — and both reach
/// the client verbatim, because each is about what the human just did and they are
/// the only one who can act on either.
///
/// ## Domain note: why "a statement cannot answer itself" is checked HERE
///
/// Not in the database: the column pair is perfectly legal SQL, and a CHECK would
/// be encoding a claim about MEANING into a constraint. It is a meaning question —
/// an instance whose answer is the accusation renders as us agreeing with them —
/// so it belongs where meaning is decided, at the boundary, with a sentence saying
/// what to do instead.
///
/// # Errors
/// Returns [`AppError::BadRequest`] naming the field for a blank answer or a
/// self-answer.
fn checked_answer<'a>(answer: &'a str, anchor: &str) -> Result<&'a str, AppError> {
    let answer = answer.trim();
    if answer.is_empty() {
        return Err(AppError::BadRequest {
            message: "a pairing has to name the record item that answers the accusation"
                .to_string(),
            details: json!({ "field": "answers_graph_node_id" }),
        });
    }
    if answer == anchor {
        return Err(AppError::BadRequest {
            message: "a statement cannot be its own answer — pair the record item that \
                      answers it"
                .to_string(),
            details: json!({ "field": "answers_graph_node_id" }),
        });
    }
    Ok(answer)
}

/// One write failure, logged with everything needed to find it and served opaquely.
///
/// Opaque to the client because none of these are about what the human typed — the
/// two refusals that ARE (a blank sentence, a self-answer) are returned as
/// `BadRequest` with their own words, right where they are detected.
fn write_failure(
    error: PipelineRepoError,
    scenario_id: Uuid,
    author: &str,
    what: &'static str,
) -> AppError {
    tracing::error!(error = %error, %scenario_id, %author, "failed to store {}", what);
    AppError::Internal {
        message: "failed to save".to_string(),
    }
}

/// Turn "the statement touched no scenario row" into a named 404.
///
/// `warn`, not `error`: a scenario deleted underneath an open page is an expected
/// consequence of two people working one case, not a fault. But it is
/// operationally distinct from both the SQL-failure path and success, and a
/// `#[tracing::instrument]` span emits nothing on an early return — so an operator
/// chasing "my accusation would not save" has something to search for.
fn require_row_touched(
    rows: u64,
    author: &str,
    scenario_id: Uuid,
    context: &str,
) -> Result<(), AppError> {
    if rows > 0 {
        return Ok(());
    }
    tracing::warn!(%author, %scenario_id, "{}", context);
    Err(AppError::NotFound {
        // Names the state AND the way out. A human on a page that will never
        // succeed again needs somewhere to go, not just a diagnosis.
        message: "that scenario no longer exists — go back to the scenario list to \
                  continue"
            .to_string(),
    })
}

/// Turn "the delete removed nothing" into a named 404.
///
/// Withdrawing something that was not there and withdrawing something that was are
/// different states, and the human needs to know which happened — the first means
/// their page is stale and what they are looking at is not what is stored. Shared
/// by both deletes so the two cannot drift into answering differently; the
/// `context` message is what tells them apart in the log.
fn require_anchor_touched(
    rows: u64,
    author: &str,
    graph_node_id: &str,
    scenario_id: Uuid,
    context: &str,
) -> Result<(), AppError> {
    if rows > 0 {
        return Ok(());
    }
    tracing::warn!(%author, %graph_node_id, %scenario_id, "{}", context);
    Err(AppError::NotFound {
        // "Out of date" implies a remedy rather than stating one; somebody who has
        // not operated this surface before does not know it means reload.
        message: "there was nothing there to withdraw — your page is out of date; \
                  reload it to see what is actually stored"
            .to_string(),
    })
}

#[cfg(test)]
#[path = "scenario_accusation_tests.rs"]
mod tests;
