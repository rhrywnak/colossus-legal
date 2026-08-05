// =============================================================================
// backend/src/domain/fact_tier.rs — how much a fact carries a scenario (2.13)
// =============================================================================
//
// A fact a human has included in a scenario is worth one of three things to that
// scenario: it CARRIES it (the certified letter that wins the point), it is
// BACKUP (true, useful, not the thing you lead with), or it is BACKGROUND (the
// seventh restatement — kept, because deleting evidence is not a curation act,
// but folded out of the way).
//
// That vocabulary is defined HERE, once, in code — the same "code lookup"
// convention `actor_role` (D1), `fact_role` (D2b) and `fact_status` (1a.1)
// established.
//
// ## Why a code-owned lookup and NOT a DB CHECK
//
// Roman's ruling of 2026-08-05 (Phase A, item 2), against the drafted
// instruction: `scenario_fact_refs` has a documented precedent for exactly this,
// stated in the `status` migration — the column is plain TEXT so an evolvable
// interaction vocabulary can grow without a migration, and the invariant lives in
// a Rust enum at the boundary instead. A CHECK on this table would have reversed
// that stance one column over, for no gain: a fourth tier is a plausible future
// (the design's own "Two tiers instead of three?" question is still open), and
// under a CHECK that future costs a migration on every environment.
//
// ## Domain note: tier is a judgment about THIS scenario, never about the fact
//
// The same Evidence node is `carries` in the scenario it wins and `background` in
// the one it merely touches. That is why the column lives on the per-scenario
// REFERENCE row beside `role_in_this_scenario`, and why nothing here ever reaches
// the graph. A fact is shared across scenarios; what it is WORTH is not.
//
// ## Rust Learning: a write-and-parse type
//
// Like `FactStatus`, this is typed at the WRITE site (a handler names
// `FactTier::Carries`, so a typo is a compile error rather than a bad row) and
// rendered to its wire token by `code()`. The row record keeps `tier` as a raw
// `String`, consistent with `status` and `role_in_this_scenario`; the decode to a
// variant happens where something actually branches on it — here, the card
// assembler, which must not show a human a tier this build cannot name.

use serde::{Deserialize, Serialize};

/// The version of the fact-tier vocabulary THIS build defines.
///
/// Bumped whenever a tier is added or removed, exactly like
/// [`crate::domain::fact_status::FACT_STATUS_LOOKUP_V`]. A stored value or a
/// downstream reader can compare against this to notice a vocabulary it does not
/// yet understand.
///
/// ## Rust Learning: a `pub const` for a build-time coupling invariant
///
/// A compile-time constant, not a config value — Standing Rule 2 does not apply.
/// The tier SET is code (changing it changes what the enum can represent); the
/// tier NAMES are configuration, and they live in the settings store where Roman
/// can rename them without a rebuild. Two different things, deliberately kept in
/// two different places: the token `carries` is structure, the words "Carries the
/// scenario" are wording.
pub const FACT_TIER_LOOKUP_V: u32 = 1;

/// How much one included fact carries one scenario.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a closed enum
///
/// Each variant maps to its snake_case wire token (`Carries` → `carries`), which
/// is both what `scenario_fact_refs.tier` stores and what the write route
/// accepts. Because the enum is CLOSED (no catch-all variant), a token this build
/// does not define fails to deserialize — a loud 400 at the HTTP parse boundary,
/// before any handler runs, and a loud typed error at the DB read boundary. Never
/// a silent default (Standing Rule 1).
///
/// ## Rust Learning: deriving `Copy` on a fieldless enum
///
/// The enum holds no data, so a value is one discriminant and cheap to copy.
/// `Copy` lets call sites pass a `FactTier` by value and keep using it afterwards
/// with no `.clone()`; `PartialEq`/`Eq` let tests assert an exact tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactTier {
    /// This fact carries the scenario — the one you lead with.
    Carries,
    /// True and useful, but not the thing the scenario stands on. The DEFAULT: a
    /// newly-included fact lands here, because the machine has no opinion about
    /// weight and inventing one would be the system pretending to a judgment only
    /// the human can make.
    Backup,
    /// Kept, but folded out of the way — the restatement, the duplicate, the
    /// merely-corroborating. Collapsed beneath the list rather than hidden: the
    /// count is always shown and one click expands it, because a curated fact that
    /// silently disappears is the failure this tier exists to avoid.
    Background,
}

impl FactTier {
    /// The full, ordered tier vocabulary — the "extensible list in code".
    ///
    /// Ordered by WEIGHT, heaviest first, because that is the order the list
    /// renders in and the order a human reads the three names as a scale. Adding a
    /// tier means adding a variant AND an entry here AND bumping
    /// [`FACT_TIER_LOOKUP_V`]; keeping the list in one place is what stops a
    /// caller re-listing the variants and drifting.
    pub const ALL: &'static [FactTier] =
        &[FactTier::Carries, FactTier::Backup, FactTier::Background];

    /// The tier a fact has when nobody has weighed it.
    ///
    /// Named rather than written as a literal at each call site: the migration's
    /// `DEFAULT 'backup'` and this constant are the same decision in two places,
    /// and a named constant is what lets the `default_matches_the_migration` test
    /// assert they agree.
    pub const DEFAULT: FactTier = FactTier::Backup;

    /// The stable wire token for this tier (matches the serde `snake_case` name).
    ///
    /// This is what a write binds into `scenario_fact_refs.tier`, exactly as
    /// `FactStatus::code()` binds `status`. Kept beside the enum so it stays in
    /// lock-step with the serde rename; `tier_tokens_match_serde` asserts they
    /// never diverge.
    pub fn code(self) -> &'static str {
        match self {
            FactTier::Carries => "carries",
            FactTier::Backup => "backup",
            FactTier::Background => "background",
        }
    }

    /// Where this tier sorts relative to the others — 0 is heaviest.
    ///
    /// ## Why the order is a function and not the `ALL` index
    ///
    /// Deriving it from `ALL.iter().position(…)` would work and would silently
    /// change every list's order the day somebody inserts a variant in the middle
    /// of that slice for an unrelated reason. An explicit rank makes the display
    /// order a decision this function states, so a reordering of `ALL` cannot move
    /// facts around on Roman's screen as a side effect.
    pub fn rank(self) -> u8 {
        match self {
            FactTier::Carries => 0,
            FactTier::Backup => 1,
            FactTier::Background => 2,
        }
    }
}

/// The error produced when a raw tier token cannot be decoded into a
/// [`FactTier`] this build defines.
///
/// ## Rust Learning: a typed parse error carrying the offending value
///
/// A real `struct` with a `token: String`, not a unit error: the failure names
/// WHAT was wrong, so an operator log reads `token = "critical"` and shows the
/// exact value no code path understands. `#[derive(thiserror::Error)]` writes the
/// `Display`/`Error` impls from the `#[error(…)]` template — the same pattern
/// [`crate::domain::fact_status::FactStatusParseError`] uses.
#[derive(Debug, thiserror::Error)]
#[error("unknown fact-tier token '{token}' — not one of carries/backup/background")]
pub struct FactTierParseError {
    /// The token that failed to decode. `pub` so a caller can log it or fold it
    /// into a richer error of its own.
    pub token: String,
}

/// ## Rust Learning: `TryFrom<&str>` — the READ boundary
///
/// [`FactTier::code`] is the WRITE boundary (typed variant → wire token); this is
/// its mirror (raw token → typed variant, or a loud error). Together they make
/// `FactTier` a true *parse* type: an illegitimate tier cannot cross the boundary
/// in either direction, so code holding a `FactTier` never re-asks "is this
/// valid?".
///
/// The database column arrives as a `String` (sqlx decodes `TEXT` to `String`,
/// not to a serde value), so decoding it through the derived `Deserialize` would
/// mean wrapping it in a `serde_json::Value` first. `TryFrom<&str>` is the right
/// boundary for that column.
///
/// The match arms EXACTLY reverse [`FactTier::code`]; any other token is an
/// `Err`, never a silent default.
impl TryFrom<&str> for FactTier {
    type Error = FactTierParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "carries" => Ok(FactTier::Carries),
            "backup" => Ok(FactTier::Backup),
            "background" => Ok(FactTier::Background),
            other => Err(FactTierParseError {
                token: other.to_string(),
            }),
        }
    }
}

/// A thin `TryFrom<String>` that delegates to the `&str` impl.
///
/// ## Rust Learning: two `TryFrom` impls, one source of truth
///
/// Callers sometimes hold an owned `String` moved out of a row record and
/// sometimes a borrowed `&str`. Both impls exist so no call site needs an
/// `.as_str()` / `.to_string()` dance — and the owned one FORWARDS rather than
/// re-listing the arms, so the decode rule lives in exactly one place.
impl TryFrom<String> for FactTier {
    type Error = FactTierParseError;

    fn try_from(token: String) -> Result<Self, Self::Error> {
        FactTier::try_from(token.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tier_tokens_match_serde() {
        // `code()` is what the DB write binds; the serde rename is what the HTTP
        // body parses. They are two hand-written mappings of the same vocabulary,
        // so they are pinned against each other — a rename of one without the
        // other would store a token the route cannot accept back.
        for &tier in FactTier::ALL {
            let json = serde_json::to_value(tier).expect("a fieldless enum serializes");
            assert_eq!(
                json,
                json!(tier.code()),
                "{tier:?}: serde token and code() disagree",
            );
        }
    }

    #[test]
    fn all_lists_every_variant_exactly_once() {
        // The guard on the "extensible list in code" convention: a variant added
        // to the enum but forgotten in `ALL` would be invisible to every caller
        // that enumerates the vocabulary, with nothing failing to say so.
        assert_eq!(FactTier::ALL.len(), 3);
        for &tier in FactTier::ALL {
            let matches = FactTier::ALL.iter().filter(|t| **t == tier).count();
            assert_eq!(matches, 1, "{tier:?} appears more than once in ALL");
        }
    }

    #[test]
    fn round_trips_every_token() {
        // Write then read must land back on the same variant, for all three.
        for &tier in FactTier::ALL {
            let decoded = FactTier::try_from(tier.code()).expect("its own token decodes");
            assert_eq!(decoded, tier);
        }
    }

    #[test]
    fn rejects_an_unknown_token_and_names_it() {
        // Standing Rule 1: a tier this build does not define is a loud, named
        // failure — never a silent collapse to Backup, which would show the human
        // "Backup" for a fact somebody had deliberately marked otherwise.
        let error = FactTier::try_from("critical").expect_err("an undefined tier must not decode");
        assert_eq!(error.token, "critical");
        assert!(
            error.to_string().contains("critical"),
            "the message must name the offending token: {error}",
        );
    }

    #[test]
    fn rejects_case_and_whitespace_variants() {
        // The stored vocabulary is exact. "Carries" and " carries" are not the
        // token, and quietly accepting them would mean two spellings of one tier
        // in the column with only one of them readable back.
        for token in ["Carries", "CARRIES", " carries", "carries "] {
            assert!(
                FactTier::try_from(token).is_err(),
                "{token:?} must not decode as a tier",
            );
        }
    }

    #[test]
    fn default_matches_the_migration() {
        // The migration writes `DEFAULT 'backup'`. If this constant and that
        // default ever disagreed, a freshly-included fact would arrive on screen
        // wearing a tier the write path never chose.
        assert_eq!(FactTier::DEFAULT.code(), "backup");
    }

    #[test]
    fn rank_orders_heaviest_first_and_is_total() {
        // The list's display order depends on this being a strict, total ordering
        // of the three tiers — two tiers sharing a rank would make the sort
        // unstable between them and move rows around between renders.
        assert!(FactTier::Carries.rank() < FactTier::Backup.rank());
        assert!(FactTier::Backup.rank() < FactTier::Background.rank());
        let mut ranks: Vec<u8> = FactTier::ALL.iter().map(|t| t.rank()).collect();
        ranks.sort_unstable();
        ranks.dedup();
        assert_eq!(ranks.len(), FactTier::ALL.len(), "two tiers share a rank");
    }

    #[test]
    fn deserializes_known_tokens_and_rejects_unknown() {
        // The HTTP parse boundary: the write route takes a tier in its body, so a
        // bad token must fail as a 400 before the handler runs.
        let parsed: FactTier = serde_json::from_value(json!("background")).expect("parses");
        assert_eq!(parsed, FactTier::Background);
        let bad: Result<FactTier, _> = serde_json::from_value(json!("archived"));
        assert!(bad.is_err(), "an undefined tier token must not parse");
    }
}
