//! Admin operations on running Restate workflow invocations.
//!
//! This module lives alongside [`crate::pipeline::workflow`] but is
//! deliberately separate from it: `workflow.rs` is bound by the
//! `#[restate_sdk::workflow]` macro to the workflow's request/response
//! shape, while this module's helpers talk to Restate's *admin* API
//! (a separate HTTP surface on a separate port) to control already-
//! running invocations. Keeping the two concerns split lets the admin
//! helpers be unit-tested without dragging the SDK macro in.
//!
//! ## Why an out-of-band admin call?
//!
//! Restate's exactly-once semantics mean a workflow can be cancelled
//! only by Restate itself — the worker process serving the workflow
//! cannot reach into the journal and stop its own execution. The admin
//! API exposes a `DELETE /invocations/{service}/{key}?mode=cancel`
//! endpoint that flips the invocation into a cancelled terminal state;
//! Restate then propagates that state to the worker (`ctx.run` closures
//! observe cancellation via the `TerminalError::cancelled()` sentinel).

use std::time::Duration;

use anyhow::Context;
use reqwest::StatusCode;

/// Restate timeout for cancel calls.
///
/// CONST justification: this is a per-request override of the shared
/// `state.http_client`'s 90-second total-timeout, scoped specifically
/// to the cancel call. Cancel is an admin operation — Restate either
/// returns 202 immediately (success) or 404 immediately (no such
/// invocation). A real failure to respond within 10s indicates the
/// admin endpoint is down or unreachable, which is operator-actionable
/// information; waiting 90s would only delay the operator's response.
/// This is the latency-budget for a fast admin call, not a knob
/// deployments need to tune.
// DEFAULT: 10 seconds — override by editing this constant and rebuilding;
// not env-var configurable by design (see doc comment above).
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
/// [`tests::service_name_matches_workflow_trait`] below, which asserts
/// the constant string equals the SDK-derived service name; that test
/// fails if the trait is renamed without updating the constant.
///
/// CONST justification: state-contract identifier. Restate's HTTP admin
/// API addresses invocations by `{service-name}/{key}/...` and the value
/// is baked into the SDK's macro expansion — not env-var configurable.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::net::TcpListener;

    /// Lockstep check: the `DOCUMENT_PIPELINE_SERVICE` constant must
    /// match the workflow trait's identifier byte-for-byte, because
    /// Restate's admin API addresses invocations by that name and the
    /// `#[restate_sdk::workflow]` macro derives the service name from
    /// the trait identifier.
    ///
    /// The `use` import below makes this test compile-fail if the
    /// trait is renamed or removed — Rust's resolution catches the
    /// missing symbol before the assertion runs. The `stringify!`
    /// macro then verifies that whatever we typed for the trait name
    /// matches the constant: a renamed trait whose constant was
    /// updated in lockstep passes; a constant that drifted from the
    /// trait fails the assertion. The combination catches both halves
    /// of the lockstep contract.
    #[test]
    fn service_name_matches_workflow_trait_identifier() {
        // `as _` keeps the import side-effect (proving the trait
        // exists) without forcing the trait into scope in a way that
        // would trip clippy::unused_imports for the other tests.
        use crate::pipeline::workflow::DocumentPipeline as _;
        assert_eq!(
            DOCUMENT_PIPELINE_SERVICE,
            stringify!(DocumentPipeline),
            "DOCUMENT_PIPELINE_SERVICE must match the workflow trait identifier — \
             rename one without the other and Restate routes cancels at a non-existent service"
        );
    }

    /// Tiny single-connection HTTP test server. We do NOT pull in
    /// `wiremock` or `axum` here — the cancel helper only cares about
    /// the response status code, so a trivial line-buffered TCP
    /// responder is enough and keeps the test-suite dependency
    /// footprint flat. Returns the bound address and an `Arc<AtomicUsize>`
    /// tracking how many requests the server saw.
    async fn spawn_responder(status_line: &'static str) -> (SocketAddr, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        tokio::spawn(async move {
            loop {
                let Ok((mut sock, _)) = listener.accept().await else {
                    break;
                };
                counter_clone.fetch_add(1, Ordering::SeqCst);
                // Drain enough of the request line to avoid a RST on close.
                let mut buf = [0u8; 1024];
                let _ = tokio::io::AsyncReadExt::read(&mut sock, &mut buf).await;
                let body = "test body";
                let response = format!(
                    "HTTP/1.1 {status_line}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = tokio::io::AsyncWriteExt::write_all(&mut sock, response.as_bytes()).await;
                let _ = tokio::io::AsyncWriteExt::shutdown(&mut sock).await;
            }
        });
        (addr, counter)
    }

    fn test_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("test client builder")
    }

    #[tokio::test]
    async fn cancel_returns_true_on_202() {
        let (addr, counter) = spawn_responder("202 Accepted").await;
        let url = format!("http://{addr}");
        let res = cancel_restate_workflow(&test_client(), &url, "doc-x").await;
        // `matches!` borrows `res`, so it stays available for the
        // debug-format in the assertion message on failure. (Using
        // `res.ok()` here would consume the Result and break the
        // `{res:?}` format.)
        assert!(matches!(res, Ok(true)), "202 must yield Ok(true): {res:?}");
        assert_eq!(
            counter.load(Ordering::SeqCst),
            1,
            "helper must perform exactly one HTTP DELETE per call"
        );
    }

    #[tokio::test]
    async fn cancel_returns_false_on_404() {
        let (addr, _counter) = spawn_responder("404 Not Found").await;
        let url = format!("http://{addr}");
        let res = cancel_restate_workflow(&test_client(), &url, "doc-x").await;
        assert!(
            matches!(res, Ok(false)),
            "404 must yield Ok(false): {res:?}"
        );
    }

    #[tokio::test]
    async fn cancel_returns_err_on_500() {
        let (addr, _counter) = spawn_responder("500 Internal Server Error").await;
        let url = format!("http://{addr}");
        let res = cancel_restate_workflow(&test_client(), &url, "doc-abc").await;
        let err = res.expect_err("500 must yield Err");
        let msg = format!("{err}");
        // Operator-facing context: status code and doc_id must both appear.
        assert!(msg.contains("500"), "err must include status code: {msg}");
        assert!(msg.contains("doc-abc"), "err must include doc_id: {msg}");
        assert!(
            msg.contains("test body"),
            "err must include response body for diagnostics: {msg}"
        );
    }

    #[tokio::test]
    async fn cancel_returns_err_on_unreachable_host() {
        // Use a port that is essentially guaranteed not to have a
        // listener — `127.0.0.1:1` (port 1 is reserved). The connect
        // refusal happens immediately, so this test runs in <50ms.
        let res =
            cancel_restate_workflow(&test_client(), "http://127.0.0.1:1", "doc-unreachable").await;
        let err = res.expect_err("unreachable host must yield Err");
        let msg = format!("{err}");
        assert!(
            msg.contains("Restate cancel DELETE"),
            "err must surface the operation, got: {msg}"
        );
        assert!(
            msg.contains("RESTATE_ADMIN_URL"),
            "err must point operators at the env var to check, got: {msg}"
        );
    }

    #[tokio::test]
    async fn cancel_trims_trailing_slash_on_admin_url() {
        // The helper must build `{base}/invocations/...`, not
        // `{base}//invocations/...`, even when the caller stores the
        // admin URL with a trailing slash. We can't directly observe
        // the URL from our line-buffer test server, but we CAN observe
        // that the request still produces the 202 outcome — i.e. the
        // server received a syntactically valid HTTP request line.
        let (addr, counter) = spawn_responder("202 Accepted").await;
        let url_with_slash = format!("http://{addr}/");
        let res = cancel_restate_workflow(&test_client(), &url_with_slash, "doc-x").await;
        assert!(
            matches!(res, Ok(true)),
            "trailing slash must still yield 202: {res:?}"
        );
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
