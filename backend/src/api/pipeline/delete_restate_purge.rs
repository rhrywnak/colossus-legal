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

use std::time::Duration;

use crate::pipeline::workflow_admin::{
    kill_restate_invocation, purge_restate_workflow, PurgeResult,
};

/// How hard to chase a killed invocation until its journal is purgeable.
///
/// ## Why this is configuration and not a constant
///
/// Kill is a hard stop, so the invocation reaches a terminal state almost
/// immediately — but "almost" is not "synchronously". Restate may answer the
/// kill with 202 (accepted, applied asynchronously), so the terminal state can
/// land just after the call returns, and how long "just after" takes depends on
/// the deployment: a loaded Restate, an admin endpoint behind a proxy, or a
/// future Restate version with a longer propagation window all move it.
///
/// That makes these deployment values, not protocol values — Standing Rule 2's
/// test applies ("to change a default, can Roman edit env vars and restart,
/// with no code change and no rebuild?") and the answer has to be yes. They are
/// read once at startup by [`crate::config::AppConfig`]; a PRESENT but
/// unparseable value is a startup error, never a silent fall back to the
/// default.
#[derive(Debug, Clone, Copy)]
pub struct RestatePurgePolicy {
    /// Purge attempts after the kill, before giving up and reporting the
    /// manual remedy.
    pub retry_attempts: u32,
    /// Wait between those attempts.
    pub retry_delay: Duration,
}

impl Default for RestatePurgePolicy {
    /// The shipped values — the ONE place they are written down.
    ///
    /// Four attempts 250ms apart bounds the added wait at ~1s, which is well
    /// inside the delete handler's own envelope while covering the async-kill
    /// window measured against Restate 1.6.2 on DEV.
    fn default() -> Self {
        Self {
            // DEFAULT: 4 attempts — override: RESTATE_PURGE_RETRY_ATTEMPTS
            retry_attempts: 4,
            // DEFAULT: 250 ms — override: RESTATE_PURGE_RETRY_DELAY_MS
            retry_delay: Duration::from_millis(250),
        }
    }
}

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
    /// The invocation was still running, so it was killed and then purged.
    ///
    /// Distinct from [`PurgeOutcome::Success`] on purpose: an operator reading
    /// the audit row should be able to tell "the workflow had already finished"
    /// from "we hard-stopped a run in progress to delete this document". Those
    /// are different events, and collapsing them would hide the fact that work
    /// was terminated mid-flight (Standing Rule 1).
    PurgedAfterKill,
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
            PurgeOutcome::PurgedAfterKill => "purged_after_kill".to_string(),
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
            PurgeOutcome::Success
                | PurgeOutcome::NotFound
                | PurgeOutcome::PurgedAfterKill
                | PurgeOutcome::Error(_)
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
/// - Both present → purge, and if Restate says the invocation is still
///   running, KILL it and purge again (see below).
///
/// ## The kill escalation (2026-08-27)
///
/// Purge only works on a terminal invocation. A document deleted mid-run
/// therefore got a 409 from Restate, which the old code folded into a generic
/// error and logged before proceeding — leaving the journal alive and, because
/// Restate workflow keys are single-use, permanently blocking that `doc_id`.
/// The operator then could not re-upload and process the same document: the
/// invoke came back `PreviouslyAccepted` → 409, and deleting again re-ran the
/// same failing purge. Measured on `doc-phillips-motion-summary-disposition-
/// 07-10-2014`, whose audit rows show `success` at 19:11, then `error: … not
/// yet completed` at 19:26, then `skipped_no_id` at 19:41.
///
/// So a `NotTerminal` answer now escalates: kill the invocation (a hard stop —
/// the document is being destroyed, so there is nothing to unwind gracefully
/// for), then re-attempt the purge a bounded number of times. Deleting a
/// document now frees its workflow key whatever state the run was in.
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
    policy: &RestatePurgePolicy,
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
        Ok(PurgeResult::Purged) => PurgeOutcome::Success,
        Ok(PurgeResult::NotFound) => PurgeOutcome::NotFound,
        // The invocation is still running. Kill it, then purge again.
        Ok(PurgeResult::NotTerminal) => {
            purge_after_kill(http_client, admin_url, document_id, inv_id, policy).await
        }
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

/// Kill an in-flight invocation, then purge its journal.
///
/// Only reached when Restate answered `NotTerminal`. Returns
/// [`PurgeOutcome::PurgedAfterKill`] on success — a distinct observable from a
/// plain `Success`, because an operator should be able to see that a run was
/// hard-stopped mid-flight rather than having finished on its own.
///
/// ## Rust Learning: a bounded retry loop over an async call
///
/// The loop re-issues the purge rather than sleeping once for a guessed
/// duration. Restate may answer the kill with 202 (accepted, applied
/// asynchronously), so the terminal state arrives shortly *after* the kill
/// returns. Polling the operation we actually care about — can we purge yet? —
/// is more honest than sleeping long enough to "probably" be safe, and it
/// returns as soon as the answer is yes rather than always paying the worst
/// case.
///
/// Every exit path is an observable outcome: killed-and-purged, killed-and-gone
/// (404), or killed-but-still-not-terminal after the budget, which records the
/// exact invocation id so an operator can finish the job by hand.
async fn purge_after_kill(
    http_client: &reqwest::Client,
    admin_url: &str,
    document_id: &str,
    inv_id: &str,
    policy: &RestatePurgePolicy,
) -> PurgeOutcome {
    tracing::warn!(
        document_id = %document_id,
        invocation_id = %inv_id,
        "Restate purge: invocation still running at delete time — killing it so \
         the workflow key is freed for a future re-upload of this document"
    );

    if let Err(e) = kill_restate_invocation(http_client, admin_url, inv_id).await {
        tracing::error!(
            document_id = %document_id,
            invocation_id = %inv_id,
            error = %e,
            "Restate kill failed — the workflow journal will remain and this document \
             id cannot be processed again until it is cleared by hand. Manual remedy: \
             PATCH {{RESTATE_ADMIN_URL}}/invocations/{inv_id}/kill, then \
             PATCH {{RESTATE_ADMIN_URL}}/invocations/{inv_id}/purge",
        );
        // The stored string carries the remedy too: an operator reading
        // `document_audit_log` months later may have no access to these logs.
        return PurgeOutcome::Error(format!(
            "kill failed: {e}. The workflow key for this document is still held. Clear it \
             manually: PATCH {{RESTATE_ADMIN_URL}}/invocations/{inv_id}/kill, then \
             PATCH {{RESTATE_ADMIN_URL}}/invocations/{inv_id}/purge"
        ));
    }

    for attempt in 1..=policy.retry_attempts {
        match purge_restate_workflow(http_client, admin_url, inv_id).await {
            Ok(PurgeResult::Purged) => {
                tracing::info!(
                    document_id = %document_id,
                    invocation_id = %inv_id,
                    attempt,
                    "Restate purge: journal purged after kill"
                );
                return PurgeOutcome::PurgedAfterKill;
            }
            // The kill removed it outright; nothing left to purge. Same net
            // effect as a purge — the key is free. Logged like every other
            // exit from this function: an operator who has just read the
            // "killing it" warning must not then read nothing.
            Ok(PurgeResult::NotFound) => {
                tracing::info!(
                    document_id = %document_id,
                    invocation_id = %inv_id,
                    attempt,
                    "Restate purge: the kill removed the invocation outright (404) — \
                     the workflow key is free"
                );
                return PurgeOutcome::PurgedAfterKill;
            }
            Ok(PurgeResult::NotTerminal) => {
                // Not terminal *yet*. Wait and ask again, unless this was the
                // last attempt — no point sleeping on the way out.
                if attempt < policy.retry_attempts {
                    tokio::time::sleep(policy.retry_delay).await;
                }
            }
            Err(e) => {
                tracing::error!(
                    document_id = %document_id,
                    invocation_id = %inv_id,
                    attempt,
                    error = %e,
                    "Restate purge after kill failed with an unexpected error"
                );
                // Same contract as the kill-failure arm above: this string is
                // what an operator reads in `document_audit_log`, possibly
                // long after the logs have rotated, so it has to carry WHERE
                // (the invocation) and WHAT TO DO, not just what failed.
                return PurgeOutcome::Error(format!(
                    "purge after kill failed (attempt {attempt} of {total}): {e}. The \
                     workflow key for invocation '{inv_id}' may still be held. Purge it \
                     manually: PATCH {{RESTATE_ADMIN_URL}}/invocations/{inv_id}/purge",
                    total = policy.retry_attempts,
                ));
            }
        }
    }

    tracing::error!(
        document_id = %document_id,
        invocation_id = %inv_id,
        attempts = policy.retry_attempts,
        "Restate invocation was killed but is still not terminal — the journal \
         remains and this document id will 409 on re-process until it is purged"
    );
    PurgeOutcome::Error(format!(
        "invocation '{inv_id}' was killed but had not reached a terminal state after \
         {attempts} purge attempts; the workflow key is still held. Purge it \
         manually: PATCH {{RESTATE_ADMIN_URL}}/invocations/{inv_id}/purge",
        attempts = policy.retry_attempts,
    ))
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

#[cfg(test)]
#[path = "delete_restate_purge_tests.rs"]
mod tests;
