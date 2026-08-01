// =============================================================================
// backend/src/domain/case_state/partition.rs — the connection-tier vocabulary
// =============================================================================
//
// L1 of the `case_state` family (see `mod.rs` for the family's layering law).
// This file partitions the graph's Evidence→Allegation edge classes into tiers
// and stops; the verdicts that later phases compute on top of it are L2 and live
// beside this file, never inside it.
//
// The Case Health dashboard answers one question at the structural level: how
// much of the Evidence we extracted is actually WIRED INTO the case, versus
// sitting inert in the graph? "Wired in" means an Evidence node carries at least
// one edge to an `Allegation`. But not every such edge is worth the same, so the
// answer is reported in TWO TIERS, and this file is where those two tiers are
// defined — once, in code.
//
// ## Domain note: why two tiers, and why ABOUT is excluded from the headline
//
// Four Evidence→Allegation edge classes exist in the v5 schema:
//
//   CORROBORATES  — this item independently confirms what the allegation asserts
//   REBUTS        — this item counters/defeats what the allegation asserts
//   CHARACTERIZES — this item evaluates conduct/character the allegation concerns
//   ABOUT         — this item is merely ON THE SUBJECT of the allegation
//
// The first three each say something about whether the allegation is TRUE — they
// move the proof one way or the other. `ABOUT` deliberately says nothing of the
// kind; it is a topical pointer, and the schemas describe it that way. Counting
// ABOUT toward the headline connection rate therefore inflates precisely the
// number this instrument exists to keep honest: a corpus of 500 items all merely
// "about" the case would report as fully connected while proving nothing.
//
// So:
//
//   PROBATIVE tier = {CORROBORATES, REBUTS, CHARACTERIZES}  ← the headline
//   TOPICAL   tier = PROBATIVE ∪ {ABOUT}                    ← shown beside it
//
// Both are displayed, separately labeled, never blended. The topical rate is not
// noise — it is the honest measure of "we have material here that the edge layer
// could promote"; it just is not the same claim as "this evidence proves things."
//
// ## Why this mirrors `fact_role`, and why it is code and not config
//
// `actor_role` (D1) and `fact_role` (D2b task 1.3) established this repo's
// sanctioned convention for a small vocabulary: a Rust enum plus a versioned
// list, whose tokens are pinned by test to the `neo4j::schema` relationship
// constants. This file is the third resident and is deliberately the same shape.
//
// Standing Rule 2 asks whether the value varies across environments, cases, or
// configurations. It does not. Which edge classes are probative is a fact about
// the v5 SCHEMA, not about any one case — every Colossus case built on this
// schema partitions its Evidence→Allegation edges exactly this way, so another
// case renders with zero code changes (the reusability checkpoint passes). A
// deployment can no more have a different probative set than it can have a
// different `CORROBORATES` edge. Changing the partition is a code change with a
// version bump, exactly like `FACT_ROLE_LOOKUP_V`.
//
// What deliberately does NOT live here: any threshold, cut-off, or verdict. This
// file partitions edge classes and stops. Pane 1 reports rates; it renders no
// verdict, and Chunk 1 introduces no verdict vocabulary (the codebase has exactly
// one, `causes_of_action_builder::derive_proof_status`, which Pane 2 will wrap).

use serde::{Deserialize, Serialize};

use crate::neo4j::schema;

/// The version of the connection-tier vocabulary THIS build defines.
///
/// Bumped whenever an edge class moves between tiers or a tier is added/removed,
/// exactly like [`crate::domain::fact_role::FACT_ROLE_LOOKUP_V`]. A stored
/// snapshot (a later chunk attaches history) or a downstream reader can compare
/// against this to notice that a rate it is holding was computed under a
/// different partition and is therefore not comparable.
///
/// ## Rust Learning: a `pub const` for a build-time coupling invariant
///
/// This is a compile-time constant, not a config value: the partition is code,
/// and changing it is a code change with a matching version bump. It can never
/// vary per deployment, so it does not belong in YAML/env — same rationale as
/// `FACT_ROLE_LOOKUP_V` and `ACTOR_ROLE_LOOKUP_V`.
///
/// This one stays `pub` while the tier sets below do not, and the difference is
/// the whole point of the layering law: a VERSION is a fact about the partition
/// that outsiders legitimately need (the payload carries it so a reader can tell
/// which definition produced a rate). A tier SET is the definition itself, and
/// handing that out is what lets a second definition of "connected" grow.
pub const CONNECTION_TIER_LOOKUP_V: u32 = 1;

/// The Evidence→Allegation edge classes that bear on whether the allegation is
/// true. The headline tier.
///
/// ## Rust Learning: `&'static [&'static str]` built from other consts
///
/// A `const` slice may reference other `const`s, so the relationship names come
/// from `neo4j::schema` rather than being re-typed as literals (Rule 16). A
/// rename in `schema.rs` therefore flows here at compile time; there is no
/// string to forget to update. `&'static` on both the slice and its items means
/// the whole thing lives in the binary with no allocation and no lifetime to
/// thread through call sites.
///
/// ## Rust Learning: a `const` with no `pub` — visibility as enforcement
///
/// Omitting `pub` makes this item private to `partition.rs`: `use` from any
/// other module is a compile ERROR, not a lint. That is deliberate and it is the
/// layering law's entire mechanism. Callers reach the same slice through
/// [`ConnectionTier::edge_types`], which means the only way to ask "what counts
/// as probative?" is to hold a `ConnectionTier` and ask it — you cannot answer
/// the question by copying a list, because you cannot see the list. Rust makes
/// "single source of truth" a property the compiler checks rather than a
/// convention review has to catch.
const PROBATIVE_EDGE_TYPES: &[&str] =
    &[schema::CORROBORATES, schema::REBUTS, schema::CHARACTERIZES];

/// The edge classes that make an item topically connected but say nothing about
/// whether the allegation is true. Exactly the difference between the two tiers.
///
/// Kept as its own const (rather than being implied by subtracting one list from
/// another) so the "what does topical add?" question has a single, greppable
/// answer, and so `topical_is_probative_plus_topical_only` can assert the two
/// lists compose correctly.
///
/// ## Why the `allow(dead_code)`
///
/// This const is read only by that composition test, so a non-test build sees no
/// consumer. That was true before A0 as well; it was merely invisible, because
/// the const was `pub` and a `pub` item is never reported as dead. Narrowing the
/// visibility surfaced the fact rather than creating it. The right answer is to
/// keep the declaration and silence the lint HERE, with this note: deleting it
/// would inline `ABOUT` into the composition test and lose the one place that
/// names the difference between the tiers, and exporting an accessor to give it
/// a production caller would widen the public surface to satisfy a lint — the
/// tail wagging the dog.
#[allow(dead_code)]
const TOPICAL_ONLY_EDGE_TYPES: &[&str] = &[schema::ABOUT];

/// Every Evidence→Allegation edge class, probative first, in display order.
///
/// This ordering is load-bearing twice over: it is the tier-2 membership set,
/// and it is the column order of the per-document breakdown table the dashboard
/// renders. Both read it from here so the table can never drift from the tier
/// definition.
const TOPICAL_EDGE_TYPES: &[&str] = &[
    schema::CORROBORATES,
    schema::REBUTS,
    schema::CHARACTERIZES,
    schema::ABOUT,
];

/// Which tier an Evidence item's connection to an Allegation falls in.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on an enum
///
/// serde renders each variant using its Rust name by default (`Probative`). The
/// `rename_all` attribute maps every variant to its `snake_case` wire token
/// (`probative`) in one line, so the token the payload carries and the token a
/// future stored snapshot records stay identical without per-variant renames. An
/// unknown token fails to deserialize rather than defaulting — the loud boundary
/// Standing Rule 1 asks for.
///
/// ## Rust Learning: deriving `Copy` on a fieldless enum
///
/// The enum holds no data, so it is one discriminant and cheap to copy. Deriving
/// `Copy` lets callers pass a tier by value without `.clone()`, and the tier
/// stays usable afterwards. `PartialEq`/`Eq` let tests assert exact round-trips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionTier {
    /// At least one CORROBORATES / REBUTS / CHARACTERIZES edge to an Allegation.
    /// This is the dashboard headline.
    Probative,
    /// At least one edge of ANY of the four classes to an Allegation — the
    /// probative set plus ABOUT. Displayed beside the headline, never blended
    /// into it.
    Topical,
}

impl ConnectionTier {
    /// The full, ordered tier vocabulary — the "extensible list in code."
    ///
    /// Adding a tier means adding a variant AND an entry here AND bumping
    /// [`CONNECTION_TIER_LOOKUP_V`]. Any caller that enumerates the vocabulary
    /// reads it from ONE place rather than re-listing the variants.
    pub const ALL: &'static [ConnectionTier] =
        &[ConnectionTier::Probative, ConnectionTier::Topical];

    /// The stable wire token for this tier (matches the serde `snake_case` name).
    ///
    /// Useful where a `&str` is needed — a log line, an error naming the tier,
    /// a documented query's comment — without routing through serde. The
    /// `tier_tokens_match_serde` test pins the two representations together.
    pub fn code(self) -> &'static str {
        match self {
            ConnectionTier::Probative => "probative",
            ConnectionTier::Topical => "topical",
        }
    }

    /// The Evidence→Allegation edge classes that count as a connection at this
    /// tier.
    ///
    /// This is the single function every query builder and every classifier must
    /// call. Nothing else in the codebase may hard-code "CORROBORATES, REBUTS,
    /// CHARACTERIZES" — that would be the second copy from which drift starts.
    ///
    /// ## Rust Learning: a public accessor returning a private const
    ///
    /// The returned slice's TYPE (`&'static [&'static str]`) is public, so this
    /// compiles even though the constants it names are private — privacy governs
    /// who may write the PATH `PROBATIVE_EDGE_TYPES`, not what may flow out of a
    /// function. That asymmetry is what makes the pattern work: outsiders get the
    /// data, but only ever by asking a tier for it, so every read is routed
    /// through the partition and shows up in a search for `edge_types()`.
    pub fn edge_types(self) -> &'static [&'static str] {
        match self {
            ConnectionTier::Probative => PROBATIVE_EDGE_TYPES,
            ConnectionTier::Topical => TOPICAL_EDGE_TYPES,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_to_snake_case_tokens() {
        // The wire tokens are the contract the payload (and a future stored
        // snapshot) depend on — pin them exactly.
        assert_eq!(json!(ConnectionTier::Probative), json!("probative"));
        assert_eq!(json!(ConnectionTier::Topical), json!("topical"));
    }

    #[test]
    fn rejects_unknown_tier_token() {
        // Standing Rule 1: a tier this build does not define is a LOUD parse
        // error, never a silent default to the more flattering tier.
        let result: Result<ConnectionTier, _> = serde_json::from_value(json!("connected"));
        assert!(result.is_err(), "an unknown tier token must not parse");
    }

    #[test]
    fn all_lists_every_variant_once() {
        // Guards `ALL` against drifting from the enum.
        assert_eq!(ConnectionTier::ALL.len(), 2);
        assert!(ConnectionTier::ALL.contains(&ConnectionTier::Probative));
        assert!(ConnectionTier::ALL.contains(&ConnectionTier::Topical));
    }

    #[test]
    fn tier_tokens_match_serde() {
        for &tier in ConnectionTier::ALL {
            assert_eq!(json!(tier), json!(tier.code()));
        }
    }

    #[test]
    fn probative_is_exactly_the_three_truth_bearing_edges() {
        // The ratified headline basis. If someone widens this set, the headline
        // silently inflates and the dashboard stops doing its job — so pin it.
        assert_eq!(
            PROBATIVE_EDGE_TYPES,
            &[schema::CORROBORATES, schema::REBUTS, schema::CHARACTERIZES]
        );
    }

    #[test]
    fn about_is_excluded_from_the_probative_tier() {
        // The single most important invariant in this file: ABOUT must never
        // count toward the headline. Asserted directly, not merely implied by
        // the list above, so the intent survives a future edit to that list.
        assert!(
            !PROBATIVE_EDGE_TYPES.contains(&schema::ABOUT),
            "ABOUT is non-probative and must never enter the headline tier"
        );
        assert!(TOPICAL_EDGE_TYPES.contains(&schema::ABOUT));
    }

    #[test]
    fn topical_is_probative_plus_topical_only() {
        // The two tiers compose: topical = probative ∪ topical-only, with no
        // duplicates and nothing invented. Catches an edge class added to one
        // list but not the other.
        let mut expected: Vec<&str> = PROBATIVE_EDGE_TYPES.to_vec();
        expected.extend_from_slice(TOPICAL_ONLY_EDGE_TYPES);
        assert_eq!(TOPICAL_EDGE_TYPES.to_vec(), expected);

        let mut deduped = TOPICAL_EDGE_TYPES.to_vec();
        deduped.sort_unstable();
        deduped.dedup();
        assert_eq!(
            deduped.len(),
            TOPICAL_EDGE_TYPES.len(),
            "an edge class is listed twice in TOPICAL_EDGE_TYPES"
        );
    }

    #[test]
    fn topical_is_a_strict_superset_of_probative() {
        // Every probative edge must also count as topical — otherwise an item
        // could be probative-connected but not topical-connected, which is
        // nonsense and would make the two rates non-comparable.
        for edge in PROBATIVE_EDGE_TYPES {
            assert!(
                TOPICAL_EDGE_TYPES.contains(edge),
                "probative edge {edge} missing from the topical tier"
            );
        }
        assert!(TOPICAL_EDGE_TYPES.len() > PROBATIVE_EDGE_TYPES.len());
    }

    #[test]
    fn edge_types_returns_the_tier_constants() {
        assert_eq!(
            ConnectionTier::Probative.edge_types(),
            PROBATIVE_EDGE_TYPES,
            "the accessor must return the same set the const declares"
        );
        assert_eq!(ConnectionTier::Topical.edge_types(), TOPICAL_EDGE_TYPES);
    }

    #[test]
    fn edge_names_come_from_the_schema_constants() {
        // Rule 16 / Rule 12: the relationship names are the `schema::` constants,
        // never re-typed literals. Asserting the literal spellings here means a
        // rename in schema.rs that forgets this file fails the build, and a
        // rename that reaches this file is a deliberate, visible edit.
        assert_eq!(schema::CORROBORATES, "CORROBORATES");
        assert_eq!(schema::REBUTS, "REBUTS");
        assert_eq!(schema::CHARACTERIZES, "CHARACTERIZES");
        assert_eq!(schema::ABOUT, "ABOUT");
    }
}
