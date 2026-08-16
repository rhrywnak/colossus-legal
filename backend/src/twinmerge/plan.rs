//! Deciding what the twin merge will do — pure, and therefore testable without a
//! database.
//!
//! Every DECISION is here; everything that touches a store is in
//! [`super::execute`]. The dry run prints exactly the object the apply path
//! executes, so what Roman reviews is what runs.

use std::collections::BTreeMap;

use crate::api::pipeline::evidence_key::evidence_id;
use crate::rekey::plan::EvidenceRow;

/// One Evidence node with everything the merge decision needs about it.
///
/// The three fields come from three different places — the graph (`row`),
/// Postgres (`curated_rows`) and the graph again (`relationships`) — and are
/// gathered before planning starts so that planning itself is pure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwinNode {
    pub row: EvidenceRow,
    /// How many rows across [`crate::oneshot::refs::EVIDENCE_CURATED_REFERENCES`]
    /// point at this node. Zero means nothing of Roman's is attached to it.
    pub curated_rows: u64,
    /// Sorted `TYPE->other_node_id` fingerprints of this node's edges.
    ///
    /// A fingerprint rather than the edge itself, because the only question
    /// asked of it is set membership — "does the survivor already have this
    /// edge?" — and a string compares in one operation.
    pub relationships: Vec<String>,
}

impl TwinNode {
    pub fn id(&self) -> &str {
        &self.row.current_id
    }

    fn is_curated(&self) -> bool {
        self.curated_rows > 0
    }
}

/// What the merge will do with one cluster of same-key nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    /// Collapse the cluster onto `survivor`, which then takes `target_id`.
    Merge {
        survivor: String,
        losers: Vec<String>,
        /// The id the survivor ends up with: the stable-arm key the whole
        /// cluster computes to. This is the point of the exercise — after the
        /// merge the key has exactly one holder, so the next `rekey_evidence`
        /// run has nothing to refuse.
        target_id: String,
    },
    /// More than one member carries curated rows. Refused; nothing is touched.
    RefusedMultipleCurated {
        /// `(node id, curated row count)` for every curated member, so the human
        /// queue can say what is at stake without a second query.
        curated: Vec<(String, u64)>,
    },
    /// A loser holds an edge the survivor does not, so deleting it would lose
    /// something. Refused; nothing is touched.
    RefusedEdgeDivergence {
        survivor: String,
        /// `(loser id, the fingerprints the survivor lacks)`.
        extra_edges: Vec<(String, Vec<String>)>,
    },
}

/// One cluster of nodes sharing a stable-arm key, with its decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterPlan {
    /// The key every member computes to. Also the survivor's id after a merge.
    pub key: String,
    pub doc_slug: String,
    pub members: Vec<TwinNode>,
    pub disposition: Disposition,
}

impl ClusterPlan {
    pub fn is_merge(&self) -> bool {
        matches!(self.disposition, Disposition::Merge { .. })
    }

    /// The nodes this cluster would delete. Empty unless it is a merge.
    pub fn losers(&self) -> &[String] {
        match &self.disposition {
            Disposition::Merge { losers, .. } => losers,
            _ => &[],
        }
    }
}

/// The whole plan: every cluster of two or more same-key nodes.
///
/// Singletons are absent by construction, not filtered later — a node with a
/// unique key is not a twin and this tool has nothing to say about it.
///
/// `BTreeMap` for the same reason the re-key uses one: two dry runs of unchanged
/// data must produce reports that diff cleanly.
#[derive(Debug, Clone, Default)]
pub struct TwinPlan {
    pub clusters: Vec<ClusterPlan>,
}

/// Per-plan totals, for the count proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlanTotals {
    pub nodes_seen: usize,
    pub clusters: usize,
    pub clusters_to_merge: usize,
    pub clusters_refused_curated: usize,
    pub clusters_refused_edges: usize,
    /// Nodes that would be deleted — one less than each merging cluster's size.
    pub nodes_to_delete: usize,
}

impl TwinPlan {
    /// Build the plan from every Evidence node in the graph.
    pub fn build(nodes: Vec<TwinNode>) -> Self {
        let mut grouped: BTreeMap<String, Vec<TwinNode>> = BTreeMap::new();
        for node in nodes {
            let key = evidence_id(
                &node.row.doc_slug,
                node.row.page,
                &node.row.verbatim_quote,
                node.row.question.as_deref(),
            );
            grouped.entry(key).or_default().push(node);
        }

        let mut clusters = Vec::new();
        for (key, mut members) in grouped {
            if members.len() < 2 {
                continue;
            }
            // Stable order inside a cluster, so the survivor choice and the
            // report are both independent of the order the graph returned rows.
            members.sort_by(|a, b| a.id().cmp(b.id()));
            let doc_slug = members[0].row.doc_slug.clone();
            let disposition = decide(&key, &members);
            clusters.push(ClusterPlan {
                key,
                doc_slug,
                members,
                disposition,
            });
        }
        TwinPlan { clusters }
    }

    /// The counts an operator checks the run against.
    pub fn totals(&self) -> PlanTotals {
        let mut t = PlanTotals {
            clusters: self.clusters.len(),
            ..PlanTotals::default()
        };
        for cluster in &self.clusters {
            t.nodes_seen += cluster.members.len();
            match &cluster.disposition {
                Disposition::Merge { losers, .. } => {
                    t.clusters_to_merge += 1;
                    t.nodes_to_delete += losers.len();
                }
                Disposition::RefusedMultipleCurated { .. } => t.clusters_refused_curated += 1,
                Disposition::RefusedEdgeDivergence { .. } => t.clusters_refused_edges += 1,
            }
        }
        t
    }

    /// Clusters that will actually be executed.
    pub fn merges(&self) -> impl Iterator<Item = &ClusterPlan> {
        self.clusters.iter().filter(|c| c.is_merge())
    }

    /// Clusters going to the human queue, in report order.
    pub fn refusals(&self) -> impl Iterator<Item = &ClusterPlan> {
        self.clusters.iter().filter(|c| !c.is_merge())
    }

    /// Whether any merge target is an id already held by a node OUTSIDE its own
    /// cluster.
    ///
    /// ## Why this check exists
    ///
    /// The survivor takes the cluster's key as its new id. If some unrelated node
    /// already carries that id — a birthday collision in the 8-hex digest, or a
    /// half-finished earlier run — the merge would weld two different statements
    /// into one node and quietly point one set of curated rows at the wrong
    /// evidence. Detectable before anything is written, so it is detected.
    ///
    /// `all_ids` is every Evidence id in the graph. Returns the offending
    /// `(target_id, outside holder)` pairs; empty means safe.
    pub fn target_conflicts(&self, all_ids: &[String]) -> Vec<(String, String)> {
        let mut conflicts = Vec::new();
        for cluster in self.merges() {
            let Disposition::Merge { target_id, .. } = &cluster.disposition else {
                continue;
            };
            let inside: Vec<&str> = cluster.members.iter().map(TwinNode::id).collect();
            for holder in all_ids {
                if holder == target_id && !inside.contains(&holder.as_str()) {
                    conflicts.push((target_id.clone(), holder.clone()));
                }
            }
        }
        conflicts
    }
}

/// Decide one cluster.
///
/// Order matters and is the ruling's order: the curated refusal is checked
/// FIRST, so a pair that is both doubly-curated and edge-divergent is reported as
/// the thing Roman has to rule on, not as a mechanical divergence he would then
/// have to look past.
fn decide(key: &str, members: &[TwinNode]) -> Disposition {
    let curated: Vec<(String, u64)> = members
        .iter()
        .filter(|m| m.is_curated())
        .map(|m| (m.id().to_string(), m.curated_rows))
        .collect();

    if curated.len() > 1 {
        return Disposition::RefusedMultipleCurated { curated };
    }

    let survivor = choose_survivor(members);
    let survivor_edges: &[String] = members
        .iter()
        .find(|m| m.id() == survivor)
        .map(|m| m.relationships.as_slice())
        .unwrap_or_default();

    let extra_edges = edges_the_survivor_lacks(members, &survivor, survivor_edges);
    if !extra_edges.is_empty() {
        return Disposition::RefusedEdgeDivergence {
            survivor,
            extra_edges,
        };
    }

    let losers = members
        .iter()
        .map(|m| m.id().to_string())
        .filter(|id| id != &survivor)
        .collect();
    Disposition::Merge {
        survivor,
        losers,
        target_id: key.to_string(),
    }
}

/// Which member survives.
///
/// The curated member if there is one — moving Roman's rulings is work the tool
/// can avoid entirely by keeping them where they are. Otherwise the
/// lexicographically smallest id, which is deterministic and, unlike "the first
/// one the graph returned", identical on every run and on every host.
///
/// Note the caller has already refused clusters with MORE than one curated
/// member, so "the curated member" is unambiguous whenever it exists.
fn choose_survivor(members: &[TwinNode]) -> String {
    members
        .iter()
        .find(|m| m.is_curated())
        .or_else(|| members.iter().min_by_key(|m| m.id()))
        .map(|m| m.id().to_string())
        .unwrap_or_default()
}

/// For each loser, the edge fingerprints the survivor does not have.
fn edges_the_survivor_lacks(
    members: &[TwinNode],
    survivor: &str,
    survivor_edges: &[String],
) -> Vec<(String, Vec<String>)> {
    members
        .iter()
        .filter(|m| m.id() != survivor)
        .filter_map(|loser| {
            let missing: Vec<String> = loser
                .relationships
                .iter()
                .filter(|edge| !survivor_edges.contains(edge))
                .cloned()
                .collect();
            (!missing.is_empty()).then(|| (loser.id().to_string(), missing))
        })
        .collect()
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
