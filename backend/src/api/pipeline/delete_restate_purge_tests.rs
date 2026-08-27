//! Unit tests for [`super`] — the Restate purge/kill escalation.
//!
//! Lives in a sibling file (rather than a `mod tests { ... }` block inside
//! `delete_restate_purge.rs`) so the runtime module stays under the 300-line
//! budget of Rule 17. Wired in via `#[cfg(test)] #[path = ...] mod tests;` —
//! the same idiom `pipeline/truncation.rs` uses for `truncation_tests.rs` and
//! `pipeline/workflow_admin.rs` uses for `workflow_admin_tests.rs`.

use super::*;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::net::TcpListener;

/// A responder that answers by URL PATH, so one server plays both the `/kill`
/// and `/purge` endpoints of a single escalation sequence.
///
/// `kill_status` is the answer to the one kill call. `purge_statuses` is
/// consumed IN ORDER by successive purge calls, with the last entry repeating
/// once the list is exhausted — so `["409 Conflict", "200 OK"]` reproduces the
/// real shape (still running, then purgeable after the kill), and a single-entry
/// list pins one answer for every attempt.
///
/// Driving the two endpoints independently is what lets the failure paths be
/// tested at all: a responder that always answers the kill with 200 can never
/// reach the kill-failed branch, and one that only ever moves 409 → 200 can
/// never reach the post-kill 404 or 500 branches.
///
/// The returned counters let a test assert the kill was actually issued, rather
/// than inferring it from the final outcome alone.
async fn spawn_kill_purge_responder(
    kill_status: &'static str,
    purge_statuses: Vec<&'static str>,
) -> (SocketAddr, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    assert!(
        !purge_statuses.is_empty(),
        "a responder needs at least one purge status to serve"
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let kills = Arc::new(AtomicUsize::new(0));
    let purges = Arc::new(AtomicUsize::new(0));
    let (k, p) = (kills.clone(), purges.clone());

    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let mut buf = [0u8; 2048];
            let n = tokio::io::AsyncReadExt::read(&mut sock, &mut buf)
                .await
                .unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();

            let status = if req.contains("/kill") {
                k.fetch_add(1, Ordering::SeqCst);
                kill_status
            } else if req.contains("/purge") {
                let seen = p.fetch_add(1, Ordering::SeqCst);
                // Saturate on the last entry so an unbounded retry loop keeps
                // getting the terminal answer the test asked for.
                purge_statuses[seen.min(purge_statuses.len() - 1)]
            } else {
                "404 Not Found"
            };

            let body = "test body";
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = tokio::io::AsyncWriteExt::write_all(&mut sock, response.as_bytes()).await;
            let _ = tokio::io::AsyncWriteExt::shutdown(&mut sock).await;
        }
    });
    (addr, kills, purges)
}

/// The shipped policy. Tests assert against the real defaults rather than a
/// tuned-down fixture, so a change to those defaults surfaces here.
fn test_policy() -> RestatePurgePolicy {
    RestatePurgePolicy::default()
}

fn purge_test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("test client builder")
}

#[tokio::test]
async fn a_running_invocation_is_killed_and_then_purged() {
    // THE regression test for the 2026-08-27 orphan. Restate says "not yet
    // completed" to the first purge; the handler must escalate to a kill
    // and purge again rather than giving up and leaving the workflow key
    // held forever.
    let (addr, kills, purges) =
        spawn_kill_purge_responder("200 OK", vec!["409 Conflict", "200 OK"]).await;
    let url = format!("http://{addr}");

    let outcome = attempt_restate_purge(
        &purge_test_client(),
        Some(&url),
        "doc-phillips-motion-summary-disposition-07-10-2014",
        Some("inv_14eUEIupiTWm1TXTZ2FOm0OZ7SPmlq7HUV"),
        &test_policy(),
    )
    .await;

    assert_eq!(
        outcome,
        PurgeOutcome::PurgedAfterKill,
        "a still-running invocation must be killed and purged, not abandoned"
    );
    assert_eq!(
        kills.load(Ordering::SeqCst),
        1,
        "exactly one kill must be issued"
    );
    assert_eq!(
        purges.load(Ordering::SeqCst),
        2,
        "purge is attempted once before the kill and once after"
    );
}

#[tokio::test]
async fn an_already_terminal_invocation_is_purged_without_a_kill() {
    // The common path must not pay for the escalation: when the workflow
    // has already finished, one purge call settles it and NOTHING is
    // killed. A kill here would be a hard-stop issued against an
    // invocation that had already completed.
    let (addr, kills, purges) = spawn_kill_purge_responder("200 OK", vec!["200 OK"]).await;
    let url = format!("http://{addr}");

    let outcome = attempt_restate_purge(
        &purge_test_client(),
        Some(&url),
        "doc-x",
        Some("inv_done"),
        &test_policy(),
    )
    .await;

    assert_eq!(outcome, PurgeOutcome::Success);
    assert_eq!(
        kills.load(Ordering::SeqCst),
        0,
        "a terminal invocation must never be killed"
    );
    assert_eq!(purges.load(Ordering::SeqCst), 1, "one purge call only");
}

#[tokio::test]
async fn a_kill_that_never_reaches_terminal_reports_the_manual_remedy() {
    // Budget exhausted: every purge attempt still says 409. The outcome
    // must be an Error that NAMES the invocation id and the exact call an
    // operator has to make — silently returning a success-shaped outcome
    // here would hide a still-blocked workflow key.
    let (addr, kills, purges) = spawn_kill_purge_responder("200 OK", vec!["409 Conflict"]).await;
    let url = format!("http://{addr}");

    let outcome = attempt_restate_purge(
        &purge_test_client(),
        Some(&url),
        "doc-y",
        Some("inv_stuck"),
        &test_policy(),
    )
    .await;

    let PurgeOutcome::Error(msg) = &outcome else {
        panic!("exhausted retries must report an Error outcome, got {outcome:?}");
    };
    assert!(msg.contains("inv_stuck"), "must name the invocation: {msg}");
    assert!(
        msg.contains("/purge"),
        "must give the operator the remedy call: {msg}"
    );
    assert_eq!(kills.load(Ordering::SeqCst), 1);
    assert_eq!(
        purges.load(Ordering::SeqCst),
        1 + test_policy().retry_attempts as usize,
        "one purge before the kill, then the full bounded retry budget"
    );
}

#[tokio::test]
async fn a_kill_that_itself_fails_is_reported_as_a_kill_failure() {
    // If the kill call errors, the escalation cannot proceed and the workflow
    // key stays held. That must surface as an Error naming the kill — not as a
    // silent success, and not mislabelled as a purge failure, because the two
    // send an operator to different places.
    let (addr, kills, purges) =
        spawn_kill_purge_responder("500 Internal Server Error", vec!["409 Conflict"]).await;
    let url = format!("http://{addr}");

    let outcome = attempt_restate_purge(
        &purge_test_client(),
        Some(&url),
        "doc-z",
        Some("inv_killfail"),
        &test_policy(),
    )
    .await;

    let PurgeOutcome::Error(msg) = &outcome else {
        panic!("a failed kill must report an Error outcome, got {outcome:?}");
    };
    assert!(
        msg.starts_with("kill failed:"),
        "the message must name the step that failed: {msg}"
    );
    assert!(msg.contains("500"), "must carry Restate's status: {msg}");
    assert_eq!(
        kills.load(Ordering::SeqCst),
        1,
        "the kill was attempted once"
    );
    assert_eq!(
        purges.load(Ordering::SeqCst),
        1,
        "only the initial purge runs — a failed kill must not start the retry loop"
    );
}

#[tokio::test]
async fn a_kill_that_removes_the_invocation_outright_counts_as_purged() {
    // Restate may drop the journal as part of the kill, so the follow-up purge
    // sees 404. The key is free, which is the whole objective — this must be
    // reported as PurgedAfterKill, not as an error and not as a bare NotFound.
    let (addr, kills, purges) =
        spawn_kill_purge_responder("200 OK", vec!["409 Conflict", "404 Not Found"]).await;
    let url = format!("http://{addr}");

    let outcome = attempt_restate_purge(
        &purge_test_client(),
        Some(&url),
        "doc-w",
        Some("inv_gone"),
        &test_policy(),
    )
    .await;

    assert_eq!(
        outcome,
        PurgeOutcome::PurgedAfterKill,
        "a 404 after the kill means the journal is gone — the key is free"
    );
    assert_eq!(kills.load(Ordering::SeqCst), 1);
    assert_eq!(purges.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_post_kill_purge_error_is_reported_as_such() {
    // The kill succeeds but the follow-up purge returns an unexpected status.
    // Distinct from a kill failure and from an exhausted budget: here Restate
    // answered something we do not understand, and the message must say which
    // step produced it.
    let (addr, kills, purges) =
        spawn_kill_purge_responder("200 OK", vec!["409 Conflict", "500 Internal Server Error"])
            .await;
    let url = format!("http://{addr}");

    let outcome = attempt_restate_purge(
        &purge_test_client(),
        Some(&url),
        "doc-v",
        Some("inv_bad"),
        &test_policy(),
    )
    .await;

    let PurgeOutcome::Error(msg) = &outcome else {
        panic!("an unexpected post-kill status must report an Error, got {outcome:?}");
    };
    assert!(
        msg.starts_with("purge after kill failed"),
        "the message must name the step that failed: {msg}"
    );
    assert!(msg.contains("inv_bad"), "must name the invocation: {msg}");
    // The stored string is what an operator reads in `document_audit_log`, so
    // it must carry the remedy and say which attempt gave up — not just that
    // something failed.
    assert!(
        msg.contains("attempt 1 of 4"),
        "must say which attempt failed, out of how many: {msg}"
    );
    assert!(
        msg.contains("/invocations/inv_bad/purge"),
        "must give the operator the manual remedy call: {msg}"
    );
    assert_eq!(kills.load(Ordering::SeqCst), 1);
    assert_eq!(
        purges.load(Ordering::SeqCst),
        2,
        "the loop must stop at the first unexpected status, not burn the budget"
    );
}

#[test]
fn purge_outcome_as_str_matches_documented_wire_shape() {
    // These string values are recorded in
    // `document_audit_log.snapshot->'restate'->>'purge_outcome'`
    // and would be queried by an operator dashboard or by an
    // audit script. The migration's doc comment and any future
    // consumer rely on these exact strings — pin them in a test
    // so a careless rename of a variant doesn't silently break
    // the wire shape.
    assert_eq!(PurgeOutcome::Success.as_str(), "success");
    assert_eq!(PurgeOutcome::NotFound.as_str(), "not_found");
    assert_eq!(PurgeOutcome::PurgedAfterKill.as_str(), "purged_after_kill");
    assert_eq!(PurgeOutcome::SkippedNoId.as_str(), "skipped_no_id");
    assert_eq!(
        PurgeOutcome::SkippedNoAdminUrl.as_str(),
        "skipped_no_admin_url"
    );
    assert_eq!(
        PurgeOutcome::Error("connection refused".to_string()).as_str(),
        "error: connection refused"
    );
}

#[test]
fn purge_outcome_was_attempted_only_for_real_calls() {
    // `was_attempted` powers the snapshot's `purge_attempted`
    // boolean. Skipped variants must report false so a downstream
    // reader can distinguish "we never called Restate" from "we
    // called Restate and it didn't find the journal."
    assert!(PurgeOutcome::Success.was_attempted());
    assert!(PurgeOutcome::NotFound.was_attempted());
    assert!(PurgeOutcome::PurgedAfterKill.was_attempted());
    assert!(PurgeOutcome::Error("x".to_string()).was_attempted());
    assert!(!PurgeOutcome::SkippedNoId.was_attempted());
    assert!(!PurgeOutcome::SkippedNoAdminUrl.was_attempted());
}

#[test]
fn inject_restate_purge_into_snapshot_adds_restate_key() {
    // The injector mutates an existing JSON object; the resulting
    // `restate` key must carry all three documented sub-fields so
    // the audit row is a complete record of what happened.
    let mut snapshot = serde_json::json!({
        "document": { "id": "doc-1" },
        "counts": { "extraction_items": 0 },
    });
    let outcome = PurgeOutcome::Success;
    inject_restate_purge_into_snapshot(&mut snapshot, Some("inv_abc"), &outcome);

    let restate = snapshot
        .get("restate")
        .expect("restate key must be present");
    assert_eq!(restate.get("invocation_id").unwrap(), "inv_abc");
    assert_eq!(restate.get("purge_attempted").unwrap(), true);
    assert_eq!(restate.get("purge_outcome").unwrap(), "success");

    // Sibling keys must be preserved — the injector is additive,
    // not destructive.
    assert!(snapshot.get("document").is_some());
    assert!(snapshot.get("counts").is_some());
}

#[tokio::test]
async fn attempt_purge_skips_when_no_admin_url() {
    // `RESTATE_ADMIN_URL` not configured — the function must short-
    // circuit BEFORE attempting the HTTP call. We pass a real
    // client and a real (but unreachable) invocation id; if the
    // skip guard fails, the test would either hang on a network
    // attempt or surface a connection error rather than the
    // expected outcome.
    let client = reqwest::Client::new();
    let outcome = attempt_restate_purge(
        &client,
        None,
        "doc-skip-no-url",
        Some("inv_anything"),
        &test_policy(),
    )
    .await;
    assert_eq!(outcome, PurgeOutcome::SkippedNoAdminUrl);
}

#[tokio::test]
async fn attempt_purge_skips_when_no_invocation_id() {
    // Admin URL configured but the document has no recorded
    // invocation id — second short-circuit branch. We pass an
    // unreachable admin URL on purpose: the skip guard must fire
    // before the HTTP layer is touched, so the unreachable host
    // should never be contacted. A failure of the guard would
    // surface as PurgeOutcome::Error from a connection refused,
    // distinguishable from the expected SkippedNoId.
    let client = reqwest::Client::new();
    let outcome = attempt_restate_purge(
        &client,
        Some("http://127.0.0.1:1"),
        "doc-skip-no-id",
        None,
        &test_policy(),
    )
    .await;
    assert_eq!(outcome, PurgeOutcome::SkippedNoId);
}

#[test]
fn inject_restate_purge_records_null_invocation_id_for_skipped_no_id() {
    // When no id was recorded, the snapshot's `invocation_id`
    // must be JSON null (not absent, not the empty string) so a
    // reader can distinguish "no id" from "id was empty string".
    let mut snapshot = serde_json::json!({});
    inject_restate_purge_into_snapshot(&mut snapshot, None, &PurgeOutcome::SkippedNoId);

    let restate = snapshot.get("restate").unwrap();
    assert!(restate.get("invocation_id").unwrap().is_null());
    assert_eq!(restate.get("purge_attempted").unwrap(), false);
    assert_eq!(restate.get("purge_outcome").unwrap(), "skipped_no_id");
}
