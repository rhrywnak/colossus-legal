//! The count proof — what the re-key claims it did, in a form Roman can check.
//!
//! The ruling requires the proof in two places: real `tracing` output while the
//! run happens, and a written report file afterwards. Both are rendered from the
//! same structures, so the file cannot say something the log did not.
//!
//! ## Why a per-table breakdown and not one total
//!
//! 947 curated rows across eight columns. A single "947 rows updated" line is
//! satisfied by a run that moved 947 rows in the wrong table. The per-table
//! expected-vs-updated pairs are what make the claim checkable — and they are
//! what the abort decision is made on, table by table, before anything commits.

use std::fmt::Write as _;

use super::plan::{PlanTotals, RekeyPlan};

// ── Exit codes ──────────────────────────────────────────────────────────────
//
// Ruled 2026-08-14 for this tool, then extended to the whole one-shot family by
// the party-merge addendum. The DEFINITIONS moved to `crate::oneshot::exit` when
// the family grew to four binaries — four copies of six constants is exactly the
// kind of duplication that drifts, and a runbook step branching on `3` must mean
// the same thing whichever binary produced it.
//
// They are re-exported here, under the names this module has always used, so
// every existing caller and every existing test is unchanged. `rekey_evidence`'s
// behaviour is byte-identical; only the address of the constants moved.
//
// ## The renumbering, and what it cost (kept here, where it happened)
//
// The ruling assigned `3` to aborted documents while `3` already meant "Postgres
// connection failure" and `2` meant "Neo4j connection failure". Rather than
// invent a sixth number, the two CONNECTION failures collapsed into `2` — which
// is what the ruling's own wording ("2 = tool/connection failure") describes.
// Nothing diagnostic was lost: the error log names which store refused the
// connection, and no runbook step branches on which one it was.

pub use crate::oneshot::exit::{
    EXIT_BAD_INPUT, EXIT_CONNECTION, EXIT_EXECUTION_FAILED, EXIT_OK, EXIT_UNSAFE_PLAN,
};

/// Ran to completion, but one or more documents aborted on a count mismatch and
/// were rolled back. Distinct from [`EXIT_CONNECTION`] on purpose: this is the
/// tool refusing to lie about counts, not the tool breaking.
///
/// The family-wide name for this is `EXIT_UNIT_ABORTED` — a "unit of work" is a
/// document here, a cluster in the merges. Same number, and the same meaning.
pub const EXIT_DOCUMENTS_ABORTED: u8 = crate::oneshot::exit::EXIT_UNIT_ABORTED;

/// What one referencing table did for one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableProof {
    /// `table.column`, because two of the eight are on one table.
    pub reference: String,
    /// Rows that pointed at an old id BEFORE the write.
    pub expected: u64,
    /// Rows the UPDATE actually changed.
    pub updated: u64,
}

impl TableProof {
    /// Whether the write did exactly what the pre-count said it would.
    ///
    /// Domain note: equality, not "at least". A row count HIGHER than expected
    /// means the UPDATE matched something the plan did not know about, which is
    /// as much of a failure as missing rows — and the reason the abort is on
    /// `!=` rather than `<`.
    pub fn is_sound(&self) -> bool {
        self.expected == self.updated
    }
}

/// What one document's unit of work did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentProof {
    pub doc_slug: String,
    /// Nodes whose id changed in the graph.
    pub nodes_rekeyed: u64,
    /// Nodes skipped because their id was already current.
    pub nodes_already_current: u64,
    /// Nodes refused because their key is shared (the twins).
    pub nodes_refused: u64,
    pub tables: Vec<TableProof>,
    /// `None` when the document completed; `Some(reason)` when it was aborted
    /// and rolled back.
    pub aborted: Option<String>,
}

impl DocumentProof {
    pub fn rows_updated(&self) -> u64 {
        self.tables.iter().map(|t| t.updated).sum()
    }

    pub fn rows_expected(&self) -> u64 {
        self.tables.iter().map(|t| t.expected).sum()
    }
}

/// The whole run.
#[derive(Debug, Clone)]
pub struct RunReport {
    /// `false` for a dry run — the default.
    pub applied: bool,
    pub totals: PlanTotals,
    pub documents: Vec<DocumentProof>,
    /// Shared-key groups, enumerated for the twin-merge session.
    pub refused_groups: Vec<(String, Vec<String>)>,
}

impl RunReport {
    pub fn rows_updated(&self) -> u64 {
        self.documents.iter().map(DocumentProof::rows_updated).sum()
    }

    pub fn nodes_rekeyed(&self) -> u64 {
        self.documents.iter().map(|d| d.nodes_rekeyed).sum()
    }

    pub fn aborted_documents(&self) -> Vec<&DocumentProof> {
        self.documents
            .iter()
            .filter(|d| d.aborted.is_some())
            .collect()
    }

    /// The process exit code this run earns.
    ///
    /// ## Why aborts get their own number (ruled 2026-08-14)
    ///
    /// The exit code is the runbook's machine-readable answer to "did everything
    /// re-key?", and `0` must not cover a run where a document was rolled back.
    /// An abort and a crash must also never share a number, or the runbook cannot
    /// tell "the tool broke" from "the tool refused to lie about counts" — the
    /// second needs investigation of the DATA, the first of the TOOL.
    ///
    /// Domain note: the refused twins do NOT affect this. They are a planned
    /// disposition — 42 nodes are expected to take it on every run until the
    /// merge session happens — and a tool that exited non-zero for doing exactly
    /// what it was designed to do would train an operator to ignore its code.
    pub fn exit_code(&self) -> u8 {
        if self.aborted_documents().is_empty() {
            EXIT_OK
        } else {
            EXIT_DOCUMENTS_ABORTED
        }
    }

    /// Build the report skeleton from a plan, before any execution.
    ///
    /// A dry run renders exactly this: the plan's own numbers, with no table
    /// proofs, because nothing was counted against the database.
    pub fn from_plan(plan: &RekeyPlan, applied: bool) -> Self {
        RunReport {
            applied,
            totals: plan.totals(),
            documents: Vec::new(),
            refused_groups: plan.refused_groups(),
        }
    }

    /// Render the report file.
    ///
    /// Plain text with a fixed shape, so two runs diff cleanly against each
    /// other — the same reason the plan's ordering is stable.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let mode = if self.applied {
            "APPLIED — the database was written"
        } else {
            "DRY RUN — nothing was written"
        };
        let _ = writeln!(out, "=== EVIDENCE RE-KEY — {mode} ===\n");

        let t = self.totals;
        let _ = writeln!(out, "PLAN");
        let _ = writeln!(out, "  Evidence nodes seen      : {}", t.nodes_seen);
        let _ = writeln!(out, "  To re-key                : {}", t.to_rekey);
        let _ = writeln!(out, "  Already current (skipped): {}", t.already_current);
        let _ = writeln!(
            out,
            "  Refused, shared key      : {}  ({} groups)",
            t.refused_shared_key,
            self.refused_groups.len()
        );

        if self.applied {
            let _ = writeln!(out, "\nEXECUTION");
            let _ = writeln!(out, "  Nodes re-keyed           : {}", self.nodes_rekeyed());
            let _ = writeln!(out, "  Referencing rows updated : {}", self.rows_updated());
            let aborted = self.aborted_documents();
            let _ = writeln!(out, "  Documents aborted        : {}", aborted.len());
            for doc in aborted {
                let _ = writeln!(
                    out,
                    "    ! {} — {}",
                    doc.doc_slug,
                    doc.aborted.as_deref().unwrap_or("(no reason recorded)")
                );
            }
        }

        if !self.documents.is_empty() {
            let _ = writeln!(out, "\nPER DOCUMENT");
            for doc in &self.documents {
                let _ = writeln!(
                    out,
                    "\n  {}{}",
                    doc.doc_slug,
                    if doc.aborted.is_some() {
                        "  [ABORTED — rolled back]"
                    } else {
                        ""
                    }
                );
                let _ = writeln!(
                    out,
                    "    nodes: {} re-keyed · {} already current · {} refused",
                    doc.nodes_rekeyed, doc.nodes_already_current, doc.nodes_refused
                );
                for table in &doc.tables {
                    let _ = writeln!(
                        out,
                        "    {:<48} expected {:>4}  updated {:>4}{}",
                        table.reference,
                        table.expected,
                        table.updated,
                        if table.is_sound() {
                            ""
                        } else {
                            "   <-- MISMATCH"
                        }
                    );
                }
            }
        }

        if !self.refused_groups.is_empty() {
            let _ = writeln!(
                out,
                "\nREFUSED — SHARED KEY ({} groups; these keep their current ids)",
                self.refused_groups.len()
            );
            let _ = writeln!(
                out,
                "  No disambiguator exists for these by ruling of 2026-08-14: the twins"
            );
            let _ = writeln!(
                out,
                "  differ only in LLM-mooded fields, and some carry different weights, so"
            );
            let _ = writeln!(
                out,
                "  any positional tiebreaker could swap a ruling. They are the twin-merge"
            );
            let _ = writeln!(out, "  session's work, not this tool's.\n");
            for (shared, ids) in &self.refused_groups {
                let _ = writeln!(out, "  would-be id {shared}");
                for id in ids {
                    let _ = writeln!(out, "    retains {id}");
                }
            }
        }

        out
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
