// =============================================================================
// backend/src/services/matrix_order.rs — the order a paragraph's evidence reads in
// =============================================================================
//
// PROOF_MATRIX_v2 §3, in one pure function. Given every item the graph links to
// one accusation — with what the linking pass said about it and what a human has
// since said — this decides three things:
//
//   1. WHICH items are hidden (`role = does_not_belong`, or `ruling = remove`),
//   2. WHICH items fold under another as "×N" (`role = duplicate_of`),
//   3. WHAT ORDER the rest read in.
//
// ## The order, and why each key is where it is
//
//   kept (human) → rank → confidence → document date, oldest first
//
// A human's `keep` leads because it is the only key here that is not a machine's
// opinion: it says somebody read this item and agreed, and on a page whose whole
// risk is being mistaken for reviewed, that has to be visible from the top.
// `rank` is the pass's own ordering within a stance and comes next.
// `confidence` separates items the pass ranked equally. `document date, oldest
// first` is the last key because the earliest record of a thing is the one a
// hearing wants — and because a stable tie-break has to exist at all.
//
// ## Why the ID is a fifth key nobody asked for
//
// Four keys can still tie: two items ranked the same, rated the same, out of the
// same undated document. Without a final total key the sort is arbitrary among
// them, and an arbitrary order on a page that CLAIMS its order means something
// would shuffle between two reads of unchanged data. The id is stable, so it
// ends the comparison.
//
// ## Why this is pure, and takes no `EvidenceRef`
//
// [`OrderInput`] is a flat struct of the six things the decision needs. It does
// not borrow the repository's DTO, so every rule below is testable from a literal
// — and the export and the drill-down get their order from ONE function rather
// than from two that agree today.

use std::collections::HashMap;

use crate::domain::matrix_edge::{EdgeRole, LinkConfidence};
use crate::domain::matrix_ruling::MatrixRuling;

/// One item as the ordering sees it.
///
/// ## Rust Learning: `&'a str` fields instead of `String`
///
/// The caller already owns the ids; copying them into this struct to sort them
/// and copying them back out would allocate twice per item for nothing. The
/// lifetime `'a` says these references must outlive the `OrderInput` — which they
/// do, because the caller holds the items the whole time it is sorting them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderInput<'a> {
    /// The Evidence node id. The stable tie-break, and the fold's join key.
    pub id: &'a str,
    /// `r.rank` — the pass's position within a stance, 1..N. `None` on the 36
    /// edges the pass never reached.
    pub rank: Option<i64>,
    /// `r.role`. `None` when absent or unreadable — see the module header of
    /// [`crate::domain::matrix_edge`] for why an unreadable role shows the item.
    pub role: Option<EdgeRole>,
    /// `r.confidence`. `None` is a real state: 286 edges predate the pass.
    pub confidence: Option<LinkConfidence>,
    /// `r.duplicate_of_card_id` — the item this one restates.
    pub duplicate_of: Option<&'a str>,
    /// The source document's date, `YYYY-MM-DD`. `None` for a document with no
    /// date and for the empty strings four documents carry.
    pub document_date: Option<&'a str>,
    /// The human's verdict, if any. `None` means unruled — nobody has read it.
    pub ruling: Option<MatrixRuling>,
}

/// Why an item is not in the default list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HiddenReason {
    /// The linking pass retracted the edge.
    MachineExcluded,
    /// A human removed it.
    HumanRemoved,
}

impl HiddenReason {
    /// The token the API serves. The frontend does not branch on it — the two
    /// hidden groups render alike — but the reason a specific item is missing
    /// from a proof list is exactly the kind of fact that must not be inferable
    /// only from the absence.
    pub fn code(self) -> &'static str {
        match self {
            HiddenReason::MachineExcluded => "does_not_belong",
            HiddenReason::HumanRemoved => "removed",
        }
    }
}

/// One item's place in the ordered list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedItem {
    /// The Evidence node id, so the caller can attach this to its own row.
    pub id: String,
    /// How many items this row stands for: itself, plus every duplicate folded
    /// into it. `1` for the overwhelming majority.
    pub occurrences: usize,
    /// `None` for a visible item.
    pub hidden_reason: Option<HiddenReason>,
}

/// Order, hide and fold one accusation's items (§3).
///
/// Returns EVERY input item exactly once, visible ones first (in §3 order) and
/// hidden ones after (in the same order), except duplicates folded into a visible
/// target — those are represented by the target's `occurrences` and do not
/// appear as rows.
///
/// ## Domain note: nothing is ever dropped
///
/// A folded duplicate is the only item that leaves the list, and it leaves it by
/// being counted somewhere the reader can see. Everything else — retracted,
/// removed, unreadable, orphaned — is still a row. This is a proof surface: an
/// item that disappears with nothing on screen to say so is the one failure mode
/// that cannot be recovered by looking harder.
pub fn order_items(items: &[OrderInput<'_>]) -> Vec<OrderedItem> {
    // Pass 1: decide visibility for every item, keyed by id, so pass 2 can ask
    // whether a duplicate's TARGET is visible without re-deriving the rule.
    let hidden: HashMap<&str, Option<HiddenReason>> =
        items.iter().map(|i| (i.id, hidden_reason(i))).collect();

    // Pass 2: fold duplicates into their targets and count them.
    let folded = fold_duplicates(items, &hidden);

    // Pass 3: sort the two groups separately and concatenate. Separately rather
    // than with visibility as a leading sort key, because the caller takes the
    // visible prefix by `hidden_reason.is_none()` and a single sort would make
    // that prefix depend on a comparator rule instead of on the partition.
    let (mut visible, mut concealed): (Vec<_>, Vec<_>) = folded
        .into_iter()
        .partition(|(item, _)| hidden[item.id].is_none());
    visible.sort_by(|a, b| compare(&a.0, &b.0));
    concealed.sort_by(|a, b| compare(&a.0, &b.0));

    visible
        .into_iter()
        .chain(concealed)
        .map(|(item, occurrences)| OrderedItem {
            id: item.id.to_string(),
            occurrences,
            hidden_reason: hidden[item.id],
        })
        .collect()
}

/// Why an item is hidden, or `None` if it is shown (§3's hide rules).
///
/// ## Domain note: an explicit `keep` overrules the machine's retraction
///
/// The pass marked 41 edges `does_not_belong`. If a human opens the hidden group,
/// reads one and presses Keep, the only sensible meaning is "the machine was
/// wrong about this one" — so `Keep` un-hides it. Without that, the Keep button
/// on a retracted item would be a control that visibly does nothing, and the
/// machine's retraction would be the one machine claim a human cannot overrule.
///
/// `Remove` always hides, whatever the role: a human's removal is never
/// overruled by anything, because nothing here outranks it.
fn hidden_reason(item: &OrderInput<'_>) -> Option<HiddenReason> {
    match item.ruling {
        Some(MatrixRuling::Remove) => Some(HiddenReason::HumanRemoved),
        Some(MatrixRuling::Keep) => None,
        None => item
            .role
            .filter(|role| role.hides_the_item())
            .map(|_| HiddenReason::MachineExcluded),
    }
}

/// Fold duplicates into their targets, returning each surviving row with its
/// occurrence count.
///
/// ## The three ways a duplicate does NOT fold, and why each one stands alone
///
/// 1. **Its target is not in this list.** A `duplicate_of_card_id` pointing at an
///    item that no longer bears on this accusation is a stale pointer — the same
///    class of defect that orphaned 26 curated rulings on 2026-07-24. Folding it
///    into nothing would delete it.
/// 2. **Its target is hidden.** Folding a live item under a retracted or removed
///    one would carry it out of the list, so a human's Remove on one row would
///    silently take a second row with it.
/// 3. **A human has ruled on it.** The ruling is a statement about THAT row. A
///    row somebody kept is not a "×1" on a different row's tally.
///
/// In all three cases the item stays a row of its own, which is the outcome that
/// shows too much rather than too little.
fn fold_duplicates<'a>(
    items: &[OrderInput<'a>],
    hidden: &HashMap<&str, Option<HiddenReason>>,
) -> Vec<(OrderInput<'a>, usize)> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut survivors: Vec<OrderInput<'a>> = Vec::with_capacity(items.len());

    for item in items {
        match fold_target(item, hidden) {
            Some(target) => *counts.entry(target).or_insert(0) += 1,
            None => survivors.push(*item),
        }
    }

    survivors
        .into_iter()
        .map(|item| {
            let folded = counts.get(item.id).copied().unwrap_or(0);
            (item, 1 + folded)
        })
        .collect()
}

/// The id this item folds into, or `None` if it stands on its own.
///
/// Split from [`fold_duplicates`] so the three standing-alone rules are one
/// readable expression rather than a nested `if` inside a loop.
fn fold_target<'a>(
    item: &OrderInput<'a>,
    hidden: &HashMap<&str, Option<HiddenReason>>,
) -> Option<&'a str> {
    if !item.role.is_some_and(EdgeRole::is_duplicate) || item.ruling.is_some() {
        return None;
    }
    let target = item.duplicate_of?;
    // A pointer to itself would fold the row into its own tally and delete it.
    if target == item.id {
        return None;
    }
    match hidden.get(target) {
        Some(None) => Some(target),
        // Absent from the map (stale pointer) or present but hidden.
        _ => None,
    }
}

/// §3's comparator, in order: kept → rank → confidence → document date → id.
///
/// ## Rust Learning: `Ordering::then_with` as a chain of tie-breaks
///
/// `a.cmp(&b)` returns an `Ordering`; `.then_with(|| …)` evaluates the next key
/// ONLY when the previous one tied. Chaining them reads top to bottom in exactly
/// the priority order §3 states, and the closure means a later key costs nothing
/// when an earlier one decides.
///
/// Each key turns its value into something already ordered the right way, so
/// there is no `.reverse()` anywhere — a reversal buried mid-chain is the classic
/// way a comparator ends up meaning something nobody intended.
fn compare(a: &OrderInput<'_>, b: &OrderInput<'_>) -> std::cmp::Ordering {
    kept_weight(a)
        .cmp(&kept_weight(b))
        .then_with(|| rank_weight(a).cmp(&rank_weight(b)))
        .then_with(|| confidence_weight(a).cmp(&confidence_weight(b)))
        .then_with(|| date_weight(a).cmp(&date_weight(b)))
        .then_with(|| a.id.cmp(b.id))
}

/// 0 for an item a human kept, 1 for everything else.
fn kept_weight(item: &OrderInput<'_>) -> u8 {
    u8::from(item.ruling != Some(MatrixRuling::Keep))
}

/// The pass's rank, with unranked items sorted last.
///
/// `i64::MAX` rather than a separate `Option` comparison because `Option<i64>`
/// orders `None` FIRST, which would float the 36 edges the pass never reached
/// above every ranked item — the opposite of what §3 wants.
fn rank_weight(item: &OrderInput<'_>) -> i64 {
    item.rank.unwrap_or(i64::MAX)
}

/// The confidence weight, with unrated items sorted last.
fn confidence_weight(item: &OrderInput<'_>) -> u8 {
    item.confidence
        .map(LinkConfidence::sort_weight)
        .unwrap_or(LinkConfidence::UNRATED_SORT_WEIGHT)
}

/// The document date as a sortable key: oldest first, undated last.
///
/// ## Why a `&str` compares correctly here, and why it is still checked
///
/// `YYYY-MM-DD` sorts lexicographically in exactly date order, so no parsing is
/// needed — but only for values that ARE that shape. Four documents carry an
/// empty string and six carry no property at all, and an empty string sorts
/// BEFORE every real date, which would put the undated documents first. So an
/// empty value is treated as absent, and absent sorts last.
///
/// A malformed non-empty value (say `2010`) is left to sort where it falls rather
/// than being discarded: it is still evidence of a date, it is visibly odd in the
/// list, and silently reclassifying it as undated would hide a data problem on
/// the surface most likely to reveal it.
fn date_weight<'a>(item: &OrderInput<'a>) -> (u8, &'a str) {
    match item.document_date {
        Some(date) if !date.trim().is_empty() => (0, date),
        _ => (1, ""),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────
//
// Three sibling files rather than one, because one passed the 300-line module
// limit (Rule 17) and because two of them answer genuinely different questions:
// what ORDER the list reads in, and which items are IN it at all. The third
// holds the builder both use.
//
// ## Rust Learning: `#[path]` at file scope, not nested
//
// Each `#[path]` below resolves relative to THIS file's directory
// (`src/services/`), which is where the three files sit. Nesting them inside a
// `mod tests { … }` would resolve them relative to `src/services/matrix_order/`
// — a directory that does not exist, so even a `../../` climb out of it fails.

#[cfg(test)]
#[path = "matrix_order_test_support.rs"]
mod test_support;

#[cfg(test)]
#[path = "matrix_order_order_tests.rs"]
mod order_tests;

#[cfg(test)]
#[path = "matrix_order_rules_tests.rs"]
mod rules_tests;
