//! Matching old Evidence nodes to new ones — pure, and therefore testable
//! without a database or a reprocess.

use serde::{Deserialize, Serialize};

use super::normalize::NearMatchSettings;
use super::tiers;

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

/// How strong the evidence for a match is.
///
/// ## Rust Learning: a fieldless enum with a `label`, not a `String`
///
/// The tier is a closed set of three, so it is an enum: an invalid tier cannot be
/// constructed, `match` on it is exhaustive, and the type is `Copy`. The rendered
/// text lives in one place (`label`) rather than at every call site, so the
/// proposal, the queue and the log can never drift into calling the same tier
/// three different things.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchTier {
    /// Page, quote and question agree byte for byte after the stable-id arm's
    /// normalization. The strongest evidence there is short of the id surviving.
    Exact,
    /// They agree once case, hyphenated line breaks, whitespace and a trailing
    /// `.`/`,` are set aside. A typographic difference, not a difference in what
    /// the document says.
    Normalized,
    /// Neither form agrees, but both near-match measures clear their thresholds.
    /// The score is printed next to every such match because it is the only tier
    /// where a human is being asked to trust a number.
    Near,
}

impl MatchTier {
    /// The short name used in the proposal, the queue and the logs.
    pub fn label(self) -> &'static str {
        match self {
            MatchTier::Exact => "tier1-exact",
            MatchTier::Normalized => "tier2-normalized",
            MatchTier::Near => "tier3-near",
        }
    }
}

/// What happened to one old node.
///
/// ## Rust Learning: why this derives `PartialEq` but not `Eq`
///
/// `Near` carries an `f64` score, and `f64` is `PartialEq` but not `Eq` because
/// NaN is not equal to itself. Deriving only `PartialEq` is the honest signature:
/// the type can be compared in a test with `assert_eq!`, and nothing can put it
/// in a `HashSet` or a `BTreeMap` key where a total ordering would be assumed.
#[derive(Debug, Clone, PartialEq)]
pub enum Match {
    /// The id survived the reprocess untouched. Nothing to remap.
    ///
    /// After the stable-id arm this should be the overwhelming majority, and a
    /// run where it is not is itself the finding.
    Unchanged,
    /// Exactly one new node matched at this tier, and no other old node claims
    /// it. `score` is 1.0 for the two exact tiers and the measured similarity for
    /// a near match.
    Unambiguous {
        new_id: String,
        tier: MatchTier,
        score: f64,
    },
    /// More than one candidate on one side or the other, at the tier that found
    /// them. Never auto-applied.
    Ambiguous {
        candidates: Vec<String>,
        tier: MatchTier,
    },
    /// No new node matched at any tier. The rows are orphaned.
    Unmatched,
}

/// One old node with its outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchedNode {
    pub old: SnapshotNode,
    pub outcome: Match,
}

/// One move the proposal may offer, with the evidence behind it.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoMove {
    pub old_id: String,
    pub new_id: String,
    pub tier: MatchTier,
    /// 1.0 at the two exact tiers; the measured similarity at tier 3.
    pub score: f64,
    pub page: Option<i64>,
    /// What is actually at stake if this line is approved wrongly.
    pub curated_rows: u64,
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
    /// Unambiguous matches broken down by the tier that found them. A run whose
    /// yield rests on tier 3 is a different run from one that rests on tier 1,
    /// and collapsing them into one number would hide that (Standing Rule 1).
    pub unambiguous_exact: usize,
    pub unambiguous_normalized: usize,
    pub unambiguous_near: usize,
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

impl RemapPlan {
    /// Match a snapshot against the post-reprocess graph.
    ///
    /// `settings` carries the tier-3 thresholds so a document can be measured at
    /// several settings without a rebuild; [`NearMatchSettings::default`] is the
    /// documented pair.
    pub fn build(snapshot: &Snapshot, new_nodes: &[NewNode], settings: NearMatchSettings) -> Self {
        RemapPlan {
            document_id: snapshot.document_id.clone(),
            nodes: tiers::match_all(snapshot, new_nodes, settings),
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
                Match::Unambiguous { tier, .. } => {
                    t.unambiguous += 1;
                    match tier {
                        MatchTier::Exact => t.unambiguous_exact += 1,
                        MatchTier::Normalized => t.unambiguous_normalized += 1,
                        MatchTier::Near => t.unambiguous_near += 1,
                    }
                }
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

    /// The moves that may be auto-applied, each carrying the evidence behind it.
    ///
    /// Returns a struct rather than an `(old, new)` pair because the tier and the
    /// score are not decoration: they are what the human approving the proposal
    /// reads to decide whether to keep the line. A pair would have forced the
    /// renderer to go looking for them again, and a renderer that has to look
    /// something up is a renderer that can fail to.
    pub fn auto_moves(&self) -> Vec<AutoMove> {
        self.nodes
            .iter()
            .filter_map(|n| match &n.outcome {
                Match::Unambiguous {
                    new_id,
                    tier,
                    score,
                } => Some(AutoMove {
                    old_id: n.old.id.clone(),
                    new_id: new_id.clone(),
                    tier: *tier,
                    score: *score,
                    page: n.old.page,
                    curated_rows: n.old.curated_rows,
                }),
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

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "plan_tier_tests.rs"]
mod tier_tests;

#[cfg(test)]
#[path = "plan_guard_tests.rs"]
mod guard_tests;
