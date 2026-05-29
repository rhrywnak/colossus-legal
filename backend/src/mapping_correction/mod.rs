//! B2 mapping-correction tool — durable add/delete/promote of `BEARS_ON`
//! Allegation→Element mappings, writing BOTH stores (Postgres
//! `authored_relationships` + the Neo4j edge) consistently.
//!
//! Driven by the `fix_bears_on_mapping` binary (a twin of
//! `load_canonical_elements`): a thin CLI that connects to Neo4j + Postgres and
//! hands off to [`apply`]'s functions, which hold all the logic so they can be
//! tested without a live database.
//!
//! ## Why a correction tool exists at all
//!
//! Pass-2 extraction writes 207 `BEARS_ON` mappings with `provenance =
//! 'extracted'`. A human correcting those during B2 review needs the change to
//! be DURABLE — to survive the next complaint reprocess. The pipeline's
//! reconciliation deletes only `provenance = 'extracted'` rows (Postgres) and
//! only edges carrying `asserted_by_document` (Neo4j), so a durable correction
//! must land as `provenance = 'authored'` with no `asserted_by_document`.
//!
//! ## Durability rests on `provenance`, not `document_id`
//!
//! Both reconciliation paths filter on `provenance = 'extracted'`, so flipping
//! a row to `'authored'` is what protects it. Nulling `document_id` (in
//! [`apply::apply_promote`]) is hygiene only: a later Pass-2 re-extraction of
//! the same pair re-populates `document_id` via the extracted-insert's
//! `ON CONFLICT` (which refreshes `document_id` but PRESERVES `provenance`).
//! The row stays durable because it stays `'authored'`.
//!
//! ## Three operations
//!
//! - **Add** — a brand-new authored mapping (row + edge, no `document_id`, no
//!   `asserted_by_document`). Refuses a pair that already exists as non-authored
//!   (directs the operator to `promote`).
//! - **Delete** — remove the row and the edge.
//! - **Promote** — turn an existing extracted mapping into a durable authored
//!   one (Postgres: `provenance='authored'`, `document_id=NULL`,
//!   `created_by='b2-correction'`; Neo4j: set provenance, strip the three
//!   extraction-origin edge properties).

pub mod apply;
pub mod csv;
pub mod neo4j_ops;

use std::fmt;

use crate::repositories::pipeline_repository::PipelineRepoError;

/// `created_by` stamp recording that a human made this mapping decision during
/// B2 review. A promoted/added mapping is a human choice, so the audit trail
/// must say so rather than keep the extractor's `'pass2'` marker.
pub const CREATED_BY_CORRECTION: &str = "b2-correction";

/// Code-defined `:Element` label for the target-node existence pre-check. Not
/// operator input, so it is safe to interpolate into Cypher.
pub const ELEMENT_LABEL: &str = "Element";

// ── Operation ─────────────────────────────────────────────────────

/// The three corrections the tool performs on one `BEARS_ON` pair.
///
/// Derives `clap::ValueEnum` so the binary's `--op` flag and the CSV parser
/// share one spelling of `add` / `delete` / `promote`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
#[clap(rename_all = "lowercase")]
pub enum Operation {
    /// Create a new authored mapping.
    Add,
    /// Remove a mapping (row + edge).
    Delete,
    /// Make an existing extracted mapping durably authored.
    Promote,
}

impl fmt::Display for Operation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Operation::Add => "add",
            Operation::Delete => "delete",
            Operation::Promote => "promote",
        };
        f.write_str(s)
    }
}

/// One requested correction: the operation plus the edge's endpoints. `from` is
/// the Allegation node id, `to` is the Element id; the relationship type is
/// always [`crate::neo4j::schema::BEARS_ON`], never operator-supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpRequest {
    pub op: Operation,
    pub from: String,
    pub to: String,
}

// ── Outcome ───────────────────────────────────────────────────────

/// What actually happened (or, under `--dry-run`, what would happen) for one
/// request. The orchestration returns this on success; failures surface as a
/// [`MappingError`] which the batch runner records per-row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeKind {
    Added,
    Deleted,
    Promoted,
    /// The request was a no-op: the desired state already held (e.g. add of an
    /// already-authored pair, delete of an absent pair).
    SkippedIdempotent,
}

impl OutcomeKind {
    /// Human label, prefixed with "would " under dry-run for the report.
    pub fn label(self, dry_run: bool) -> String {
        let verb = match self {
            OutcomeKind::Added => "added",
            OutcomeKind::Deleted => "deleted",
            OutcomeKind::Promoted => "promoted",
            OutcomeKind::SkippedIdempotent => "skipped (idempotent)",
        };
        if dry_run {
            format!("would be {verb}")
        } else {
            verb.to_string()
        }
    }
}

// ── Summary (batch report) ────────────────────────────────────────

/// Tally across a batch run. The binary prints this as the final report and
/// uses [`Summary::has_failures`] to choose its exit code.
#[derive(Debug, Default)]
pub struct Summary {
    pub dry_run: bool,
    pub added: usize,
    pub deleted: usize,
    pub promoted: usize,
    pub skipped: usize,
    /// One entry per failed request: the request and the error message.
    pub failed: Vec<(OpRequest, String)>,
}

impl Summary {
    /// Fold one successful outcome into the tally.
    pub fn record(&mut self, kind: OutcomeKind) {
        match kind {
            OutcomeKind::Added => self.added += 1,
            OutcomeKind::Deleted => self.deleted += 1,
            OutcomeKind::Promoted => self.promoted += 1,
            OutcomeKind::SkippedIdempotent => self.skipped += 1,
        }
    }

    /// Record a per-row failure (batch mode keeps going on error).
    pub fn record_failure(&mut self, req: OpRequest, message: String) {
        self.failed.push((req, message));
    }

    /// True if any row failed — the binary exits non-zero in that case.
    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }
}

impl fmt::Display for Summary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode = if self.dry_run { " (dry-run — no writes)" } else { "" };
        writeln!(f, "── BEARS_ON mapping correction summary{mode} ──")?;
        writeln!(f, "  added:    {}", self.added)?;
        writeln!(f, "  deleted:  {}", self.deleted)?;
        writeln!(f, "  promoted: {}", self.promoted)?;
        writeln!(f, "  skipped:  {}", self.skipped)?;
        writeln!(f, "  failed:   {}", self.failed.len())?;
        for (req, message) in &self.failed {
            writeln!(f, "    ✗ {} {} -> {}: {message}", req.op, req.from, req.to)?;
        }
        Ok(())
    }
}

// ── Error type ────────────────────────────────────────────────────

/// Errors raised by the correction tool. Each variant maps to a documented
/// process exit code via [`MappingError::exit_code`], mirroring
/// `CanonicalLoaderError`.
#[derive(Debug, thiserror::Error)]
pub enum MappingError {
    /// A Neo4j request failed (connection up, query/exec error). Exit 3.
    #[error("Neo4j query failed during {operation}: {source}")]
    Neo4j {
        operation: &'static str,
        #[source]
        source: neo4rs::Error,
    },

    /// A Neo4j row column could not be decoded. Exit 3.
    #[error("Failed to decode Neo4j row during {operation}: {source}")]
    Neo4jDecode {
        operation: &'static str,
        #[source]
        source: neo4rs::DeError,
    },

    /// A Postgres operation failed. Exit 3.
    #[error("Postgres operation '{operation}' failed: {source}")]
    Postgres {
        operation: &'static str,
        #[source]
        source: PipelineRepoError,
    },

    /// ADD validation: an endpoint node does not exist in the graph. The MERGE
    /// would be a silent no-op, so we refuse before writing anything. Exit 4.
    #[error("{side} node '{id}' not found in graph — cannot create mapping")]
    NodeMissing { side: &'static str, id: String },

    /// ADD refusal: the pair already exists with a non-authored provenance.
    /// Adding would leave a half-promoted, non-durable row; the operator should
    /// promote instead. Exit 4.
    #[error(
        "mapping {from} -> {to} already exists as '{provenance}' — use `--op promote` \
         to make it durable, not `--op add`"
    )]
    AlreadyExists {
        from: String,
        to: String,
        provenance: String,
    },

    /// PROMOTE precondition: no such mapping row to promote. Exit 4.
    #[error("no mapping {from} -> {to} to promote (Postgres row not found)")]
    MappingNotFound { from: String, to: String },

    /// Partial write: Postgres committed but the following Neo4j write failed.
    /// The stores are momentarily inconsistent; every step is idempotent, so
    /// re-running the identical command finishes the job. Exit 3.
    #[error(
        "PARTIAL WRITE: Postgres updated but Neo4j {operation} FAILED ({source}). \
         Re-run the identical command to finish — all steps are idempotent."
    )]
    Neo4jAfterPostgres {
        operation: &'static str,
        #[source]
        source: Box<MappingError>,
    },

    /// Partial write (delete path): the Neo4j edge was removed but the Postgres
    /// row delete failed. Re-run to finish (idempotent). Exit 3.
    #[error(
        "PARTIAL WRITE: Neo4j edge deleted but Postgres row delete FAILED ({source}). \
         Re-run the identical command to finish — all steps are idempotent."
    )]
    PostgresAfterNeo4j {
        #[source]
        source: Box<MappingError>,
    },

    /// A `--from-file` CSV line was malformed. Exit 1.
    #[error("CSV parse error on line {line}: {message}")]
    Csv { line: usize, message: String },
}

impl MappingError {
    /// Map to a documented process exit code (mirrors the loader's scheme):
    /// - `1` input/parse problem (CSV)
    /// - `3` store operation / partial-write failure
    /// - `4` validation / precondition failure
    pub fn exit_code(&self) -> u8 {
        match self {
            MappingError::Csv { .. } => 1,
            MappingError::NodeMissing { .. }
            | MappingError::AlreadyExists { .. }
            | MappingError::MappingNotFound { .. } => 4,
            MappingError::Neo4j { .. }
            | MappingError::Neo4jDecode { .. }
            | MappingError::Postgres { .. }
            | MappingError::Neo4jAfterPostgres { .. }
            | MappingError::PostgresAfterNeo4j { .. } => 3,
        }
    }
}
