//! Canonical names of the Neo4j relationship types the data model defines.
//!
//! ## Why constants and not configuration
//!
//! These are graph-schema identifiers fixed at data-model time, not
//! environment- or case-specific configuration. They do not vary across
//! deployments; changing one requires a graph migration, not a config edit,
//! so they are deliberately compiled constants rather than config values
//! (Standing Rule 2 does not apply to schema identifiers). This mirrors the
//! node-label constant block in `canonical_elements::cypher`.
//!
//! Centralized here so that every read query, loader, and test references one
//! constant. A bare `-[:SOME_NAME]->` literal scattered across query strings
//! is how the previous name drifted out of sync during a rename; one constant
//! makes the next rename a single-line edit and lets the rules-enforcer ban
//! the literals (see `.claude/agents/rules-enforcer.md`, Rule 12).
//!
//! ## Rust Learning: `&'static str` constants
//!
//! A `pub const NAME: &str = "…"` is a compile-time constant string slice with
//! `'static` lifetime — it lives in the binary's read-only data, costs nothing
//! to reference, and can be interpolated into a `format!` Cypher template the
//! same way a literal would be. Because Cypher cannot *parameterize* a
//! relationship type (only node properties), interpolation is the only way to
//! keep the type name in one place; the values here are trusted, code-defined
//! literals, never user input, so that interpolation carries no injection risk.

/// `Allegation -[:BEARS_ON]-> Element`. The central reasoning relationship:
/// the Allegation's facts help establish (bear on) this specific Element of a
/// Count's legal theory. Persisted via `authored_relationships` (cross-tier:
/// extracted Allegation → canonical Element). At trial the attorney must prove
/// each Element, so this edge is the proof-chain backbone.
pub const BEARS_ON: &str = "BEARS_ON";

/// `LegalCount -[:HAS_ELEMENT]-> Element`. Structural: the Count is composed of
/// these Elements. Reconstructed mechanically from `Element.parent_count_id`.
pub const HAS_ELEMENT: &str = "HAS_ELEMENT";

/// `Element -[:HAS_THEORY]-> (BreachTheory | ImproperActTheory)`. One Element
/// can be satisfied by several theories of *how* it was met; the discriminator
/// property on the edge distinguishes the theory kind.
pub const HAS_THEORY: &str = "HAS_THEORY";

/// `LegalCount -[:SEEKS_DECLARATION]-> DeclarationSought`. The declaratory
/// relief the Count asks the court to grant.
pub const SEEKS_DECLARATION: &str = "SEEKS_DECLARATION";

/// `Allegation -[:ABOUT]-> Party`. The Allegation concerns (is against /
/// regarding) this party.
pub const ABOUT: &str = "ABOUT";

/// `MotionClaim -[:PROVES]-> Allegation`. A claim made in a motion proves /
/// asserts the Allegation.
pub const PROVES: &str = "PROVES";

/// `MotionClaim -[:RELIES_ON]-> Evidence`. The claim is supported by this piece
/// of evidence.
pub const RELIES_ON: &str = "RELIES_ON";

/// `Evidence -[:CORROBORATES]-> Allegation`. A discovery/evidence item
/// independently confirms (corroborates) a complaint Allegation.
///
/// Domain note: this is the cross-document proof edge the discovery pass-2
/// extraction authors — Phillips' sworn admission corroborating a complaint
/// paragraph. Combined with `BEARS_ON` + `HAS_ELEMENT` it forms the proof
/// chain `Evidence → Allegation → Element → LegalCount` the Proof Matrix walks.
/// Label-only (no edge properties).
pub const CORROBORATES: &str = "CORROBORATES";

/// `Evidence -[:CONTAINED_IN]-> Document`. The evidence item appears within
/// this source document.
pub const CONTAINED_IN: &str = "CONTAINED_IN";

/// `Statement -[:STATED_BY]-> Party`. The speaker who made the statement.
///
/// Domain note: `STATED_BY` is the speaker who made the statement under oath;
/// it is distinct from `ABOUT` (who the statement concerns) — different
/// relationships, different queries.
pub const STATED_BY: &str = "STATED_BY";

/// `Statement -[:CHARACTERIZES]-> target`. The statement frames or
/// characterizes the target (e.g. a party, act, or event).
pub const CHARACTERIZES: &str = "CHARACTERIZES";

/// `MotionClaim -[:CONTRADICTS]-> target`. The claim directly contradicts the
/// target statement or allegation.
pub const CONTRADICTS: &str = "CONTRADICTS";

/// `MotionClaim -[:REBUTS]-> target`. The claim rebuts (answers/refutes) the
/// target.
pub const REBUTS: &str = "REBUTS";

/// `Allegation -[:CAUSED_BY]-> cause`. The harm or event was caused by the
/// linked node.
pub const CAUSED_BY: &str = "CAUSED_BY";

/// `entity -[:APPEARS_IN]-> Document`. The entity is mentioned in / appears in
/// the document.
pub const APPEARS_IN: &str = "APPEARS_IN";

/// `Allegation -[:SUPPORTS]-> LegalCount`. Legacy count-level support edge;
/// also the *rendered* label the graph view shows for the synthetic
/// Allegation→Count link that masks the `BEARS_ON`+`HAS_ELEMENT` hops.
pub const SUPPORTS: &str = "SUPPORTS";

#[cfg(test)]
mod tests {
    use super::*;

    /// Disk/code invariant (Standing Rule 21, applied to schema identifiers):
    /// the constant *value* is the wire/graph string. A typo here would silently
    /// build queries that match nothing, so assert the exact spelling.
    #[test]
    fn relationship_constant_values_are_exact() {
        assert_eq!(BEARS_ON, "BEARS_ON");
        assert_eq!(HAS_ELEMENT, "HAS_ELEMENT");
        assert_eq!(HAS_THEORY, "HAS_THEORY");
        assert_eq!(SEEKS_DECLARATION, "SEEKS_DECLARATION");
        assert_eq!(ABOUT, "ABOUT");
        assert_eq!(PROVES, "PROVES");
        assert_eq!(RELIES_ON, "RELIES_ON");
        assert_eq!(CORROBORATES, "CORROBORATES");
        assert_eq!(CONTAINED_IN, "CONTAINED_IN");
        assert_eq!(STATED_BY, "STATED_BY");
        assert_eq!(CHARACTERIZES, "CHARACTERIZES");
        assert_eq!(CONTRADICTS, "CONTRADICTS");
        assert_eq!(REBUTS, "REBUTS");
        assert_eq!(CAUSED_BY, "CAUSED_BY");
        assert_eq!(APPEARS_IN, "APPEARS_IN");
        assert_eq!(SUPPORTS, "SUPPORTS");
    }
}
