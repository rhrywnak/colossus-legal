//! Deciding what the Evidence re-key will do — pure, and therefore testable
//! without a database (task ID_ARM, P1b).
//!
//! The execution half talks to two stores and cannot be unit-tested; this half
//! makes every DECISION and touches nothing. Given the Evidence rows as the graph
//! reports them, it produces a plan: which nodes get a new id, which are already
//! current, and which are refused. The binary's dry-run mode prints exactly this
//! plan and stops, so what Roman reviews before `--apply` is the same object the
//! apply path executes.
//!
//! ## The three dispositions, and why refusal is one of them
//!
//! Measured on DEV 2026-08-14: 21 pairs of Evidence nodes produce the SAME key
//! under the new arm — the same statement extracted twice, identical on every
//! stable field. Seven pairs carry curated rows on BOTH twins and three carry
//! CONFLICTING weights (one `carries`, one `backup`, in one scenario). So the
//! twins are not interchangeable, and any tiebreaker that assigns ids by position
//! could swap Roman's weights on the demo-facing scenarios.
//!
//! Ruled 2026-08-14: **no disambiguator, ever.** A key with more than one holder
//! is REFUSED — both twins keep the ids they have — and the pair is enumerated in
//! the report for the separate twin-merge session. Refusing is a first-class
//! outcome here, not an error: 42 of 525 nodes are expected to take it.
//!
//! ## Why `AlreadyCurrent` exists
//!
//! Idempotency, which the ruling requires: a run interrupted after three
//! documents must be resumable. A node whose stored id already equals its
//! computed id has been done, so a second run plans nothing for it. This also
//! means the FIRST run and the SECOND run of a completed re-key produce
//! recognisably different plans (483 re-keys vs 483 already-current), which is
//! how an operator can tell from the dry-run alone whether the work has run.

use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::api::pipeline::evidence_key::evidence_id;

/// One Evidence node, as the graph reports it.
///
/// `doc_slug` is `Document.id`, which is already a slug — the same value the
/// ingest arm keys on, so the plan cannot compute a key from a different
/// spelling of the document than the pipeline would.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRow {
    /// The id the node carries TODAY.
    pub current_id: String,
    pub doc_slug: String,
    pub page: Option<i64>,
    pub verbatim_quote: String,
    pub question: Option<String>,
}

/// What the re-key will do with one node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    /// Give it this new id, and update every row that references the old one.
    Rekey { new_id: String },
    /// Its stored id already equals its computed id — nothing to do.
    AlreadyCurrent,
    /// Its key is shared with another node. Both keep their current ids.
    ///
    /// `key_holders` is how many nodes share the key (2 for every measured case;
    /// carried rather than assumed so a future 3-way shows up as itself).
    RefusedSharedKey { new_id: String, key_holders: usize },
}

/// One node with its decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedNode {
    pub row: EvidenceRow,
    pub disposition: Disposition,
}

impl PlannedNode {
    /// The new id, when this node is actually being re-keyed.
    pub fn rekey_target(&self) -> Option<&str> {
        match &self.disposition {
            Disposition::Rekey { new_id } => Some(new_id),
            _ => None,
        }
    }
}

/// The whole plan, grouped into the unit of work the ruling specifies.
///
/// ## Why `BTreeMap` and not `HashMap`
///
/// The unit of work is ONE DOCUMENT, and the operator reads a per-document count
/// proof. A `HashMap` would order those sections differently on every run, so two
/// dry-runs of identical data would produce reports that diff against each other
/// for no reason. `BTreeMap` sorts by document slug and the report is stable.
#[derive(Debug, Clone, Default)]
pub struct RekeyPlan {
    pub by_document: BTreeMap<String, Vec<PlannedNode>>,
}

/// Per-plan totals, for the count proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlanTotals {
    pub nodes_seen: usize,
    pub to_rekey: usize,
    pub already_current: usize,
    pub refused_shared_key: usize,
}

impl RekeyPlan {
    /// Build the plan from every Evidence row in the graph.
    ///
    /// The collision pass runs over ALL rows before any disposition is decided,
    /// which is the whole reason this takes the full set rather than streaming
    /// per document: two nodes sharing a key are (in every measured case) in the
    /// same document, but nothing guarantees that, and a per-document pass would
    /// silently re-key a cross-document collision.
    pub fn build(rows: Vec<EvidenceRow>) -> Self {
        // Pass 1 — compute every key and count its holders.
        let keyed: Vec<(EvidenceRow, String)> = rows
            .into_iter()
            .map(|row| {
                let id = evidence_id(
                    &row.doc_slug,
                    row.page,
                    &row.verbatim_quote,
                    row.question.as_deref(),
                );
                (row, id)
            })
            .collect();

        let mut holders: HashMap<&str, usize> = HashMap::new();
        for (_, new_id) in &keyed {
            *holders.entry(new_id.as_str()).or_insert(0) += 1;
        }
        // Own the counts so the borrow on `keyed` ends before we consume it.
        let holders: HashMap<String, usize> = holders
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        // Pass 2 — decide, and group by document.
        let mut by_document: BTreeMap<String, Vec<PlannedNode>> = BTreeMap::new();
        for (row, new_id) in keyed {
            let count = holders.get(&new_id).copied().unwrap_or(1);
            let disposition = if count > 1 {
                Disposition::RefusedSharedKey {
                    new_id,
                    key_holders: count,
                }
            } else if row.current_id == new_id {
                Disposition::AlreadyCurrent
            } else {
                Disposition::Rekey { new_id }
            };
            by_document
                .entry(row.doc_slug.clone())
                .or_default()
                .push(PlannedNode { row, disposition });
        }

        // Stable order WITHIN a document too, for the same diffability reason.
        for nodes in by_document.values_mut() {
            nodes.sort_by(|a, b| a.row.current_id.cmp(&b.row.current_id));
        }

        RekeyPlan { by_document }
    }

    /// Every planned node, in document order.
    pub fn nodes(&self) -> impl Iterator<Item = &PlannedNode> {
        self.by_document.values().flatten()
    }

    /// The counts an operator checks the run against.
    pub fn totals(&self) -> PlanTotals {
        let mut t = PlanTotals::default();
        for node in self.nodes() {
            t.nodes_seen += 1;
            match node.disposition {
                Disposition::Rekey { .. } => t.to_rekey += 1,
                Disposition::AlreadyCurrent => t.already_current += 1,
                Disposition::RefusedSharedKey { .. } => t.refused_shared_key += 1,
            }
        }
        t
    }

    /// The refused nodes, grouped by the key they share — the twin enumeration
    /// the completion report owes.
    ///
    /// Returns `(shared_new_id, current_ids)` sorted, so the list is stable.
    pub fn refused_groups(&self) -> Vec<(String, Vec<String>)> {
        let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for node in self.nodes() {
            if let Disposition::RefusedSharedKey { new_id, .. } = &node.disposition {
                groups
                    .entry(new_id.clone())
                    .or_default()
                    .push(node.row.current_id.clone());
            }
        }
        for ids in groups.values_mut() {
            ids.sort();
        }
        groups.into_iter().collect()
    }

    /// Whether any planned NEW id collides with an id already in use by a
    /// DIFFERENT node that is not itself being re-keyed.
    ///
    /// ## Why this check exists at all
    ///
    /// The new ids are an 8-hex-character digest. Over 525 nodes the birthday
    /// risk is tiny but not zero, and the failure it would cause is the worst one
    /// available: two statements MERGEd into one node, with one set of curated
    /// rows silently pointing at the survivor. Unlike the mooded-blob hash this
    /// replaces, that outcome is DETECTABLE before anything is written — so it is
    /// detected, and the run refuses rather than discovering it afterwards.
    ///
    /// Returns the offending `(new_id, current_ids)` pairs; empty means safe.
    pub fn target_conflicts(&self) -> Vec<(String, Vec<String>)> {
        // Every id that will exist AFTER the run: re-key targets, plus the
        // current ids of everything not being re-keyed.
        let mut claims: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for node in self.nodes() {
            let after = match &node.disposition {
                Disposition::Rekey { new_id } => new_id.clone(),
                _ => node.row.current_id.clone(),
            };
            claims
                .entry(after)
                .or_default()
                .push(node.row.current_id.clone());
        }
        claims
            .into_iter()
            .filter(|(_, holders)| holders.len() > 1)
            .map(|(id, mut holders)| {
                holders.sort();
                (id, holders)
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
