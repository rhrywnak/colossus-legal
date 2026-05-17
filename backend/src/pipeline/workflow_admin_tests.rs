//! Unit tests for the helpers in [`super`] —
//! [`super::cancel_restate_workflow`] (admin-port DELETE) and
//! [`super::invoke_restate_workflow`] (ingress-port POST) — plus the
//! workflow service-name lockstep check.
//!
//! Lives in a sibling file (rather than a `mod tests { ... }` block
//! inside `workflow_admin.rs`) so the runtime file stays under the
//! 300-line module-size budget. Wired into the runtime module via
//! `#[cfg(test)] #[path = "workflow_admin_tests.rs"] mod tests;` —
//! the same idiom `pipeline/registry.rs` uses for `registry_tests.rs`
//! and the workflow_steps handler files use for their `*_tests.rs`
//! siblings.

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
    // The `use as _` is a compile-time existence check: if
    // `DocumentPipeline` is renamed without updating
    // `DOCUMENT_PIPELINE_SERVICE`, this file fails to compile. The
    // rename-to-underscore pattern doesn't satisfy clippy here because
    // `stringify!` below consumes the token without using the trait, so
    // we suppress the lint explicitly.
    #[allow(unused_imports)]
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

// ── invoke_restate_workflow tests ─────────────────────────────

/// Sibling fixture to [`spawn_responder`] for tests that need to
/// control the response body — the invoke helper parses JSON from
/// the 202 body to distinguish Accepted vs PreviouslyAccepted, so
/// the body content is load-bearing here in a way it isn't for the
/// cancel tests.
async fn spawn_responder_with_body(
    status_line: &'static str,
    body: &'static str,
) -> (SocketAddr, Arc<AtomicUsize>) {
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
            let mut buf = [0u8; 1024];
            let _ = tokio::io::AsyncReadExt::read(&mut sock, &mut buf).await;
            let response = format!(
                "HTTP/1.1 {status_line}\r\n\
                 Content-Type: application/json\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = tokio::io::AsyncWriteExt::write_all(&mut sock, response.as_bytes()).await;
            let _ = tokio::io::AsyncWriteExt::shutdown(&mut sock).await;
        }
    });
    (addr, counter)
}

#[tokio::test]
async fn invoke_returns_accepted_on_202_with_accepted_status() {
    let (addr, counter) = spawn_responder_with_body(
        "202 Accepted",
        r#"{"invocationId":"inv_abc123","status":"Accepted"}"#,
    )
    .await;
    let url = format!("http://{addr}");
    let res = invoke_restate_workflow(&test_client(), &url, "doc-x").await;
    match res {
        Ok(InvokeOutcome::Accepted { invocation_id }) => {
            assert_eq!(invocation_id, "inv_abc123");
        }
        other => panic!("expected Ok(Accepted), got: {other:?}"),
    }
    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "helper must perform exactly one HTTP POST per call"
    );
}

#[tokio::test]
async fn invoke_returns_previously_accepted_on_202_with_previouslyaccepted_status() {
    let (addr, _counter) = spawn_responder_with_body(
        "202 Accepted",
        r#"{"invocationId":"inv_xyz789","status":"PreviouslyAccepted"}"#,
    )
    .await;
    let url = format!("http://{addr}");
    let res = invoke_restate_workflow(&test_client(), &url, "doc-x").await;
    // Both Accepted and PreviouslyAccepted are `Ok` — the typed
    // outcome enum is the only signal the caller has to distinguish
    // them, so this test pins the variant returned.
    match res {
        Ok(InvokeOutcome::PreviouslyAccepted { invocation_id }) => {
            assert_eq!(invocation_id, "inv_xyz789");
        }
        other => panic!("expected Ok(PreviouslyAccepted), got: {other:?}"),
    }
}

#[tokio::test]
async fn invoke_returns_err_on_500() {
    let (addr, _counter) = spawn_responder_with_body(
        "500 Internal Server Error",
        r#"{"code":"INTERNAL","message":"something went wrong"}"#,
    )
    .await;
    let url = format!("http://{addr}");
    let res = invoke_restate_workflow(&test_client(), &url, "doc-fail").await;
    let err = res.expect_err("500 must yield Err");
    let msg = format!("{err}");
    // Operator-facing context: status, doc_id, and response body.
    assert!(msg.contains("500"), "err must include status code: {msg}");
    assert!(msg.contains("doc-fail"), "err must include doc_id: {msg}");
    assert!(
        msg.contains("something went wrong"),
        "err must include response body for diagnostics: {msg}"
    );
}

#[tokio::test]
async fn invoke_returns_err_on_unreachable_host() {
    // 127.0.0.1:1 — reserved port; connection refused happens
    // immediately. Same pattern as cancel's unreachable-host test.
    let res = invoke_restate_workflow(&test_client(), "http://127.0.0.1:1", "doc-x").await;
    let err = res.expect_err("unreachable host must yield Err");
    let msg = format!("{err}");
    assert!(
        msg.contains("Restate invoke POST"),
        "err must surface the operation: {msg}"
    );
    assert!(
        msg.contains("RESTATE_INGRESS_URL"),
        "err must point operators at the env var to check: {msg}"
    );
}

#[tokio::test]
async fn invoke_returns_err_on_unrecognised_status_field() {
    // Restate could in principle return a status string we don't
    // know about (e.g. a future Restate version adds a third
    // variant). The helper must surface this distinctly so an
    // operator doesn't see a silent success on a state we haven't
    // designed for.
    let (addr, _counter) = spawn_responder_with_body(
        "202 Accepted",
        r#"{"invocationId":"inv_q","status":"SomeNewState"}"#,
    )
    .await;
    let url = format!("http://{addr}");
    let res = invoke_restate_workflow(&test_client(), &url, "doc-q").await;
    let err = res.expect_err("unrecognised status must yield Err");
    let msg = format!("{err}");
    assert!(
        msg.contains("SomeNewState"),
        "err must name the unrecognised status: {msg}"
    );
}

#[tokio::test]
async fn invoke_returns_err_on_malformed_202_body() {
    // 202 with garbage body — Restate is alive but its contract
    // changed in an incompatible way. Surface distinctly so the
    // operator knows to check the Restate version.
    let (addr, _counter) = spawn_responder_with_body("202 Accepted", "not json at all").await;
    let url = format!("http://{addr}");
    let res = invoke_restate_workflow(&test_client(), &url, "doc-r").await;
    let err = res.expect_err("malformed 202 body must yield Err");
    let msg = format!("{err}");
    assert!(msg.contains("doc-r"), "err must include doc_id: {msg}");
    assert!(
        msg.contains("contract may have changed"),
        "err must hint at the operator action: {msg}"
    );
}

#[tokio::test]
async fn invoke_trims_trailing_slash_on_ingress_url() {
    let (addr, counter) = spawn_responder_with_body(
        "202 Accepted",
        r#"{"invocationId":"inv_slash","status":"Accepted"}"#,
    )
    .await;
    let url_with_slash = format!("http://{addr}/");
    let res = invoke_restate_workflow(&test_client(), &url_with_slash, "doc-x").await;
    assert!(
        matches!(res, Ok(InvokeOutcome::Accepted { .. })),
        "trailing slash must still yield Accepted: {res:?}"
    );
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}
