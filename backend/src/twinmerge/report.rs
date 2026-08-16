//! The count proof and the human queue — what the merge claims it did, and what
//! it refused to do without Roman.
//!
//! Two outputs, deliberately two files. The proof is for checking a run; the
//! queue is the agenda for a merge session and gets read on its own, hours later,
//! by someone who does not want to scroll past a table of row counts to find it.

use std::fmt::Write as _;

use super::plan::{ClusterPlan, Disposition, PlanTotals, TwinPlan};
use crate::oneshot::exit::{EXIT_OK, EXIT_UNIT_ABORTED};

/// What one referencing column did — the family-wide proof type.
///
/// Re-exported rather than redefined: all four one-shot tools count the same
/// thing the same way, and three copies of one struct is how a `!=` quietly
/// becomes a `>=` in one of them. See [`crate::oneshot::refs::TableProof`].
pub use crate::oneshot::refs::TableProof;

/// What one cluster's unit of work did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterProof {
    pub key: String,
    pub survivor: String,
    pub losers: Vec<String>,
    pub tables: Vec<TableProof>,
    /// Graph nodes actually deleted.
    pub nodes_deleted: u64,
    /// Edges actually deleted with them.
    pub edges_deleted: u64,
    /// `None` when the cluster completed; `Some(reason)` when it was rolled back.
    pub aborted: Option<String>,
}

impl ClusterProof {
    pub fn rows_updated(&self) -> u64 {
        self.tables.iter().map(|t| t.updated).sum()
    }
}

/// The whole run.
#[derive(Debug, Clone)]
pub struct RunReport {
    /// `false` for a dry run — the default.
    pub applied: bool,
    pub totals: PlanTotals,
    pub clusters: Vec<ClusterProof>,
    /// Every refused cluster, rendered for the human queue.
    pub queue: Vec<QueueEntry>,
}

/// One cluster the tool will not merge, with everything a merge session needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueEntry {
    pub key: String,
    pub doc_slug: String,
    pub page: Option<i64>,
    pub quote: String,
    pub question: Option<String>,
    pub reason: String,
    /// One line per member: what it is and what is attached to it.
    pub members: Vec<String>,
}

impl RunReport {
    pub fn rows_updated(&self) -> u64 {
        self.clusters.iter().map(ClusterProof::rows_updated).sum()
    }

    pub fn nodes_deleted(&self) -> u64 {
        self.clusters.iter().map(|c| c.nodes_deleted).sum()
    }

    pub fn aborted_clusters(&self) -> Vec<&ClusterProof> {
        self.clusters
            .iter()
            .filter(|c| c.aborted.is_some())
            .collect()
    }

    /// The process exit code this run earns.
    ///
    /// Domain note: the refused clusters do NOT affect this. Seven pairs are
    /// expected to go to the human queue on every run until the merge session
    /// happens, and a tool that exited non-zero for doing exactly what it was
    /// designed to do would train an operator to ignore its code.
    pub fn exit_code(&self) -> u8 {
        if self.aborted_clusters().is_empty() {
            EXIT_OK
        } else {
            EXIT_UNIT_ABORTED
        }
    }

    /// Build the skeleton from a plan, before any execution.
    pub fn from_plan(plan: &TwinPlan, applied: bool) -> Self {
        RunReport {
            applied,
            totals: plan.totals(),
            clusters: Vec::new(),
            queue: plan.refusals().map(queue_entry).collect(),
        }
    }

    /// Render the count proof.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let mode = if self.applied {
            "APPLIED — the database was written"
        } else {
            "DRY RUN — nothing was written"
        };
        let _ = writeln!(out, "=== EVIDENCE TWIN MERGE — {mode} ===\n");

        let t = self.totals;
        let _ = writeln!(out, "PLAN");
        let _ = writeln!(out, "  Same-key clusters seen   : {}", t.clusters);
        let _ = writeln!(out, "  Nodes in those clusters  : {}", t.nodes_seen);
        let _ = writeln!(out, "  Clusters to merge        : {}", t.clusters_to_merge);
        let _ = writeln!(out, "  Nodes to delete          : {}", t.nodes_to_delete);
        let _ = writeln!(
            out,
            "  Refused, curated on 2+   : {}",
            t.clusters_refused_curated
        );
        let _ = writeln!(
            out,
            "  Refused, edges diverge   : {}",
            t.clusters_refused_edges
        );

        if self.applied {
            let _ = writeln!(out, "\nEXECUTION");
            let _ = writeln!(out, "  Nodes deleted            : {}", self.nodes_deleted());
            let _ = writeln!(out, "  Referencing rows updated : {}", self.rows_updated());
            let aborted = self.aborted_clusters();
            let _ = writeln!(out, "  Clusters aborted         : {}", aborted.len());
            for cluster in aborted {
                let _ = writeln!(
                    out,
                    "    ! {} — {}",
                    cluster.key,
                    cluster.aborted.as_deref().unwrap_or("(no reason recorded)")
                );
            }
        }

        self.render_per_cluster(&mut out);
        self.render_queue_summary(&mut out);
        out
    }

    fn render_per_cluster(&self, out: &mut String) {
        if self.clusters.is_empty() {
            return;
        }
        let _ = writeln!(out, "\nPER CLUSTER");
        for cluster in &self.clusters {
            let flag = if cluster.aborted.is_some() {
                "  [ABORTED — rolled back]"
            } else {
                ""
            };
            let _ = writeln!(out, "\n  {}{}", cluster.key, flag);
            let _ = writeln!(out, "    survives: {}", cluster.survivor);
            for loser in &cluster.losers {
                let _ = writeln!(out, "    merged in: {loser}");
            }
            let _ = writeln!(
                out,
                "    graph: {} node(s) deleted, {} edge(s) deleted",
                cluster.nodes_deleted, cluster.edges_deleted
            );
            for table in &cluster.tables {
                let mismatch = if table.is_sound() {
                    ""
                } else {
                    "   <-- MISMATCH"
                };
                let _ = writeln!(
                    out,
                    "    {:<48} expected {:>4}  updated {:>4}{}",
                    table.reference, table.expected, table.updated, mismatch
                );
            }
        }
    }

    fn render_queue_summary(&self, out: &mut String) {
        if self.queue.is_empty() {
            return;
        }
        let _ = writeln!(
            out,
            "\nHUMAN QUEUE — {} cluster(s) NOT merged; see the queue file",
            self.queue.len()
        );
        for entry in &self.queue {
            let _ = writeln!(out, "  {} — {}", entry.key, entry.reason);
        }
    }

    /// Render the human queue as its own file.
    ///
    /// Written even when empty, and saying so — an absent file is ambiguous
    /// between "nothing was refused" and "the tool never got that far", and those
    /// need different responses from the operator.
    pub fn render_queue(&self) -> String {
        let mut out = String::from("=== EVIDENCE TWIN MERGE — HUMAN QUEUE ===\n\n");
        if self.queue.is_empty() {
            out.push_str(
                "No cluster was refused. Every same-key cluster the plan found was \
                 mergeable\nwithout a ruling. Nothing here needs Roman.\n",
            );
            return out;
        }
        let _ = writeln!(
            out,
            "{} cluster(s) need a ruling. The tool merged nothing in any of them and\n\
             left every node and every row exactly as it found them.\n",
            self.queue.len()
        );
        for (n, entry) in self.queue.iter().enumerate() {
            let _ = writeln!(out, "── {} ─────────────────────────────────────", n + 1);
            let _ = writeln!(out, "  key      : {}", entry.key);
            let _ = writeln!(out, "  reason   : {}", entry.reason);
            let _ = writeln!(
                out,
                "  document : {} page {}",
                entry.doc_slug,
                entry
                    .page
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "—".to_string())
            );
            if let Some(question) = &entry.question {
                let _ = writeln!(out, "  question : {question}");
            }
            let _ = writeln!(out, "  quote    : {}", entry.quote);
            for member in &entry.members {
                let _ = writeln!(out, "    {member}");
            }
            out.push('\n');
        }
        out
    }
}

/// Turn a refused cluster into its queue entry.
fn queue_entry(cluster: &ClusterPlan) -> QueueEntry {
    let first = &cluster.members[0];
    let (reason, members) = match &cluster.disposition {
        Disposition::RefusedMultipleCurated { curated } => (
            format!(
                "{} of {} members carry curated rows — only Roman can decide which \
                 ruling survives",
                curated.len(),
                cluster.members.len()
            ),
            cluster
                .members
                .iter()
                .map(|m| format!("{} — {} curated row(s)", m.id(), m.curated_rows))
                .collect(),
        ),
        Disposition::RefusedEdgeDivergence {
            survivor,
            extra_edges,
        } => (
            "a member holds edges the survivor does not; deleting it would lose them".to_string(),
            std::iter::once(format!("{survivor} — proposed survivor"))
                .chain(extra_edges.iter().map(|(loser, edges)| {
                    format!(
                        "{loser} — holds edges the survivor lacks: {}",
                        edges.join(", ")
                    )
                }))
                .collect(),
        ),
        // Not reachable: `refusals()` filters merges out before this is called.
        // Handled rather than unwrapped so a future disposition cannot panic here.
        Disposition::Merge { survivor, .. } => (
            "not refused".to_string(),
            vec![format!("{survivor} — survivor")],
        ),
    };

    QueueEntry {
        key: cluster.key.clone(),
        doc_slug: cluster.doc_slug.clone(),
        page: first.row.page,
        quote: first.row.verbatim_quote.clone(),
        question: first.row.question.clone(),
        reason,
        members,
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
