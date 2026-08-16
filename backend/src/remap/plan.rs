//! Matching old Evidence nodes to new ones — pure, and therefore testable
//! without a database or a reprocess.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::api::pipeline::evidence_key::normalize;

/// One Evidence node as it stood before the reprocess.
///
/// ## Rust Learning: `Serialize` + `Deserialize` on the same struct
///
/// The snapshot is written by one invocation and read by another, possibly days
/// later, so it needs both halves of serde. Deriving them together is what makes
/// the file a durable record rather than an in-memory convenience — and JSON is
/// chosen here (unlike the human-edited proposal) precisely because nobody should
/// hand-edit a snapshot: it is the record of what WAS.
/// `deny_unknown_fields` on purpose: a snapshot is read by a LATER invocation,
/// possibly a later build, and a field this build does not understand means the
/// two disagree about what was captured. Silently ignoring it would let a remap
/// match against a record it only half understands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotNode {
    pub id: String,
    pub page: Option<i64>,
    pub verbatim_quote: String,
    pub question: Option<String>,
    /// How many curated rows referenced this node when the snapshot was taken.
    ///
    /// Domain note: this is what makes an unmatched node urgent or ignorable. An
    /// orphan with 0 curated rows costs nothing; an orphan with 12 is a piece of
    /// Roman's ruling about to fall on the floor, and the queue sorts on it.
    pub curated_rows: u64,
}

/// Everything captured before a reprocess.
/// `deny_unknown_fields` for the same reason as [`SnapshotNode`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub document_id: String,
    /// Free-text note of when and by what. Not parsed — a human reading a stale
    /// snapshot needs to know it is stale.
    pub taken_note: String,
    pub nodes: Vec<SnapshotNode>,
}

impl Snapshot {
    pub fn curated_nodes(&self) -> usize {
        self.nodes.iter().filter(|n| n.curated_rows > 0).count()
    }
}

/// One Evidence node as it stands after the reprocess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewNode {
    pub id: String,
    pub page: Option<i64>,
    pub verbatim_quote: String,
    pub question: Option<String>,
}

/// What happened to one old node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Match {
    /// The id survived the reprocess untouched. Nothing to remap.
    ///
    /// After the stable-id arm this should be the overwhelming majority, and a
    /// run where it is not is itself the finding.
    Unchanged,
    /// Exactly one new node has this key, and no other old node claims it.
    Unambiguous { new_id: String },
    /// More than one candidate on one side or the other. Never auto-applied.
    Ambiguous { candidates: Vec<String> },
    /// No new node carries this key. The rows are orphaned.
    Unmatched,
}

/// One old node with its outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedNode {
    pub old: SnapshotNode,
    pub outcome: Match,
}

/// The whole plan for one document.
#[derive(Debug, Clone)]
pub struct RemapPlan {
    pub document_id: String,
    pub nodes: Vec<MatchedNode>,
}

/// Per-plan totals — the per-document report the task asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlanTotals {
    pub old_nodes: usize,
    pub unchanged: usize,
    pub unambiguous: usize,
    pub ambiguous: usize,
    pub unmatched: usize,
    /// Curated rows attached to nodes that are ambiguous or unmatched — the
    /// number that measures the actual risk, as opposed to the node count.
    pub curated_rows_at_risk: u64,
}

impl PlanTotals {
    /// The yield the Morris gate test checks against the measured 87.8% floor.
    ///
    /// Counts `Unchanged` as a success, because it is the best possible outcome:
    /// the id survived and no row had to move at all.
    pub fn yield_percent(&self) -> f64 {
        if self.old_nodes == 0 {
            return 100.0;
        }
        let good = (self.unchanged + self.unambiguous) as f64;
        (good / self.old_nodes as f64) * 100.0
    }
}

/// The key two nodes must share to be the same statement.
///
/// `(page, normalized quote, normalized question)` — the task's key, and the
/// same normalization the stable-id arm uses, so the two cannot disagree about
/// what "the same quote" means.
fn match_key(page: Option<i64>, quote: &str, question: Option<&str>) -> String {
    format!(
        "{}\u{1f}{}\u{1f}{}",
        page.map(|p| p.to_string()).unwrap_or_default(),
        normalize(quote),
        question.map(normalize).unwrap_or_default()
    )
}

impl RemapPlan {
    /// Match a snapshot against the post-reprocess graph.
    pub fn build(snapshot: &Snapshot, new_nodes: &[NewNode]) -> Self {
        let new_by_key = index_new_nodes(new_nodes);
        let old_counts = count_old_keys(snapshot);
        let surviving: Vec<&str> = new_nodes.iter().map(|n| n.id.as_str()).collect();

        let nodes = snapshot
            .nodes
            .iter()
            .map(|old| {
                let key = match_key(old.page, &old.verbatim_quote, old.question.as_deref());
                MatchedNode {
                    outcome: decide(old, &key, &new_by_key, &old_counts, &surviving),
                    old: old.clone(),
                }
            })
            .collect();

        RemapPlan {
            document_id: snapshot.document_id.clone(),
            nodes,
        }
    }

    pub fn totals(&self) -> PlanTotals {
        let mut t = PlanTotals {
            old_nodes: self.nodes.len(),
            ..PlanTotals::default()
        };
        for node in &self.nodes {
            match &node.outcome {
                Match::Unchanged => t.unchanged += 1,
                Match::Unambiguous { .. } => t.unambiguous += 1,
                Match::Ambiguous { .. } => {
                    t.ambiguous += 1;
                    t.curated_rows_at_risk += node.old.curated_rows;
                }
                Match::Unmatched => {
                    t.unmatched += 1;
                    t.curated_rows_at_risk += node.old.curated_rows;
                }
            }
        }
        t
    }

    /// The `(old id, new id)` pairs that may be auto-applied.
    pub fn auto_moves(&self) -> Vec<(String, String)> {
        self.nodes
            .iter()
            .filter_map(|n| match &n.outcome {
                Match::Unambiguous { new_id } => Some((n.old.id.clone(), new_id.clone())),
                _ => None,
            })
            .collect()
    }

    /// Nodes needing a human, worst first — most curated rows at the top.
    pub fn queue(&self) -> Vec<&MatchedNode> {
        let mut queue: Vec<&MatchedNode> = self
            .nodes
            .iter()
            .filter(|n| matches!(n.outcome, Match::Ambiguous { .. } | Match::Unmatched))
            .collect();
        queue.sort_by(|a, b| {
            b.old
                .curated_rows
                .cmp(&a.old.curated_rows)
                .then(a.old.id.cmp(&b.old.id))
        });
        queue
    }
}

/// New nodes grouped by match key.
fn index_new_nodes(new_nodes: &[NewNode]) -> BTreeMap<String, Vec<String>> {
    let mut index: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in new_nodes {
        let key = match_key(node.page, &node.verbatim_quote, node.question.as_deref());
        index.entry(key).or_default().push(node.id.clone());
    }
    for ids in index.values_mut() {
        ids.sort();
    }
    index
}

/// How many OLD nodes hold each key.
///
/// The twin class means two old nodes can share a key. If they do, neither can
/// be matched unambiguously even when exactly one new node exists — one of them
/// would silently take the other's rows.
fn count_old_keys(snapshot: &Snapshot) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for node in &snapshot.nodes {
        let key = match_key(node.page, &node.verbatim_quote, node.question.as_deref());
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

/// Decide one old node's outcome.
fn decide(
    old: &SnapshotNode,
    key: &str,
    new_by_key: &BTreeMap<String, Vec<String>>,
    old_counts: &BTreeMap<String, usize>,
    surviving: &[&str],
) -> Match {
    if surviving.contains(&old.id.as_str()) {
        return Match::Unchanged;
    }
    let candidates = match new_by_key.get(key) {
        Some(ids) => ids,
        None => return Match::Unmatched,
    };
    let old_holders = old_counts.get(key).copied().unwrap_or(1);
    if candidates.len() == 1 && old_holders == 1 {
        return Match::Unambiguous {
            new_id: candidates[0].clone(),
        };
    }
    Match::Ambiguous {
        candidates: candidates.clone(),
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
