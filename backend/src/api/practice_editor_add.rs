//! Adding a question by hand, and proving one before it is written.
//!
//! Split from [`super::practice_editor`] on 2026-08-19 when Part B carried that
//! module past Rule 17's limit. The seam is honest as well as arithmetical: the
//! sibling CHANGES questions that already exist and every one of its three
//! writes is a two-line edit plus its change row, while adding one is mostly
//! REFUSAL — nine ways a typed question can be wrong, all of them proved before
//! a transaction opens.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    auth::AuthUser,
    domain::practice_params::TACTIC_CARD_MAX,
    dto::practice_review::{AddQuestionRequest, DeckChangeResponse},
    error::AppError,
    repositories::pipeline_repository::{
        practice::{list_deck, PracticeQuestionRecord},
        practice_editor::{insert_question, log_change, next_sort_order, NewChange, NewQuestion},
    },
    state::AppState,
};

use super::practice::repo_error;
use super::practice_editor::fence_editor;
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// Add a question somebody typed on the page.
///
/// # Errors
/// 400 for an unsigned change, an unknown kind, a blank text, a redirect with
/// no `follows`, or a `follows` naming no cross question in this deck; 404 when
/// the scenario does not exist or is reached through the wrong case.
pub async fn post_add_question(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(body): Json<AddQuestionRequest>,
) -> Result<Json<DeckChangeResponse>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;
    let by = fence_editor(&state, &body.editing_as)?;

    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;
    let plan = plan_question(&state, &body, &deck)?;

    let question_id = write_question(&state, scenario_id, &body, &plan, &by).await?;

    tracing::info!(%scenario_id, %question_id, kind = %plan.kind, by = %by, "practice deck: a question was added");
    Ok(Json(DeckChangeResponse { question_id }))
}

/// Write the question and its `added` change row, in one transaction.
///
/// Split from the handler so that function is the four steps it reads as —
/// fence the case, fence the editor, plan, write — and because the two writes
/// are ONE act: a question nobody can see the arrival of is a question Marie is
/// asked with no explanation of where it came from.
async fn write_question(
    state: &AppState,
    scenario_id: uuid::Uuid,
    body: &AddQuestionRequest,
    plan: &AddPlan,
    by: &str,
) -> Result<uuid::Uuid, AppError> {
    let mut tx = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| repo_error("begin", e))?;
    let sort_order = next_sort_order(&mut tx, scenario_id)
        .await
        .map_err(|e| repo_error("next_sort_order", e))?;
    let question_id = insert_question(
        &mut tx,
        &NewQuestion {
            scenario_id,
            side: plan.side,
            kind: plan.kind,
            text: body.text.trim(),
            tactic: plan.tactic,
            follows_key: plan.follows.as_deref(),
            watch_for: body
                .watch_for
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty()),
            source_kind: plan.source_kind,
            source_ref: plan.source_ref.as_deref(),
            receipt: None,
            sort_order,
            created_by: by,
        },
    )
    .await
    .map_err(|e| repo_error("insert_question", e))?;

    log_change(
        &mut tx,
        &NewChange {
            scenario_id,
            question_id,
            change_kind: "added",
            field: None,
            before_value: None,
            // The SIDE, which is the one fact about a new question the change
            // list needs — and it is stored rather than joined because the
            // question's row may have moved by the time the list is read.
            after_value: Some(plan.side),
            changed_by: by,
        },
    )
    .await
    .map_err(|e| repo_error("log_change", e))?;
    tx.commit().await.map_err(|e| repo_error("commit", e))?;
    Ok(question_id)
}

/// What an add request becomes, once proved.
struct AddPlan {
    side: &'static str,
    kind: &'static str,
    tactic: Option<i16>,
    follows: Option<String>,
    source_kind: &'static str,
    source_ref: Option<String>,
}

/// The side and kind one requested kind is.
///
/// The form asks ONE question because the side follows from the kind: a cross is
/// George's, and the other two are Chuck's.
fn side_and_kind(kind: &str) -> Result<(&'static str, &'static str), AppError> {
    match kind {
        "cross" => Ok(("george", "cross")),
        "direct" => Ok(("chuck", "direct")),
        "redirect" => Ok(("chuck", "redirect")),
        other => Err(AppError::BadRequest {
            message: "kind must be cross, direct or redirect".to_string(),
            details: serde_json::json!({ "field": "kind", "value": other }),
        }),
    }
}

/// A tactic belongs to a cross question and nowhere else.
///
/// Accepting one on a Chuck question would put a trap tag on a friendly
/// question, which is the opposite of what the tag means.
fn fence_tactic(kind: &str, tactic: Option<i16>) -> Result<Option<i16>, AppError> {
    match (kind, tactic) {
        ("cross", Some(t)) if (1..=TACTIC_CARD_MAX).contains(&t) => Ok(Some(t)),
        ("cross", None) => Ok(None),
        ("cross", Some(t)) => Err(AppError::BadRequest {
            message: format!("tactic must be a card number from 1 to {TACTIC_CARD_MAX}"),
            details: serde_json::json!({ "field": "tactic", "value": t }),
        }),
        (_, Some(_)) => Err(AppError::BadRequest {
            message: "only a George question carries a tactic".to_string(),
            details: serde_json::json!({ "field": "tactic" }),
        }),
        (_, None) => Ok(None),
    }
}

/// Prove a redirect names a cross question that is in THIS deck.
///
/// The same check the deck file's validator makes, made here because a question
/// typed on the page never passes through the file. `follows_key` is
/// deliberately not a foreign key, so this is the whole of it.
fn fence_follows(
    kind: &str,
    follows: Option<&str>,
    deck: &[PracticeQuestionRecord],
) -> Result<Option<String>, AppError> {
    let follows = follows.map(str::trim).filter(|v| !v.is_empty());
    if kind != "redirect" {
        if follows.is_some() {
            return Err(AppError::BadRequest {
                message: "only a redirect follows a George question".to_string(),
                details: serde_json::json!({ "field": "follows" }),
            });
        }
        return Ok(None);
    }
    let key = follows.ok_or_else(|| AppError::BadRequest {
        message: "a redirect must say which George question it follows".to_string(),
        details: serde_json::json!({ "field": "follows" }),
    })?;
    if !deck
        .iter()
        .any(|q| q.kind == "cross" && q.deck_key.as_deref() == Some(key))
    {
        return Err(AppError::BadRequest {
            message: format!("no George question in this deck has the key \"{key}\""),
            details: serde_json::json!({ "field": "follows", "value": key }),
        });
    }
    Ok(Some(key.to_string()))
}

/// Prove an add request before a transaction opens.
///
/// Split from the handler so that function reads as the four steps it is —
/// fence the case, fence the editor, plan, write — and because everything here
/// is a refusal about what a CLIENT sent, which is a different subject from
/// writing a question.
fn plan_question(
    state: &AppState,
    body: &AddQuestionRequest,
    deck: &[PracticeQuestionRecord],
) -> Result<AddPlan, AppError> {
    if body.text.trim().is_empty() {
        return Err(AppError::BadRequest {
            message: "a question must have words in it".to_string(),
            details: serde_json::json!({ "field": "text" }),
        });
    }
    let (side, kind) = side_and_kind(&body.kind)?;
    let tactic = fence_tactic(kind, body.tactic)?;
    let follows = fence_follows(kind, body.follows.as_deref(), deck)?;

    let (source_kind, source_ref) = resolve_attach(state, body, deck)?;
    Ok(AddPlan {
        side,
        kind,
        tactic,
        follows,
        source_kind,
        source_ref,
    })
}

/// What the add form's "Attach to" choice becomes on the row.
///
/// ## Domain note: an instance's ref is BORROWED from a sibling question
///
/// `source_ref` on an instance question is the graph node id the seed resolved,
/// and this page does not read the graph. So attaching to "instance 2" means
/// taking the ref the deck's own second instance question already carries —
/// which is exactly what "attach to the same thing that one is attached to"
/// means, and it cannot invent an id that points nowhere.
///
/// A scenario whose deck has no question on that instance yet cannot offer it,
/// and [`crate::services::practice_editor_options`] says so in its own header.
fn resolve_attach(
    state: &AppState,
    body: &AddQuestionRequest,
    deck: &[PracticeQuestionRecord],
) -> Result<(&'static str, Option<String>), AppError> {
    let (Some(kind), Some(index)) = (body.source_kind.as_deref(), body.source_index) else {
        // "no receipt" — the honest answer when a question traces to nothing.
        return Ok(("manual", None));
    };
    let wanted = match kind {
        "instance" => "instance",
        "point" => "point",
        other => {
            return Err(AppError::BadRequest {
                message: "source_kind must be instance or point".to_string(),
                details: serde_json::json!({ "field": "source_kind", "value": other }),
            })
        }
    };
    let nth = usize::try_from(index - 1).map_err(|_| AppError::BadRequest {
        message: "source_index counts from 1".to_string(),
        details: serde_json::json!({ "field": "source_index", "value": index }),
    })?;
    let borrowed = deck
        .iter()
        .filter(|q| q.source_kind == wanted)
        .nth(nth)
        .and_then(|q| q.source_ref.clone())
        .ok_or_else(|| AppError::BadRequest {
            message: format!(
                "this scenario's deck has no question on {wanted} {index} to attach to"
            ),
            details: serde_json::json!({ "field": "source_index", "value": index }),
        })?;
    let _ = state;
    Ok((wanted_static(wanted), Some(borrowed)))
}

/// The `&'static str` one validated source kind is.
///
/// `wanted` above is already one of two literals, but it borrows from a `match`
/// on client input; this hands back the crate's own constant so nothing derived
/// from a request reaches the column.
fn wanted_static(kind: &str) -> &'static str {
    if kind == "instance" {
        "instance"
    } else {
        "point"
    }
}
