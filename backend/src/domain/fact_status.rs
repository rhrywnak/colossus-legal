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

/// The error produced when a raw status token cannot be decoded into a
/// [`FactStatus`] this build defines.
///
/// ## Rust Learning: a typed parse error carrying the offending value
///
/// The error is a real `struct` with a `token: String` field, NOT a unit `()`
/// or a bare string. Carrying the bad token means the failure names *what* was
/// wrong (Standing Rule 1): a log or a `?`-propagated error can show
/// `token = "archived"`, so an operator sees the exact value that no code path
/// understands — not just "parse failed". `#[derive(thiserror::Error)]` writes
/// the `Display`/`Error` impls from the `#[error("…")]` template, the same
/// pattern `ThemeScanError` and `BiasRepositoryError` use.
#[derive(Debug, thiserror::Error)]
#[error("unknown fact-status token '{token}' — not one of undecided/included/dropped")]
pub struct FactStatusParseError {
    /// The token that failed to decode. `pub` so a caller can log it or fold it
    /// into a richer error of its own.
    pub token: String,
}

/// ## Rust Learning: `TryFrom<&str>` — the READ boundary (parse-don't-validate, deferred half)
///
/// 1a.1 gave `FactStatus` its WRITE boundary — [`FactStatus::code`], turning a
/// typed variant into a wire token. This is the matching READ boundary: a raw
/// token becomes a typed variant, or a loud [`FactStatusParseError`]. Together
/// they make `FactStatus` a true *parse* type — an illegitimate state cannot
/// cross the boundary in EITHER direction, so downstream code that holds a
/// `FactStatus` never has to re-ask "is this a valid state?".
///
/// ### Why the existing `Deserialize` derive doesn't serve this
///
/// serde's derived `Deserialize` already turns `"dropped"` into
/// `FactStatus::Dropped` — but only from a *serde/JSON value* (see the
/// `deserializes_known_tokens` test). The database column arrives as a raw
/// `String` (sqlx decodes `TEXT` to `String`, not to a serde `Value`), so
/// decoding it through serde would mean wrapping it in a `serde_json::Value`
/// first. `TryFrom<&str>` decodes the `&str` directly — the right boundary for
/// the sqlx column. This new `&str`-boundary reject test is the sibling of the
/// JSON-boundary [`rejects_unknown_status_token`] test.
///
/// The match arms EXACTLY reverse [`FactStatus::code`]; any other token is an
/// `Err`, never a silent default (Standing Rule 1).
impl TryFrom<&str> for FactStatus {
    type Error = FactStatusParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "undecided" => Ok(FactStatus::Undecided),
            "included" => Ok(FactStatus::Included),
            "dropped" => Ok(FactStatus::Dropped),
            other => Err(FactStatusParseError {
                token: other.to_string(),
            }),
        }
    }
}

/// A thin `TryFrom<String>` that delegates to the `&str` impl.
///
/// ## Rust Learning: two `TryFrom` impls, one source of truth
///
/// Callers sometimes hold an owned `String` (a row field moved out of a record)
/// and sometimes a borrowed `&str`. Providing both spares every call site an
/// `.as_str()` or `.to_string()` dance. The owned impl does NOT re-list the
/// match arms — it borrows and forwards to the `&str` impl, so the decode rule
/// lives in exactly one place and the two can never drift.
impl TryFrom<String> for FactStatus {
    type Error = FactStatusParseError;

    fn try_from(token: String) -> Result<Self, Self::Error> {
        FactStatus::try_from(token.as_str())
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

    #[test]
    fn try_from_round_trips_every_variant_via_code() {
        // The READ boundary must be the exact inverse of the WRITE boundary:
        // decoding a variant's own `code()` token must yield that same variant,
        // for EVERY variant. This closes the loop `variant -> code() -> try_from
        // -> variant` so a future edit that adds a variant to `code()` but forgets
        // the `try_from` arm (or vice versa) fails here.
        for &status in FactStatus::ALL {
            let decoded = FactStatus::try_from(status.code())
                .expect("a token produced by code() must decode back");
            assert_eq!(decoded, status, "round-trip mismatch for {status:?}");
        }
    }

    #[test]
    fn try_from_owned_string_delegates_to_str() {
        // The `String` impl must agree with the `&str` impl (it forwards to it).
        let decoded = FactStatus::try_from("included".to_string()).expect("known token");
        assert_eq!(decoded, FactStatus::Included);
    }

    #[test]
    fn try_from_rejects_unknown_token_loudly() {
        // The `&str`-boundary sibling of `rejects_unknown_status_token` (which
        // guards the JSON boundary). A token this build does not define is a LOUD
        // error carrying the offending value — never a silent default to
        // Undecided, which would mis-bucket a fact the Theme Scan re-judges
        // (Standing Rule 1).
        let err = FactStatus::try_from("archived").expect_err("unknown token must not decode");
        assert_eq!(err.token, "archived", "the error must carry the bad token");
        assert!(
            err.to_string().contains("archived"),
            "the Display message must surface the bad token: {err}"
        );
    }
}
