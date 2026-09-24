//! The Chat page's model providers, found per request (CC_TASK_MODEL_JOBS_PANEL_v1,
//! item 8 / B4).
//!
//! ## What this replaces, and why
//!
//! The Chat page used to build one provider per active Anthropic model ONCE, at
//! boot, into a map, and to copy the `chat_default_model` row ONCE into
//! `AppState`. A model added on Admin → Models, or a new default saved on the
//! Overview panel, did nothing until a restart — and a model switched off after
//! boot stayed callable. Now every call asks `llm_models` whether the model may
//! serve, and builds its provider the first time it is needed.
//!
//! ## Rust Learning: `std::sync::RwLock` around a cache in shared state
//!
//! `AppState` is cloned into every request, so the cache sits behind an `Arc`
//! (in `AppState`) and a lock (here). An `RwLock` lets many requests READ the
//! cache at once and takes the exclusive write lock only to add a provider. It is
//! the `std` lock, not tokio's, because no `.await` ever happens while it is
//! held — the database read comes first, the lock after — and a `std` lock held
//! across an `.await` is the one mistake that would make it wrong.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use colossus_extract::providers::AnthropicProvider;
use colossus_extract::LlmProvider;
use sqlx::PgPool;

use crate::repositories::pipeline_repository::models::get_model_by_id;
use crate::services::chat_default::{verify_chat_default, ChatDefaultFault};

/// Per-chat-model `max_tokens` passed to `AnthropicProvider::new`. The Chat
/// endpoint always wraps the provider in `RigSynthesizer::new(_, 4096)` at
/// request time, so this is used only if a caller invokes the provider directly.
// STRUCTURAL: moved verbatim from `main.rs` with the map it served; it is the
// provider's construction argument, overridden by the synthesizer on every call.
const CHAT_MAX_TOKENS: u32 = 4096;

/// Why the Chat page cannot answer with a model. Each names its model and cause.
#[derive(Debug, thiserror::Error)]
pub enum ChatProviderError {
    #[error("no ANTHROPIC_API_KEY is configured, so the Chat page has no model provider")]
    NoKey,
    #[error("the models list could not be read for {model_id}: {source}")]
    Lookup {
        model_id: String,
        source: sqlx::Error,
    },
    #[error("model {model_id} cannot answer on the Chat page: {}", fault.remedy())]
    NotUsable {
        model_id: String,
        fault: ChatDefaultFault,
    },
    #[error("the provider for {model_id} could not be built: {detail}")]
    Build { model_id: String, detail: String },
}

/// The providers, built on first use.
pub struct ChatProviders {
    api_key: Option<String>,
    cache: RwLock<HashMap<String, Arc<dyn LlmProvider>>>,
}

impl ChatProviders {
    /// No provider is built here: each is built the first time a call needs it.
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key,
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Whether an API key is configured at all.
    pub fn configured(&self) -> bool {
        self.api_key.is_some()
    }

    /// The provider for `model_id`, if the models list says it may serve now.
    ///
    /// # Errors
    /// [`ChatProviderError`]: no key, an unreadable models list, a model that is
    /// missing / switched off / not Anthropic, or a provider that would not build.
    pub async fn for_model(
        &self,
        pool: &PgPool,
        model_id: &str,
    ) -> Result<Arc<dyn LlmProvider>, ChatProviderError> {
        let api_key = self.api_key.as_ref().ok_or(ChatProviderError::NoKey)?;
        let row =
            get_model_by_id(pool, model_id)
                .await
                .map_err(|source| ChatProviderError::Lookup {
                    model_id: model_id.to_string(),
                    source,
                })?;
        // `true`: whether the provider builds is answered below, by building it.
        verify_chat_default(row.as_ref(), true).map_err(|fault| ChatProviderError::NotUsable {
            model_id: model_id.to_string(),
            fault,
        })?;
        if let Some(found) = self.cached(model_id) {
            return Ok(found);
        }
        let built: Arc<dyn LlmProvider> = Arc::new(
            AnthropicProvider::new(
                api_key.clone(),
                model_id.to_string(),
                CHAT_MAX_TOKENS,
                None, // natural variation for chat
                None, // request_timeout_secs: provider default (600s)
            )
            .map_err(|e| ChatProviderError::Build {
                model_id: model_id.to_string(),
                detail: e.to_string(),
            })?,
        );
        self.store(model_id, &built);
        tracing::info!(model = model_id, "chat provider built on first use");
        Ok(built)
    }

    fn cached(&self, model_id: &str) -> Option<Arc<dyn LlmProvider>> {
        match self.cache.read() {
            Ok(map) => map.get(model_id).cloned(),
            // A poisoned lock means a thread panicked mid-insert; the map itself
            // is still whole (an insert either happened or did not), so read it.
            Err(poisoned) => poisoned.into_inner().get(model_id).cloned(),
        }
    }

    fn store(&self, model_id: &str, provider: &Arc<dyn LlmProvider>) {
        let mut map = match self.cache.write() {
            Ok(map) => map,
            Err(poisoned) => poisoned.into_inner(),
        };
        map.insert(model_id.to_string(), Arc::clone(provider));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn without_a_key_nothing_is_looked_up() {
        let providers = ChatProviders::new(None);
        assert!(!providers.configured());
        // A lazy pool never connects unless a query runs; NoKey returns first.
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused@127.0.0.1:1/none")
            .expect("a lazy pool needs only a well-formed URL");
        let err = providers.for_model(&pool, "claude-opus-5").await.err();
        assert!(matches!(err, Some(ChatProviderError::NoKey)));
    }

    #[test]
    fn every_failure_says_what_failed_and_for_which_model() {
        assert!(ChatProviderError::NoKey
            .to_string()
            .contains("ANTHROPIC_API_KEY"));
        let lookup = ChatProviderError::Lookup {
            model_id: "m1".into(),
            source: sqlx::Error::RowNotFound,
        }
        .to_string();
        assert!(
            lookup.contains("m1") && lookup.contains("could not be read"),
            "{lookup}"
        );
        let build = ChatProviderError::Build {
            model_id: "m1".into(),
            detail: "bad key".into(),
        }
        .to_string();
        assert!(build.contains("m1") && build.contains("bad key"), "{build}");
    }

    #[test]
    fn each_refusal_names_its_model() {
        let e = ChatProviderError::NotUsable {
            model_id: "m1".into(),
            fault: ChatDefaultFault::Inactive,
        };
        let text = e.to_string();
        assert!(
            text.contains("m1") && text.contains("deactivated"),
            "{text}"
        );
    }
}
