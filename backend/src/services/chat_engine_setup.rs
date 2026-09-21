//! Building the chat engine's backend from the environment, once, at boot.
//!
//! ## Why the extraction engine's env keys, and not new ones
//!
//! Both engines talk to the same provider, with the same key, through the same
//! kind of HTTP/1.1 streaming client. Reading the SAME variables
//! (`ANTHROPIC_API_KEY`, `ANTHROPIC_BASE_URL`, `ANTHROPIC_API_VERSION`,
//! `LLM_STREAM_IDLE_TIMEOUT_SECS`, and the connect / keep-alive pair) means no new
//! deployment configuration and no Ansible change — and one place to tune the
//! stall detector for both. The names and defaults are the extraction engine's
//! own constants, so they cannot drift.

use std::sync::Arc;
use std::time::Duration;

use colossus_chat::{AnthropicBackend, EngineConfig};

use crate::pipeline::anthropic_engine::{
    read_secs_env, read_string_env, ANTHROPIC_API_KEY_ENV, API_VERSION_ENV, BASE_URL_ENV,
    CONNECT_TIMEOUT_SECS_ENV, DEFAULT_API_VERSION, DEFAULT_BASE_URL, DEFAULT_CONNECT_TIMEOUT_SECS,
    DEFAULT_IDLE_TIMEOUT_SECS, DEFAULT_TCP_KEEPALIVE_SECS, IDLE_TIMEOUT_SECS_ENV,
    TCP_KEEPALIVE_SECS_ENV,
};

/// The `anthropic-beta` value that turns on server-side compaction.
///
/// CONST: protocol vocabulary paired with the `compact_20260112` edit type the
/// crate sends — the two change together, in code, when the API revises them.
/// Whether compaction RUNS is the `question_chat_compaction_trigger_tokens` row;
/// the header alone does nothing.
const COMPACTION_BETA: &str = "compact-2026-01-12";

/// Build the chat backend, or `None` with a logged reason when there is no API
/// key — the same "no key, no chat" state the Chat providers map has, reported
/// by name at every chat request as a 503 rather than a panic.
pub fn build_chat_backend() -> Option<Arc<AnthropicBackend>> {
    let Ok(api_key) = std::env::var(ANTHROPIC_API_KEY_ENV) else {
        tracing::warn!(
            env_var = ANTHROPIC_API_KEY_ENV,
            "question chat is OFF: {ANTHROPIC_API_KEY_ENV} is unset, so every chat request \
             will be refused with a named 503"
        );
        return None;
    };
    let config = EngineConfig {
        base_url: read_string_env(BASE_URL_ENV, DEFAULT_BASE_URL),
        api_key,
        api_version: read_string_env(API_VERSION_ENV, DEFAULT_API_VERSION),
        betas: vec![COMPACTION_BETA.to_string()],
        connect_timeout: Duration::from_secs(read_secs_env(
            CONNECT_TIMEOUT_SECS_ENV,
            DEFAULT_CONNECT_TIMEOUT_SECS,
        )),
        tcp_keepalive: Duration::from_secs(read_secs_env(
            TCP_KEEPALIVE_SECS_ENV,
            DEFAULT_TCP_KEEPALIVE_SECS,
        )),
        idle_timeout: Duration::from_secs(read_secs_env(
            IDLE_TIMEOUT_SECS_ENV,
            DEFAULT_IDLE_TIMEOUT_SECS,
        )),
    };
    tracing::info!(
        base_url = %config.base_url,
        idle_timeout_secs = config.idle_timeout.as_secs(),
        "question chat engine configured (streaming, citations, compaction beta)"
    );
    match AnthropicBackend::new(config) {
        Ok(backend) => Some(Arc::new(backend)),
        Err(e) => {
            tracing::error!(error = %e, "question chat is OFF: its HTTP client could not be built");
            None
        }
    }
}
