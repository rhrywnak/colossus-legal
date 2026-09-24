//! The non-streamed POST behind [`super::ChatBackend::prewarm`]: send a cache
//! pre-warm and read back what it cost in tokens.
//!
//! ## Rust Learning: a child module sees its parent's private fields
//!
//! `AnthropicBackend`'s fields (`http`, `config`, `messages_url`) are private to
//! the `backend` module. This file is declared INSIDE that module
//! (`#[path = "backend_prewarm.rs"] mod prewarm;` in `backend.rs`), which makes
//! it a child module, and Rust's privacy rule is "visible to the module it is
//! declared in and to all of that module's descendants". So the `impl` block
//! below can reach the shared client and its settings without making them `pub`.
//! It is a separate file only to keep `backend.rs` under the size rule.

use serde_json::Value;

use super::{preview, AnthropicBackend, ChatTransportError};
use crate::transport::{self, TransportError};
use crate::usage::Usage;

/// STRUCTURAL: the response field carrying token accounting (wire shape).
const USAGE_FIELD: &str = "usage";

impl AnthropicBackend {
    /// POST a pre-warm body (built by [`crate::request::build_prewarm_body`]) and
    /// return the provider's token accounting.
    ///
    /// Like `count`, this carries a whole-request `.timeout(...)`: a pre-warm
    /// answers with one JSON object and no stream, so there are no events for an
    /// idle detector to watch. It reuses `idle_timeout` for the reason `count`
    /// gives: "fail if nothing has happened in this window" is the same promise.
    pub(super) async fn prewarm_post(&self, body: &Value) -> Result<Usage, ChatTransportError> {
        let bytes = serde_json::to_vec(body).map_err(|e| TransportError::Status {
            status: 0,
            body: format!("the pre-warm body could not be serialized: {e}"),
        })?;
        let mut request = self
            .http
            .post(&self.messages_url)
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
        let status = response.status().as_u16();
        // A body that cannot be read is reported as such, not as an empty body.
        let text = match response.text().await {
            Ok(t) => t,
            Err(e) => format!("(the pre-warm response body could not be read: {e})"),
        };
        read_prewarm(status, &text, self.config.error_preview_chars)
    }
}

/// Turn one pre-warm response into its [`Usage`], or into the named failure it is.
///
/// # Why: never a default `Usage`
///
/// A pre-warm's whole value is the read/write split in its usage. A 200 whose
/// body has no `usage` object is refused by name rather than returned as an
/// all-`None` usage, which a caller would read as "nothing happened".
///
/// # Errors
/// Whatever [`transport::classify_status`] makes of a non-success status, or
/// [`TransportError::Status`] for a success that carries no usable `usage`.
pub(super) fn read_prewarm(
    status: u16,
    text: &str,
    preview_chars: usize,
) -> Result<Usage, ChatTransportError> {
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
            "the pre-warm response carried no readable `{USAGE_FIELD}` ({detail}): {}",
            preview(text, preview_chars)
        ),
    };
    let parsed: Value = serde_json::from_str(text).map_err(|e| unreadable(&e.to_string()))?;
    let usage = parsed
        .get(USAGE_FIELD)
        .filter(|u| u.is_object())
        .ok_or_else(|| unreadable("the object is absent"))?;
    let mut out = Usage::default();
    out.absorb(usage);
    if out.cache_read_input_tokens.is_none() && out.cache_creation_input_tokens.is_none() {
        return Err(unreadable(
            "it reports neither a cache read nor a cache write",
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_of(r: Result<Usage, ChatTransportError>) -> String {
        match r {
            Err(TransportError::Status { body, .. }) => body,
            other => panic!("expected a named failure, got {other:?}"),
        }
    }

    /// The Stage P receipt, verbatim in shape: a read of the whole case file.
    #[test]
    fn a_pre_warm_reply_is_its_usage() {
        let text = r#"{"id":"msg_1","content":[],"stop_reason":"max_tokens",
            "usage":{"input_tokens":4,"cache_read_input_tokens":368832,
                     "cache_creation_input_tokens":0,"output_tokens":0}}"#;
        let u = read_prewarm(200, text, 40).unwrap();
        assert_eq!(u.cache_read_input_tokens, Some(368_832));
        assert_eq!(u.cache_creation_input_tokens, Some(0));
        assert_eq!(u.output_tokens, Some(0));
    }

    /// Three different ways a 200 is useless, three different messages.
    #[test]
    fn a_useless_success_is_named_not_zeroed() {
        let not_json = body_of(read_prewarm(200, "<html>gateway</html>", 40));
        assert!(not_json.contains("expected value"), "{not_json}");
        let no_usage = body_of(read_prewarm(200, r#"{"content":[]}"#, 40));
        assert!(no_usage.contains("the object is absent"), "{no_usage}");
        let no_cache = body_of(read_prewarm(200, r#"{"usage":{"input_tokens":9}}"#, 60));
        assert!(no_cache.contains("neither a cache read"), "{no_cache}");
    }

    /// A refusal (the provider rejecting the combination, say) keeps its text,
    /// so the log shows the provider's own sentence.
    #[test]
    fn a_refusal_carries_the_providers_text() {
        match read_prewarm(
            400,
            r#"{"error":{"message":"max_tokens: 0 not allowed"}}"#,
            200,
        ) {
            Err(TransportError::Status { status: 400, body }) => {
                assert!(body.contains("not allowed"), "{body}");
            }
            other => panic!("expected Status 400, got {other:?}"),
        }
    }
}
