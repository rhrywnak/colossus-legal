// =============================================================================
// backend/src/domain/fact_role.rs — the fact-role vocabulary (D2b, task 1.3)
// =============================================================================
//
// When Theme Scan (D2b) judges an Evidence quote against a scenario's accusation,
// it assigns the quote a ROLE describing how that quote bears on the accusation:
// it SUPPORTS it, CORROBORATES it, CONTRADICTS it, or REBUTS it. That vocabulary
// is defined HERE, once, in code — the fact-role counterpart to `actor_role`.
//
// ## Why this mirrors `actor_role` exactly
//
// `actor_role` (D1) established the sanctioned "code-owned lookup" convention: a
// small Rust enum plus a versioned list, NOT a Postgres enum (a migration per
// role is heavy and couples the vocabulary to one database) and NOT bare string
// literals compared in match arms (a typo'd `"coroborates"` fails silently, with
// no single list to extend). The D1 file states outright that "task 1.3's
// fact-role vocabulary is expected to mirror this exact shape" — so this file is
// that mirror, deliberately structured the same way for the same reasons.
//
// ## Why the four roles are the graph's relationship types, not invented words
//
// The four roles are exactly the four Evidence→(Evidence|Allegation) edge types
// the graph already models — `schema::SUPPORTS`, `schema::CORROBORATES`,
// `schema::CONTRADICTS`, `schema::REBUTS`. The scan does NOT introduce a new
// role vocabulary; it reuses the one the knowledge graph is already built on, so
// a scan verdict names the same relationship the manual authoring path would.
// (`makes` / `denies` were floated during design but have no edge in `schema.rs`
// and would be orphan terms — deliberately excluded.) The `code()` token is the
// lowercase of the relationship-type constant; the `code_matches_schema_edge`
// test pins that correspondence so the two can never drift.
//
// Domain note: SUPPORTS and CORROBORATES are DISTINCT, not synonyms — the graph
// keeps them as separate edges. CORROBORATES is the narrow cross-document proof
// edge (a discovery/evidence item independently confirming a complaint
// Allegation — Phillips' sworn admission confirming a complaint paragraph);
// SUPPORTS is the broader "this quote backs the accusation" signal. CONTRADICTS
// is a direct factual conflict; REBUTS is a sworn statement that counters/defeats
// what a *different* speaker asserted. Different signals, queried differently, so
// the distinction is first-class — never collapsed to a free-text note.

use serde::{Deserialize, Serialize};

use crate::neo4j::schema;

/// The version of the fact-role vocabulary THIS build defines.
///
/// Bumped whenever a role is added or removed, exactly like
/// [`crate::domain::actor_role::ACTOR_ROLE_LOOKUP_V`]. A stored verdict or a
/// downstream reader can compare against this to notice a vocabulary it does not
/// yet understand.
///
/// ## Rust Learning: a `pub const` for a build-time coupling invariant
///
/// This is a compile-time constant, not a config value (Standing Rule 2 does not
/// apply): the role set is code, and changing it is a code change with a matching
/// version bump — it can never vary per deployment, so it does not belong in
/// YAML/env. Same rationale as `ACTOR_ROLE_LOOKUP_V` and `CURRENT_SCHEMA_V`.
pub const FACT_ROLE_LOOKUP_V: u32 = 1;

/// The role an Evidence quote plays with respect to a scenario's accusation.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on an enum
///
/// serde renders each variant using its Rust name by default (`Supports`). The
/// `rename_all` attribute maps every variant to its `snake_case` wire token
/// (`supports`) in one line, so the token stored in
/// `scenario_fact_refs.role_in_this_scenario` and the token the LLM returns in a
/// verdict stay identical without per-variant `rename`s. An unknown token (a
/// role this build does not define) fails to deserialize — the LOUD boundary the
/// verdict parse relies on (Standing Rule 1): a bad role from the model is a
/// per-item parse error the scan COUNTS as a failure, never a silently-dropped
/// or defaulted value.
///
/// ## Rust Learning: deriving `Copy` on a fieldless enum
///
/// This enum holds no data, so it is cheap to copy (a single discriminant). We
/// derive `Copy` so callers pass a `FactRole` by value without a `.clone()` —
/// the role stays usable after being handed to a function. `PartialEq`/`Eq` let
/// tests assert an exact role round-trips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactRole {
    /// The quote backs the accusation (the broad support signal).
    Supports,
    /// The quote independently confirms the accusation's underlying fact (the
    /// narrow cross-document proof signal).
    Corroborates,
    /// The quote directly conflicts with the accusation's factual claim.
    Contradicts,
    /// The quote is a sworn statement that counters / defeats the accusation.
    Rebuts,
}

impl FactRole {
    /// The full, ordered role vocabulary — the "extensible list in code."
    ///
    /// Adding a role means adding a variant AND an entry here AND bumping
    /// [`FACT_ROLE_LOOKUP_V`]. Kept as an associated `const` slice so any caller
    /// that needs to enumerate the vocabulary (a validation loop, a prompt
    /// builder, an endpoint that serves the list) reads it from ONE place rather
    /// than re-listing the variants and risking drift.
    pub const ALL: &'static [FactRole] = &[
        FactRole::Supports,
        FactRole::Corroborates,
        FactRole::Contradicts,
        FactRole::Rebuts,
    ];

    /// The stable wire token for this role (matches the serde `snake_case` name).
    ///
    /// Useful where a `&str` is needed — the value bound to
    /// `scenario_fact_refs.role_in_this_scenario` on an upsert, a log line, an
    /// error naming the role — without routing through serde. Kept in lock-step
    /// with the serde rename by living next to the enum; the
    /// `role_tokens_match_serde` test asserts they never diverge.
    pub fn code(self) -> &'static str {
        match self {
            FactRole::Supports => "supports",
            FactRole::Corroborates => "corroborates",
            FactRole::Contradicts => "contradicts",
            FactRole::Rebuts => "rebuts",
        }
    }

    /// The Neo4j relationship-type constant this role corresponds to.
    ///
    /// ## Why map back to `schema::`
    ///
    /// The role vocabulary IS the graph's edge vocabulary (see the module note).
    /// Returning the `schema::` constant — rather than an uppercased literal —
    /// means a rename in `schema.rs` flows here at compile time (Rule 16: no
    /// magic relationship-name strings), and the `code_matches_schema_edge` test
    /// pins that `code()` is exactly the lowercase of this edge so a future role
    /// cannot be added with a mismatched token.
    pub fn relationship_type(self) -> &'static str {
        match self {
            FactRole::Supports => schema::SUPPORTS,
            FactRole::Corroborates => schema::CORROBORATES,
            FactRole::Contradicts => schema::CONTRADICTS,
            FactRole::Rebuts => schema::REBUTS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_to_snake_case_tokens() {
        // The wire tokens are the contract the stored jsonb and the LLM verdict
        // both depend on — pin them exactly.
        assert_eq!(json!(FactRole::Supports), json!("supports"));
        assert_eq!(json!(FactRole::Corroborates), json!("corroborates"));
        assert_eq!(json!(FactRole::Contradicts), json!("contradicts"));
        assert_eq!(json!(FactRole::Rebuts), json!("rebuts"));
    }

    #[test]
    fn deserializes_known_tokens() {
        let parsed: FactRole = serde_json::from_value(json!("corroborates")).expect("known role");
        assert_eq!(parsed, FactRole::Corroborates);
    }

    #[test]
    fn rejects_unknown_role_token() {
        // Standing Rule 1: a role this build does not define is a LOUD parse
        // error, not a silent default. This is what lets the verdict parser count
        // an out-of-set `proposed_role` as a per-item FAILURE rather than writing
        // a garbage role or silently dropping the quote.
        let result: Result<FactRole, _> = serde_json::from_value(json!("makes"));
        assert!(result.is_err(), "an unknown role token must not parse");
    }

    #[test]
    fn all_lists_every_variant_once() {
        // Guards against `ALL` drifting from the enum — if a variant is added but
        // not appended here, this count check is the first place it shows up.
        assert_eq!(FactRole::ALL.len(), 4);
        assert!(FactRole::ALL.contains(&FactRole::Supports));
        assert!(FactRole::ALL.contains(&FactRole::Corroborates));
        assert!(FactRole::ALL.contains(&FactRole::Contradicts));
        assert!(FactRole::ALL.contains(&FactRole::Rebuts));
    }

    #[test]
    fn role_tokens_match_serde() {
        // `code()` and the serde rename are two hand-written representations of
        // the same token; assert they agree so neither can silently drift.
        for &role in FactRole::ALL {
            assert_eq!(json!(role), json!(role.code()));
        }
    }

    #[test]
    fn code_matches_schema_edge() {
        // The role token is exactly the lowercase of its graph edge type. This
        // ties the vocabulary to `schema.rs` (Rule 16 — the relationship names
        // come from the `schema::` constants, not re-typed literals) and stops a
        // future role from being added with a token that disagrees with its edge.
        for &role in FactRole::ALL {
            assert_eq!(
                role.code(),
                role.relationship_type().to_lowercase(),
                "fact-role token must be the lowercase of its schema edge type"
            );
        }
    }
}
