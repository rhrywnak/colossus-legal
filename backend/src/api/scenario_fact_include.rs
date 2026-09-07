//! The link an INCLUDE now writes beside the ruling (FACT_CARD_v2 §2).
//!
//! Split out of `api::scenario_facts`, which was at the 300-line module limit
//! before this task. The seam is real: everything here is about the SECOND thing
//! an include does — recording which accusation the fact bears on and which way
//! it cuts — while the handler beside it is about the ruling itself.
//!
//! ## Why the include grew two more writes
//!
//! Including a fact used to write a `scenario_fact_refs` row and nothing else,
//! which left the Proof Matrix unaware that a human had judged the statement. The
//! same person then had to make the same judgment again on another page. §2 makes
//! the three ONE act — the reference row, the link, and the Matrix `keep` —
//! and the browser prefills the accusation from the card's `supports[0]` so in
//! the ordinary case nobody types anything.
//!
//! The Matrix half arrived at the 2026-09-07 integration (ruling R1), when
//! `feat/proof-matrix-v2` and `feat/fact-card-v2` met on one branch. Until then
//! `evidence_allegation_rulings` did not exist on this side and the fallback in
//! FACT_CARD_v2 §2 applied: write the link alone and name the gap.

use chrono::Utc;

use crate::domain::fact_card::CardStance;
use crate::domain::matrix_ruling::MatrixRuling;
use crate::dto::{FactAction, FactActionRequest};
use crate::error::AppError;
use crate::repositories::pipeline_repository::evidence_allegation_rulings::RulingWrite;
use crate::state::AppState;

/// The accusation and stance an INCLUDE must carry, or the refusal that names
/// what is missing (§2).
///
/// ## Why this is validated here rather than in the type
///
/// `action` and the two link fields are sibling fields, not a tagged union, so
/// "required for include and refused for the others" cannot be expressed in the
/// request struct — the same shape `defer`'s reason takes, and it is refused in
/// the same place for the same reason: this layer can name the missing field.
///
/// Returns `None` for every action that is not an include, which is what makes
/// drop, un-drop, defer and reopen unaffected by this change.
pub(crate) fn include_link(
    payload: &FactActionRequest,
) -> Result<Option<(&str, CardStance)>, AppError> {
    if !matches!(payload.action, FactAction::Include) {
        return Ok(None);
    }
    let (Some(allegation_id), Some(stance)) = (payload.allegation_id.as_deref(), payload.stance)
    else {
        return Err(AppError::BadRequest {
            message: "Including a fact needs the accusation it bears on and which way it cuts."
                .to_string(),
            details: serde_json::json!({ "fields": ["allegation_id", "stance"] }),
        });
    };
    if allegation_id.trim().is_empty() {
        return Err(AppError::BadRequest {
            message: "Including a fact needs the accusation it bears on.".to_string(),
            details: serde_json::json!({ "field": "allegation_id" }),
        });
    }
    Ok(Some((allegation_id.trim(), stance)))
}

/// Record the human's link AND the Matrix `keep` for a fact they just included.
///
/// ## Domain note: ONE act, two surfaces (§2, integration ruling R1)
///
/// A human who includes a fact under an accusation has made the same judgment the
/// Proof Matrix asks for — this item belongs under this paragraph. Writing only
/// the link left the Matrix showing an unread row and asked the same person the
/// same question on another page. Both are written here, from the one click.
///
/// ## Why TWO transactions and not one
///
/// PROOF_MATRIX_v2 §2 ruled it: a Matrix failure must not roll back the link.
/// The two tables answer different questions — the link says which accusation
/// this statement bears on and which way it cuts, the ruling says a human
/// confirmed it belongs — and the link is the one the scenario page reads. A
/// single transaction would make a Matrix outage silently cost the link as well,
/// which is the larger loss. So `save_link` commits, and then `save_ruling`
/// commits; each opens its own.
///
/// ## Why a failure in EITHER half still fails the request
///
/// Nothing here is best-effort. The include already wrote the reference row, so
/// reporting success with a half missing would leave a state no reader could see
/// and no human would have a reason to correct. Both errors name exactly what IS
/// stored and what is not, and both writes are upserts — including the fact again
/// completes the act rather than duplicating it.
pub(crate) async fn link_included_fact(
    state: &AppState,
    graph_node_id: &str,
    allegation_id: &str,
    stance: CardStance,
    author: &str,
) -> Result<(), AppError> {
    use crate::repositories::pipeline_repository::{save_link, LinkWrite};

    // ONE timestamp for both halves, so the link and the ruling that record the
    // same click carry the same instant and can be read as one act.
    let written_at = Utc::now();

    save_link(
        &state.pipeline_pool,
        &LinkWrite {
            graph_node_id,
            allegation_id,
            // The two vocabularies are mapped in ONE place — see
            // `CardStance::to_link_cut` for why a card that SUPPORTS an
            // accusation against Marie is a HAZARD to her.
            cut: stance.to_link_cut(),
            authored_by: author,
            written_at,
        },
    )
    .await
    .map(|_| ())
    .map_err(|e| {
        tracing::error!(
            error = %e,
            %graph_node_id,
            %allegation_id,
            %author,
            stance = stance.code(),
            "the fact WAS included but its accusation link was not written — the \
             fact is in the scenario and invisible to the accusation it was \
             included for; include it again to retry"
        );
        // The recovery step is in the BODY, not only in the log: the ruling row
        // was already written, so the UI will not re-prompt on its own and the
        // human has no other reason to try again.
        AppError::Internal {
            message: format!(
                "failed to record the accusation link between {graph_node_id} and \
                 {allegation_id} — the fact is included but not linked; include it \
                 again to write the link"
            ),
        }
    })?;

    keep_on_the_matrix(state, graph_node_id, allegation_id, author, written_at).await
}

/// The Matrix ruling one include makes, as a value.
///
/// Pure, and separate from the write, so the three decisions it encodes — which
/// id goes in which slot, that the verdict is `Keep`, and that the note stays
/// empty — are assertable without a database. Getting `evidence_id` and
/// `allegation_id` the wrong way round is a real risk: both are `&str`, and the
/// compiler cannot tell them apart.
///
/// ## Rust Learning: one lifetime for the borrows that travel together
///
/// `RulingWrite<'a>` holds three `&'a str`s, so this signature ties all three
/// inputs to the same `'a` — the returned struct may not outlive the shortest of
/// them. That is exactly the guarantee wanted: the write is consumed by the very
/// next statement, and nothing here can outlive the request that owns the ids.
fn keep_write<'a>(
    graph_node_id: &'a str,
    allegation_id: &'a str,
    author: &'a str,
    written_at: chrono::DateTime<Utc>,
) -> RulingWrite<'a> {
    RulingWrite {
        // The Matrix names an Evidence node `evidence_id`; the scenario page
        // calls the same string `graph_node_id`. One id, two surfaces'
        // vocabulary — and it is the node's `id`, never an `evidence_id`
        // property, which does not exist on the node.
        evidence_id: graph_node_id,
        allegation_id,
        ruling: MatrixRuling::Keep,
        ruled_by: author,
        note: None,
        written_at,
    }
}

/// The Matrix half of an include: this item belongs under this accusation (R1).
///
/// Split out so [`link_included_fact`] reads as the two commits it is, and so the
/// argument for `Keep` lives beside the call that makes it.
///
/// ## Domain note: `Keep` whichever way the fact cuts
///
/// The stance says which way the statement cuts — `supports` the accusation, or
/// `rebuts` it. The Matrix verdict says something different: that a human read
/// this pairing and it belongs on the page. A fact Marie includes BECAUSE it
/// rebuts the accusation is exactly the kind of item the Matrix must keep, so
/// `Remove` would strike the very evidence the include was for. The stance is
/// already recorded, once, on the link.
///
/// ## Why the note is `None`
///
/// `note` is an optional sentence a human typed on the Matrix page. Nobody typed
/// one here, and composing one ("included from the scenario page") would put
/// words into a record whose whole value is that they are a person's own. The
/// ledger already records the actor and the instant.
async fn keep_on_the_matrix(
    state: &AppState,
    graph_node_id: &str,
    allegation_id: &str,
    author: &str,
    written_at: chrono::DateTime<Utc>,
) -> Result<(), AppError> {
    use crate::repositories::pipeline_repository::evidence_allegation_rulings::save_ruling;

    save_ruling(
        &state.pipeline_pool,
        &keep_write(graph_node_id, allegation_id, author, written_at),
    )
    .await
    .map(|_| ())
    .map_err(|e| {
        tracing::error!(
            error = %e,
            %graph_node_id,
            %allegation_id,
            %author,
            "the fact was included AND linked, but the Proof Matrix keep was not \
             written — the link stands (its own transaction committed), and the \
             Matrix will show this row as unread; include it again to write the \
             keep"
        );
        AppError::Internal {
            message: format!(
                "the fact is included and linked, but the Proof Matrix keep for \
                 {graph_node_id} under {allegation_id} was not written — include \
                 it again to record it"
            ),
        }
    })
}

#[cfg(test)]
#[path = "scenario_fact_include_tests.rs"]
mod tests;
