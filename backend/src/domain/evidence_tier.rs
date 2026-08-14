// =============================================================================
// backend/src/domain/evidence_tier.rs — how hard one piece of proof is to
// dispute (task 396, P1)
// =============================================================================
//
// The Proof Matrix used to lead with ONE number per Element: how many distinct
// Evidence items corroborate an Allegation that bears on it. That number counts a
// sworn admission from the opposing party and a qualified "to the best of my
// recollection" as the same thing, which is the noise disease the matrix half of
// the de-noising work exists to cure.
//
// So a corroborating item now carries a TIER, and the headline is the STRONG
// count. Roman's ruling of 2026-08-13, on the record:
//
//     "STRONG means what the opposing side cannot dispute — their own admissions
//      and the court's findings. Our own sworn affidavit statements rank OTHER,
//      visible and ranked, never hidden."
//
// ## Why the tier is decided by a PAIR, not by `evidence_strength` alone
//
// Measured on DEV before this was designed: `evidence_strength =
// 'sworn_party_admission'` appears under BOTH `statement_type = 'admission'` (21
// items) and `statement_type = 'partial_admission'` (12 items). Strength alone
// therefore cannot separate a firm admission from a hedged one — the very
// distinction the headline is meant to make. The key is the pair
// `(statement_type, evidence_strength)`, which is also the grouping
// `dto::proof_review::CategoryCount` already reports.
//
// ## Why the SET is code and the MAP is configuration
//
// Exactly the split `fact_tier` documents: the three tier tokens are structure
// (adding a fourth changes what this enum can represent, and that is a code
// change with a version bump), while WHICH extraction pairs land in which tier is
// a judgment about this corpus that Roman must be able to retune from the
// Settings page when a new document type starts producing new pairs. So the
// vocabulary is the enum below and the map is three `app_settings` rows, read at
// boot and refused loudly if a row is missing or malformed.
//
// ## Domain note: an unmapped pair is NOT an error and NOT a tier
//
// A pair the map does not name gets `None` — it is still counted in the raw
// approved figure and still rendered in the drill-down, carrying no chip. That is
// deliberate: a new document type appearing in the corpus must never make an item
// vanish from a proof surface, and must never be silently promoted into the
// headline either. It shows up unranked, which is a visible prompt to map it.

use std::collections::HashMap;

/// The version of the evidence-tier vocabulary THIS build defines.
///
/// Bumped whenever a tier is added or removed, exactly like
/// [`crate::domain::fact_tier::FACT_TIER_LOOKUP_V`]. A downstream reader can
/// compare against this to notice a vocabulary it does not yet understand.
///
/// ## Rust Learning: a `pub const` for a build-time coupling invariant
///
/// A compile-time constant, not a config value — Standing Rule 2 does not apply
/// to the SET. The tier tokens are structure; the pair→tier MAP and the tier
/// NAMES are configuration and live in the settings store.
pub const EVIDENCE_TIER_LOOKUP_V: u32 = 1;

/// How hard one corroborating item is to dispute.
///
/// ## Rust Learning: `#[derive(Copy)]` on a fieldless enum
///
/// The enum holds no data, so a value is one discriminant and cheap to copy.
/// `Copy` lets call sites pass a tier by value and keep using it afterwards with
/// no `.clone()`; `PartialEq`/`Eq`/`Hash` let it key a map and be asserted
/// exactly in tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EvidenceTier {
    /// What the opposing side cannot dispute: their own sworn admissions and the
    /// court's own findings and orders. This is the headline number.
    Strong,
    /// Sworn, but qualified — a partial admission, or an evasion the extraction
    /// still scored as corroborating. True and usable; not what you lead with.
    Hedged,
    /// Everything else the map names — today, our own sworn affidavit testimony.
    /// Visible and ranked, never hidden; it simply is not an opponent's
    /// admission, so it does not carry the headline.
    Other,
}

impl EvidenceTier {
    /// The full, ordered tier vocabulary — the "extensible list in code".
    ///
    /// Ordered by STRENGTH, strongest first, because that is the order the
    /// drill-down ranks in. Adding a tier means adding a variant AND an entry
    /// here AND bumping [`EVIDENCE_TIER_LOOKUP_V`].
    pub const ALL: &'static [EvidenceTier] = &[
        EvidenceTier::Strong,
        EvidenceTier::Hedged,
        EvidenceTier::Other,
    ];

    /// The stable wire token for this tier — what the settings row names and what
    /// the chip lookup keys on.
    pub fn code(self) -> &'static str {
        match self {
            EvidenceTier::Strong => "strong",
            EvidenceTier::Hedged => "hedged",
            EvidenceTier::Other => "other",
        }
    }

    /// Where this tier sorts relative to the others — 0 is strongest.
    ///
    /// ## Why an explicit rank and not the `ALL` index
    ///
    /// Deriving it from `ALL.iter().position(…)` would silently reorder every
    /// drill-down the day somebody inserts a variant in the middle of that slice
    /// for an unrelated reason. `fact_tier::rank` states the same argument.
    pub fn rank(self) -> u8 {
        match self {
            EvidenceTier::Strong => 0,
            EvidenceTier::Hedged => 1,
            EvidenceTier::Other => 2,
        }
    }
}

/// The error produced when a raw tier token cannot be decoded.
#[derive(Debug, thiserror::Error)]
#[error("unknown evidence-tier token '{token}' — not one of strong/hedged/other")]
pub struct EvidenceTierParseError {
    /// The token that failed to decode. `pub` so a caller can log it.
    pub token: String,
}

/// ## Rust Learning: `TryFrom<&str>` — the READ boundary
///
/// [`EvidenceTier::code`] is the WRITE boundary (typed variant → token); this is
/// its mirror. The arms EXACTLY reverse `code()`; any other token is an `Err`,
/// never a silent default (Standing Rule 1).
impl TryFrom<&str> for EvidenceTier {
    type Error = EvidenceTierParseError;

    fn try_from(token: &str) -> Result<Self, Self::Error> {
        match token {
            "strong" => Ok(EvidenceTier::Strong),
            "hedged" => Ok(EvidenceTier::Hedged),
            "other" => Ok(EvidenceTier::Other),
            other => Err(EvidenceTierParseError {
                token: other.to_string(),
            }),
        }
    }
}

/// The separator between the two halves of a mapped pair, as a stored row writes
/// it: `admission+sworn_party_admission`.
///
/// Named rather than typed as a literal at each site so the parser, the error
/// message and the tests cannot drift on it.
const PAIR_SEPARATOR: char = '+';

/// The error produced when a stored pair list cannot be parsed.
///
/// ## Rust Learning: an error that names its ROW as well as its value
///
/// The map is assembled from three rows, and "malformed pair" with no key is a
/// message an operator cannot act on. Carrying `key` means the boot refusal reads
/// `matrix_tier_strong_pairs: malformed entry 'admission'` and names the exact
/// Settings row to correct.
#[derive(Debug, thiserror::Error)]
#[error(
    "{key}: malformed entry '{entry}' — every entry must read \
     statement_type{separator}evidence_strength, with both halves non-blank"
)]
pub struct PairParseError {
    /// The settings key whose value could not be parsed.
    pub key: String,
    /// The offending entry, verbatim.
    pub entry: String,
    /// The separator the entry was missing, so the message is self-describing.
    pub separator: char,
}

/// The `(statement_type, evidence_strength)` → tier lookup, built from the store.
///
/// ## Rust Learning: a newtype around a `HashMap`
///
/// Wrapping the map rather than passing a bare `HashMap<(String, String),
/// EvidenceTier>` around does two things. It gives the lookup a NAME at every
/// call site (`tier_map.tier_for(…)` reads as a question about the domain, not
/// about a collection), and it lets the only constructor be the checked one
/// below — so anything holding an `EvidenceTierMap` knows the rows parsed.
///
/// `Default` is derived so an empty map is expressible in tests; the boot path
/// never uses it, because every key is required.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceTierMap {
    pairs: HashMap<(String, String), EvidenceTier>,
}

impl EvidenceTierMap {
    /// Build the map from one already-split entry list per tier.
    ///
    /// The caller passes `(tier, key, entries)` triples — `key` only so a parse
    /// failure can name the Settings row. Splitting the comma-separated row into
    /// entries is the settings store's job (it owns `token_list_of`); turning
    /// entries into a map is this module's.
    ///
    /// ## Rust Learning: `&[(…)]` of borrowed slices
    ///
    /// Taking a slice of tuples rather than three named parameters means adding a
    /// fourth tier later changes the CALLER, not this signature — and the
    /// function body already loops, so there is no per-tier code to forget.
    ///
    /// # Errors
    /// Returns [`PairParseError`] naming the row and the first entry that is not
    /// `statement_type+evidence_strength` with both halves non-blank.
    pub fn from_entries(
        groups: &[(EvidenceTier, &str, &[String])],
    ) -> Result<Self, PairParseError> {
        let mut pairs = HashMap::new();
        for (tier, key, entries) in groups {
            for entry in *entries {
                let (statement_type, evidence_strength) = split_pair(key, entry)?;
                // A pair listed under two tiers is not rejected: the LAST group
                // supplied wins. The boot path supplies them strongest-first, so
                // the WEAKEST listing is the one that survives. Deliberate — a
                // duplicate is a Settings typo, and refusing to boot over one
                // would take the whole product down for a mistake whose safe
                // reading (rank it lower, never higher) is obvious.
                //
                // The property is pinned from both ends: this module's
                // `a_pair_listed_in_two_tiers_takes_the_weaker_one` proves
                // last-wins, and `settings_store_tests`'
                // `a_pair_seeded_under_two_tiers_ranks_as_the_weaker_one` proves
                // the boot path really does supply them in that order — because
                // nothing but those three hand-written lines makes it true.
                pairs.insert((statement_type, evidence_strength), *tier);
            }
        }
        Ok(EvidenceTierMap { pairs })
    }

    /// The tier for one item's pair, or `None` when the map does not name it.
    ///
    /// `None` is a real answer, not a failure — see the module note on why an
    /// unmapped pair stays visible and unranked rather than vanishing or being
    /// promoted.
    pub fn tier_for(
        &self,
        statement_type: Option<&str>,
        evidence_strength: Option<&str>,
    ) -> Option<EvidenceTier> {
        // Both halves are required to name a pair: an item missing either
        // property cannot be mapped, and guessing from the half that is present
        // would put items in the headline on the strength of half a key.
        let statement_type = statement_type?.trim();
        let evidence_strength = evidence_strength?.trim();
        self.pairs
            .get(&(statement_type.to_string(), evidence_strength.to_string()))
            .copied()
    }

    /// How many pairs the map names. Used by the boot log so an operator can see
    /// the vocabulary was loaded, and by tests.
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    /// Whether the map names no pairs at all.
    ///
    /// ## Rust Learning: clippy wants `is_empty` beside `len`
    ///
    /// `clippy::len_without_is_empty` fires on any type exposing `len()` without
    /// it, because callers reach for `is_empty()` first and a missing one pushes
    /// them to `len() == 0`.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }
}

/// Split one stored entry into its `(statement_type, evidence_strength)` halves.
///
/// Both halves are trimmed and must be non-blank: `admission+` is a typo that
/// would otherwise map every item with a blank strength into a tier.
fn split_pair(key: &str, entry: &str) -> Result<(String, String), PairParseError> {
    let malformed = || PairParseError {
        key: key.to_string(),
        entry: entry.to_string(),
        separator: PAIR_SEPARATOR,
    };
    let (statement_type, evidence_strength) =
        entry.split_once(PAIR_SEPARATOR).ok_or_else(malformed)?;
    let statement_type = statement_type.trim();
    let evidence_strength = evidence_strength.trim();
    if statement_type.is_empty() || evidence_strength.is_empty() {
        return Err(malformed());
    }
    Ok((statement_type.to_string(), evidence_strength.to_string()))
}

/// A map for TESTS ONLY, carrying the six pairs measured on DEV.
///
/// ## Why this is `#[cfg(test)]` and not a `Default` impl
///
/// The same argument `Settings::for_test` makes: a `Default` would be a
/// compiled-in tier map — the exact thing Standing Rule 2 bans — and would be
/// reachable from production by accident through `unwrap_or_default()`. Gating it
/// on `cfg(test)` means it cannot exist in a release binary at all.
///
/// `wording_matrix`'s seed test separately proves the MIGRATION seeds these same
/// six pairs, so the fixture and the shipped rows cannot drift apart silently.
#[cfg(test)]
impl EvidenceTierMap {
    pub fn for_test() -> Self {
        let strong = [
            "admission+sworn_party_admission",
            "court_finding+court_finding",
            "court_order+court_order",
        ]
        .map(str::to_string)
        .to_vec();
        let hedged = [
            "partial_admission+sworn_party_admission",
            "partial_admission+sworn_party_evasion",
        ]
        .map(str::to_string)
        .to_vec();
        let other = ["factual_assertion+sworn_testimony"]
            .map(str::to_string)
            .to_vec();
        EvidenceTierMap::from_entries(&[
            (EvidenceTier::Strong, "matrix_tier_strong_pairs", &strong),
            (EvidenceTier::Hedged, "matrix_tier_hedged_pairs", &hedged),
            (EvidenceTier::Other, "matrix_tier_other_pairs", &other),
        ])
        .expect("the ruled fixture parses")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_string()).collect()
    }

    /// The measured DEV vocabulary, mapped exactly as Roman ruled on 2026-08-13.
    ///
    /// Delegates to the shared `for_test` fixture rather than re-listing the six
    /// pairs: two copies of one vocabulary is how a test starts asserting against
    /// a map the product does not have.
    fn ruled_map() -> EvidenceTierMap {
        EvidenceTierMap::for_test()
    }

    #[test]
    fn tier_tokens_round_trip() {
        for &tier in EvidenceTier::ALL {
            let decoded = EvidenceTier::try_from(tier.code()).expect("its own token decodes");
            assert_eq!(decoded, tier);
        }
    }

    #[test]
    fn rejects_an_unknown_tier_token_and_names_it() {
        // Standing Rule 1: a tier this build does not define is a loud, named
        // failure — never a silent collapse into `other`, which would put an
        // unmapped class into a tier a human would read as deliberate.
        let error = EvidenceTier::try_from("firm").expect_err("undefined tier must not decode");
        assert_eq!(error.token, "firm");
        assert!(error.to_string().contains("firm"));
    }

    #[test]
    fn rank_orders_strongest_first_and_is_total() {
        assert!(EvidenceTier::Strong.rank() < EvidenceTier::Hedged.rank());
        assert!(EvidenceTier::Hedged.rank() < EvidenceTier::Other.rank());
        let mut ranks: Vec<u8> = EvidenceTier::ALL.iter().map(|t| t.rank()).collect();
        ranks.sort_unstable();
        ranks.dedup();
        assert_eq!(
            ranks.len(),
            EvidenceTier::ALL.len(),
            "two tiers share a rank"
        );
    }

    #[test]
    fn all_lists_every_variant_exactly_once() {
        assert_eq!(EvidenceTier::ALL.len(), 3);
        for &tier in EvidenceTier::ALL {
            let matches = EvidenceTier::ALL.iter().filter(|t| **t == tier).count();
            assert_eq!(matches, 1, "{tier:?} appears more than once in ALL");
        }
    }

    /// The whole reason the key is a PAIR: `sworn_party_admission` is STRONG under
    /// `admission` and HEDGED under `partial_admission`. Measured on DEV — 21
    /// items in the first class, 12 in the second. If this ever collapsed to
    /// strength alone, twelve qualified answers would join the headline number
    /// Chuck reads as "cannot be disputed".
    #[test]
    fn the_same_strength_ranks_differently_under_two_statement_types() {
        let map = ruled_map();
        assert_eq!(
            map.tier_for(Some("admission"), Some("sworn_party_admission")),
            Some(EvidenceTier::Strong),
        );
        assert_eq!(
            map.tier_for(Some("partial_admission"), Some("sworn_party_admission")),
            Some(EvidenceTier::Hedged),
        );
    }

    #[test]
    fn maps_every_measured_dev_pair() {
        let map = ruled_map();
        // The six (statement_type, evidence_strength) combinations measured across
        // every CORROBORATES edge on DEV, 2026-08-13.
        let cases = [
            ("admission", "sworn_party_admission", EvidenceTier::Strong),
            ("court_finding", "court_finding", EvidenceTier::Strong),
            ("court_order", "court_order", EvidenceTier::Strong),
            (
                "partial_admission",
                "sworn_party_admission",
                EvidenceTier::Hedged,
            ),
            (
                "partial_admission",
                "sworn_party_evasion",
                EvidenceTier::Hedged,
            ),
            ("factual_assertion", "sworn_testimony", EvidenceTier::Other),
        ];
        for (statement_type, strength, expected) in cases {
            assert_eq!(
                map.tier_for(Some(statement_type), Some(strength)),
                Some(expected),
                "{statement_type}+{strength} must map to {expected:?}",
            );
        }
        assert_eq!(map.len(), cases.len());
    }

    /// A pair a Settings typo put in two lists takes the WEAKER tier.
    ///
    /// The documented safe reading, asserted. The dangerous direction is the
    /// other one: if the first listing won, a pair fat-fingered into the strong
    /// row would promote itself into the headline number Chuck reads as
    /// "cannot be disputed". Losing a tier of rank is recoverable; inventing one
    /// is not.
    #[test]
    fn a_pair_listed_in_two_tiers_takes_the_weaker_one() {
        let both = entries(&["admission+sworn_party_admission"]);
        let map = EvidenceTierMap::from_entries(&[
            (EvidenceTier::Strong, "matrix_tier_strong_pairs", &both),
            (EvidenceTier::Hedged, "matrix_tier_hedged_pairs", &both),
        ])
        .expect("a duplicate is a typo, not a boot refusal");
        assert_eq!(
            map.tier_for(Some("admission"), Some("sworn_party_admission")),
            Some(EvidenceTier::Hedged),
            "the later (weaker) listing must win — a typo must never promote a \
             pair into the headline",
        );
        assert_eq!(map.len(), 1, "the duplicate collapses to one entry");
    }

    /// An unmapped pair is `None` — visible and unranked, never promoted into the
    /// headline and never dropped. A new document type must not be able to inflate
    /// the number Chuck leads with.
    #[test]
    fn an_unmapped_pair_has_no_tier() {
        let map = ruled_map();
        assert_eq!(
            map.tier_for(Some("attorney_argument"), Some("attorney_assertion")),
            None,
        );
    }

    /// A recitation is not in the map at all, because pass 2 creates no
    /// finding-edge from one — the tier is structurally empty on this surface, and
    /// Roman ruled it must not be rendered as an always-zero row.
    #[test]
    fn a_recited_position_is_unmapped() {
        assert_eq!(
            ruled_map().tier_for(Some("party_recitation"), Some("recited_party_position")),
            None,
        );
    }

    /// Half a key is not a key. An item carrying only one of the two properties
    /// cannot be tiered — guessing from the half that is present would let a
    /// missing `statement_type` land an item in the headline.
    #[test]
    fn a_half_present_pair_has_no_tier() {
        let map = ruled_map();
        assert_eq!(map.tier_for(None, Some("sworn_party_admission")), None);
        assert_eq!(map.tier_for(Some("admission"), None), None);
        assert_eq!(map.tier_for(None, None), None);
    }

    /// Stored values are trimmed by the store, but a human editing the row types
    /// spaces after commas — so the parser trims each half too.
    #[test]
    fn entries_tolerate_surrounding_whitespace() {
        let strong = entries(&["  admission + sworn_party_admission  "]);
        let map = EvidenceTierMap::from_entries(&[(
            EvidenceTier::Strong,
            "matrix_tier_strong_pairs",
            &strong,
        )])
        .expect("whitespace is trimmed, not rejected");
        assert_eq!(
            map.tier_for(Some("admission"), Some("sworn_party_admission")),
            Some(EvidenceTier::Strong),
        );
    }

    /// A row missing the separator is a BOOT REFUSAL naming the row and the entry
    /// — not a silently skipped pair, which would quietly drop items out of the
    /// headline with nothing in the log.
    #[test]
    fn an_entry_without_the_separator_is_refused_and_names_its_row() {
        let bad = entries(&["admission"]);
        let error = EvidenceTierMap::from_entries(&[(
            EvidenceTier::Strong,
            "matrix_tier_strong_pairs",
            &bad,
        )])
        .expect_err("a malformed entry must refuse");
        assert_eq!(error.key, "matrix_tier_strong_pairs");
        assert_eq!(error.entry, "admission");
        let message = error.to_string();
        assert!(message.contains("matrix_tier_strong_pairs"), "{message}");
        assert!(message.contains("admission"), "{message}");
    }

    /// `admission+` would otherwise map every item whose strength is blank.
    #[test]
    fn an_entry_with_a_blank_half_is_refused() {
        for entry in ["admission+", "+sworn_party_admission", " + "] {
            let bad = entries(&[entry]);
            assert!(
                EvidenceTierMap::from_entries(&[(
                    EvidenceTier::Strong,
                    "matrix_tier_strong_pairs",
                    &bad
                )])
                .is_err(),
                "{entry:?} must be refused",
            );
        }
    }

    /// An empty row is legal: a tier with no pairs simply names nothing, which is
    /// what lets a deployment run with (say) no `other` class without editing code.
    #[test]
    fn an_empty_entry_list_is_legal() {
        let map =
            EvidenceTierMap::from_entries(&[(EvidenceTier::Other, "matrix_tier_other_pairs", &[])])
                .expect("an empty tier is legal");
        assert!(map.is_empty());
    }
}
