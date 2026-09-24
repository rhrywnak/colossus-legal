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
use crate::usage::Usage;

/// The transport error at this crate's stream-error type.
pub type ChatTransportError = TransportError<ChatStreamError>;

/// STRUCTURAL: the Messages endpoint path (protocol, not a setting).
const MESSAGES_PATH: &str = "/v1/messages";

/// STRUCTURAL: the token-counting endpoint path (protocol, not a setting).
/// Free — it counts input tokens and generates nothing.
const COUNT_TOKENS_PATH: &str = "/v1/messages/count_tokens";

/// The field `count_tokens` answers with.
///
/// STRUCTURAL: the response's wire shape.
const COUNT_FIELD: &str = "input_tokens";

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

    /// How many input tokens `body` is, asked of the provider rather than guessed.
    ///
    /// Free: the endpoint counts and generates nothing. `body` comes from
    /// [`crate::request::build_count_body`], so it is the body that would be sent.
    ///
    /// ## Why this is on the trait and not a free function
    ///
    /// Standing Rule 10 — callers hold the trait. It is also what makes a size
    /// guard testable: a scripted backend answers a known count with no socket
    /// and no key, so "the guard used the measurement" is an assertion rather
    /// than an integration test.
    ///
    /// # Errors
    /// Any transport failure, or a response without a readable count.
    async fn count_tokens(&self, body: &Value) -> Result<u64, ChatTransportError>;

    /// Send a cache pre-warm and return what it read and wrote.
    ///
    /// `body` comes from [`crate::request::build_prewarm_body`]: the prefix a
    /// real turn sends, with `max_tokens: 0` and no stream. The provider fills
    /// or refreshes the cache and generates nothing. The caller decides what the
    /// usage means (a read kept the cache loaded; a write reloaded it).
    ///
    /// # Errors
    /// Any transport failure, a refusal carrying the provider's text, or a
    /// response with no readable usage.
    async fn prewarm(&self, body: &Value) -> Result<Usage, ChatTransportError>;
}

/// The Anthropic Messages API over HTTP/1.1 with SSE.
pub struct AnthropicBackend {
    http: reqwest::Client,
    config: EngineConfig,
    messages_url: String,
    count_tokens_url: String,
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
        let base = config.base_url.trim_end_matches('/').to_string();
        let messages_url = format!("{base}{MESSAGES_PATH}");
        let count_tokens_url = format!("{base}{COUNT_TOKENS_PATH}");
        Ok(Self {
            http,
            config,
            messages_url,
            count_tokens_url,
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

    /// The non-streamed POST behind [`ChatBackend::count_tokens`].
    ///
    /// ## Why this one DOES carry a whole-request `.timeout(...)`
    ///
    /// The streamed call deliberately has none: [`crate::transport`] drives it
    /// with an idle timeout, which is the right bound for a response that
    /// arrives over seconds in many pieces. A count is one small JSON reply with
    /// no events to be idle between, so the idle detector has nothing to watch
    /// and the request could hang forever. It reuses `idle_timeout` rather than
    /// introducing a new setting: "fail if nothing has happened in this window"
    /// is the same promise, and the window is already operator-tunable
    /// (`LLM_STREAM_IDLE_TIMEOUT_SECS`).
    async fn count(&self, body: &Value) -> Result<u64, ChatTransportError> {
        let bytes = serde_json::to_vec(body).map_err(|e| TransportError::Status {
            status: 0,
            body: format!("the count request body could not be serialized: {e}"),
        })?;
        let mut request = self
            .http
            .post(&self.count_tokens_url)
            .timeout(self.config.idle_timeout)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", &self.config.api_version)
            .header("content-type", "application/json");
        if !self.config.betas.is_empty() {
            request = request.header("anthropic-beta", self.config.betas.join(","));
        }
        let response = request
            .body(bytes)
            .send()
            .await
            .map_err(|source| TransportError::Http { source })?;
        let status = response.status();
        // A body that cannot be read is reported as such, not as an empty body —
        // the same rule `open` follows above.
        let text = match response.text().await {
            Ok(t) => t,
            Err(e) => format!("(the count response body could not be read: {e})"),
        };
        read_count(status.as_u16(), &text, self.config.error_preview_chars)
    }
}

/// Turn one `count_tokens` response into a number, or into the named failure it
/// really is.
///
/// Separated from the socket so all three outcomes are testable without one:
/// a non-success status, a success whose body is not JSON (including the marker
/// `count` writes when the body could not even be read), and a success whose JSON
/// carries no `input_tokens`.
///
/// # Why: never a zero
///
/// A count that silently became `0` would sail through any size guard and leave
/// nothing in the logs. Every path that is not a real number is an `Err`.
///
/// # Errors
/// [`TransportError::Status`], or whatever [`transport::classify_status`] makes
/// of a non-success status (rate limit, overload, and so on).
fn read_count(status: u16, text: &str, preview_chars: usize) -> Result<u64, ChatTransportError> {
    if !(200..300).contains(&status) {
        return Err(transport::classify_status(
            status,
            None,
            text,
            preview_chars,
        ));
    }
    let unreadable = |detail: &str| TransportError::Status {
        status,
        body: format!(
            "the count response carried no readable `{COUNT_FIELD}` ({detail}): {}",
            preview(text, preview_chars)
        ),
    };
    // The parse error is not discarded: it becomes the `detail` the message
    // above quotes, so an operator reads WHY the body could not be understood.
    let parsed: Value = serde_json::from_str(text).map_err(|e| unreadable(&e.to_string()))?;
    parsed
        .get(COUNT_FIELD)
        .and_then(Value::as_u64)
        .ok_or_else(|| unreadable("the field is absent or not a whole number"))
}

/// The first `limit` characters of `text`, for an error message.
///
/// ## Rust Learning: why `chars().take(n)` and not `&text[..n]`
///
/// Slicing a `String` by byte index panics when the index lands inside a
/// multi-byte character, and an API error body is exactly where a stray
/// non-ASCII byte turns up. Taking characters cannot split one.
fn preview(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
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

    async fn count_tokens(&self, body: &Value) -> Result<u64, ChatTransportError> {
        self.count(body).await
    }

    async fn prewarm(&self, body: &Value) -> Result<Usage, ChatTransportError> {
        self.prewarm_post(body).await
    }
}

#[path = "backend_prewarm.rs"]
mod prewarm;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportError;

    fn err(status: u16, body: &str) -> String {
        match read_count(status, body, 40) {
            Ok(n) => panic!("expected a failure, got {n}"),
            Err(TransportError::Status { body, .. }) => body,
            Err(other) => format!("{other}"),
        }
    }

    #[test]
    fn a_well_formed_count_is_the_number() {
        assert_eq!(
            read_count(200, r#"{"input_tokens": 368810}"#, 40).ok(),
            Some(368_810)
        );
    }

    /// Three ways a 200 can be useless, three DIFFERENT messages — and never a
    /// zero, which would sail through the size guard unnoticed.
    #[test]
    fn a_useless_success_names_which_kind_of_useless_it_is() {
        let not_json = err(200, "<html>gateway</html>");
        assert!(
            not_json.contains("no readable `input_tokens`"),
            "{not_json}"
        );
        assert!(not_json.contains("expected value"), "{not_json}");

        let missing = err(200, r#"{"output_tokens": 12}"#);
        assert!(missing.contains("the field is absent"), "{missing}");

        let wrong_type = err(200, r#"{"input_tokens": "lots"}"#);
        assert!(wrong_type.contains("not a whole number"), "{wrong_type}");

        // The body that `count` writes when the response could not even be READ
        // still arrives here as a named failure, carrying its own reason — as
        // long as the configured preview is long enough to reach it, which is
        // why this one case asks for a wider preview than the 40 above.
        let unread = match read_count(200, "(the count response body could not be read: eof)", 200)
        {
            Err(TransportError::Status { body, .. }) => body,
            other => panic!("expected a named failure, got {other:?}"),
        };
        assert!(unread.contains("could not be read: eof"), "{unread}");

        assert_ne!(not_json, missing);
        assert_ne!(missing, wrong_type);
    }

    #[test]
    fn a_long_body_is_previewed_and_never_split_mid_character() {
        let body = "é".repeat(500);
        let message = err(200, &body);
        // 40 CHARACTERS of preview, not 40 bytes — and no panic on the boundary.
        assert!(message.contains(&"é".repeat(40)), "{message}");
        assert!(!message.contains(&"é".repeat(41)), "{message}");
    }

    /// A non-success status is classified by the shared transport rules, not
    /// re-invented here — so a 429 stays the front-door refusal it is rather than
    /// becoming a generic "bad count".
    #[test]
    fn a_non_success_status_is_classified_by_transport() {
        assert!(matches!(
            read_count(429, "slow down", 40),
            Err(TransportError::Rejected { .. })
        ));
        assert!(matches!(
            read_count(500, "boom", 40),
            Err(TransportError::Status { status: 500, .. })
        ));
        assert!(matches!(
            read_count(404, "no such endpoint", 40),
            Err(TransportError::Status { status: 404, .. })
        ));
    }
}
