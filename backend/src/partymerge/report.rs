//! The count proof for the party merge — including the one number the addendum
//! names as acceptance: statements conserved, per cluster.
//!
//! ## Why statement conservation is proved rather than assumed
//!
//! The whole point of merging Tighe is that 101 sworn statements stop being split
//! across two nodes. If the merge dropped an edge, the People page would show one
//! tidy node with 62 statements and nothing anywhere would say that 39 went
//! missing — a fragmented judge is visible, a silently truncated one is not. So
//! the count is taken before, taken again after, and a cluster whose total moved
//! is rolled back whole.

use std::fmt::Write as _;

use super::plan::{Disposition, MergePlan, PlanTotals};
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
    pub label: String,
    pub survivor: String,
    pub merged_in: Vec<String>,
    /// Statements the survivor must end up with: its own plus every member's.
    pub statements_expected: u64,
    /// Statements the survivor actually has, counted in the graph after the
    /// write and before the commit.
    pub statements_after: u64,
    pub nodes_deleted: u64,
    pub edges_repointed: u64,
    /// Name variants recorded on the survivor, so the merged names stay findable.
    pub aliases_added: Vec<String>,
    pub tables: Vec<TableProof>,
    pub aborted: Option<String>,
}

impl ClusterProof {
    /// The addendum's acceptance test for one cluster.
    pub fn statements_conserved(&self) -> bool {
        self.statements_expected == self.statements_after
    }

    pub fn rows_updated(&self) -> u64 {
        self.tables.iter().map(|t| t.updated).sum()
    }

    /// The first thing wrong with this cluster, if anything is — `None` when it
    /// may commit.
    ///
    /// ## Why this is a method on the proof and not a helper in `execute`
    ///
    /// It is a DECISION, and decisions do not belong in a module that cannot be
    /// unit-tested. Three distinct conditions abort a cluster and they are not
    /// interchangeable: a Postgres count mismatch means a referencing row moved
    /// that the plan did not know about; a statement mismatch means the graph
    /// merge lost sworn testimony; a node-count mismatch means the delete did
    /// not do what it was told. An operator reads exactly this string, so each
    /// gets its own sentence.
    ///
    /// Order is deliberate: the Postgres proof is checked first because it is
    /// the cheapest to interpret and the most likely to be a real data problem.
    pub fn failure_reason(&self) -> Option<String> {
        if let Some(bad) = self.tables.iter().find(|t| !t.is_sound()) {
            return Some(format!(
                "{}: expected {}, updated {}",
                bad.reference, bad.expected, bad.updated
            ));
        }
        if !self.statements_conserved() {
            return Some(format!(
                "statements: expected {}, found {} after the merge",
                self.statements_expected, self.statements_after
            ));
        }
        if self.nodes_deleted != self.merged_in.len() as u64 {
            return Some(format!(
                "nodes deleted: expected {}, deleted {}",
                self.merged_in.len(),
                self.nodes_deleted
            ));
        }
        None
    }
}

/// The whole run.
#[derive(Debug, Clone)]
pub struct RunReport {
    pub applied: bool,
    pub totals: PlanTotals,
    /// One line per cluster, from the plan — including the skips, which are as
    /// much a record of the session as the merges.
    pub planned: Vec<(String, String)>,
    pub clusters: Vec<ClusterProof>,
}

impl RunReport {
    pub fn nodes_deleted(&self) -> u64 {
        self.clusters.iter().map(|c| c.nodes_deleted).sum()
    }

    pub fn aborted_clusters(&self) -> Vec<&ClusterProof> {
        self.clusters
            .iter()
            .filter(|c| c.aborted.is_some())
            .collect()
    }

    pub fn exit_code(&self) -> u8 {
        if self.aborted_clusters().is_empty() {
            EXIT_OK
        } else {
            EXIT_UNIT_ABORTED
        }
    }

    /// Build the skeleton from a plan, before any execution.
    pub fn from_plan(plan: &MergePlan, applied: bool) -> Self {
        let planned = plan
            .clusters
            .iter()
            .map(|c| (c.label.clone(), describe(&c.disposition)))
            .collect();
        RunReport {
            applied,
            totals: plan.totals(),
            planned,
            clusters: Vec::new(),
        }
    }

    /// Render the count proof.
    pub fn render(&self) -> String {
        let mut out = String::new();
        let mode = if self.applied {
            "APPLIED — the graph was written"
        } else {
            "DRY RUN — nothing was written"
        };
        let _ = writeln!(out, "=== PARTY MERGE — {mode} ===\n");

        let t = self.totals;
        let _ = writeln!(out, "PLAN (from the rulings file)");
        let _ = writeln!(out, "  Clusters ruled           : {}", t.clusters_ruled);
        let _ = writeln!(out, "  To merge                 : {}", t.clusters_to_merge);
        let _ = writeln!(out, "  Skipped by ruling        : {}", t.clusters_skipped);
        let _ = writeln!(
            out,
            "  Already merged (no-op)   : {}",
            t.clusters_already_merged
        );
        let _ = writeln!(
            out,
            "  Nodes merging in         : {}   <- the People page must drop by exactly this",
            t.nodes_to_merge_in
        );
        let _ = writeln!(out, "  Statements moving        : {}", t.statements_to_move);

        let _ = writeln!(out, "\nPER RULING");
        for (label, disposition) in &self.planned {
            let _ = writeln!(out, "  {label} — {disposition}");
        }

        if self.applied {
            self.render_execution(&mut out);
        }
        self.render_per_cluster(&mut out);
        out
    }

    fn render_execution(&self, out: &mut String) {
        let _ = writeln!(out, "\nEXECUTION");
        let _ = writeln!(out, "  Nodes deleted            : {}", self.nodes_deleted());
        let conserved = self
            .clusters
            .iter()
            .filter(|c| c.aborted.is_none() && c.statements_conserved())
            .count();
        let _ = writeln!(
            out,
            "  Clusters conserving statements : {}/{}",
            conserved,
            self.clusters.len()
        );
        let aborted = self.aborted_clusters();
        let _ = writeln!(out, "  Clusters aborted         : {}", aborted.len());
        for cluster in aborted {
            let _ = writeln!(
                out,
                "    ! {} — {}",
                cluster.label,
                cluster.aborted.as_deref().unwrap_or("(no reason recorded)")
            );
        }
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
            let _ = writeln!(out, "\n  {}{}", cluster.label, flag);
            let _ = writeln!(out, "    survivor : {}", cluster.survivor);
            for member in &cluster.merged_in {
                let _ = writeln!(out, "    merged in: {member}");
            }
            let verdict = if cluster.statements_conserved() {
                ""
            } else {
                "   <-- STATEMENTS LOST"
            };
            let _ = writeln!(
                out,
                "    statements: expected {} · after {}{}",
                cluster.statements_expected, cluster.statements_after, verdict
            );
            let _ = writeln!(
                out,
                "    graph: {} node(s) deleted, {} edge(s) repointed",
                cluster.nodes_deleted, cluster.edges_repointed
            );
            if !cluster.aliases_added.is_empty() {
                let _ = writeln!(
                    out,
                    "    aliases recorded on the survivor: {}",
                    cluster.aliases_added.join(", ")
                );
            }
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
}

/// One line describing what the plan decided about a cluster.
fn describe(disposition: &Disposition) -> String {
    match disposition {
        Disposition::Merge {
            survivor,
            members,
            expected_statements,
        } => format!(
            "merge {} node(s) into {} ({} statement(s) expected after)",
            members.len(),
            survivor.id,
            expected_statements
        ),
        Disposition::Skipped { reason } => format!("SKIPPED — {reason}"),
        Disposition::AlreadyMerged { survivor } => {
            format!("already merged into {survivor} on an earlier run — nothing to do")
        }
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
