//! Is the stored Chat default a model this process can actually answer with?
//!
//! CC_TASK_CHAT_DEFAULT_MODEL_v1. One predicate and its four faults.
//!
//! ## Why this is a module and not four lines in `main.rs`
//!
//! It decides whether the process STARTS. A rule that can only be exercised by
//! booting a server against a database is a rule nobody runs — and the previous
//! version of this decision was exactly that: a startup `tracing::error!` that
//! fired on PROD on 2026-09-19 while the service came up and answered 400 to
//! every `/ask` that did not name a model.
//!
//! So the RULE lives here, in the library, where `cargo test` reaches it, and
//! `main.rs` keeps only what a binary owns: the fetch, and the exit.
//!
//! ## Domain note: what "can answer with" means
//!
//! Chat dispatch is Anthropic-only — `main::build_chat_providers` skips every
//! other provider — and it builds its map from the ACTIVE rows. So a usable
//! default is a row that exists, is active, is Anthropic, and actually built.
//! Four ways to fail, four different first moves for whoever reads the refusal.

use crate::repositories::pipeline_repository::models;

/// Why the configured Chat default cannot be served, when it cannot.
///
/// Four states, four remedies, and they must not be collapsed into one
/// "bad default" (Standing Rule 1): the person reading a refused boot has no
/// other source for which of the four it is.
#[derive(Debug, PartialEq, Eq)]
pub enum ChatDefaultFault {
    /// No `llm_models` row carries that id at all — a typo, or a deleted row.
    NoSuchModel,
    /// The row exists and is deactivated. The Admin toggle is the remedy.
    Inactive,
    /// The row is live but is not Anthropic. Chat dispatch is Anthropic-only
    /// (`build_chat_providers` skips every other provider), so a vLLM model is
    /// a legal `llm_models` row and an illegal Chat default.
    NotAnthropic,
    /// The row is live and Anthropic, and the provider still did not build —
    /// `AnthropicProvider::new` refused it. The row is right; this process
    /// cannot honour it.
    ProviderFailed,
}

impl ChatDefaultFault {
    /// What an operator reading a refused boot should DO about it.
    pub fn remedy(&self) -> &'static str {
        match self {
            Self::NoSuchModel => {
                "no llm_models row has that id — check it for a typo, or add the row"
            }
            Self::Inactive => "that model is deactivated — activate it in the Admin model list",
            Self::NotAnthropic => {
                "that model is not an anthropic row, and Chat dispatch is anthropic-only \
                 — point chat_default_model at an anthropic model"
            }
            Self::ProviderFailed => {
                "the row is live but its provider would not build — check the model's \
                 endpoint and credentials in the Admin model list"
            }
        }
    }
}

/// Is the configured Chat default one this process can actually answer with?
///
/// ## Why this is PURE, beside an async builder
///
/// The whole value of the check is that it refuses a boot, and a check that can
/// only be exercised by starting a server against a database is a check nobody
/// runs. Split this way, all four faults are unit tests, and the async half in
/// `main.rs` does nothing but fetch a row and exit.
///
/// `row` is the `llm_models` row for the configured id WHATEVER its state —
/// active or not — which is what lets `Inactive` be told from `NoSuchModel`.
/// The id itself is not a parameter: it is already the key the row was fetched
/// by, and taking it again would be a second chance to pass the wrong one.
pub fn verify_chat_default(
    row: Option<&models::LlmModelRecord>,
    in_provider_map: bool,
) -> Result<(), ChatDefaultFault> {
    let Some(row) = row else {
        return Err(ChatDefaultFault::NoSuchModel);
    };
    if !row.is_active {
        return Err(ChatDefaultFault::Inactive);
    }
    // STRUCTURAL: the `llm_models.provider` vocabulary for Anthropic-hosted
    // models, and a standing architectural ruling — Chat dispatch is
    // Anthropic-only, because `main::build_chat_providers` builds its map by
    // skipping every other provider. Not a deployment choice: making this
    // configurable would mean a chat path that can dispatch to vLLM, which is a
    // feature nobody has built, not a value anybody can set.
    if row.provider != "anthropic" {
        return Err(ChatDefaultFault::NotAnthropic);
    }
    if !in_provider_map {
        return Err(ChatDefaultFault::ProviderFailed);
    }
    Ok(())
}

#[cfg(test)]
#[path = "chat_default_tests.rs"]
mod tests;
