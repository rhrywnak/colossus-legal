//! Out-of-band operations on the Restate workflow runtime.
//!
//! This module lives alongside [`crate::pipeline::workflow`] but is
//! deliberately separate from it: `workflow.rs` is bound by the
//! `#[restate_sdk::workflow]` macro to the workflow's request/response
//! shape, while this module's helpers talk to Restate's HTTP surface
//! to control workflow invocations from outside the workflow itself.
//! Keeping the two concerns split lets these helpers be unit-tested
//! without dragging the SDK macro in.
//!
//! ## Two Restate ports, two operations
//!
//! Restate exposes two HTTP servers on distinct ports:
//!
//! - **Ingress (port 8080)** — accepts `POST` requests that invoke
//!   workflows. [`invoke_restate_workflow`] posts to
//!   `POST /DocumentPipeline/{doc_id}/run/send` to start a new
//!   workflow invocation for a document. The `/send` suffix selects
//!   the async invocation mode: Restate returns the invocation id
//!   immediately and runs the workflow in the background.
//! - **Admin (port 9070)** — exposes invocation-management endpoints.
//!   [`cancel_restate_workflow`] posts to
//!   `DELETE /invocations/DocumentPipeline/{doc_id}?mode=cancel` to
//!   stop a running invocation. Restate's exactly-once semantics mean
//!   a workflow can only be cancelled this way — the worker process
//!   serving the workflow cannot reach into the journal and stop its
//!   own execution. Restate then propagates the cancel signal to the
//!   worker (`ctx.run` closures observe it as `TerminalError`).
//!
//! Each port's URL is read from a separate `AppConfig` field
//! ([`crate::config::AppConfig::restate_ingress_url`] and
//! [`crate::config::AppConfig::restate_admin_url`]) so a deployment
//! can configure them independently.

use std::time::Duration;

use anyhow::Context;
use reqwest::StatusCode;

/// Restate timeout for cancel calls.
///
/// See the line-comment markers immediately above the declaration for
/// the formal `// CONST:` and `// DEFAULT:` justifications. The full
/// rationale follows.
///
/// This is a per-request override of the shared `state.http_client`'s
/// 90-second total-timeout, scoped specifically to the cancel call.
/// Cancel is an admin operation — Restate either returns 202
/// immediately (success) or 404 immediately (no such invocation).
/// A real failure to respond within 10s indicates the admin endpoint
/// is down or unreachable, which is operator-actionable information;
/// waiting 90s would only delay the operator's response. This is the
/// latency-budget for a fast admin call, not a knob deployments need
/// to tune.
// CONST: latency budget for a fast admin call; not env-var configurable
// because deployments do not need to tune cancel latency independently
// from the rest of the HTTP stack.
// DEFAULT: 10 seconds — override by editing this constant and rebuilding.
const RESTATE_CANCEL_TIMEOUT: Duration = Duration::from_secs(10);

/// The Restate workflow service name. Matches the `#[restate_sdk::workflow]`
/// trait name in [`crate::pipeline::workflow::DocumentPipeline`].
///
/// SYNC-WITH: `pub trait DocumentPipeline` in `pipeline/workflow.rs:182`.
/// Renaming the workflow trait without renaming this constant in lockstep
/// would silently route cancel requests at a non-existent service —
/// Restate would return 404 for every cancel and the dual-cancel handler
/// would mistake the breakage for "no invocation exists." The lockstep
/// drift is detected at test time by
/// [`tests::service_name_matches_workflow_trait_identifier`] in
/// `workflow_admin_tests.rs`, which asserts the constant string equals
/// the SDK-derived service name; that test fails if the trait is
/// renamed without updating the constant.
// CONST: state-contract identifier baked into the Restate SDK's macro
// expansion; not env-var configurable because deployments cannot rename
// the workflow service without recompiling the workflow trait too.
const DOCUMENT_PIPELINE_SERVICE: &str = "DocumentPipeline";

/// Cancel a Restate workflow invocation for the given document.
///
/// Calls `DELETE {restate_admin_url}/invocations/DocumentPipeline/{doc_id}?mode=cancel`
/// with a 10-second timeout. Restate's documented return codes:
///
/// - **202 Accepted** → the invocation was found and a cancel signal
///   was dispatched; returns `Ok(true)`.
/// - **404 Not Found** → no invocation exists for this `(service, key)`
///   tuple, either because the workflow never ran for this document or
///   because it has already reached a terminal state; returns `Ok(false)`.
/// - **Any other status** → returns `Err`, surfacing the status code
///   and the response body so an operator can diagnose whether Restate
///   itself is misbehaving (5xx) or whether we built the URL wrong
///   (4xx other than 404).
///
/// ## Best-effort semantics
///
/// Callers should treat `Ok(false)` as "no Restate work to cancel — try
/// the other path" and `Err(_)` as "Restate is configured but the admin
/// call didn't return a recognised outcome." The dual-cancel handler in
/// `api/pipeline/process.rs` uses these signals to decide whether to
/// return success, 404, or propagate an Internal error.
///
/// ## Rust Learning: `anyhow::Error` for boundary-layer errors
///
/// This module sits at the boundary between HTTP-layer details
/// (`reqwest::Error`, `StatusCode`) and the API handler that ultimately
/// returns an `AppError`. `anyhow::Error` is the right shape here: it
/// can wrap any `std::error::Error` with `.context(...)`, which is
/// exactly what we need to enrich a bare `reqwest::Error` with the URL
/// and operation that failed. The caller converts to its own typed
/// error at the boundary.
///
/// ## Rust Learning: `&reqwest::Client` parameter
///
/// `reqwest::Client` internally wraps an `Arc` over its connection
/// pool, so cloning is cheap and idiomatic. We take it by reference
/// (`&Client`) anyway because every call site already owns one via
/// `AppState.http_client` — the borrow is briefer than the clone and
/// makes the read-only intent clear at the signature.
#[tracing::instrument(skip(http_client), fields(doc_id = %doc_id))]
pub async fn cancel_restate_workflow(
    http_client: &reqwest::Client,
    restate_admin_url: &str,
    doc_id: &str,
) -> Result<bool, anyhow::Error> {
    // Trim trailing `/` so we don't build URLs like `http://host//invocations/...`.
    // Restate is lenient about doubled slashes but proxies between us and
    // it may not be.
    let base = restate_admin_url.trim_end_matches('/');
    let url = format!("{base}/invocations/{DOCUMENT_PIPELINE_SERVICE}/{doc_id}?mode=cancel");

    let response = http_client
        .delete(&url)
        // Per-request timeout override: replaces the shared client's
        // 90s default (configured in `main.rs` for RAG/synthesis calls).
        // Cancel must answer fast or fail fast — see RESTATE_CANCEL_TIMEOUT.
        .timeout(RESTATE_CANCEL_TIMEOUT)
        .send()
        .await
        .with_context(|| {
            format!(
                "Restate cancel DELETE to '{url}' failed before a response was received \
                 (network, DNS, or timeout). Check RESTATE_ADMIN_URL and that the \
                 Restate admin endpoint is reachable."
            )
        })?;

    let status = response.status();
    match status {
        StatusCode::ACCEPTED => {
            tracing::info!(
                doc_id = %doc_id,
                "Restate cancel: invocation found and cancel signal dispatched (202)"
            );
            Ok(true)
        }
        StatusCode::NOT_FOUND => {
            tracing::info!(
                doc_id = %doc_id,
                "Restate cancel: no invocation found for document (404) — \
                 either never ran on Restate or already terminal"
            );
            Ok(false)
        }
        other => {
            // Drain the body for the error message so the operator sees
            // what Restate said. We surface up to 512 chars; longer
            // responses are usually HTML error pages we can truncate.
            let body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {e}>"));
            let truncated = if body.len() > 512 {
                format!("{}…<truncated>", &body[..512])
            } else {
                body
            };
            Err(anyhow::anyhow!(
                "Restate cancel returned unexpected status {other} for '{doc_id}' \
                 (URL: {url}). Body: {truncated}"
            ))
        }
    }
}

// ── Invoke: ingress-port workflow invocation ────────────────────

/// Restate timeout for ingress invoke calls.
///
/// See the line-comment markers immediately above the declaration for
/// the formal `// CONST:` and `// DEFAULT:` justifications. The full
/// rationale follows.
///
/// Parallel to [`RESTATE_CANCEL_TIMEOUT`] — the ingress invoke is a
/// fast admin-like call (Restate returns 202 with the invocation id
/// immediately and runs the workflow in the background, per the
/// `/send` suffix). A failure to respond within 10s indicates the
/// ingress endpoint is unreachable, which the handler surfaces as a
/// 500 to the operator.
// CONST: latency budget for a fast ingress invocation; not env-var
// configurable because deployments do not need to tune invoke latency
// independently from the rest of the HTTP stack.
// DEFAULT: 10 seconds — override by editing this constant and rebuilding.
const RESTATE_INVOKE_TIMEOUT: Duration = Duration::from_secs(10);

/// Outcome of a Restate workflow invocation attempt.
///
/// Restate's ingress `/send` endpoint returns HTTP 202 in two
/// semantically distinct cases — a brand-new invocation and a replay
/// of an existing one. The HTTP status alone cannot distinguish them;
/// we have to read the JSON body's `status` field. Returning a typed
/// enum (instead of stringly-typed error messages or a `Result<String, _>`
/// that callers pattern-match on the message) lets the caller route
/// each case to the right HTTP response without fragile string
/// inspection.
#[derive(Debug, PartialEq, Eq)]
pub enum InvokeOutcome {
    /// Restate returned `"status":"Accepted"` — a new invocation was
    /// created. The `invocation_id` carries Restate's identifier for
    /// the new invocation (`inv_…`), useful for the audit log and
    /// for cross-referencing logs against the Restate admin UI.
    Accepted { invocation_id: String },
    /// Restate returned `"status":"PreviouslyAccepted"` — an invocation
    /// for this `(service, key)` tuple already exists. Under our
    /// workflow design (key = `doc_id`) this means the document has
    /// already been submitted to Restate; the caller maps this to a
    /// 409 Conflict so the operator knows they need to either delete
    /// the document and re-upload, or purge the existing Restate
    /// invocation, before retrying.
    PreviouslyAccepted { invocation_id: String },
}

/// Internal shape used to parse Restate's ingress `/send` response body.
///
/// Restate returns a JSON object with two fields: `invocationId` and
/// `status`. We parse only those two — anything else Restate adds in
/// future versions is ignored by serde.
// serde: allows unknown fields because Restate is an external service
// we do not control. A future Restate version may add new top-level
// fields to the `/send` response (telemetry, idempotency hints,
// etc.) — `deny_unknown_fields` would turn every such addition into
// a hard parse failure for us, breaking document processing on a
// dependency upgrade we did not author. Forward compatibility is
// load-bearing here.
#[derive(Debug, serde::Deserialize)]
struct RestateInvokeResponse {
    #[serde(rename = "invocationId")]
    invocation_id: String,
    status: String,
}

/// Invoke the Restate `DocumentPipeline` workflow for a document.
///
/// `POST {restate_ingress_url}/DocumentPipeline/{doc_id}/run/send`
/// with body `"{doc_id}"` (a JSON string) and a 10-second timeout.
/// The `/send` suffix selects the async invocation mode: Restate
/// returns 202 immediately with the assigned invocation id, then runs
/// the workflow in the background.
///
/// Restate's documented response codes for `/send`:
///
/// - **202 Accepted** with body `{"invocationId":"inv_...","status":"Accepted"}`
///   → returns `Ok(InvokeOutcome::Accepted { invocation_id })`.
/// - **202 Accepted** with body `{"invocationId":"inv_...","status":"PreviouslyAccepted"}`
///   → returns `Ok(InvokeOutcome::PreviouslyAccepted { invocation_id })`.
///   The keyed-workflow's invocation for this `doc_id` already exists.
/// - **Any other status** → returns `Err`, surfacing the status code
///   and the response body for operator diagnostics.
///
/// ## Why a JSON string as the body
///
/// The workflow's `run(doc_id: String)` handler is declared with
/// `String` as its argument type. Restate's ingress expects the
/// request body to be the JSON encoding of that argument — a JSON
/// string with surrounding quotes, e.g. `"doc-abc"`. We build it
/// with `serde_json::Value::String(doc_id.to_string()).to_string()`
/// rather than hand-quoting, so any character that would need
/// JSON-escaping (a doc_id containing a quote, backslash, or
/// non-ASCII) is handled correctly without ad-hoc string munging.
#[tracing::instrument(skip(http_client), fields(doc_id = %doc_id))]
pub async fn invoke_restate_workflow(
    http_client: &reqwest::Client,
    restate_ingress_url: &str,
    doc_id: &str,
) -> Result<InvokeOutcome, anyhow::Error> {
    let base = restate_ingress_url.trim_end_matches('/');
    let url = format!("{base}/{DOCUMENT_PIPELINE_SERVICE}/{doc_id}/run/send");

    // The workflow's `run` handler takes `String` — Restate wants the
    // JSON encoding of that argument, which for a plain string is
    // `"<value>"` (with quotes). Build via serde_json so any escapes
    // in doc_id are handled correctly rather than via raw `format!`.
    let body = serde_json::Value::String(doc_id.to_string()).to_string();

    let response = http_client
        .post(&url)
        .header("content-type", "application/json")
        .body(body)
        // Per-request timeout override — see RESTATE_INVOKE_TIMEOUT.
        .timeout(RESTATE_INVOKE_TIMEOUT)
        .send()
        .await
        .with_context(|| {
            format!(
                "Restate invoke POST to '{url}' failed before a response was received \
                 (network, DNS, or timeout). Check RESTATE_INGRESS_URL and that the \
                 Restate ingress endpoint is reachable."
            )
        })?;

    let status = response.status();
    if status != StatusCode::ACCEPTED {
        let body = response
            .text()
            .await
            .unwrap_or_else(|e| format!("<failed to read body: {e}>"));
        let truncated = if body.len() > 512 {
            format!("{}…<truncated>", &body[..512])
        } else {
            body
        };
        return Err(anyhow::anyhow!(
            "Restate invoke returned unexpected status {status} for '{doc_id}' \
             (URL: {url}). Body: {truncated}"
        ));
    }

    // 202 path: parse the JSON body to distinguish Accepted vs
    // PreviouslyAccepted. Both share the HTTP status code, so the
    // body is the only signal.
    let parsed: RestateInvokeResponse = response.json().await.with_context(|| {
        format!(
            "Restate invoke returned 202 for '{doc_id}' but the body did not parse \
             as the expected `{{invocationId, status}}` shape. The Restate ingress \
             contract may have changed; check the Restate version."
        )
    })?;

    match parsed.status.as_str() {
        "Accepted" => {
            tracing::info!(
                doc_id = %doc_id,
                invocation_id = %parsed.invocation_id,
                "Restate invoke: new invocation accepted (202 Accepted)"
            );
            Ok(InvokeOutcome::Accepted {
                invocation_id: parsed.invocation_id,
            })
        }
        "PreviouslyAccepted" => {
            tracing::info!(
                doc_id = %doc_id,
                invocation_id = %parsed.invocation_id,
                "Restate invoke: invocation already exists (202 PreviouslyAccepted)"
            );
            Ok(InvokeOutcome::PreviouslyAccepted {
                invocation_id: parsed.invocation_id,
            })
        }
        other => Err(anyhow::anyhow!(
            "Restate invoke returned 202 for '{doc_id}' with an unrecognised \
             status field '{other}' (expected 'Accepted' or 'PreviouslyAccepted'). \
             The Restate ingress contract may have changed."
        )),
    }
}

// Unit tests for the helpers above plus the workflow service-name
// lockstep check live in `workflow_admin_tests.rs` (kept out-of-line
// to stay under the 300-line module-size budget; matches the
// `pipeline/registry.rs` / `registry_tests.rs` and
// `workflow_steps/extract_text.rs` / `extract_text_tests.rs` idioms).
#[cfg(test)]
#[path = "workflow_admin_tests.rs"]
mod tests;
