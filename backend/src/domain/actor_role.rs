// =============================================================================
// backend/src/domain/actor_role.rs — the accusation-chain role vocabulary (D1)
// =============================================================================
//
// A party named as a `wielder` on a scenario definition plays a ROLE in how the
// accusation reached the record: it either ORIGINATED the accusation, REPEATED
// one someone else made, or ADOPTED one into its own posture. That vocabulary is
// defined HERE, once, in code.
//
// ## Why a code-owned lookup, not a Postgres enum or bare strings
//
// The instruction is explicit: `actor_role` is NOT a DB enum (a migration to add
// a role would be heavy and couples the vocabulary to one database), and NOT bare
// string literals compared in match arms across the codebase (a typo'd `"repated"`
// would fail silently and there would be no single list to extend). Instead it is
// a small Rust enum plus a versioned list — the "code lookup" convention. D1 is
// the FIRST place this convention appears; task 1.3's fact-role vocabulary is
// expected to mirror this exact shape, so this file is the sanctioned precedent.
//
// Domain note: originated / repeated / adopted are DIFFERENT legal signals. The
// party that ORIGINATED an accusation is the source; a party that merely REPEATED
// it after a proven rebuttal is the Count IV "baseless repeat" signal; a party
// that ADOPTED it took on someone else's accusation as its own position. The role
// is queried differently downstream, so the distinction is first-class, not a
// free-text note.

use serde::{Deserialize, Serialize};

/// The version of the actor-role vocabulary THIS build defines.
///
/// Bumped whenever a role is added or removed. A reader can compare a stored
/// definition's expectations against this to notice a vocabulary it does not yet
/// understand, mirroring how `CURRENT_SCHEMA_V` gates the definition body shape.
///
/// ## Rust Learning: a `pub const` for a build-time coupling invariant
///
/// This is a compile-time constant, not a config value (Standing Rule 2 does not
/// apply): the role set is code, and changing it is a code change with a matching
/// version bump — it can never vary per deployment, so it does not belong in
/// YAML/env. Same rationale as `CURRENT_SCHEMA_V` in `dto/scenario_crud.rs`.
pub const ACTOR_ROLE_LOOKUP_V: u32 = 1;

/// A party's role in a scenario's accusation chain.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on an enum
///
/// serde renders each variant using its Rust name by default (`Originated`). The
/// `rename_all` attribute maps every variant to its `snake_case` wire token
/// (`originated`) in one line, so the JSON stored in the definition jsonb and the
/// TypeScript `ActorRole` union stay identical without per-variant `rename`s. An
/// unknown token (a role this build does not define) fails to deserialize — the
/// LOUD boundary the definition parse relies on (Standing Rule 1): a bad role is
/// an error at parse time, never a silently-dropped or defaulted value.
///
/// ## Rust Learning: deriving `Copy` on a fieldless enum
///
/// This enum holds no data, so it is cheap to copy (a single discriminant). We
/// derive `Copy` so callers can pass an `ActorRole` by value without a `.clone()`
/// or worrying about a move — `role` stays usable after it is handed to a
/// function. `PartialEq`/`Eq` let tests assert an exact role round-trips.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorRole {
    /// This party is the source of the accusation.
    Originated,
    /// This party repeated an accusation someone else first made.
    Repeated,
    /// This party took on another's accusation as its own position.
    Adopted,
}

impl ActorRole {
    /// The full, ordered role vocabulary — the "extensible list in code."
    ///
    /// Adding a role means adding a variant AND an entry here AND bumping
    /// [`ACTOR_ROLE_LOOKUP_V`]. Kept as an associated `const` slice so any future
    /// caller that needs to enumerate the vocabulary (e.g. a validation loop, or
    /// an endpoint that serves the list) reads it from ONE place rather than
    /// re-listing the variants and risking drift.
    pub const ALL: &'static [ActorRole] = &[
        ActorRole::Originated,
        ActorRole::Repeated,
        ActorRole::Adopted,
    ];

    /// The stable wire token for this role (matches the serde `snake_case` name).
    ///
    /// Useful where a `&str` is needed (logging, an error message naming the role)
    /// without routing through serde. Kept in lock-step with the serde rename by
    /// living next to the enum — the `role_tokens_match_serde` test asserts they
    /// never diverge.
    pub fn code(self) -> &'static str {
        match self {
            ActorRole::Originated => "originated",
            ActorRole::Repeated => "repeated",
            ActorRole::Adopted => "adopted",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_to_snake_case_tokens() {
        // The wire tokens are the contract the stored jsonb and the frontend
        // `ActorRole` union both depend on — pin them exactly.
        assert_eq!(json!(ActorRole::Originated), json!("originated"));
        assert_eq!(json!(ActorRole::Repeated), json!("repeated"));
        assert_eq!(json!(ActorRole::Adopted), json!("adopted"));
    }

    #[test]
    fn deserializes_known_tokens() {
        let parsed: ActorRole = serde_json::from_value(json!("repeated")).expect("known role");
        assert_eq!(parsed, ActorRole::Repeated);
    }

    #[test]
    fn rejects_unknown_role_token() {
        // Standing Rule 1: a role this build does not define is a LOUD parse error,
        // not a silent default. This is what makes a malformed `wielder` fail at
        // the definition boundary rather than deep in a query.
        let result: Result<ActorRole, _> = serde_json::from_value(json!("fabricated"));
        assert!(result.is_err(), "an unknown role token must not parse");
    }

    #[test]
    fn all_lists_every_variant_once() {
        // Guards against `ALL` drifting from the enum — if a variant is added but
        // not appended here, this count check is the first place it shows up.
        assert_eq!(ActorRole::ALL.len(), 3);
        assert!(ActorRole::ALL.contains(&ActorRole::Originated));
        assert!(ActorRole::ALL.contains(&ActorRole::Repeated));
        assert!(ActorRole::ALL.contains(&ActorRole::Adopted));
    }

    #[test]
    fn role_tokens_match_serde() {
        // `code()` and the serde rename are two hand-written representations of the
        // same token; assert they agree so neither can silently drift from the
        // other (a wire/`code()` mismatch would be an invisible bug otherwise).
        for &role in ActorRole::ALL {
            assert_eq!(json!(role), json!(role.code()));
        }
    }
}
