//! Diffing primitives: content hashing, per-node managed-property field lists,
//! change classification, and the small value types the plan and report share.
//!
//! Split out of [`super::plan`] to keep each module within the 300-line limit.
//! [`super::plan`] re-exports [`ChangeKind`], [`NodePlan`], and [`Tally`], so
//! the rest of the crate refers to them as `plan::ChangeKind` etc.

use super::schema::{DeclarationDef, ElementDef, TheoryDef};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// How a single node compares to what's already in the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// No node with this key exists yet.
    Created,
    /// A node exists but its managed properties differ (or were never hashed).
    Updated,
    /// A node exists and its content hash already matches — skip the write.
    Unchanged,
}

/// A planned upsert for one node, carrying the parsed definition, its computed
/// content hash (written alongside the node), and its classification.
#[derive(Debug, Clone)]
pub struct NodePlan<T> {
    pub def: T,
    pub hash: String,
    pub kind: ChangeKind,
}

/// Created/updated/unchanged counts for a slice of node plans.
#[derive(Debug, Default, Clone, Copy)]
pub struct Tally {
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
}

impl Tally {
    /// Tally a slice of node plans by classification.
    pub fn of<T>(nodes: &[NodePlan<T>]) -> Self {
        let mut t = Tally::default();
        for n in nodes {
            match n.kind {
                ChangeKind::Created => t.created += 1,
                ChangeKind::Updated => t.updated += 1,
                ChangeKind::Unchanged => t.unchanged += 1,
            }
        }
        t
    }
}

/// ## Rust Learning: `AddAssign`
///
/// Implementing `+=` lets the report fold per-Count tallies into a grand total
/// with `total += Tally::of(&count.elements)`, instead of summing each field
/// by hand at every call site.
impl std::ops::AddAssign for Tally {
    fn add_assign(&mut self, rhs: Self) {
        self.created += rhs.created;
        self.updated += rhs.updated;
        self.unchanged += rhs.unchanged;
    }
}

/// Deterministic hash over a node's managed properties.
///
/// Fields are sorted by name, then each name/value pair is fed to SHA-256 with
/// explicit separators. A `None` value and `Some("")` hash differently (a
/// distinct marker byte), so "property absent" and "property empty" stay
/// distinguishable (Standing Rule 1).
pub(crate) fn content_hash(fields: &[(&str, Option<String>)]) -> String {
    let mut sorted: Vec<&(&str, Option<String>)> = fields.iter().collect();
    sorted.sort_by_key(|(name, _)| *name);

    let mut hasher = Sha256::new();
    for (name, value) in sorted {
        hasher.update(name.as_bytes());
        hasher.update([0u8]); // name/value separator
        match value {
            Some(s) => {
                hasher.update([1u8]); // "present" marker
                hasher.update(s.as_bytes());
            }
            None => hasher.update([2u8]), // distinct "absent" marker
        }
        hasher.update([0u8]); // field separator
    }
    format!("{:x}", hasher.finalize())
}

// The field lists below MUST match the SET clauses in `cypher.rs`, so the hash
// reflects exactly what is stored. `parent_count_id` is included because it is
// a managed property even though it is derived from the Count.

/// The hashed property set for an `Element` node, as `(name, value)` pairs.
///
/// Mirrors the `SET` clause of [`super::cypher::upsert_element`]. Optional
/// fields pass through their `Option` so absent/empty stay distinguishable;
/// `order_in_count` and the derived `parent_count_id` are stringified.
pub(crate) fn element_fields(
    e: &ElementDef,
    count_number: u32,
) -> Vec<(&'static str, Option<String>)> {
    vec![
        ("element_name", Some(e.element_name.clone())),
        ("title", Some(e.title.clone())),
        ("order_in_count", Some(e.order_in_count.to_string())),
        (
            "what_plaintiff_must_prove",
            Some(e.what_plaintiff_must_prove.clone()),
        ),
        (
            "controlling_authority",
            Some(e.controlling_authority.clone()),
        ),
        ("statutory_anchor", e.statutory_anchor.clone()),
        ("case_specific_notes", e.case_specific_notes.clone()),
        ("theory_variant", e.theory_variant.clone()),
        ("parent_count_id", Some(count_number.to_string())),
    ]
}

/// The hashed property set for a `BreachTheory` / `ImproperActTheory` node.
///
/// Mirrors the `SET` clause of [`super::cypher::upsert_breach_theory`] /
/// [`super::cypher::upsert_improper_act_theory`]. `statutory_anchor` is `None`
/// for improper-act theories, so that property simply never materializes there.
pub(crate) fn theory_fields(
    t: &TheoryDef,
    count_number: u32,
) -> Vec<(&'static str, Option<String>)> {
    vec![
        ("definition", Some(t.definition.clone())),
        ("statutory_anchor", t.statutory_anchor.clone()),
        ("awad_examples", Some(t.awad_examples.clone())),
        ("parent_count_id", Some(count_number.to_string())),
    ]
}

/// The hashed property set for a `DeclarationSought` node.
///
/// Mirrors the `SET` clause of [`super::cypher::upsert_declaration`]. The
/// `operative` bool is stringified; `inoperative_reason` passes through its
/// `Option` (present only for non-operative declarations).
pub(crate) fn declaration_fields(
    d: &DeclarationDef,
    count_number: u32,
) -> Vec<(&'static str, Option<String>)> {
    vec![
        ("declaration", Some(d.declaration.clone())),
        ("legal_basis", Some(d.legal_basis.clone())),
        ("operative", Some(d.operative.to_string())),
        ("inoperative_reason", d.inoperative_reason.clone()),
        ("parent_count_id", Some(count_number.to_string())),
    ]
}

/// Classify a node by comparing its desired hash against the stored hash.
pub(crate) fn classify(
    key: &str,
    existing: &HashMap<String, Option<String>>,
    desired_hash: &str,
) -> ChangeKind {
    match existing.get(key) {
        None => ChangeKind::Created,
        Some(None) => ChangeKind::Updated, // exists but predates content hashing
        Some(Some(h)) if h == desired_hash => ChangeKind::Unchanged,
        Some(Some(_)) => ChangeKind::Updated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    #[test]
    fn content_hash_is_deterministic_and_order_independent() {
        let a = content_hash(&[("x", s("1")), ("y", s("2"))]);
        let b = content_hash(&[("x", s("1")), ("y", s("2"))]);
        // Same fields in a different order hash identically (fields are sorted).
        let c = content_hash(&[("y", s("2")), ("x", s("1"))]);
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn content_hash_distinguishes_none_from_empty_string() {
        // Standing Rule 1: "property absent" and "property empty" must differ.
        let absent = content_hash(&[("field", None)]);
        let empty = content_hash(&[("field", s(""))]);
        assert_ne!(absent, empty);
    }

    #[test]
    fn content_hash_changes_when_a_value_changes() {
        let before = content_hash(&[("title", s("Duty"))]);
        let after = content_hash(&[("title", s("Duty (revised)"))]);
        assert_ne!(before, after);
    }

    #[test]
    fn tally_counts_each_change_kind() {
        let nodes = vec![
            NodePlan {
                def: 0u8,
                hash: String::new(),
                kind: ChangeKind::Created,
            },
            NodePlan {
                def: 0u8,
                hash: String::new(),
                kind: ChangeKind::Created,
            },
            NodePlan {
                def: 0u8,
                hash: String::new(),
                kind: ChangeKind::Updated,
            },
            NodePlan {
                def: 0u8,
                hash: String::new(),
                kind: ChangeKind::Unchanged,
            },
        ];
        let t = Tally::of(&nodes);
        assert_eq!((t.created, t.updated, t.unchanged), (2, 1, 1));
    }

    #[test]
    fn tally_add_assign_sums_fields() {
        let mut total = Tally::default();
        total += Tally {
            created: 1,
            updated: 2,
            unchanged: 3,
        };
        total += Tally {
            created: 10,
            updated: 20,
            unchanged: 30,
        };
        assert_eq!(
            (total.created, total.updated, total.unchanged),
            (11, 22, 33)
        );
    }
}
