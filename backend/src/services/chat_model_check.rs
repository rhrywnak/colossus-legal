//! Is the configured chat model one the chat can actually use?
//!
//! Asked twice, with one answer: when someone SAVES `question_chat_model` on the
//! Settings page, and when the backend BOOTS (ruling D3, 2026-09-21). A model the
//! chat cannot use is one that is absent, inactive, or not `grounded` — without
//! the Citations API there are no platform-enforced quotes, and the chat's prompt
//! promises "no citation, no claim".

use sqlx::PgPool;

use crate::domain::chat_params::KEY_QUESTION_CHAT_MODEL;
use crate::repositories::pipeline_repository::models::{get_model_by_id, LlmModelRecord};
use crate::services::settings_store::SettingsError;

/// Why a model row cannot serve the chat, in words for the person who chose it —
/// or `None` when it can.
pub fn unusable_reason(row: Option<&LlmModelRecord>) -> Option<String> {
    match row {
        None => Some("no llm_models row has that id".to_string()),
        Some(m) if !m.is_active => Some(format!(
            "{} is deactivated in the Admin model list",
            m.display_name
        )),
        Some(m) if !m.grounded => Some(format!(
            "{} is not grounded — it cannot return platform-checked quotations, \
             and the discussion shows no claim it cannot quote",
            m.display_name
        )),
        Some(_) => None,
    }
}

/// The save-time half: refuse a `question_chat_model` edit that names a model the
/// chat cannot use. Any other key passes untouched.
///
/// # Errors
/// [`SettingsError::ModelNotUsable`] naming the reason, or [`SettingsError::Read`]
/// when the model table cannot be read.
pub async fn check_chat_model_on_save(
    pool: &PgPool,
    key: &str,
    candidate: &str,
) -> Result<(), SettingsError> {
    if key != KEY_QUESTION_CHAT_MODEL {
        return Ok(());
    }
    let row = get_model_by_id(pool, candidate)
        .await
        .map_err(|source| SettingsError::Read {
            source: source.into(),
        })?;
    match unusable_reason(row.as_ref()) {
        None => Ok(()),
        Some(reason) => Err(SettingsError::ModelNotUsable {
            key: key.to_string(),
            value: candidate.to_string(),
            reason,
        }),
    }
}

/// The boot half: the model the snapshot names must still qualify, or the process
/// does not start — the same refusal `assert_chat_default_is_live` makes for the
/// Chat default, and for the same reason: discovering it mid-conversation means
/// discovering it in front of the witness.
pub async fn assert_chat_model_usable(pool: &PgPool, model: &str) {
    let row = match get_model_by_id(pool, model).await {
        Ok(row) => row,
        Err(e) => {
            tracing::error!(model, error = %e, "BOOT REFUSED: the llm_models table could not be read to check question_chat_model");
            std::process::exit(1);
        }
    };
    match unusable_reason(row.as_ref()) {
        None => tracing::info!(model, "question chat model is active and grounded"),
        Some(reason) => {
            tracing::error!(model, %reason,
                "BOOT REFUSED: question_chat_model names a model the chat cannot use. \
                 Edit the question_chat_model settings row, or fix the model it names.");
            std::process::exit(1);
        }
    }
}

/// Every boot precondition of the question chat, in one call from `main.rs`
/// (ruling D3: the model is active and grounded; ruling D5: orphaned threads are
/// counted out loud). The prompt and narrative FILES are checked with the other
/// prompts, in `settings_template_file::assert_prompts_deployed`.
pub async fn assert_chat_ready(state: &crate::state::AppState) {
    let model = state.settings.current().question_chat.model.clone();
    assert_chat_model_usable(&state.pipeline_pool, &model).await;
    log_orphan_threads(&state.pipeline_pool).await;
}

/// Ruling D5's boot count: question threads whose question no longer exists.
/// They are kept (nothing is ever deleted) and said out loud, so an orphan is a
/// known number rather than a surprise.
pub async fn log_orphan_threads(pool: &PgPool) {
    match crate::repositories::pipeline_repository::chat_discussions::orphan_question_threads(pool)
        .await
    {
        Ok(0) => tracing::info!("question chat: no orphaned threads"),
        Ok(n) => tracing::warn!(
            orphans = n,
            "question chat: threads whose question no longer exists (kept, not shown)"
        ),
        Err(e) => {
            tracing::error!(error = %e, "question chat: the orphan-thread count could not be read")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(active: bool, grounded: bool) -> LlmModelRecord {
        LlmModelRecord {
            id: "m".into(),
            display_name: "Model M".into(),
            provider: "anthropic".into(),
            api_endpoint: None,
            max_context_tokens: Some(1_000_000),
            max_output_tokens: None,
            cost_per_input_token: None,
            cost_per_output_token: None,
            is_active: active,
            created_at: chrono::Utc::now(),
            notes: None,
            default_temperature: None,
            temperature_mode: None,
            timeout_secs: None,
            structured_output_mode: None,
            max_concurrency: None,
            billing_class: "billed".into(),
            grounded,
        }
    }

    #[test]
    fn only_an_active_grounded_row_is_usable() {
        assert_eq!(unusable_reason(Some(&row(true, true))), None);
        assert!(unusable_reason(None).unwrap().contains("no llm_models row"));
        assert!(unusable_reason(Some(&row(false, true)))
            .unwrap()
            .contains("deactivated"));
        assert!(unusable_reason(Some(&row(true, false)))
            .unwrap()
            .contains("not grounded"));
    }
}
