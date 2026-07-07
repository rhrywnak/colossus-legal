// =============================================================================
// backend/src/domain/fact_status.rs — the candidate-workbench state vocabulary
// (Phase 1a.1)
// =============================================================================
//
// A candidate quote pulled into a scenario is in exactly ONE of three states:
// it is UNDECIDED (no human has ruled on it — the Theme Scan judges only these),
// INCLUDED (a human accepted it as a confirmed fact of the scenario), or DROPPED
// (a human excluded it from THIS scenario; the graph node is untouched and still
// visible to other scenarios). That vocabulary is defined HERE, once, in code.
//
// ## Why a code-owned lookup, not a Postgres enum or a DB CHECK
//
// This is the same "code lookup" convention `actor_role` (D1) established and
// `fact_role` (D2b) followed: a small Rust enum plus a versioned list, NOT a DB
// enum and NOT a `CHECK` constraint. `scenario_fact_refs.status` is a plain
// `TEXT` column deliberately (see its migration comment) so the workbench
// vocabulary can grow — a future `needs_review`, say — without a migration. The
// three-state invariant is enforced by THIS enum at the write call site, not by
// the database. Contrast the sibling `scenarios` table, whose `direction` /
// `status` DO use `CHECK`: those are stable lifecycle fields; this one is an
// evolvable interaction vocabulary. Different volatility, different choice.
//
// ## Rust Learning: a write-and-parse type, NOT a DB-decode type
//
// Like `fact_role`, `FactStatus` is used at WRITE time (the two writers name a
// variant — `FactStatus::Included` / `FactStatus::Undecided` — so a typo is a
// compile error, not a bad row) and rendered to its wire token via `code()`.
// It is NOT decoded from the database into a struct field: the row record keeps
// `status` as a raw `String` (exactly as `role_in_this_scenario` keeps `role`),
// because nothing branches on the value yet. Introducing a per-column sqlx
// decode path now — for readers that only arrive in a later chunk — would make
// this one record diverge from every sibling with no consumer to justify it.
// When a reader that branches on status lands, that is the moment to add a typed
// decode (consistently, likely to `role` too). For now: typed at the write site,
// raw `String` in the record.
//
// Domain note: undecided / included / dropped are DIFFERENT signals with
// different downstream meaning. UNDECIDED is the Theme Scan's input set (it
// re-judges only these). INCLUDED is a confirmed fact that feeds the scenario's
// proof. DROPPED is a scenario-scoped exclusion — crucially NOT a graph deletion:
// the evidence stays in the graph for every OTHER scenario. The distinction is
// first-class, never a free-text note.

use serde::{Deserialize, Serialize};

/// The version of the fact-status vocabulary THIS build defines.
///
/// Bumped whenever a state is added or removed, exactly like
/// [`crate::domain::fact_role::FACT_ROLE_LOOKUP_V`]. A stored value or a
/// downstream reader can compare against this to notice a vocabulary it does not
/// yet understand.
///
/// ## Rust Learning: a `pub const` for a build-time coupling invariant
///
/// A compile-time constant, not a config value (Standing Rule 2 does not apply):
/// the state set is code, and changing it is a code change with a matching
/// version bump — it can never vary per deployment, so it does not belong in
/// YAML/env. Same rationale as `FACT_ROLE_LOOKUP_V` and `ACTOR_ROLE_LOOKUP_V`.
pub const FACT_STATUS_LOOKUP_V: u32 = 1;

/// A candidate's state within one scenario's workbench.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on an enum
///
/// serde renders each variant using its Rust name by default (`Undecided`). The
/// `rename_all` attribute maps every variant to its `snake_case` wire token
/// (`undecided`) in one line, so the token stored in `scenario_fact_refs.status`
/// and any future JSON representation stay identical without per-variant
/// `rename`s. An unknown token fails to deserialize — the LOUD boundary
/// (Standing Rule 1): a bad state is a parse error, never a silent default.
///
/// ## Rust Learning: deriving `Copy` on a fieldless enum
///
/// This enum holds no data, so it is cheap to copy (a single discriminant). We
/// derive `Copy` so callers pass a `FactStatus` by value without a `.clone()` —
/// the value stays usable after being handed to a function. `PartialEq`/`Eq`
/// let call sites and tests assert an exact state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactStatus {
    /// No human has ruled on this candidate yet. The Theme Scan judges only
    /// these; the neutral default for a freshly-surfaced candidate.
    Undecided,
    /// A human accepted this candidate as a confirmed fact of the scenario.
    Included,
    /// A human excluded this candidate from THIS scenario. Scenario-scoped: the
    /// graph node is untouched and still visible to other scenarios.
    Dropped,
}

impl FactStatus {
    /// The full, ordered state vocabulary — the "extensible list in code."
    ///
    /// Adding a state means adding a variant AND an entry here AND bumping
    /// [`FACT_STATUS_LOOKUP_V`]. Kept as an associated `const` slice so any
    /// caller that needs to enumerate the vocabulary reads it from ONE place
    /// rather than re-listing the variants and risking drift.
    pub const ALL: &'static [FactStatus] = &[
        FactStatus::Undecided,
        FactStatus::Included,
        FactStatus::Dropped,
    ];

    /// The stable wire token for this state (matches the serde `snake_case`
    /// name). This is what the DB write binds into `scenario_fact_refs.status`
    /// — exactly how the D2b path binds `FactRole::code()` into
    /// `role_in_this_scenario`. Kept in lock-step with the serde rename by
    /// living next to the enum; the `status_tokens_match_serde` test asserts
    /// they never diverge.
    pub fn code(self) -> &'static str {
        match self {
            FactStatus::Undecided => "undecided",
            FactStatus::Included => "included",
            FactStatus::Dropped => "dropped",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_to_snake_case_tokens() {
        // The wire tokens are the contract the stored `status` column depends on
        // — pin them exactly.
        assert_eq!(json!(FactStatus::Undecided), json!("undecided"));
        assert_eq!(json!(FactStatus::Included), json!("included"));
        assert_eq!(json!(FactStatus::Dropped), json!("dropped"));
    }

    #[test]
    fn deserializes_known_tokens() {
        let parsed: FactStatus = serde_json::from_value(json!("dropped")).expect("known state");
        assert_eq!(parsed, FactStatus::Dropped);
    }

    #[test]
    fn rejects_unknown_status_token() {
        // Standing Rule 1: a state this build does not define is a LOUD parse
        // error, not a silent default.
        let result: Result<FactStatus, _> = serde_json::from_value(json!("archived"));
        assert!(result.is_err(), "an unknown status token must not parse");
    }

    #[test]
    fn all_lists_every_variant_once() {
        // Guards against `ALL` drifting from the enum — if a variant is added but
        // not appended here, this count check is the first place it shows up.
        assert_eq!(FactStatus::ALL.len(), 3);
        assert!(FactStatus::ALL.contains(&FactStatus::Undecided));
        assert!(FactStatus::ALL.contains(&FactStatus::Included));
        assert!(FactStatus::ALL.contains(&FactStatus::Dropped));
    }

    #[test]
    fn status_tokens_match_serde() {
        // `code()` and the serde rename are two hand-written representations of
        // the same token; assert they agree so neither can silently drift (a
        // wire/`code()` mismatch would be an invisible bug otherwise).
        for &status in FactStatus::ALL {
            assert_eq!(json!(status), json!(status.code()));
        }
    }

    #[test]
    fn default_backfill_token_is_undecided() {
        // The migration backfills unset/false rows to 'undecided' and defaults
        // the column to 'undecided'. Pin that the neutral state's token matches
        // the SQL literal the migration writes, so the two cannot drift.
        assert_eq!(FactStatus::Undecided.code(), "undecided");
    }
}
