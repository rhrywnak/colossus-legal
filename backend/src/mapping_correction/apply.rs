//! Dual-store orchestration: apply one (or a batch of) `BEARS_ON` corrections
//! across Postgres `authored_relationships` and the Neo4j edge.
//!
//! ## Write ordering & partial-failure handling
//!
//! There is no transaction spanning Postgres and Neo4j, so consistency rests on
//! ordering + idempotency:
//!
//! - **Add / Promote** write Postgres FIRST, then Neo4j. If the Neo4j step
//!   fails, the error is [`MappingError::Neo4jAfterPostgres`] — a loud
//!   partial-write whose remedy is "re-run the identical command" (the Postgres
//!   upsert/update is idempotent, so the re-run only completes the Neo4j side).
//! - **Delete** removes the Neo4j edge FIRST, then the Postgres row, so a
//!   failure never leaves an authored graph edge orphaned from its
//!   source-of-truth row. Reported as [`MappingError::PostgresAfterNeo4j`].
//!
//! Every primitive (`MERGE`, upsert, `MATCH … DELETE`, `SET`/`REMOVE`) is
//! idempotent, so "fail cleanly with a clear message, then re-run" is always
//! safe — there is never a silent partial write.

use neo4rs::Graph;
use sqlx::PgPool;

use super::neo4j_ops::{delete_edge, merge_authored_edge, node_exists, promote_edge};
use super::{
    MappingError, OpRequest, Operation, OutcomeKind, Summary, CREATED_BY_CORRECTION, ELEMENT_LABEL,
};
use crate::neo4j::schema::BEARS_ON;
use crate::repositories::pipeline_repository::{
    delete_authored_relationship, get_authored_relationship, promote_relationship_to_authored,
    upsert_authored_relationship, PROVENANCE_AUTHORED,
};

/// Dispatch one request to its operation handler.
pub async fn apply_one(
    graph: &Graph,
    pool: &PgPool,
    case_slug: &str,
    req: &OpRequest,
    dry_run: bool,
) -> Result<OutcomeKind, MappingError> {
    match req.op {
        Operation::Add => apply_add(graph, pool, case_slug, req, dry_run).await,
        Operation::Delete => apply_delete(graph, pool, req, dry_run).await,
        Operation::Promote => apply_promote(graph, pool, req, dry_run).await,
    }
}

/// ADD: create a new authored mapping. Refuses a pair that already exists as
/// non-authored (directs to promote) and a pair whose nodes are missing.
async fn apply_add(
    graph: &Graph,
    pool: &PgPool,
    case_slug: &str,
    req: &OpRequest,
    dry_run: bool,
) -> Result<OutcomeKind, MappingError> {
    if let Some(rec) = get_authored_relationship(pool, &req.from, &req.to, BEARS_ON)
        .await
        .map_err(pg_err("get_authored_relationship"))?
    {
        if rec.provenance != PROVENANCE_AUTHORED {
            return Err(MappingError::AlreadyExists {
                from: req.from.clone(),
                to: req.to.clone(),
                provenance: rec.provenance,
            });
        }
        // Already authored: ensure the edge is present (idempotent), then skip.
        if !dry_run {
            merge_authored_edge(graph, &req.from, &req.to).await?;
        }
        return Ok(OutcomeKind::SkippedIdempotent);
    }

    // A MERGE whose MATCH finds nothing is a silent no-op, so refuse up front.
    ensure_nodes_exist(graph, req).await?;
    if dry_run {
        return Ok(OutcomeKind::Added);
    }

    // Postgres first, then Neo4j (partial failure → re-run finishes).
    upsert_authored_relationship(
        pool,
        case_slug,
        &req.from,
        &req.to,
        BEARS_ON,
        None,
        PROVENANCE_AUTHORED,
        Some(CREATED_BY_CORRECTION),
    )
    .await
    .map_err(pg_err("upsert_authored_relationship"))?;
    merge_authored_edge(graph, &req.from, &req.to)
        .await
        .map_err(after_pg("merge_authored_edge"))?;
    Ok(OutcomeKind::Added)
}

/// Verify both endpoint nodes exist before an ADD. The Allegation source is
/// matched purely by `id` (no label), the target as `:Element` — the same shape
/// the ingest writer uses. Names the offending side on failure.
async fn ensure_nodes_exist(graph: &Graph, req: &OpRequest) -> Result<(), MappingError> {
    if !node_exists(graph, &req.from, None).await? {
        return Err(MappingError::NodeMissing {
            side: "Allegation (from)",
            id: req.from.clone(),
        });
    }
    if !node_exists(graph, &req.to, Some(ELEMENT_LABEL)).await? {
        return Err(MappingError::NodeMissing {
            side: "Element (to)",
            id: req.to.clone(),
        });
    }
    Ok(())
}

/// DELETE: remove the edge then the row. Reports skipped when both were already
/// absent.
async fn apply_delete(
    graph: &Graph,
    pool: &PgPool,
    req: &OpRequest,
    dry_run: bool,
) -> Result<OutcomeKind, MappingError> {
    if dry_run {
        let exists = get_authored_relationship(pool, &req.from, &req.to, BEARS_ON)
            .await
            .map_err(pg_err("get_authored_relationship"))?
            .is_some();
        return Ok(if exists {
            OutcomeKind::Deleted
        } else {
            OutcomeKind::SkippedIdempotent
        });
    }

    // Neo4j first, then Postgres (never orphan an authored edge from its row).
    let edges = delete_edge(graph, &req.from, &req.to).await?;
    let rows = delete_authored_relationship(pool, &req.from, &req.to, BEARS_ON)
        .await
        .map_err(|source| MappingError::PostgresAfterNeo4j {
            source: Box::new(MappingError::Postgres {
                operation: "delete_authored_relationship",
                source,
            }),
        })?;
    Ok(if edges == 0 && rows == 0 {
        OutcomeKind::SkippedIdempotent
    } else {
        OutcomeKind::Deleted
    })
}

/// PROMOTE: make an existing extracted mapping durably authored. Fails if no
/// row exists; reports skipped (but still runs, idempotently) if already
/// authored.
async fn apply_promote(
    graph: &Graph,
    pool: &PgPool,
    req: &OpRequest,
    dry_run: bool,
) -> Result<OutcomeKind, MappingError> {
    let Some(rec) = get_authored_relationship(pool, &req.from, &req.to, BEARS_ON)
        .await
        .map_err(pg_err("get_authored_relationship"))?
    else {
        return Err(MappingError::MappingNotFound {
            from: req.from.clone(),
            to: req.to.clone(),
        });
    };
    let already_authored = rec.provenance == PROVENANCE_AUTHORED;
    if dry_run {
        return Ok(idempotent_or(already_authored, OutcomeKind::Promoted));
    }

    // Postgres first, then Neo4j (partial failure → re-run finishes).
    promote_relationship_to_authored(pool, &req.from, &req.to, BEARS_ON, CREATED_BY_CORRECTION)
        .await
        .map_err(pg_err("promote_relationship_to_authored"))?;
    promote_edge(graph, &req.from, &req.to)
        .await
        .map_err(after_pg("promote_edge"))?;
    Ok(idempotent_or(already_authored, OutcomeKind::Promoted))
}

/// Run a batch of requests, continuing past per-row failures and tallying the
/// result. Each row prints a one-line ✓/✗ trace; the returned [`Summary`] is the
/// final report.
pub async fn apply_batch(
    graph: &Graph,
    pool: &PgPool,
    case_slug: &str,
    requests: &[OpRequest],
    dry_run: bool,
) -> Summary {
    let mut summary = Summary {
        dry_run,
        ..Default::default()
    };
    for req in requests {
        match apply_one(graph, pool, case_slug, req, dry_run).await {
            Ok(kind) => {
                println!(
                    "  ✓ {} {} -> {}: {}",
                    req.op,
                    req.from,
                    req.to,
                    kind.label(dry_run)
                );
                summary.record(kind);
            }
            Err(e) => {
                eprintln!("  ✗ {} {} -> {}: {e}", req.op, req.from, req.to);
                summary.record_failure(req.clone(), e.to_string());
            }
        }
    }
    summary
}

// ── Small error-mapping helpers ───────────────────────────────────

/// Map a [`PipelineRepoError`] into [`MappingError::Postgres`] tagged with the
/// failing operation name. Returns a closure for use with `.map_err`.
fn pg_err(
    operation: &'static str,
) -> impl Fn(crate::repositories::pipeline_repository::PipelineRepoError) -> MappingError {
    move |source| MappingError::Postgres { operation, source }
}

/// Wrap a Neo4j-side [`MappingError`] as a partial-write that happened AFTER a
/// successful Postgres commit (add / promote ordering).
fn after_pg(operation: &'static str) -> impl Fn(MappingError) -> MappingError {
    move |source| MappingError::Neo4jAfterPostgres {
        operation,
        source: Box::new(source),
    }
}

/// Collapse the "already in target state" flag into the reported outcome.
fn idempotent_or(already: bool, done: OutcomeKind) -> OutcomeKind {
    if already {
        OutcomeKind::SkippedIdempotent
    } else {
        done
    }
}
