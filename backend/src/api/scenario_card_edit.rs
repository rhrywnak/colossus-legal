//! `PUT /cases/:slug/scenarios/:scenario_id/facts/:graph_node_id/card` — one
//! field of one witness's card (FACT_CARD_v2 §2).
//!
//! ## Why the request is a TAGGED ENUM and not `{field, value}`
//!
//! The five fields hold four different shapes: three strings, an integer
//! position, and a list of accusations. A `{field: String, value: Value}` body
//! would make every one of those a runtime check in the handler, and the `field`
//! token would be a string this build had to validate against a vocabulary. An
//! internally-tagged enum does both at the serde boundary: an unknown field is a
//! 400 before the handler runs, and each variant's value is already the right
//! type.
//!
//! It also makes the ledger-only `card` token UNSENDABLE. There is no variant for
//! it, so no client can replace five fields in one write and leave the record
//! saying only that something happened.
//!
//! ## Why one field per request
//!
//! §2's edit is "click a field → inline edit → PUT one field", and the draft mark
//! is per field precisely so a card with an edited Answer can still show a
//! drafted Watch out. A whole-card PUT would re-stamp all five authorships from
//! one edit.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};

use crate::auth::{require_edit, AuthUser};
use crate::domain::fact_card::{CardField, CardStance, SUPPORTS_CAP};
use crate::error::AppError;
use crate::repositories::pipeline_repository::scenario_fact_cards::{write_field, FieldWrite};
use crate::state::AppState;

use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// One accusation, as an editor submits it.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportInput {
    pub allegation_id: String,
    /// An unknown token fails to parse — see [`CardStance`].
    pub stance: CardStance,
}

/// The edit, as one of five typed shapes.
///
/// `#[serde(tag = "field")]` reads `{"field": "answer", "value": "…"}` and
/// selects the variant by that token. `deny_unknown_fields` on each variant
/// rejects a typo'd key rather than dropping it.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "field", rename_all = "snake_case", deny_unknown_fields)]
pub enum CardFieldUpdate {
    /// What Marie says in three seconds. `null` clears it.
    Title { value: Option<String> },
    /// Which talking point this backs, by 1-based position. `null` clears it.
    BacksPosition { value: Option<i32> },
    /// The accusations this card names. An empty list clears them.
    Supports { value: Vec<SupportInput> },
    /// How the other side uses it. `null` clears it.
    WatchOut { value: Option<String> },
    /// Her reply. `null` clears it.
    Answer { value: Option<String> },
}

impl CardFieldUpdate {
    /// Which column this edit addresses.
    pub fn field(&self) -> CardField {
        match self {
            CardFieldUpdate::Title { .. } => CardField::Title,
            CardFieldUpdate::BacksPosition { .. } => CardField::BacksPosition,
            CardFieldUpdate::Supports { .. } => CardField::Supports,
            CardFieldUpdate::WatchOut { .. } => CardField::WatchOut,
            CardFieldUpdate::Answer { .. } => CardField::Answer,
        }
    }

    /// The value as the column stores it, or the refusal that it cannot be.
    ///
    /// ## Why every field becomes TEXT here
    ///
    /// One writer serves all five columns, and the only difference between them
    /// is a `::jsonb` cast the repository applies. Rendering the position and the
    /// accusation list to text HERE is what keeps that true — the alternative was
    /// five bind types and five near-identical statements.
    ///
    /// A blank string is stored as NULL: a human clearing a field with the
    /// keyboard and one pressing a Clear control are the same act, and the card
    /// renders both as the em dash. Keeping `""` would make the row hold a value
    /// the reader cannot see.
    ///
    /// # Errors
    /// Returns [`AppError::BadRequest`] when the accusation list exceeds the cap
    /// or names a blank id.
    pub fn stored_value(&self) -> Result<Option<String>, AppError> {
        Ok(match self {
            CardFieldUpdate::Title { value }
            | CardFieldUpdate::WatchOut { value }
            | CardFieldUpdate::Answer { value } => value
                .as_deref()
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string),
            CardFieldUpdate::BacksPosition { value } => value.map(|v| v.to_string()),
            CardFieldUpdate::Supports { value } => {
                validate_supports(value)?;
                if value.is_empty() {
                    None
                } else {
                    // Serializing our OWN typed list, so this cannot fail for a
                    // reason a caller could act on; `map_err` keeps the `?` honest
                    // rather than reaching for `expect` in a request path.
                    Some(
                        serde_json::to_string(value).map_err(|e| AppError::Internal {
                            message: format!("could not encode the accusation list: {e}"),
                        })?,
                    )
                }
            }
        })
    }
}

/// The two rules an accusation list must satisfy.
///
/// ## Why the cap is refused rather than truncated
///
/// Truncating would store less than the human asked for and say nothing. The
/// renderer truncates on READ as a backstop against a row edited around the API;
/// the API itself refuses, because here there is somebody to tell.
fn validate_supports(value: &[SupportInput]) -> Result<(), AppError> {
    if value.len() > SUPPORTS_CAP {
        return Err(AppError::BadRequest {
            message: format!(
                "A card names at most {SUPPORTS_CAP} accusations — this one names {}.",
                value.len()
            ),
            details: serde_json::json!({ "field": "supports", "cap": SUPPORTS_CAP }),
        });
    }
    if value.iter().any(|s| s.allegation_id.trim().is_empty()) {
        return Err(AppError::BadRequest {
            message: "An accusation needs an id.".to_string(),
            details: serde_json::json!({ "field": "supports" }),
        });
    }
    Ok(())
}

/// `PUT …/facts/:graph_node_id/card` — store one field.
///
/// Returns `200 OK` with no body: the caller already knows what it sent, and the
/// card it re-reads afterwards is the authority on what is stored.
#[instrument(
    skip(state, user, payload),
    fields(slug = %slug, scenario_id = %scenario_id, graph_node_id = %graph_node_id)
)]
pub async fn put_card_field(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, graph_node_id)): Path<(String, String, String)>,
    Json(payload): Json<CardFieldUpdate>,
) -> Result<StatusCode, AppError> {
    require_edit(&user)?;

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let field = payload.field();
    let value = payload.stored_value()?;

    write_field(
        &state.pipeline_pool,
        &FieldWrite {
            scenario_id: id,
            graph_node_id: &graph_node_id,
            field,
            value: value.as_deref(),
            // The editor becomes the author, which is what clears the draft mark
            // on THIS field and no other (§2).
            author: &user.username,
            // A human editing their own card explains nothing to anyone: the
            // sentence they wrote IS the explanation, and the ledger already
            // records who wrote it and when. The note carries a LOADER's reason
            // for a choice (R2), which has no equivalent here.
            note: None,
            written_at: Utc::now(),
        },
    )
    .await
    .map_err(|e| {
        tracing::error!(
            error = %e,
            field = field.code(),
            author = %user.username,
            "failed to store a fact-card field; nothing was written and the ledger \
             records nothing — the sentence on screen is not the sentence stored"
        );
        // The body names the field and the node, not just the failure: an
        // operator reading a 500 in the network tab should be able to find the
        // row without first correlating a timestamp against the log.
        AppError::Internal {
            message: format!(
                "failed to save the {} field of card {graph_node_id} — nothing was \
                 written; edit the field again to retry",
                field.code()
            ),
        }
    })?;

    info!(
        author = %user.username,
        field = field.code(),
        cleared = value.is_none(),
        "stored a fact-card field"
    );
    Ok(StatusCode::OK)
}

#[cfg(test)]
#[path = "scenario_card_edit_tests.rs"]
mod tests;
