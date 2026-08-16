//! Turning Roman's rulings plus the live census into a plan — pure, and
//! therefore testable without a database.
//!
//! The plan is where the rulings meet reality: a node Roman named may not exist,
//! may already have been merged by an earlier run, or may be a different KIND of
//! thing than the survivor. Each of those is a different outcome and none of them
//! is a silent one.

use crate::partymerge::census::PartyNode;
use crate::partymerge::rulings::{ClusterRuling, Ruling, RulingsFile};

/// What the merge will do with one ruled cluster.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    /// Merge every member into the survivor.
    Merge {
        survivor: PartyNode,
        members: Vec<PartyNode>,
        /// The statement total the merged node must end up with.
        ///
        /// Domain note: this is the acceptance test the addendum names — Tighe's
        /// 39 + 62 must become one node with 101. It is computed BEFORE the write
        /// and checked after it; a cluster whose total moved is rolled back.
        expected_statements: u64,
    },
    /// Roman said leave it alone.
    Skipped { reason: String },
    /// Every member named is already gone: this cluster ran on an earlier
    /// attempt. Not an error — it is what idempotency looks like.
    AlreadyMerged { survivor: String },
}

/// One ruled cluster with its decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterPlan {
    pub label: String,
    pub disposition: Disposition,
}

impl ClusterPlan {
    pub fn is_merge(&self) -> bool {
        matches!(self.disposition, Disposition::Merge { .. })
    }
}

/// The whole plan.
#[derive(Debug, Clone, Default)]
pub struct MergePlan {
    pub clusters: Vec<ClusterPlan>,
}

/// Per-plan totals, for the count proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlanTotals {
    pub clusters_ruled: usize,
    pub clusters_to_merge: usize,
    pub clusters_skipped: usize,
    pub clusters_already_merged: usize,
    /// Nodes that will be deleted — the number the People page must drop by.
    pub nodes_to_merge_in: usize,
    /// Statements those nodes carry, all of which must land on a survivor.
    pub statements_to_move: u64,
}

/// Why a ruling could not be turned into a plan.
///
/// Every variant means "nothing was written and nothing will be" — these are
/// checked before execution starts, because a cluster refused half-way through a
/// run leaves the People page in a state nobody planned.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PlanError {
    #[error(
        "cluster '{label}': survivor '{survivor}' is not in the graph. Nothing \
         was written. Check the id against the census in the template"
    )]
    MissingSurvivor { label: String, survivor: String },

    #[error(
        "cluster '{label}': '{member}' is a {member_label} but survivor \
         '{survivor}' is a {survivor_label}. Merging across labels would weld a \
         person to an organisation; refused"
    )]
    LabelMismatch {
        label: String,
        member: String,
        member_label: String,
        survivor: String,
        survivor_label: String,
    },
}

impl MergePlan {
    /// Build the plan from the rulings and the live census.
    pub fn build(rulings: &RulingsFile, census: &[PartyNode]) -> Result<Self, PlanError> {
        let mut clusters = Vec::new();
        for ruling in &rulings.clusters {
            clusters.push(plan_cluster(ruling, census)?);
        }
        Ok(MergePlan { clusters })
    }

    pub fn totals(&self) -> PlanTotals {
        let mut t = PlanTotals {
            clusters_ruled: self.clusters.len(),
            ..PlanTotals::default()
        };
        for cluster in &self.clusters {
            match &cluster.disposition {
                Disposition::Merge {
                    members,
                    survivor,
                    expected_statements,
                } => {
                    t.clusters_to_merge += 1;
                    t.nodes_to_merge_in += members.len();
                    // What MOVES is the total minus what the survivor already
                    // had — the number an operator can check against the
                    // People page and the statement counts side by side.
                    t.statements_to_move += expected_statements - survivor.statement_count;
                }
                Disposition::Skipped { .. } => t.clusters_skipped += 1,
                Disposition::AlreadyMerged { .. } => t.clusters_already_merged += 1,
            }
        }
        t
    }

    pub fn merges(&self) -> impl Iterator<Item = &ClusterPlan> {
        self.clusters.iter().filter(|c| c.is_merge())
    }
}

/// Decide one cluster.
fn plan_cluster(ruling: &ClusterRuling, census: &[PartyNode]) -> Result<ClusterPlan, PlanError> {
    let label = ruling.label.clone();
    let (survivor_id, member_ids) = match &ruling.ruling {
        Ruling::Skip { reason } => {
            return Ok(ClusterPlan {
                label,
                disposition: Disposition::Skipped {
                    reason: reason.clone(),
                },
            })
        }
        Ruling::Merge { survivor, members } => (survivor, members),
    };

    let survivor = find(census, survivor_id).ok_or_else(|| PlanError::MissingSurvivor {
        label: label.clone(),
        survivor: survivor_id.clone(),
    })?;

    // A member that is gone has already been merged. That is idempotency, not a
    // failure: a run interrupted after three clusters must be safe to repeat.
    let members: Vec<PartyNode> = member_ids
        .iter()
        .filter_map(|id| find(census, id))
        .collect();
    if members.is_empty() {
        return Ok(ClusterPlan {
            label,
            disposition: Disposition::AlreadyMerged {
                survivor: survivor.id.clone(),
            },
        });
    }

    for member in &members {
        if member.label != survivor.label {
            return Err(PlanError::LabelMismatch {
                label,
                member: member.id.clone(),
                member_label: member.label.clone(),
                survivor: survivor.id.clone(),
                survivor_label: survivor.label.clone(),
            });
        }
    }

    let expected_statements =
        survivor.statement_count + members.iter().map(|m| m.statement_count).sum::<u64>();
    Ok(ClusterPlan {
        label,
        disposition: Disposition::Merge {
            survivor,
            members,
            expected_statements,
        },
    })
}

/// Look one node up in the census by id.
fn find(census: &[PartyNode], id: &str) -> Option<PartyNode> {
    census.iter().find(|p| p.id == id).cloned()
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
