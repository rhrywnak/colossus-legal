//! Correcting a machine-written interrogatory question (task 1.7F Part B).
//!
//! Two writes: save a human's replacement text, and revert to the machine's.
//!
//! ## Domain note: the route is CASE-WIDE, and its shape says so (ruling R1)
//!
//! `/cases/:slug/evidence/:graph_node_id/summary` — there is no scenario in the
//! path, because there is no scenario in the fact. One summary per piece of
//! evidence: correct it while working S-2 and it reads corrected in S-4, because
//! the same quote is the same quote. A `/scenarios/:id/` segment here would be
//! the first step back toward one C-code meaning two different things depending
//! on which page a reader opened it from — the defect ruling R1 exists to prevent.
//!
//! The case slug stays in the path for authorisation and context, not as part of
//! the key: the row is keyed by the evidence node alone.
//!
//! ## Ruling R2: revert deletes, it never copies back
//!
//! The machine's words live in Neo4j as `Evidence.question` and nothing here can
//! reach them. `DELETE` removes the override row and the card composes the
//! machine's sentence again — unchanged, because it was never edited. That is why
//! there is no "restore" payload and nothing to get wrong.

use axum::{
    extract::{Path, State},
    routing::put,
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    auth::{require_edit, AuthUser},
    error::AppError,
    repositories::pipeline_repository::{
        delete_summary_override, upsert_summary_override, SummaryOverrideWrite,
    },
    state::AppState,
};

/// The route group for evidence-level human corrections.
pub fn routes() -> Router<AppState> {
    Router::new().route(
        "/cases/:slug/evidence/:graph_node_id/summary",
        put(save_summary_override).delete(revert_summary_override),
    )
}

/// What a human typed.
///
/// `deny_unknown_fields` for the reason every DTO here carries it: a client that
/// sends `summaryText` instead of `summary_text` must be told, not silently
/// treated as having sent an empty correction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveSummaryRequest {
    /// The replacement question. Trimmed and refused when blank — a blank
    /// correction is a revert that failed to happen, and storing it would leave
    /// the card showing nothing where a question used to be.
    pub summary_text: String,
}

/// What the server did, said out loud.
///
/// The composed `authorship_label` is returned so the client can show the badge
/// immediately without a second read — the same words the card payload will carry
/// on its next load, composed by the same rule (v2 §7 item 2: display decisions
/// live on this side of the wire).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryOverrideResponse {
    pub graph_node_id: String,
    /// The text now in force. Absent after a revert — the machine's sentence is
    /// in the graph, and the caller re-reads the card to get it rather than being
    /// handed a copy from a table that does not hold it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_text: Option<String>,
    /// `"Roman · 4 Aug 2026"`, or absent once reverted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorship_label: Option<String>,
    /// Whether a stored correction is now in force. The client branches on this
    /// rather than on the presence of a string.
    pub overridden: bool,
}

/// `PUT …/evidence/:graph_node_id/summary` — replace the machine's question.
///
/// Upsert: correcting the same item twice is ordinary work, not a conflict.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, graph_node_id = %graph_node_id))]
pub async fn save_summary_override(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, graph_node_id)): Path<(String, String)>,
    Json(payload): Json<SaveSummaryRequest>,
) -> Result<Json<SummaryOverrideResponse>, AppError> {
    require_edit(&user)?;

    let text = validate_summary_text(&payload.summary_text)?;

    let written_at = Utc::now();
    upsert_summary_override(
        &state.pipeline_pool,
        &SummaryOverrideWrite {
            graph_node_id: &graph_node_id,
            summary_text: text,
            authored_by: &user.username,
            written_at,
        },
    )
    .await
    .map_err(|e| {
        tracing::error!(
            error = %e,
            slug = %slug,
            graph_node_id = %graph_node_id,
            author = %user.username,
            "failed to save an evidence summary override"
        );
        AppError::Internal {
            message: "failed to save the corrected question".to_string(),
        }
    })?;

    tracing::info!(
        author = %user.username,
        graph_node_id = %graph_node_id,
        chars = text.len(),
        "saved an evidence summary override"
    );

    Ok(Json(SummaryOverrideResponse {
        graph_node_id,
        summary_text: Some(text.to_string()),
        authorship_label: Some(crate::services::scenario_card::human_authorship_label(
            &user.username,
            written_at,
        )),
        overridden: true,
    }))
}

/// The one validation rule on this surface, extracted so it can be tested.
///
/// Refused HERE with a sentence naming the alternative, rather than left to the
/// table's CHECK: a human who meant to clear the field wants Revert, and a 500
/// from a constraint would not tell them so (Standing Rule 1).
///
/// Returns the TRIMMED text, so the caller cannot store the untrimmed version by
/// forgetting — the check and the value it validated travel together.
fn validate_summary_text(raw: &str) -> Result<&str, AppError> {
    let text = raw.trim();
    if text.is_empty() {
        return Err(AppError::BadRequest {
            message: "A corrected question cannot be blank. To restore the \
                      system's own wording, use revert instead."
                .to_string(),
            details: serde_json::json!({ "field": "summary_text" }),
        });
    }
    Ok(text)
}

/// `DELETE …/evidence/:graph_node_id/summary` — restore the machine's question.
///
/// The machine's words are not touched by this (they are in the graph); deleting
/// the override is what makes them visible again.
#[tracing::instrument(skip(state, user), fields(slug = %slug, graph_node_id = %graph_node_id))]
pub async fn revert_summary_override(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, graph_node_id)): Path<(String, String)>,
) -> Result<Json<SummaryOverrideResponse>, AppError> {
    require_edit(&user)?;

    let removed = delete_summary_override(&state.pipeline_pool, &graph_node_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                slug = %slug,
                graph_node_id = %graph_node_id,
                // Who was working when it failed. Every other log point in this
                // handler names them, and a DB failure is exactly the case where
                // correlating to one session matters.
                author = %user.username,
                "failed to revert an evidence summary override"
            );
            AppError::Internal {
                message: "failed to restore the system's question".to_string(),
            }
        })?;

    // Reverting something nobody had corrected is not an error — the caller wanted
    // the machine's text and the machine's text is what they now have. It IS worth
    // recording at a different level, because a revert that removed nothing means
    // the client's view of who wrote that question was stale.
    if removed {
        tracing::info!(
            author = %user.username,
            graph_node_id = %graph_node_id,
            "reverted an evidence summary override to the system text"
        );
    } else {
        tracing::warn!(
            author = %user.username,
            graph_node_id = %graph_node_id,
            "revert found no override to remove — the client's badge was stale"
        );
    }

    Ok(Json(SummaryOverrideResponse {
        graph_node_id,
        summary_text: None,
        authorship_label: None,
        overridden: false,
    }))
}

// No GET here, by design: the card payload already carries the current question
// and its authorship, composed by the same rule. A read endpoint beside it would
// be a second source for one fact, and the two would eventually disagree.

#[cfg(test)]
mod tests {
    use super::*;

    /// A blank correction is refused, and the refusal names the way out.
    ///
    /// Both halves matter: the 400 stops a row that would blank a card's question
    /// line, and the sentence tells the human that Revert is what they wanted.
    #[test]
    fn a_blank_correction_is_refused_and_points_at_revert() {
        for blank in ["", "   ", "\n\t "] {
            let error = validate_summary_text(blank).expect_err("a blank correction is refused");
            match error {
                AppError::BadRequest { message, .. } => {
                    assert!(
                        message.contains("revert"),
                        "the refusal must name the alternative, or the human is \
                         told only that they are wrong: {message}"
                    );
                }
                other => panic!("a blank correction must be a 400, not {other:?}"),
            }
        }
    }

    /// The validated value comes back TRIMMED, so the caller cannot store the
    /// padded version by forgetting to trim it separately.
    #[test]
    fn the_validated_text_is_the_trimmed_text() {
        let text = validate_summary_text("  Did you receive the letter?  ")
            .expect("a non-blank correction is accepted");
        assert_eq!(text, "Did you receive the letter?");
    }
}
