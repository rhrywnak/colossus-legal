// =============================================================================
// backend/src/services/matrix_strength.rs — one place the matrix's numbers are
// decided (task 396, P1)
// =============================================================================
//
// The Proof Matrix now says three things about an Element's proof, and they have
// to agree: the headline STRONG count on the row, the "· N approved" depth line
// beside it, and the ranked drill-down under it. They agree because all three are
// produced from this one function over the same descriptors — the alternative
// (a Cypher aggregate for the row and a Rust fold for the drill-down) is exactly
// how two numbers describing one thing drift apart.
//
// ## What "collapse" means here, and the trap it had to be steered around
//
// Roman's ruling: near-identical statements count ONCE, carrying "×N". The
// obvious key — same speaker + same normalized quote — was MEASURED against DEV
// before it was built, and it over-collapses badly:
//
//     George Phillips        "yes."   ×3
//     Catholic Family Svcs   "yes"    ×2
//
// Those are five distinct sworn admissions to five distinct interrogatories that
// happen to share their answer word. Collapsing them would delete three real
// admissions from the number Chuck reads. So for a Q/A item the key includes the
// QUESTION, and only documentary evidence (which answers nobody) keys on speaker
// + quote alone. Amendment ruled 2026-08-13.
//
// ## Why normalization stops at case and whitespace
//
// It does NOT strip punctuation or trailing periods. Stripping widens the net,
// and over-collapsing is the failure this module was just corrected for — a
// duplicate that survives is a visible row a human can merge later, while a
// wrongly merged pair is proof that silently ceased to exist.

use std::collections::HashSet;

use crate::domain::evidence_tier::{EvidenceTier, EvidenceTierMap};

/// One corroborating item, as the graph describes it.
///
/// Deliberately a flat, owned struct rather than a borrow of the repository's
/// `EvidenceRef`: the two callers hold different row types (the matrix read
/// projects an element-wide list, the detail read holds per-allegation
/// `EvidenceRef`s), and a shared shape is what lets one function serve both.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorroboratingItem {
    /// `Evidence.id` — the identity used to drop true duplicates before any
    /// near-duplicate reasoning happens.
    pub id: String,
    /// `Evidence.statement_type`. Half of the tier key.
    pub statement_type: Option<String>,
    /// `Evidence.evidence_strength`. The other half.
    pub evidence_strength: Option<String>,
    /// The speaker, via `(Evidence)-[:STATED_BY]->(Party)`. `None` for an item
    /// with no speaker edge — which does not merge with anything, see
    /// [`collapse_key`].
    pub speaker: Option<String>,
    /// The interrogatory question this answers, or `None` for documentary
    /// evidence. Presence of a question is what selects the collapse key.
    pub question: Option<String>,
    /// `Evidence.verbatim_quote`.
    pub quote: Option<String>,
}

/// One row of the ranked, collapsed list.
///
/// Domain note: `occurrences` is the "×N" the row prints. It is 1 for the
/// overwhelming majority of rows, and a row printing "×1" would be noise — the
/// renderer shows the marker only above 1, which is a display decision the
/// wording row expresses, not something this module pre-judges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollapsedItem {
    /// The item that represents the group — the first one seen, so the ordering
    /// the caller supplied decides which quote is shown.
    pub lead: CorroboratingItem,
    /// The tier, or `None` when the map does not name this item's pair. Unmapped
    /// items rank LAST and carry no chip; they are never hidden.
    pub tier: Option<EvidenceTier>,
    /// How many distinct Evidence nodes collapsed into this row (1 = no
    /// duplicates). This is the "×N".
    pub occurrences: usize,
    /// The ids of the items that merged into `lead`, excluding the lead itself.
    /// Kept so an operator log — or a later "show the duplicates" affordance —
    /// can name exactly what was collapsed. Nothing is thrown away.
    pub merged_ids: Vec<String>,
}

/// The two numbers a matrix row prints.
///
/// ## Rust Learning: why `Default` is derived here
///
/// An Element with no corroborating Evidence never appears in the strength read's
/// result map, so the caller reads it with `unwrap_or_default()`. Deriving
/// `Default` makes "zero of both" the named, obvious reading of that absence
/// rather than a hand-written zero literal at the call site.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StrengthTally {
    /// Collapsed items whose tier is [`EvidenceTier::Strong`] — the headline.
    pub strong: usize,
    /// Every collapsed item, whatever its tier — the "· N approved" depth line.
    ///
    /// Domain note: this is the count AFTER duplicate collapse, because Roman
    /// ruled the counts must agree with the drill-down, and the drill-down shows
    /// one row per collapsed group. A pre-collapse figure would make the small
    /// print disagree with the list it describes.
    pub approved: usize,
}

/// Normalize one piece of text for comparison: lowercase, collapse every run of
/// whitespace to a single space, trim the ends.
///
/// ## Rust Learning: `split_whitespace` as the whitespace collapser
///
/// `split_whitespace()` already skips runs of any whitespace (spaces, tabs, the
/// newlines an OCR'd quote is full of), so joining its pieces with one space both
/// collapses and trims in a single pass — no regex, no allocation per run.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The key two items must share to collapse into one row, or `None` when this
/// item must never merge with anything.
///
/// Returns `None` when the quote is absent or blank: an item with no words has
/// nothing to be near-identical TO, and letting every quoteless item share one
/// key would collapse unrelated proof into a single row.
///
/// A `None` speaker is carried into the key as an empty marker rather than
/// refusing to merge: two quoteless-speaker items with the identical question and
/// answer really are the same statement recorded twice, and that is exactly the
/// C-9≡C-11 class.
fn collapse_key(item: &CorroboratingItem) -> Option<(String, String, String)> {
    let quote = normalize(item.quote.as_deref()?);
    if quote.is_empty() {
        return None;
    }
    let speaker = normalize(item.speaker.as_deref().unwrap_or_default());
    // The question half is empty for documentary evidence, which is the
    // "speaker + quote" key the ruling names for that class. For a Q/A item it is
    // the third component that keeps three distinct "yes." admissions apart.
    let question = normalize(item.question.as_deref().unwrap_or_default());
    Some((speaker, question, quote))
}

/// Collapse duplicates, tier every survivor, and rank the result strongest first.
///
/// Ordering is `tier rank` ascending with unmapped items last, then the order the
/// caller supplied. That secondary key matters: the callers hand items in
/// paragraph / document order, so two Strong items stay in the order a reader
/// expects rather than moving between renders.
///
/// ## Rust Learning: `sort_by_key` on a tuple, and why the sort must be STABLE
///
/// `Vec::sort_by_key` is a stable sort — equal keys keep their relative input
/// order. That is what makes "then the order the caller supplied" true without
/// carrying an explicit index in the key. `sort_unstable_by_key` would be faster
/// and would let two equally-ranked rows swap places between two renders of the
/// same data, which reads as the page shuffling itself.
///
/// ## Rust Learning: `Ord for Option<T>` puts `None` FIRST
///
/// Sorting on the bare `Option<u8>` rank looks right and is backwards: the
/// derived ordering makes `None < Some(_)`, so every UNMAPPED item would head the
/// list — a new document type would arrive at the top of Chuck's drill-down,
/// above the sworn admissions. `map_or(RANK_UNMAPPED, …)` states the intent
/// instead of inheriting an ordering from the wrapper type. Caught by
/// `ranking_is_strong_then_hedged_then_other_then_unmapped`, which is why that
/// test supplies its items in deliberately reversed order.
pub fn collapse_and_rank(
    items: &[CorroboratingItem],
    tier_map: &EvidenceTierMap,
) -> Vec<CollapsedItem> {
    let mut seen_ids: HashSet<&str> = HashSet::new();
    let mut groups: Vec<CollapsedItem> = Vec::new();
    // Parallel to `groups`: the collapse key of each group, so a new item can find
    // its group without re-deriving every existing one.
    let mut keys: Vec<Option<(String, String, String)>> = Vec::new();

    for item in items {
        // A repeated id is the SAME node arriving twice (the fan-out queries can
        // surface one Evidence on several rows). It is not a near-duplicate and
        // must not inflate "×N".
        if !seen_ids.insert(item.id.as_str()) {
            continue;
        }

        let key = collapse_key(item);
        // `Some(k)` finds its group; `None` never matches, so an unquoted item
        // always starts its own row.
        let existing = key
            .as_ref()
            .and_then(|k| keys.iter().position(|other| other.as_ref() == Some(k)));

        match existing {
            Some(index) => {
                groups[index].occurrences += 1;
                groups[index].merged_ids.push(item.id.clone());
            }
            None => {
                let tier = tier_map.tier_for(
                    item.statement_type.as_deref(),
                    item.evidence_strength.as_deref(),
                );
                groups.push(CollapsedItem {
                    lead: item.clone(),
                    tier,
                    occurrences: 1,
                    merged_ids: Vec::new(),
                });
                keys.push(key);
            }
        }
    }

    groups.sort_by_key(|group| group.tier.map_or(RANK_UNMAPPED, EvidenceTier::rank));
    groups
}

/// The sort rank an item with no mapped tier gets: after every named tier.
///
/// A named constant rather than `u8::MAX` inline, so the "unmapped ranks last"
/// rule is a statement in the code and not an arithmetic accident — and so
/// adding a fourth tier cannot silently collide with it.
const RANK_UNMAPPED: u8 = u8::MAX;

/// The headline and depth numbers for one already-collapsed list.
///
/// Takes the collapsed rows rather than the raw items so the two numbers cannot
/// be computed from a different input than the list they sit above — the whole
/// point of this module.
pub fn tally(groups: &[CollapsedItem]) -> StrengthTally {
    StrengthTally {
        strong: groups
            .iter()
            .filter(|g| g.tier == Some(EvidenceTier::Strong))
            .count(),
        approved: groups.len(),
    }
}

#[cfg(test)]
#[path = "matrix_strength_tests.rs"]
mod tests;
