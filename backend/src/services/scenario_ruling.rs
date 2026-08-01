//! The anchored-ruling write path: capture the anchor, then persist the state row
//! and the ledger row as one atomic act (§12.1, tracker task 1.1).
//!
//! ## Why this is a service and not part of the handler
//!
//! Two HTTP routes make rulings — `add_scenario_fact` (a bare include) and
//! `apply_fact_action` (include / drop / defer / undrop) — and any future machine
//! ruling path will make a third. The anchor law says EVERY ruling records an
//! anchor, so the capture and the write have to live in one place that all of them
//! go through. A helper inside one handler module would leave the next caller free
//! to skip it, which is exactly how the 2026-07-24 loss became possible: there was
//! no single choke point where "a ruling happened" could be observed.
//!
//! ## Why the anchor is read from the graph and never taken from the client
//!
//! The alternative — having the browser post back the document/page/quote/speaker
//! it is already displaying — costs no round trip and is wrong. The anchor's whole
//! job is to be a TRUSTWORTHY record of what was ruled on; an anchor supplied by
//! the caller records what the caller *claimed*, which a stale tab gets wrong and a
//! bad actor can forge. Reading it server-side at ruling time means the anchor
//! comes from the same source of truth the rest of the system reads, and every
//! future machine path gets the identical treatment with no new contract.
//!
//! ## The refusal law
//!
//! A ruling that cannot be anchored is REFUSED, loudly, with a message a human can
//! act on. It is never written with a partial anchor — a half-anchor is worse than
//! no ruling, because it looks recorded while being unrecoverable. Three refusals:
//!
//! * the graph no longer holds the node (you cannot anchor what is gone);
//! * the item has no source document (it is not record evidence);
//! * an include or exclude on an item with no verbatim quote (citability law) —
//!   and that message names defer as the honest alternative.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::bias::{dto::BiasInstance, repository::BiasRepository};
use crate::domain::fact_status::FactStatus;
use crate::domain::ruling_anchor::{normalize_quote, RulingKind};
use crate::repositories::pipeline_repository::{
    insert_ruling_anchor, PipelineRepoError, RulingAnchorWrite,
};
// Imported by their FULL module path, not from the crate-level re-export, because
// both are deliberately withheld from it (see the note in
// `pipeline_repository::mod`). This module is their one legitimate caller: these
// anchor-less state writes are safe here precisely because the anchor write sits
// beside each of them, inside the same transaction.
use crate::repositories::pipeline_repository::scenario_store::{delete_fact_ref, upsert_fact_ref};

/// Everything a ruling needs beyond its anchor: who ruled, what on, and the
/// workbench fields that ride along.
///
/// ## Rust Learning: a borrowed params struct with one lifetime
///
/// Every `&'a str` here is borrowed from the handler's already-owned data (path
/// segments, the parsed request body), so the struct copies nothing. One lifetime
/// parameter `'a` covers them all because they all come from the same call frame
/// and live exactly as long as the handler's own locals — there is no need for
/// separate lifetimes, which would add noise without adding expressiveness.
#[derive(Debug)]
pub struct RulingRequest<'a> {
    pub scenario_id: Uuid,
    pub graph_node_id: &'a str,
    pub kind: RulingKind,
    /// The authenticated username, recorded in the ledger.
    pub ruled_by: &'a str,
    /// The role this fact plays, when the caller supplies one.
    pub role: Option<&'a str>,
    /// A free-text note, when the caller supplies one.
    pub note: Option<&'a str>,
    /// Required for defer, rejected for everything else.
    pub defer_reason: Option<&'a str>,
}

/// Why a ruling could not be recorded.
///
/// ## Rust Learning: a typed error enum instead of returning `AppError` here
///
/// This service does not know about HTTP, so it names failures in its own domain
/// terms and lets `api::scenario_facts` decide each one's status code. That keeps
/// the service unit-testable without an HTTP stack and keeps the status-code policy
/// in one place. `#[derive(thiserror::Error)]` writes the `Display` impl from the
/// `#[error("…")]` templates; each message is written to be read by a HUMAN in a
/// toast, not only by an operator in a log.
#[derive(Debug, thiserror::Error)]
pub enum RulingError {
    /// The graph has no node with this id any more.
    #[error(
        "this item is no longer in the graph, so the ruling cannot be anchored — \
         reload the candidate list"
    )]
    NodeGone,

    /// The item carries no source document.
    #[error(
        "this item has no source document, so a ruling on it could never be cited \
         or recovered — it cannot be ruled on"
    )]
    NoDocument,

    /// An include or exclude on an item with no usable verbatim quote.
    ///
    /// The message is the one Roman specified: it states the problem AND the
    /// honest alternative, because a refusal with no way forward just strands the
    /// human on a candidate they legitimately need to deal with.
    #[error(
        "This item has no verbatim quote and can't be cited — defer it; it stays \
         in the queue."
    )]
    NotCitable,

    /// A defer with no reason, or a reason supplied on a non-defer.
    #[error("{message}")]
    ReasonMismatch { message: String },

    /// The graph read failed.
    #[error("failed to read the item from the graph: {source}")]
    GraphRead {
        #[source]
        source: crate::bias::repository::BiasRepositoryError,
    },

    /// A database write failed.
    #[error("failed to record the ruling: {source}")]
    Write {
        #[source]
        source: PipelineRepoError,
    },
}

/// The anchor fields read from the graph, already validated against the kind of
/// ruling being made.
///
/// Owned `String`s (not borrows of the `BiasInstance`) because the instance is
/// dropped at the end of the read, while these must survive into the write. The
/// clone is of a handful of small fields on a low-frequency human action.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AnchorMaterial {
    /// `None` only for a removal recorded against a vanished node — see
    /// [`AnchorMaterial::vanished`].
    document_id: Option<String>,
    page: Option<i64>,
    quote_verbatim: Option<String>,
    speaker: Option<String>,
}

impl AnchorMaterial {
    /// The anchor for a ruling on an item the graph no longer holds.
    ///
    /// Every field absent, which is the honest record: nothing about the item was
    /// readable at ruling time. Only a removal ever reaches this — every other
    /// ruling refuses a missing node — so a fully-empty anchor in the ledger is
    /// unambiguous evidence of "the human discarded a reference to something that
    /// had already gone".
    fn vanished() -> Self {
        Self {
            document_id: None,
            page: None,
            quote_verbatim: None,
            speaker: None,
        }
    }
}

/// Pull the anchor fields out of a graph instance and enforce the refusal law.
///
/// Pure (no I/O), so every branch of the law is unit-testable without a graph.
///
/// ## Domain note: what "has a quote" means
///
/// A quote that normalizes to the empty string — whitespace only — is NOT a quote.
/// Storing it would give task 2.5 an empty comparison surface that matches
/// everything or nothing, so it is treated exactly like an absent quote. Checking
/// the NORMALIZED form (rather than `trim().is_empty()`) means this test asks the
/// same question the matcher will later ask.
fn extract_anchor(
    instance: &BiasInstance,
    kind: RulingKind,
) -> Result<AnchorMaterial, RulingError> {
    let document_id = instance.document.as_ref().map(|d| d.id.clone());
    if kind.requires_document() && document_id.is_none() {
        return Err(RulingError::NoDocument);
    }

    // An empty-after-normalization quote is no quote at all.
    let quote_verbatim = instance
        .verbatim_quote
        .as_ref()
        .filter(|q| !normalize_quote(q).is_empty())
        .cloned();

    if kind.requires_quote() && quote_verbatim.is_none() {
        return Err(RulingError::NotCitable);
    }

    Ok(AnchorMaterial {
        document_id,
        page: instance.page_number,
        quote_verbatim,
        // An empty speaker string is treated as absent: `evidence_by_ids` decodes a
        // missing STATED_BY edge to `coalesce(…, '')`, so an empty string here IS
        // the graph's way of saying "no speaker", not a speaker whose name is "".
        speaker: instance
            .stated_by
            .as_ref()
            .map(|a| a.name.clone())
            .filter(|n| !n.trim().is_empty()),
    })
}

/// Check the defer-reason pairing before any I/O.
///
/// The database CHECK backstops this, but a constraint violation surfaces as an
/// opaque 500. Validating here turns both mistakes into a precise 400 that names
/// what was wrong.
fn validate_reason(kind: RulingKind, defer_reason: Option<&str>) -> Result<(), RulingError> {
    let has_reason = defer_reason.is_some_and(|r| !r.trim().is_empty());

    if kind.requires_reason() && !has_reason {
        return Err(RulingError::ReasonMismatch {
            message: "a defer must state a reason — that reason is what \
                      distinguishes a parked candidate from one nobody has \
                      looked at"
                .to_string(),
        });
    }
    if !kind.requires_reason() && defer_reason.is_some() {
        return Err(RulingError::ReasonMismatch {
            message: format!("a reason may only accompany a defer, not {}", kind.code()),
        });
    }
    Ok(())
}

/// Record one ruling: capture its anchor, then write the state row and the ledger
/// row in a single transaction.
///
/// Returns the `anchor_id` appended to the ledger so the caller can log exactly
/// which anchor it wrote.
///
/// ## Rust Learning: `&mut *tx` — reborrowing a transaction for two writes
///
/// `sqlx::Transaction` implements `PgExecutor` only through a mutable reference,
/// and each `.execute()` consumes the executor it is given. Writing `&mut *tx`
/// REBORROWS the transaction for the duration of that one call, so the transaction
/// itself is not moved and remains usable for the next write and the final commit.
/// Passing `&mut tx` directly would move it into the first call.
///
/// ## Why a transaction rather than two sequential writes
///
/// Either both rows land or neither does. Without that, a crash between them
/// leaves a ruling with no anchor (the 2026-07-24 state, recreated) or an anchor
/// for a ruling that was never applied. `commit()`'s result is propagated, never
/// discarded — a failed commit is a failed ruling.
///
/// # Errors
/// Returns [`RulingError`] if the anchor cannot be captured (see the refusal law
/// in the module doc) or if either write fails.
async fn capture_anchor(
    graph: &BiasRepository,
    request: &RulingRequest<'_>,
) -> Result<AnchorMaterial, RulingError> {
    validate_reason(request.kind, request.defer_reason)?;

    // Read the item as the graph holds it RIGHT NOW — this is the anchor's source
    // of truth, and the read is what makes the anchor unforgeable.
    let instances = graph
        .evidence_by_ids(&[request.graph_node_id.to_string()])
        .await
        .map_err(|source| RulingError::GraphRead { source })?;

    // A miss here is the `join_facts` warn-and-continue case, but at RULING time it
    // must refuse rather than warn: writing a ruling whose anchor cannot be read is
    // precisely the 2026-07-24 failure, and returning success would recreate it.
    //
    // The single exception is a REMOVAL, which must never be blocked — a vanished
    // node is often exactly why the human is discarding the reference. It records a
    // fully-absent anchor instead, which says precisely that.
    match instances
        .into_iter()
        .find(|i| i.evidence_id == request.graph_node_id)
    {
        Some(instance) => extract_anchor(&instance, request.kind),
        None if request.kind.tolerates_missing_node() => {
            tracing::warn!(
                graph_node_id = %request.graph_node_id,
                ruling = %request.kind.code(),
                "the graph no longer holds this item; recording the ruling with an \
                 empty anchor rather than refusing it"
            );
            Ok(AnchorMaterial::vanished())
        }
        None => Err(RulingError::NodeGone),
    }
}

/// Build the ledger row for one captured anchor.
///
/// Split out so `record_ruling` and `record_removal` assemble the row identically
/// — a second hand-built `RulingAnchorWrite` is exactly where the two paths would
/// drift, and a drift here is a ledger that means different things depending on
/// which verb produced the row.
fn anchor_write<'a>(
    request: &'a RulingRequest<'a>,
    anchor: &'a AnchorMaterial,
    ruled_at: DateTime<Utc>,
) -> RulingAnchorWrite<'a> {
    RulingAnchorWrite {
        scenario_id: request.scenario_id,
        graph_node_id: request.graph_node_id,
        kind: request.kind,
        document_id: anchor.document_id.as_deref(),
        page: anchor.page,
        quote_verbatim: anchor.quote_verbatim.as_deref(),
        speaker: anchor.speaker.as_deref(),
        ruled_at,
        ruled_by: request.ruled_by,
        defer_reason: request.defer_reason,
    }
}

pub async fn record_ruling(
    pool: &PgPool,
    graph: &BiasRepository,
    request: &RulingRequest<'_>,
    status: FactStatus,
    ruled_at: DateTime<Utc>,
) -> Result<Uuid, RulingError> {
    let anchor = capture_anchor(graph, request).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| RulingError::Write { source: e.into() })?;

    upsert_fact_ref(
        &mut *tx,
        request.scenario_id,
        request.graph_node_id,
        request.role,
        status,
        request.note,
        // confidence: this is a HUMAN ruling, so there is no model score. `None` is
        // the permanent, correct value — not a placeholder.
        None,
        request.defer_reason,
    )
    .await
    .map_err(|source| RulingError::Write { source })?;

    let anchor_id = insert_ruling_anchor(&mut *tx, &anchor_write(request, &anchor, ruled_at))
        .await
        .map_err(|source| RulingError::Write { source })?;

    tx.commit()
        .await
        .map_err(|e| RulingError::Write { source: e.into() })?;

    Ok(anchor_id)
}

/// Record a REMOVAL: capture its anchor, delete the state row, and append the
/// ledger row — all in one transaction.
///
/// Returns `None` when the reference was not on this scenario (nothing removed,
/// so nothing to record; the handler turns that into a 404). `Some(anchor_id)`
/// when a row was genuinely removed and the removal is now in the ledger.
///
/// ## Domain note: why the delete runs BEFORE the ledger write
///
/// The order matters for the `None` case. Deleting first tells us whether there
/// was anything to remove; only then is a ledger row justified. The reverse order
/// would append "the human removed X" and then discover X was never there,
/// leaving a ledger entry for an event that did not happen — and the ledger's
/// value rests entirely on every row being a real act.
///
/// Both are in one transaction, so the `None` path rolls back cleanly and a crash
/// between them can never delete a reference without recording why it went.
///
/// ## Why the anchor is captured BEFORE the transaction opens
///
/// The graph read is network I/O to Neo4j; holding a Postgres transaction open
/// across it would pin a connection for the round trip for no benefit. Nothing in
/// the anchor depends on the delete, so it is read first and the transaction stays
/// as short as the two writes.
///
/// # Errors
/// Returns [`RulingError`] if a write fails. A removal is never refused for
/// missing content — see [`RulingKind::Remove`].
pub async fn record_removal(
    pool: &PgPool,
    graph: &BiasRepository,
    request: &RulingRequest<'_>,
    ruled_at: DateTime<Utc>,
) -> Result<Option<Uuid>, RulingError> {
    let anchor = capture_anchor(graph, request).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| RulingError::Write { source: e.into() })?;

    let removed = delete_fact_ref(&mut *tx, request.scenario_id, request.graph_node_id)
        .await
        .map_err(|source| RulingError::Write { source })?;

    if removed == 0 {
        // Nothing was on this scenario. Roll back rather than commit an empty
        // transaction, and tell the caller so it can answer 404 — "removed
        // nothing" and "removed something" must stay distinguishable.
        tx.rollback()
            .await
            .map_err(|e| RulingError::Write { source: e.into() })?;
        return Ok(None);
    }

    let anchor_id = insert_ruling_anchor(&mut *tx, &anchor_write(request, &anchor, ruled_at))
        .await
        .map_err(|source| RulingError::Write { source })?;

    tx.commit()
        .await
        .map_err(|e| RulingError::Write { source: e.into() })?;

    Ok(Some(anchor_id))
}

#[cfg(test)]
#[path = "scenario_ruling_tests.rs"]
mod tests;
