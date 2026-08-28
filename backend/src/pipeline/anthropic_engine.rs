//! Streaming Anthropic adapter implementing [`ExtractionEngine`].
//!
//! ## Design requirement R4 — thin adapter
//!
//! This module is the single place in the colossus-legal backend that knows the
//! Anthropic Messages API's wire shape. Domain code talks to
//! `Arc<dyn ExtractionEngine>`; the implementation lives here.
//!
//! ## Why this file no longer speaks Rig (2026-08-28)
//!
//! It used to: `RigExtractionEngine` handed a request to `rig-core` 0.36 and
//! awaited a single response body. Two things forced the change.
//!
//! 1. **The 600s wall.** A non-streaming Messages request cannot outlive
//!    Anthropic's ~10-minute ceiling, and a 36-page transcript at
//!    `max_tokens = 64000` does. See
//!    [`crate::pipeline::anthropic_stream`] for the incident.
//! 2. **Rig's streaming path discards `stop_reason`.** `rig-core` 0.36's
//!    Anthropic streaming response type carries `usage` and nothing else; it
//!    reads `stop_reason` only as a signal to break its own loop. Adopting it
//!    would have silently disarmed the truncation gate that census R-3 exists to
//!    enforce — trading one silent-failure defect for the one we just fixed.
//!
//! So the transport is now ours: a `reqwest_13` client posting `"stream": true`
//! and an SSE accumulator that rebuilds the same message shape the pipeline
//! already consumed. Downstream parsing, grounding, and cost accounting are
//! untouched.
//!
//! ## HTTP/1.1 enforcement (unchanged, and still non-negotiable)
//!
//! Calling `api.anthropic.com` from inside a Podman container hangs
//! indefinitely when negotiating HTTP/2 over TLS — documented in
//! `colossus-extract/src/providers/anthropic.rs:30–45`. The client is built with
//! `.http1_only()`. **Do not remove it without an end-to-end container test
//! confirming the replacement does not hang.**
//!
//! The `reqwest_13` alias (reqwest 0.13 under a renamed package) predates this
//! change: it was pulled in because Rig implemented its client trait only for
//! that version. It stays because it is the client this module now builds
//! directly, and because the backend's primary client remains on 0.12 for its
//! many other consumers.
//!
//! ## Rate-limit handling — better than it was
//!
//! Rig discarded the `retry-after` header, so every 429 came back as "no value"
//! and the bridge substituted a 60s default. We hold the response ourselves now,
//! so the provider's actual `retry-after` is preserved and
//! [`ExtractionEngineError::RateLimited`] carries a real number when Anthropic
//! sends one.
//!
//! ## Environment variables
//!
//! - `ANTHROPIC_API_KEY` (required) — Anthropic API key.
//! - `ANTHROPIC_BASE_URL` (optional, default `https://api.anthropic.com`).
//! - `ANTHROPIC_API_VERSION` (optional, default `2023-06-01`) — the
//!   `anthropic-version` header.
//! - `LLM_STREAM_IDLE_TIMEOUT_SECS` (optional, default `120`) — fail the call if
//!   no server-sent event completes for this long. **Replaces**
//!   `EXTRACTION_ENGINE_TIMEOUT_SECS`, which was the 600s whole-request cap that
//!   caused the incident; a deployment that still sets the old key gets a
//!   startup warning naming this one, rather than a silently ignored setting.
//! - `EXTRACTION_ENGINE_CONNECT_TIMEOUT_SECS` (optional, default `10`) — TCP
//!   connect timeout. A streaming call cannot carry a total-duration timeout, so
//!   this is one of the two compensating controls Standing Rule 13 asks for.
//! - `EXTRACTION_ENGINE_TCP_KEEPALIVE_SECS` (optional, default `60`) — TCP
//!   keep-alive interval.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use serde_json::{json, Value};

use crate::pipeline::anthropic_transport::{self, TransportError};
use crate::pipeline::extraction_engine::{
    BatchExtractionItem, ExtractionEngine, ExtractionEngineError, LlmCallResult,
};

// ── Config keys ─────────────────────────────────────────────────
//
// These are NOT "business values" prohibited by Rule 2 — they are the canonical
// names of the environment-variable configuration keys this module reads.
// Centralising them as named constants lets the operator `grep` for the literal
// env var and find exactly one definition.

/// Anthropic API key env var.
const ANTHROPIC_API_KEY_ENV: &str = "ANTHROPIC_API_KEY";
/// Optional override for the API base URL.
const BASE_URL_ENV: &str = "ANTHROPIC_BASE_URL";
/// Optional override for the `anthropic-version` header.
const API_VERSION_ENV: &str = "ANTHROPIC_API_VERSION";
/// Optional override for the inter-event idle timeout, in seconds.
const IDLE_TIMEOUT_SECS_ENV: &str = "LLM_STREAM_IDLE_TIMEOUT_SECS";
/// Optional override for the TCP connect timeout, in seconds.
const CONNECT_TIMEOUT_SECS_ENV: &str = "EXTRACTION_ENGINE_CONNECT_TIMEOUT_SECS";
/// Optional override for the TCP keep-alive interval, in seconds.
const TCP_KEEPALIVE_SECS_ENV: &str = "EXTRACTION_ENGINE_TCP_KEEPALIVE_SECS";
/// The RETIRED whole-request timeout key. Read only so a deployment that still
/// sets it is told, loudly, that it no longer does anything — a config key that
/// silently stops mattering is the kind of quiet drift Standing Rule 1 forbids.
const RETIRED_TIMEOUT_SECS_ENV: &str = "EXTRACTION_ENGINE_TIMEOUT_SECS";

/// Default API base URL when `ANTHROPIC_BASE_URL` is unset.
///
/// CONST-with-override: the provider's canonical public endpoint, not a
/// per-deployment address (those are the Neo4j / Qdrant / Postgres hosts, which
/// have no in-code default at all). It is env-overridable so a proxy or a
/// record/replay harness can be pointed at without a rebuild.
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// Default `anthropic-version` header value.
///
/// CONST-with-override: the Messages API version this module's wire types were
/// written against. Overridable so a version bump can be trialled by config, but
/// defaulted in code because the parsing here is written for THIS version.
const DEFAULT_API_VERSION: &str = "2023-06-01";

/// Path of the Messages endpoint, appended to the base URL.
const MESSAGES_PATH: &str = "/v1/messages";

/// Default inter-event idle timeout when `LLM_STREAM_IDLE_TIMEOUT_SECS` is unset.
///
/// 120s is roughly twenty times Anthropic's `ping` cadence on a healthy stream,
/// so it cannot fire on a merely-thinking model, while still abandoning a wedged
/// connection two minutes after it wedges rather than ten.
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 120;

/// Default TCP connect timeout.
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Default TCP keep-alive interval. Carried over unchanged from the previous
/// adapter: some networks drop idle connections during long responses.
const DEFAULT_TCP_KEEPALIVE_SECS: u64 = 60;

/// Streaming Anthropic implementation of [`ExtractionEngine`].
///
/// ## Rust Learning: why no `Clone` derive here
///
/// This is intended to live behind `Arc<dyn ExtractionEngine>` — the trait
/// object is what gets cloned (cheaply), not the struct. Implementing `Clone` on
/// the struct would invite call sites to clone the engine itself instead of the
/// `Arc`, which is more expensive and obscures the intended sharing model.
pub struct AnthropicStreamingEngine {
    /// HTTP/1.1-only client, built once and reused for every call. Carries a
    /// connect timeout and TCP keep-alive but deliberately NO total-duration
    /// timeout — see [`crate::pipeline::anthropic_transport`].
    http: reqwest_13::Client,
    /// API key, sent as the `x-api-key` header.
    api_key: String,
    /// Fully-qualified Messages endpoint (base URL + [`MESSAGES_PATH`]).
    messages_url: String,
    /// Value of the `anthropic-version` header.
    api_version: String,
    /// Fail a call if no server-sent event completes within this window.
    idle_timeout: Duration,
}

impl AnthropicStreamingEngine {
    /// Construct the engine from environment variables.
    ///
    /// # Errors
    ///
    /// Returns [`ExtractionEngineError::Configuration`] when `ANTHROPIC_API_KEY`
    /// is unset, or when the underlying `reqwest_13::Client` fails to build
    /// (typically a missing system TLS provider).
    pub fn from_env() -> Result<Self, ExtractionEngineError> {
        let api_key = std::env::var(ANTHROPIC_API_KEY_ENV).map_err(|_| {
            ExtractionEngineError::Configuration(format!(
                "{ANTHROPIC_API_KEY_ENV} is unset — required to construct \
                 AnthropicStreamingEngine"
            ))
        })?;

        if let Ok(raw) = std::env::var(RETIRED_TIMEOUT_SECS_ENV) {
            tracing::warn!(
                retired_env_var = RETIRED_TIMEOUT_SECS_ENV,
                raw_value = %raw,
                replacement_env_var = IDLE_TIMEOUT_SECS_ENV,
                "{RETIRED_TIMEOUT_SECS_ENV} is set but NO LONGER APPLIED — the extraction \
                 transport streams, and a whole-request timeout is what cut off a healthy \
                 64000-token response at 600s on 2026-08-28. Remove the key and set \
                 {IDLE_TIMEOUT_SECS_ENV} instead if you need to tune the stall detector."
            );
        }

        let base_url = read_string_env(BASE_URL_ENV, DEFAULT_BASE_URL);
        let api_version = read_string_env(API_VERSION_ENV, DEFAULT_API_VERSION);
        let idle_secs = read_secs_env(IDLE_TIMEOUT_SECS_ENV, DEFAULT_IDLE_TIMEOUT_SECS);
        let connect_secs = read_secs_env(CONNECT_TIMEOUT_SECS_ENV, DEFAULT_CONNECT_TIMEOUT_SECS);
        let keepalive_secs = read_secs_env(TCP_KEEPALIVE_SECS_ENV, DEFAULT_TCP_KEEPALIVE_SECS);

        // NOTE the absent `.timeout(...)`. It is absent on purpose and its
        // absence is the fix; see the module doc and `anthropic_transport`.
        let http = reqwest_13::Client::builder()
            .connect_timeout(Duration::from_secs(connect_secs))
            .tcp_keepalive(Duration::from_secs(keepalive_secs))
            .http1_only()
            .build()
            .map_err(|e| {
                ExtractionEngineError::Configuration(format!(
                    "Failed to build reqwest 0.13 client for the Anthropic adapter: {e}"
                ))
            })?;

        tracing::info!(
            base_url = %base_url,
            api_version = %api_version,
            idle_timeout_secs = idle_secs,
            connect_timeout_secs = connect_secs,
            "Anthropic extraction engine configured for STREAMING transport \
             (no whole-request timeout; stall detection is inter-event)"
        );

        Ok(Self {
            http,
            api_key,
            messages_url: format!("{}{MESSAGES_PATH}", base_url.trim_end_matches('/')),
            api_version,
            idle_timeout: Duration::from_secs(idle_secs),
        })
    }
}

/// Read a string env var, falling back to `default` when unset or empty.
fn read_string_env(name: &str, default: &str) -> String {
    match std::env::var(name) {
        Ok(raw) if !raw.trim().is_empty() => raw.trim().to_string(),
        // An empty value is treated as unset rather than as a request for an
        // empty base URL, which could not produce a valid request anyway.
        _ => default.to_string(),
    }
}

/// Read an env var as `u64` seconds, falling back to `default` on absence or
/// parse failure.
///
/// Mirrors the silent-fallback convention already used by
/// `AppContext::from_deps_and_env` for `PIPELINE_LLM_CONCURRENCY` — established
/// house style. A present-but-unparseable value emits a `tracing::warn!`, so the
/// failure stays observable per Rule 1 even though the engine still starts.
fn read_secs_env(name: &str, default: u64) -> u64 {
    let Ok(raw) = std::env::var(name) else {
        return default;
    };
    match raw.parse::<u64>() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                env_var = name,
                raw_value = %raw,
                error = %e,
                default = default,
                "Invalid duration env var — falling back to default"
            );
            default
        }
    }
}

/// Build the JSON request body for one streaming Messages call.
///
/// Pure, so the exact wire shape — including the `"stream": true` that is the
/// point of this whole change — is assertable without a socket.
///
/// `temperature` is omitted entirely when `None`: Claude Opus 4.7 and later
/// reject the key outright rather than ignoring it, which is why the resolved
/// parameter is an `Option` all the way down from `domain::llm_params`.
pub fn build_request_body(
    system_prompt: Option<&str>,
    user_prompt: &str,
    model: &str,
    max_tokens: u32,
    temperature: Option<f64>,
) -> Value {
    let mut body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "stream": true,
        "messages": [{ "role": "user", "content": user_prompt }],
    });

    // ## Rust Learning: mutating a `serde_json::Value` in place
    //
    // `as_object_mut()` borrows the underlying map so optional keys can be
    // inserted conditionally. The alternative — building several complete
    // literals behind `if`s — duplicates the required keys once per branch, and
    // duplicated literals are how two shapes quietly come to differ.
    if let Some(map) = body.as_object_mut() {
        if let Some(system) = system_prompt {
            map.insert("system".to_string(), Value::String(system.to_string()));
        }
        if let Some(temp) = temperature {
            // A non-finite temperature cannot be represented in JSON; skipping
            // it is the only alternative to emitting `null`, which the API
            // rejects. It cannot arise from `domain::llm_params`, so this arm
            // exists to make the impossible case explicit rather than silent.
            match serde_json::Number::from_f64(temp) {
                Some(n) => {
                    map.insert("temperature".to_string(), Value::Number(n));
                }
                None => tracing::error!(
                    temperature = temp,
                    model,
                    "Non-finite temperature could not be encoded as JSON — the key is \
                     OMITTED from this request and the model will use its own default"
                ),
            }
        }
    }

    body
}

/// Turn a non-success response into a [`TransportError`], consuming its body.
///
/// Separate from `extract` so the status/header reads and the body drain sit in
/// one place, and so the happy path reads as the straight line it is.
async fn transport_error_from_response(response: reqwest_13::Response) -> TransportError {
    let status = response.status().as_u16();
    let retry_after = response
        .headers()
        .get(anthropic_transport::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    // The body is read for the error message only. A read failure here must not
    // mask the status we already know, so it degrades to a named placeholder
    // rather than replacing the error (Standing Rule 1: the status survives).
    let text = response
        .text()
        .await
        .unwrap_or_else(|e| format!("<response body unreadable: {e}>"));
    anthropic_transport::classify_status(status, retry_after.as_deref(), &text)
}

/// Map a [`TransportError`] into the engine's error taxonomy.
///
/// Only 429 gets its own variant: the orchestrator's rate-limit wrapper is the
/// one caller that acts differently on a specific failure kind. Everything else
/// — stalls, non-2xx, malformed or incomplete streams — is `LlmCallFailed`, and
/// the step classifier above decides terminal-vs-retryable from the retry
/// policy rather than from the failure's shape.
fn map_transport_error(err: TransportError, model: &str) -> ExtractionEngineError {
    match err {
        TransportError::RateLimited { retry_after_secs } => ExtractionEngineError::RateLimited {
            model: model.to_string(),
            retry_after_secs,
        },
        other => ExtractionEngineError::LlmCallFailed {
            model: model.to_string(),
            source: Box::new(other),
        },
    }
}

#[async_trait]
impl ExtractionEngine for AnthropicStreamingEngine {
    async fn extract(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        model: &str,
        max_tokens: u32,
        temperature: Option<f64>,
    ) -> Result<LlmCallResult, ExtractionEngineError> {
        let start = Instant::now();
        let body = build_request_body(system_prompt, user_prompt, model, max_tokens, temperature);

        let response = self
            .http
            .post(&self.messages_url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.api_version)
            .header("content-type", "application/json")
            .header("accept", "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|source| map_transport_error(TransportError::Http { source }, model))?;

        if !response.status().is_success() {
            return Err(map_transport_error(
                transport_error_from_response(response).await,
                model,
            ));
        }

        let mut chunks = anthropic_transport::ResponseChunks::new(response);
        let message = anthropic_transport::drive(&mut chunks, self.idle_timeout)
            .await
            .map_err(|e| map_transport_error(e, model))?;

        if message.text.is_empty() {
            // A response with no text blocks is a failed extraction — Rule 1
            // says distinct states need distinct observables. Returning
            // `Ok(LlmCallResult { response_text: "", … })` would let downstream
            // code silently produce zero entities and mark the document
            // "complete with no extractions", which is indistinguishable from a
            // successful empty page.
            let source_msg: String = format!(
                "model {model} returned no text content \
                 (response contained only tool_use / reasoning / image blocks)"
            );
            return Err(ExtractionEngineError::LlmCallFailed {
                model: model.to_string(),
                source: source_msg.into(),
            });
        }

        Ok(LlmCallResult {
            response_text: message.text,
            input_tokens: message.input_tokens,
            output_tokens: message.output_tokens,
            // Carried out of `message_delta`, which is where the streaming
            // protocol puts it. This is the field that says whether the answer
            // above is the whole answer — see `pipeline::truncation`.
            stop_reason: Some(message.stop_reason),
            request_id: message.message_id,
            duration: start.elapsed(),
        })
    }

    async fn extract_batch(
        &self,
        items: &[BatchExtractionItem],
        concurrency: usize,
    ) -> Vec<Result<LlmCallResult, ExtractionEngineError>> {
        // `concurrency = 0` would make `buffer_unordered(0)` yield nothing (no
        // slots open, no progress); clamp to at least 1 so a misconfigured
        // caller still gets serial behaviour rather than a silent hang. Rule 1:
        // a misconfigured value must not produce zero work.
        let concurrency = concurrency.max(1);

        // ## Rust Learning: tagging futures to preserve input order
        //
        // `buffer_unordered(n)` runs up to `n` futures concurrently and yields
        // each result as soon as it completes — completion order, not input
        // order. The trait contract promises input-ordered output, so we attach
        // the input index to each future's return value, collect the tagged
        // tuples, then sort by index before stripping the tag.
        //
        // ## Rust Learning: capturing by index, not by `&T`
        //
        // The natural-looking `items.iter().enumerate().map(|(idx, item)| async
        // move { … })` trips the `FnOnce` HRTB inference: `slice::Iter::Item` is
        // `&'a BatchExtractionItem` for a SPECIFIC `'a`, but the async-block
        // closure is asked to satisfy `FnOnce` for ANY pair of lifetimes.
        // Iterating `0..items.len()` and capturing an owned `usize` moves the
        // borrow inside the async block, where its lifetime is tied to the
        // future `extract_batch` already owns.
        let futures = (0..items.len()).map(|idx| async move {
            let item = &items[idx];
            let result = self
                .extract(
                    item.system_prompt.as_deref(),
                    &item.user_prompt,
                    &item.model,
                    item.max_tokens,
                    item.temperature,
                )
                .await;
            (idx, result)
        });

        let mut indexed: Vec<(usize, Result<LlmCallResult, ExtractionEngineError>)> =
            stream::iter(futures)
                .buffer_unordered(concurrency)
                .collect()
                .await;
        indexed.sort_by_key(|(idx, _)| *idx);
        indexed.into_iter().map(|(_, r)| r).collect()
    }
}

#[cfg(test)]
#[path = "anthropic_engine_tests.rs"]
mod tests;
