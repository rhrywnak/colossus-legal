//! The nine — now eleven — ways a typed question can be wrong.
//!
//! Split from [`super::practice_editor_add`] on 2026-09-10 when task
//! DECK_DRAG_AND_ADD carried that module past Rule 17's limit. The seam is the
//! one its header has always drawn: adding a question is mostly REFUSAL, and
//! every refusal is proved BEFORE a transaction opens, so a request that names
//! something impossible is answered without a row having been written and moved
//! back.
//!
//! The sibling owns the write; this owns whether there is anything to write.
//!
//! ## CRITICAL — the pipeline pool
//!
//! The deck this reads lives in `colossus_legal_v2`, and is handed in already
//! read: nothing here touches a database.

use crate::{
    domain::practice_params::TACTIC_CARD_MAX,
    dto::practice_review::AddQuestionRequest,
    error::AppError,
    repositories::pipeline_repository::{
        practice::PracticeQuestionRecord, practice_reorder::NewPosition,
    },
    state::AppState,
};

/// What an add request becomes, once proved.
pub(super) struct AddPlan {
    pub(super) side: &'static str,
    pub(super) kind: &'static str,
    pub(super) tactic: Option<i16>,
    pub(super) follows: Option<String>,
    pub(super) source_kind: &'static str,
    pub(super) source_ref: Option<String>,
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
pub(super) fn plan_question(
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

/// Read the requested position, refusing the ones this deck cannot express.
///
/// Returns the [`NewPosition`] the placement uses, so the request's two fields
/// are read in ONE place and the rest of the module works from a value that
/// cannot be self-contradictory.
///
/// Three different mistakes, answered differently on purpose (Roman's ruling of
/// 2026-09-10 covers the first two):
///
/// * a row this scenario does not hold → **404**, the same observable the
///   scenario fence gives, because the caller is naming something that is not
///   there;
/// * a row on the OTHER side → **400** naming the field, because the row IS
///   there and the request is asking for something the deck cannot express. A
///   404 would say the question does not exist, which would be false and would
///   send whoever met it looking for a deleted row.
///
/// Runs before the transaction opens, like every other refusal in this module.
pub(super) fn fence_after(
    body: &AddQuestionRequest,
    plan: &AddPlan,
    deck: &[PracticeQuestionRecord],
    scenario_id: uuid::Uuid,
) -> Result<NewPosition, AppError> {
    if body.at_start && body.after.is_some() {
        return Err(AppError::BadRequest {
            message: "a new question goes either at the top of its side or below \
                      one row, not both"
                .to_string(),
            details: serde_json::json!({ "field": "at_start" }),
        });
    }
    if body.at_start {
        return Ok(NewPosition::Start);
    }
    let Some(after) = body.after else {
        return Ok(NewPosition::End);
    };
    let Some(row) = deck.iter().find(|q| q.id == after) else {
        return Err(AppError::NotFound {
            // Names the scenario as well as the id, and says what a valid value
            // looks like — the same standard the 400 below already meets. A 404
            // that says only "not found" leaves a caller with nothing to try next,
            // and the two commonest causes (a stale tab naming a deleted row, and
            // a request aimed at the wrong scenario) are told apart by exactly the
            // scenario id this now carries.
            message: format!(
                "no question in scenario {scenario_id} has the id {after} — pass \
                 the id of a question that is in this deck, or omit `after` to add \
                 to the end of the side"
            ),
        });
    };
    if row.side != plan.side {
        return Err(AppError::BadRequest {
            message: format!(
                "a {} question cannot be placed after a {} one — a new question \
                 goes among its own side's rows",
                plan.side, row.side
            ),
            details: serde_json::json!({ "field": "after", "value": after }),
        });
    }
    Ok(NewPosition::After(after))
}
