//! The one rule about which models may be offered to the discussion.
//!
//! `llm_models.grounded` is the column `services::chat_model_check` reads, so
//! what this module decides is literally which models the Discuss chat will
//! accept. It is a sibling of [`super::models`] rather than part of it for the
//! reason that file's own test module already gives: the CRUD surface's subject
//! is HTTP shape, and this one's is a single domain invariant about a PAIR of
//! columns. Keeping them apart also keeps both under Rule 17.

use crate::error::AppError;
use crate::repositories::pipeline_repository::models;
use crate::state::AppState;

use super::models::UpdateModelRequest;

/// The one provider a model may be `grounded` on.
///
/// STRUCTURAL: grounding is not a preference, it is a capability of a specific
/// API. "Grounded" means the Anthropic Messages API with Citations, which is what
/// lets `colossus_chat::citation` check a quotation against the stored document
/// before the discussion shows it. A vLLM or OpenAI row can be excellent and
/// still cannot do that, so the value is compiled in beside the provider list it
/// belongs to rather than configured.
// STRUCTURAL: an API capability, not a deployment tunable.
const GROUNDED_PROVIDER: &str = "anthropic";

/// Refuse `grounded = true` for a provider that cannot produce checked quotes.
///
/// `provider` and `grounded` are the EFFECTIVE pair — what the row will hold once
/// this write lands — not necessarily what the request body carries. See
/// [`refuse_invalid_grounding`], which assembles that pair.
///
/// # Errors
/// [`AppError::BadRequest`] naming the provider, in the words an operator who
/// just ticked a checkbox needs.
pub(super) fn validate_grounded(grounded: Option<bool>, provider: &str) -> Result<(), AppError> {
    if grounded != Some(true) || provider == GROUNDED_PROVIDER {
        return Ok(());
    }
    Err(AppError::BadRequest {
        message: format!(
            "'{provider}' models cannot quote the record. Only {GROUNDED_PROVIDER} \
             models return quotations the system can check against the stored \
             document, so only they can be offered to the discussion. Either set \
             this model's provider to {GROUNDED_PROVIDER}, or clear \
             'Can quote the record' and save it without that permission."
        ),
        details: serde_json::json!({"field": "grounded"}),
    })
}

/// Refuse any update that would leave the row grounded on a provider that
/// cannot quote the record.
///
/// # Why the request body alone is not enough — BOTH halves can be missing
///
/// The invariant is about the row that RESULTS, and a PATCH may supply either
/// half of it, or neither:
///
/// | body says | the other half comes from | why it matters |
/// |---|---|---|
/// | `{grounded: true}` alone | the stored provider | the Edit form sends exactly this when somebody only ticks the box; judged against the body alone there is no provider at all, and a vLLM row would be grounded through the back door |
/// | `{provider: "vllm"}` alone | the stored `grounded` | `grounded` is left alone by the COALESCE, so a row that was grounded stays grounded — on a provider that cannot quote |
/// | both | itself | no read needed |
///
/// The second row is the one a guard on `grounded` alone misses, and it is the
/// same invalid state reached from the other direction.
///
/// Two shapes settle it with no read at all: a request that WITHDRAWS the
/// permission can never produce an invalid row, and a request that names
/// `anthropic` is valid whatever the flag ends up as. Everything else reads the
/// stored row once.
///
/// # Errors
/// [`AppError::BadRequest`] from [`validate_grounded`]; [`AppError::NotFound`]
/// when the id names no row; [`AppError::Internal`] when the read fails.
pub(super) async fn refuse_invalid_grounding(
    state: &AppState,
    model_id: &str,
    input: &UpdateModelRequest,
) -> Result<(), AppError> {
    // Withdrawing the permission cannot produce an invalid row, whatever the
    // provider becomes.
    if input.grounded == Some(false) {
        return Ok(());
    }
    // A row that is (or becomes) anthropic is valid whatever `grounded` ends up
    // as, so there is nothing to look up.
    if input.provider.as_deref() == Some(GROUNDED_PROVIDER) {
        return Ok(());
    }
    // Neither half is being touched: this update cannot change the pairing.
    if input.grounded.is_none() && input.provider.is_none() {
        return Ok(());
    }
    let (provider, grounded) = match (input.provider.as_deref(), input.grounded) {
        (Some(p), Some(g)) => (p.to_string(), g),
        _ => {
            let row = models::get_model_by_id(&state.pipeline_pool, model_id)
                .await
                .map_err(|e| AppError::Internal {
                    message: format!(
                        "Failed to read model '{model_id}' before changing whether it \
                         may quote the record: {e}"
                    ),
                })?
                .ok_or_else(|| AppError::NotFound {
                    message: format!(
                        "Model '{model_id}' not found — check the id, or add the model \
                         in Admin -> Models before setting this permission"
                    ),
                })?;
            (
                input.provider.clone().unwrap_or(row.provider),
                input.grounded.unwrap_or(row.grounded),
            )
        }
    };
    validate_grounded(Some(grounded), &provider)
}

#[cfg(test)]
#[path = "models_grounding_tests.rs"]
mod tests;
