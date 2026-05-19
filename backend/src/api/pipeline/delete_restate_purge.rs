//! Restate workflow-journal purge logic for the document delete handler.
//!
//! Lives alongside [`super::delete`] but in its own file so each module
//! stays focused: `delete.rs` owns the audit-snapshot build, the
//! Postgres / Neo4j / Qdrant / filesystem cleanup ordering, and the
//! handler's transactional contract; this module owns the Restate
//! purge call and its outcome reporting.
//!
//! The split mirrors the `api/pipeline/cancel.rs` ↔
//! `pipeline/workflow_admin.rs` split: handler-level orchestration in
//! the API module, the Restate-protocol helper in `pipeline/`. Here
//! the API-side piece carries enough surface (the outcome enum, the
//! snapshot injector) to warrant its own file rather than crowding
//! `delete.rs` past the 300-line module budget.

use crate::pipeline::workflow_admin::purge_restate_workflow;

/// Outcome of the Restate purge attempt at delete time.
///
/// Five variants map 1:1 onto the wire-shape strings recorded in the
/// audit snapshot's `restate.purge_outcome` field. Keeping them as a
/// typed enum (rather than building the snapshot from `match` arms
/// scattered through the handler) means the outcome string lives in
/// exactly one place — [`PurgeOutcome::as_str`] — so a typo in any
/// downstream consumer (e.g. a future operator-facing dashboard
/// reading `document_audit_log.snapshot->'restate'->>'purge_outcome'`)
/// is caught at the enum boundary.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum PurgeOutcome {
    /// Restate returned 200/202 — the journal was found and purged.
    Success,
    /// Restate returned 404 — no journal exists for this invocation id
    /// (either already purged or never created). Treated as success
    /// for delete-handler purposes; the audit log records the
    /// distinction.
    NotFound,
    /// `documents.restate_invocation_id` was NULL on the row, so no
    /// purge call was attempted. Normal for documents that have never
    /// had Process clicked, and for pre-migration rows whose
    /// invocation id was never captured.
    SkippedNoId,
    /// `RESTATE_ADMIN_URL` is unset in the backend environment, so the
    /// purge call was skipped without contacting Restate. Mirrors the
    /// same branch in the cancel handler — admin operations degrade
    /// gracefully when the admin URL is not configured.
    SkippedNoAdminUrl,
    /// The purge call returned an unexpected status or failed at the
    /// transport layer. The contained string is the operator-facing
    /// error message produced by [`purge_restate_workflow`], suitable
    /// for direct inclusion in the audit snapshot.
    Error(String),
}

impl PurgeOutcome {
    /// Wire-shape string recorded in the audit snapshot. The error
    /// variant prefixes the message with `error: ` so a snapshot
    /// reader can branch on the prefix without parsing JSON further.
    pub(super) fn as_str(&self) -> String {
        match self {
            PurgeOutcome::Success => "success".to_string(),
            PurgeOutcome::NotFound => "not_found".to_string(),
            PurgeOutcome::SkippedNoId => "skipped_no_id".to_string(),
            PurgeOutcome::SkippedNoAdminUrl => "skipped_no_admin_url".to_string(),
            PurgeOutcome::Error(msg) => format!("error: {msg}"),
        }
    }

    /// True when the purge helper was actually called (Success,
    /// NotFound, or Error). Used by the snapshot injector to populate
    /// `purge_attempted` without re-pattern-matching at the call site.
    pub(super) fn was_attempted(&self) -> bool {
        matches!(
            self,
            PurgeOutcome::Success | PurgeOutcome::NotFound | PurgeOutcome::Error(_)
        )
    }
}

/// Attempt to purge the Restate workflow journal for a document.
///
/// Three-branch dispatch driven by what's available:
///
/// - `restate_admin_url` is `None` → `SkippedNoAdminUrl` (config gate).
/// - `invocation_id` is `None` → `SkippedNoId` (no workflow ever ran
///   for this document, or pre-migration row).
/// - Both present → call [`purge_restate_workflow`] and map its
///   `Ok(true)` / `Ok(false)` / `Err` into `Success` / `NotFound` /
///   `Error(msg)`.
///
/// All branches log at info level for the skip cases and at error
/// level for the failure case, with `document_id` as a structured field
/// so an operator tailing logs can identify which DELETE triggered the
/// outcome without re-correlating against a request trace. The snapshot
/// writer captures the same outcome so the audit row is self-contained.
///
/// ## Why primitives instead of `&AppState`
///
/// Earlier drafts took `&AppState` directly. The signature was changed
/// to take the four values the function actually needs so the two
/// skip-branch dispatch cases can be unit-tested without constructing
/// a full `AppState` fixture (which requires lazy Postgres pools, a
/// Neo4j graph stub, an embedding-provider stub, and an audit
/// repository — too much scaffolding for a four-line dispatch). The
/// caller in `delete.rs` plumbs the values from `state` at the call
/// site, which costs four extra lines there and saves a fixture-shaped
/// dependency on `state` here.
pub(super) async fn attempt_restate_purge(
    http_client: &reqwest::Client,
    restate_admin_url: Option<&str>,
    document_id: &str,
    invocation_id: Option<&str>,
) -> PurgeOutcome {
    let Some(admin_url) = restate_admin_url else {
        tracing::info!(
            document_id = %document_id,
            "Restate purge: RESTATE_ADMIN_URL not configured, skipping"
        );
        return PurgeOutcome::SkippedNoAdminUrl;
    };

    let Some(inv_id) = invocation_id else {
        tracing::info!(
            document_id = %document_id,
            "Restate purge: no invocation_id recorded on document, skipping"
        );
        return PurgeOutcome::SkippedNoId;
    };

    match purge_restate_workflow(http_client, admin_url, inv_id).await {
        Ok(true) => PurgeOutcome::Success,
        Ok(false) => PurgeOutcome::NotFound,
        Err(e) => {
            tracing::error!(
                document_id = %document_id,
                invocation_id = %inv_id,
                error = %e,
                "Restate purge call failed — orphan workflow journal may remain. \
                 Operator can purge manually via the Restate admin API."
            );
            PurgeOutcome::Error(format!("{e}"))
        }
    }
}

/// Splice the Restate purge outcome into the audit snapshot under a
/// `restate` key.
///
/// The snapshot is built by `build_audit_snapshot` (in `delete.rs`)
/// before the purge runs, so the snapshot captures pre-deletion DB
/// state; this function adds the purge result so the audit row in
/// `document_audit_log.snapshot` is a complete record of what
/// happened during this DELETE. We mutate the existing JSON object
/// rather than rebuilding it because the snapshot already carries six
/// other sibling keys we'd otherwise need to plumb through a new
/// builder signature.
pub(super) fn inject_restate_purge_into_snapshot(
    snapshot: &mut serde_json::Value,
    invocation_id: Option<&str>,
    outcome: &PurgeOutcome,
) {
    let restate_block = serde_json::json!({
        "invocation_id": invocation_id,
        "purge_attempted": outcome.was_attempted(),
        "purge_outcome": outcome.as_str(),
    });
    if let Some(obj) = snapshot.as_object_mut() {
        obj.insert("restate".to_string(), restate_block);
    } else {
        // Snapshot is always built as a JSON object by
        // build_audit_snapshot — if a future refactor changes that
        // contract, fail loudly in the audit row rather than silently
        // dropping the purge record. The strange-shape snapshot still
        // gets written; the operator sees the missing `restate` key
        // and the error log line and knows the snapshot contract
        // drifted.
        tracing::error!(
            "Audit snapshot was not a JSON object — cannot attach restate purge record. \
             The snapshot contract in build_audit_snapshot has changed."
        );
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
        let outcome =
            attempt_restate_purge(&client, None, "doc-skip-no-url", Some("inv_anything")).await;
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
        let outcome =
            attempt_restate_purge(&client, Some("http://127.0.0.1:1"), "doc-skip-no-id", None)
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
}
