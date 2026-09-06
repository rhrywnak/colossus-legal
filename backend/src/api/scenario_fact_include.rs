//! The link an INCLUDE now writes beside the ruling (FACT_CARD_v2 §2).
//!
//! Split out of `api::scenario_facts`, which was at the 300-line module limit
//! before this task. The seam is real: everything here is about the SECOND thing
//! an include does — recording which accusation the fact bears on and which way
//! it cuts — while the handler beside it is about the ruling itself.
//!
//! ## Why the include grew a second write
//!
//! Including a fact used to write a `scenario_fact_refs` row and nothing else,
//! which left the Proof Matrix unaware that a human had judged the statement. The
//! same person then had to make the same judgment again on another page. §2 makes
//! the two ONE act, and the browser prefills the accusation from the card's
//! `supports[0]` so in the ordinary case nobody types anything.

use chrono::Utc;

use crate::domain::fact_card::CardStance;
use crate::dto::{FactAction, FactActionRequest};
use crate::error::AppError;
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

/// Record the human's link for a fact they just included (§2).
///
/// ## Domain note: ONE act, two surfaces — and the second is not built yet
///
/// §2 asks for the link AND the Proof Matrix `keep` ruling. The Matrix's
/// `evidence_allegation_rulings` table lives on `feat/proof-matrix-v2`, which is
/// not merged into this branch, so this writes the LINK ONLY. When the two
/// branches meet, a `save_ruling(… MatrixRuling::Keep …)` belongs here, beside
/// the link and inside the same loop — the instruction's own fallback, and it is
/// named in the task report so it cannot be forgotten.
///
/// ## Why a failure here fails the whole ruling
///
/// The include already wrote the reference row. Reporting success with the link
/// missing would leave a fact included in a scenario and invisible to the
/// accusation it was included FOR — and the human would have no reason to try
/// again. The error names both halves so an operator can see which is stored.
pub(crate) async fn link_included_fact(
    state: &AppState,
    graph_node_id: &str,
    allegation_id: &str,
    stance: CardStance,
    author: &str,
) -> Result<(), AppError> {
    use crate::repositories::pipeline_repository::{save_link, LinkWrite};

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
            written_at: Utc::now(),
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
    })
}

#[cfg(test)]
#[path = "scenario_fact_include_tests.rs"]
mod tests;
