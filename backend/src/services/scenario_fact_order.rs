//! Where a dragged fact lands — the sparse-ordinal arithmetic, pure (task 2.13).
//!
//! The signed design's first law is that every card is independent: dragging one
//! fact must write ONE row and must not move any other card's stored position. A
//! dense `0,1,2,3…` order cannot do that (inserting at the front renumbers
//! everything), so ordinals are assigned SPARSELY in steps of
//! [`ORDINAL_STEP`] and a drop between two neighbours takes the midpoint of
//! theirs.
//!
//! ## Why the arithmetic is HERE and not in the browser
//!
//! The client knows what the human dragged and between which two rows they
//! dropped it — that is all it sends. The number is computed from what is
//! STORED, so a stale page cannot write a position derived from a list that has
//! since changed, and Rule 12 holds: the judgment lives on the server.
//!
//! ## The unplaced tail, and why it needs provisional numbers
//!
//! Most facts have never been dragged, so their `sort_ordinal` is NULL — a real
//! state meaning "the human has not placed this", NOT a missing value. Those sort
//! AFTER every placed fact, in their existing C-ordinal order. To drop something
//! between two of them we still need two numbers to take a midpoint of, so this
//! module derives a PROVISIONAL ordinal for each unplaced fact from its position
//! in that tail. Provisional numbers are never stored — only the dragged card's
//! computed ordinal is written — which is what keeps a drag to exactly one row.

use crate::domain::fact_tier::FactTier;

/// The gap left between adjacent ordinals when one is assigned fresh.
///
/// ## Rust Learning: `const` for a structural constant, not a tunable
///
/// Standing Rule 2 governs values that vary across environments, cases or
/// configurations — this varies across none of them. It is the branching factor
/// of the ordinal scheme: 1024 admits ten successive midpoint inserts into the
/// same gap (1024 → 512 → 256 → … → 1) before the space is exhausted, which is
/// far past what a human does between two facts in one sitting. Making it
/// editable would let a setting change silently reduce how many drags succeed
/// before the refusal below starts firing.
pub const ORDINAL_STEP: i32 = 1024;

/// One included fact, as the ordering needs to see it.
///
/// Deliberately NOT the full `ScenarioFactRefRecord`: this module does arithmetic
/// on positions and must not be able to reach a quote, a ruling or a note. The
/// narrow struct is the boundary that keeps it that way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderableFact {
    pub graph_node_id: String,
    /// The stored position, or `None` when the human has never placed this fact.
    pub sort_ordinal: Option<i32>,
    /// The candidate ordinal behind the `C-n` chip, which orders the unplaced
    /// tail. `None` for a fact gather has not numbered yet — those sort last.
    pub code_ordinal: Option<i32>,
    /// The weight tier. Ordering runs WITHIN a tier: the list groups by tier
    /// first, so dragging never silently re-weights a fact and starring never
    /// silently re-orders one.
    pub tier: FactTier,
}

/// Why a requested move cannot be stored.
///
/// ## Rust Learning: a typed refusal enum, not `Option<i32>`
///
/// Returning `None` would collapse three different failures into one silence and
/// leave the handler with nothing to tell the human (Standing Rule 1). Each
/// variant here names a different thing that is wrong, and each maps to its own
/// HTTP status and its own settings-served message.
///
/// ## Why every message ends in a next step
///
/// These strings reach the human VERBATIM, as `{reason}` inside the stored
/// failure template — the route deliberately does not swap them for a generic
/// "could not save" (see `move_refusal_to_app_error`). A message that names what
/// went wrong but not what to do leaves them dragging the same row again, which
/// is how a correct refusal gets reported as a broken feature. Each one
/// therefore says WHAT happened and WHAT to do about it: reload for the two
/// stale-page cases, drop it elsewhere for the exhausted gap.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MoveRefusal {
    /// The fact being dragged is not among this scenario's included facts.
    #[error(
        "the fact being moved is not in this scenario — reload the page to see the current list"
    )]
    DraggedNotHere,
    /// A named neighbour is not among them — a stale page describing a list that
    /// has changed underneath it.
    #[error("the fact it was dropped beside is no longer in this scenario — reload the page to see the current list")]
    NeighbourNotHere,
    /// The two neighbours are adjacent integers, so there is no number between
    /// them.
    ///
    /// ## Why this REFUSES rather than renumbering
    ///
    /// The obvious repair is to renumber the list and retry. That would write
    /// every row — breaking the one-row law — and it would do so silently, at an
    /// unpredictable moment, on the surface where the human's own ordering
    /// judgment lives. Ruled 2026-08-05: detect and refuse loudly; compaction is
    /// slice 2's if this ever actually fires. Ten successive drops into one gap
    /// are needed to reach it.
    #[error("there is no room left between those two facts — try dropping it in a slightly different position")]
    GapExhausted,
}

/// Every fact in display order, each paired with the ordinal the list SEES.
///
/// Placed facts (a stored `sort_ordinal`) come first in ordinal order; unplaced
/// ones follow in `code_ordinal` order, carrying a provisional number derived
/// from their position so a midpoint can be taken next to them. Ties everywhere
/// break on `graph_node_id`, which is unique per scenario — so the order is
/// TOTAL, and two renders of the same data can never disagree.
///
/// Grouping is by tier first ([`FactTier::rank`]), because the list renders
/// `carries` above `backup` above `background`: a fact's weight decides which
/// block it is in, and its ordinal decides where it sits inside that block.
fn effective_order(facts: &[OrderableFact]) -> Vec<(String, i32)> {
    let placed = sorted_placed(facts);
    let unplaced = sorted_unplaced(facts);

    // The provisional numbers start above every stored one so the tail always
    // sorts after the placed block, whatever ordinals the human has written.
    let ceiling = placed
        .iter()
        .filter_map(|f| f.sort_ordinal)
        .max()
        .unwrap_or(0);

    let mut out: Vec<(String, i32)> = placed
        .iter()
        .filter_map(|f| f.sort_ordinal.map(|o| (f.graph_node_id.clone(), o)))
        .collect();

    out.extend(unplaced.iter().enumerate().map(|(index, fact)| {
        (
            fact.graph_node_id.clone(),
            provisional_ordinal(ceiling, index),
        )
    }));

    out
}

/// The facts the human HAS placed, in the order they will render.
///
/// Tier first (the block), then the stored ordinal (the seat), then the node id
/// so ties break deterministically — the node id is unique per scenario, which
/// is what makes the whole ordering TOTAL rather than merely mostly-decided.
fn sorted_placed(facts: &[OrderableFact]) -> Vec<&OrderableFact> {
    let mut placed: Vec<&OrderableFact> =
        facts.iter().filter(|f| f.sort_ordinal.is_some()).collect();
    placed.sort_by(|a, b| {
        (a.tier.rank(), a.sort_ordinal, &a.graph_node_id).cmp(&(
            b.tier.rank(),
            b.sort_ordinal,
            &b.graph_node_id,
        ))
    });
    placed
}

/// The facts nobody has placed, in the order they will render behind the others.
///
/// `code_ordinal.is_none()` sits FIRST in the sort tuple so un-numbered
/// candidates land at the END of the tail (`false < true`), matching the card
/// list's own "un-numbered sorts last" rule rather than inventing a second
/// convention for the same question.
fn sorted_unplaced(facts: &[OrderableFact]) -> Vec<&OrderableFact> {
    let mut unplaced: Vec<&OrderableFact> =
        facts.iter().filter(|f| f.sort_ordinal.is_none()).collect();
    unplaced.sort_by(|a, b| {
        (
            a.tier.rank(),
            a.code_ordinal.is_none(),
            a.code_ordinal,
            &a.graph_node_id,
        )
            .cmp(&(
                b.tier.rank(),
                b.code_ordinal.is_none(),
                b.code_ordinal,
                &b.graph_node_id,
            ))
    });
    unplaced
}

/// The number an unplaced fact is TREATED as having, given its place in the tail.
///
/// Never stored — it exists only so a drop beside an unplaced fact has two real
/// numbers to take a midpoint of. That is what keeps a drag to one written row.
///
/// ## Rust Learning: `saturating_*` instead of `+` and `*`
///
/// A stored ordinal near `i32::MAX` would make this overflow: a panic in a debug
/// build, a silent wrap to a NEGATIVE ordinal in a release one — which would
/// reorder the human's list through arithmetic nobody asked for. Saturating
/// arithmetic clamps at the bound instead, and a saturated tail still sorts after
/// the placed block, which is the only property this number has to keep.
fn provisional_ordinal(ceiling: i32, index_in_tail: usize) -> i32 {
    ceiling.saturating_add(
        ORDINAL_STEP.saturating_mul(i32::try_from(index_in_tail + 1).unwrap_or(i32::MAX)),
    )
}

/// The ordinal to store for `dragged`, dropped between `after` and `before`.
///
/// `after` is the fact it now sits BELOW (`None` when dropped at the very top);
/// `before` is the fact it now sits ABOVE (`None` at the very bottom). Naming the
/// two neighbours rather than an index is what makes a stale page safe: an id
/// that has left the scenario is a named refusal, whereas index 4 of a list that
/// has changed is a silently wrong placement.
///
/// # Errors
/// See [`MoveRefusal`] — the dragged fact or a neighbour not being here, or two
/// neighbours with no integer between them.
pub fn plan_move(
    facts: &[OrderableFact],
    dragged: &str,
    after: Option<&str>,
    before: Option<&str>,
) -> Result<i32, MoveRefusal> {
    if !facts.iter().any(|f| f.graph_node_id == dragged) {
        return Err(MoveRefusal::DraggedNotHere);
    }

    let order = effective_order(facts);
    let lower = neighbour_ordinal(&order, dragged, after)?;
    let upper = neighbour_ordinal(&order, dragged, before)?;
    ordinal_between(lower, upper)
}

/// One neighbour's effective ordinal, or `None` when there is no neighbour there.
///
/// The dragged card is excluded from its own lookup: it is being lifted out of
/// the list and put back somewhere else, so its CURRENT position must not
/// influence where it lands. Without that exclusion, dropping a fact back between
/// the same two rows would compute a different number each time and the list
/// would creep.
///
/// A named neighbour that is not in the order is [`MoveRefusal::NeighbourNotHere`]
/// — a stale page describing a list that has changed underneath it, which is a
/// different failure from every other one here and says so.
fn neighbour_ordinal(
    order: &[(String, i32)],
    dragged: &str,
    neighbour: Option<&str>,
) -> Result<Option<i32>, MoveRefusal> {
    let Some(id) = neighbour else {
        return Ok(None);
    };
    order
        .iter()
        .find(|(node, _)| node == id && node != dragged)
        .map(|(_, ordinal)| Some(*ordinal))
        .ok_or(MoveRefusal::NeighbourNotHere)
}

/// The number that sits between two bounds, or the refusal that none does.
///
/// The four cases are the four places a row can be dropped: between two facts,
/// at the bottom, at the top, or as the only fact in its block.
fn ordinal_between(lower: Option<i32>, upper: Option<i32>) -> Result<i32, MoveRefusal> {
    match (lower, upper) {
        // Between two facts: the midpoint, unless they are already adjacent.
        (Some(low), Some(high)) => {
            // Also catches a caller naming the neighbours in the WRONG ORDER,
            // which describes a drop that cannot exist. Refusing beats computing
            // a confident number from a nonsensical pair.
            if high <= low + 1 {
                return Err(MoveRefusal::GapExhausted);
            }
            // `low + (high - low) / 2` rather than `(low + high) / 2`: the latter
            // overflows i32 when both ordinals are large, and the two forms are
            // otherwise identical.
            Ok(low + (high - low) / 2)
        }
        // Dropped at the bottom: a step past the last fact.
        (Some(low), None) => Ok(low.saturating_add(ORDINAL_STEP)),
        // Dropped at the top: a step before the first. Going negative is fine and
        // expected — the ordinal space is an ordering, not a rank, and nothing
        // reads it as a count.
        (None, Some(high)) => Ok(high.saturating_sub(ORDINAL_STEP)),
        // The only fact in its block: any number will do, so take the step itself
        // rather than 0, leaving room to drop something above it later.
        (None, None) => Ok(ORDINAL_STEP),
    }
}

#[cfg(test)]
#[path = "scenario_fact_order_tests.rs"]
mod tests;
