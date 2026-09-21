//! The wire half: one streamed Messages call, behind a trait.
//!
//! ## Rust Learning: why the model call is a trait (`ChatBackend`)
//!
//! The tool loop ([`crate::engine`]) is the part with real logic — rounds, parallel
//! tool calls, refusals, citation checks. Putting the HTTP call behind a trait lets
//! that logic be tested against a scripted backend that returns canned messages,
//! with no socket and no tokens. It is also the seam a second provider would plug
//! into; ruling D3 (2026-09-21) says only Anthropic ships now, and a model that
//! cannot produce platform-enforced citations is refused by the caller, not
//! half-supported here.

use std::time::Duration;

use serde_json::Value;

use crate::accumulate::{AssistantMessage, ChatAccumulator, ChatStreamError};
use crate::transport::{self, ResponseChunks, TransportError};

/// The transport error at this crate's stream-error type.
pub type ChatTransportError = TransportError<ChatStreamError>;

/// STRUCTURAL: the Messages endpoint path (protocol, not a setting).
const MESSAGES_PATH: &str = "/v1/messages";

/// Connection settings. Every value comes from the caller's configuration; the
/// crate holds no defaults of its own (Standing Rule 2).
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// API base URL, e.g. the provider's public endpoint or a record/replay proxy.
    pub base_url: String,
    /// API key, sent as `x-api-key`.
    pub api_key: String,
    /// The `anthropic-version` header value.
    pub api_version: String,
    /// `anthropic-beta` values, joined with commas. Empty = header omitted.
    pub betas: Vec<String>,
    /// TCP connect timeout.
    pub connect_timeout: Duration,
    /// TCP keep-alive interval.
    pub tcp_keepalive: Duration,
    /// Fail a call when no event completes in this window.
    pub idle_timeout: Duration,
    /// How much of an error body or malformed event a failure quotes.
    pub error_preview_chars: usize,
}

/// One streamed model call.
#[async_trait::async_trait]
pub trait ChatBackend: Send + Sync {
    /// POST `body`; forward prose deltas to `on_text` as they arrive; return the
    /// finished message.
    ///
    /// # Errors
    /// Any transport or stream failure.
    async fn call(
        &self,
        body: &Value,
        on_text: &(dyn Fn(String) + Send + Sync),
    ) -> Result<AssistantMessage, ChatTransportError>;
}

/// The Anthropic Messages API over HTTP/1.1 with SSE.
pub struct AnthropicBackend {
    http: reqwest::Client,
    config: EngineConfig,
    messages_url: String,
}

impl AnthropicBackend {
    /// Build the backend and its one shared HTTP client.
    ///
    /// NOTE the absent whole-request `.timeout(...)` — deliberate; see
    /// [`crate::transport`]. `.http1_only()` is mandatory: api.anthropic.com hangs
    /// over HTTP/2 inside Podman containers (backend `Cargo.toml`, `reqwest_13`).
    ///
    /// # Errors
    /// The client failed to build (typically a missing TLS provider).
    pub fn new(config: EngineConfig) -> Result<Self, reqwest::Error> {
        let http = reqwest::Client::builder()
            .connect_timeout(config.connect_timeout)
            .tcp_keepalive(config.tcp_keepalive)
            .http1_only()
            .build()?;
        let messages_url = format!("{}{MESSAGES_PATH}", config.base_url.trim_end_matches('/'));
        Ok(Self {
            http,
            config,
            messages_url,
        })
    }

    async fn open(&self, body: &Value) -> Result<reqwest::Response, ChatTransportError> {
        let bytes = serde_json::to_vec(body).map_err(|e| TransportError::Status {
            status: 0,
            body: format!("the request body could not be serialized: {e}"),
        })?;
        let mut request = self
            .http
            .post(&self.messages_url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", &self.config.api_version)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream");
        if !self.config.betas.is_empty() {
            request = request.header("anthropic-beta", self.config.betas.join(","));
        }
        let response = request
            .body(bytes)
            .send()
            .await
            .map_err(|source| TransportError::Http { source })?;
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get(transport::RETRY_AFTER)
            // best-effort: a retry-after that is not ASCII is "the provider did
            // not say" — the same `None` an absent header gives, and the status
            // itself is still reported.
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        // A body that cannot be read is reported as such, not as an empty body.
        let text = match response.text().await {
            Ok(t) => t,
            Err(e) => format!("(the error body could not be read: {e})"),
        };
        Err(transport::classify_status(
            status,
            retry_after.as_deref(),
            &text,
            self.config.error_preview_chars,
        ))
    }
}

#[async_trait::async_trait]
impl ChatBackend for AnthropicBackend {
    async fn call(
        &self,
        body: &Value,
        on_text: &(dyn Fn(String) + Send + Sync),
    ) -> Result<AssistantMessage, ChatTransportError> {
        let response = self.open(body).await?;
        let mut chunks = ResponseChunks::new(response);
        transport::drive(
            &mut chunks,
            ChatAccumulator::new(self.config.error_preview_chars),
            self.config.idle_timeout,
            |fold: &mut ChatAccumulator| {
                let fresh = fold.take_text();
                if !fresh.is_empty() {
                    on_text(fresh);
                }
            },
        )
        .await
    }
}
