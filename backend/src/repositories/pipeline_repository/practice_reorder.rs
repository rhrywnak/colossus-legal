//! Placing one question at an arbitrary position in its side.
//!
//! ## Why this is not `swap_sort_order` with a loop
//!
//! The ▲▼ arrows move a question one step, and one step is a SWAP of two
//! `sort_order` values — cheap, and the only thing the keyboard fallback needs.
//! A drag is different in kind: it lifts a row out and drops it anywhere, so
//! "move Q5 above Q1" is four swaps, four `UPDATE`s and — the part that matters —
//! four change-log rows for one gesture Chuck made once.
//!
//! So a drop RE-SEQUENCES the side: read the side in order, take the dragged
//! question out, put it back at the requested place, and write the new positions.
//! One gesture, one transaction, one change row.
//!
//! ## Domain note: within its own side, always
//!
//! George's questions and Chuck's are two ordered lists that happen to share a
//! table. Dragging a cross question into the middle of the directs would produce
//! a deck that deals a Chuck question in a George sitting, which is not a
//! re-order — it is a different question. The side is filtered before anything
//! is computed, and a target on the other side yields `None`.
//!
//! ## The whole deck comes back, not one side (the 2026-09-10 save bug)
//!
//! Until tonight [`resequenced`] returned only the dragged row's SIDE, and
//! [`write_order`] numbered exactly those ids `0..N-1`. That is a collision, and
//! it fired on every two-sided deck:
//!
//! * `practice_questions_order_unique UNIQUE (scenario_id, sort_order)` —
//!   migration `20260817213319_practice_session_v0.sql:114` — spans the whole
//!   SCENARIO, not one side. (`write_order`'s own doc used to say "per side". It
//!   was wrong, and the wrongness of that one sentence is most of this bug.)
//! * Measured on DEV, S-1: 32 George rows and 10 of Chuck's, interleaved through
//!   `sort_order` 1..42 — George at 1-5, Chuck at 6-9, and so on.
//! * So a George drag renumbered 32 rows to `0..31` while Chuck's still held 6,
//!   7, 8, 9, 11 … 16. The `UPDATE` collided, the transaction rolled back, and
//!   the route answered 500. Chuck's drag had never once saved.
//!
//! The fix is to hand `write_order` a permutation of the WHOLE deck, so
//! renumbering `0..N-1` cannot collide with anything: there is nothing left over.
//!
//! ## And the other side does not move (the rule that keeps this small)
//!
//! A whole-deck order could be built two ways, and only one of them is safe.
//!
//! The tempting one is "this side's new order, then everything else" — which
//! silently rewrites the position of every row on the other side, for a gesture
//! that touched neither. What this module does instead is **preserve the
//! slots**: the deck's shape is a sequence of positions, each already belonging
//! to one side or the other, and a drag permutes the dragged side's rows among
//! the dragged side's positions ONLY. Every other row comes back at exactly the
//! index it went in at.
//!
//! The consequence worth stating: the within-side answer is byte-for-byte what
//! this module returned before tonight. Nothing about what a drag MEANS changed.
//! What changed is how much of the deck comes back with it.

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::practice::PracticeQuestionRecord;
use super::PipelineRepoError;

/// The WHOLE deck's ids in their new order, or `None` if the drop names no
/// position.
///
/// Pure, so the ordering rule is testable without a database — which is the
/// whole reason it is a free function rather than three lines inside the write.
///
/// `before` is the question the dragged one lands ON TOP OF, i.e. immediately
/// above. `None` means "put it last **on its side**", which is what dropping
/// past the final row means. Returns `None` when the move cannot name a
/// position: the two ids are the same, the dragged question is not in this deck,
/// or the target is on the other side.
///
/// The returned vector is a PERMUTATION of `deck` — every id exactly once — and
/// that is the property [`write_order`] depends on. See the module doc.
///
/// ## Rust Learning: `Option` as the "nothing to do" answer
///
/// Not an error. A drop onto itself is a gesture a person makes constantly by
/// accident, and answering it with a 400 would put a red notice on screen for
/// having changed one's mind mid-drag. `None` here means the caller returns
/// success having written nothing — the same shape the ▲▼ arrows use when a
/// question is already at the end of its side.
pub fn resequenced(
    deck: &[PracticeQuestionRecord],
    dragged: Uuid,
    before: Option<Uuid>,
) -> Option<Vec<Uuid>> {
    let side = deck.iter().find(|q| q.id == dragged)?.side.as_str();
    let slots: Vec<Slot> = deck.iter().map(Slot::of).collect();
    let reordered = side_order(&side_ids(&slots, side), dragged, before)?;
    Some(with_side_reordered(&slots, side, &reordered))
}

/// The whole deck's ids with a NEWLY INSERTED question moved into place.
///
/// The add path's half of the same rule (task DECK_DRAG_AND_ADD part 3). The new
/// row has just been written at `next_sort_order` — the end of the deck — and
/// `at` says where on its own side it should sit.
///
/// `deck` is the deck as it was read BEFORE the insert, so `new_id` is not in it;
/// the new row is appended here as the deck's last slot, which is exactly where
/// the `INSERT` put it. [`NewPosition::End`] means "leave it there", i.e. the
/// behaviour the add route has always had.
///
/// # Panics / refusals
/// Returns `None` when [`NewPosition::After`] names a row this deck does not
/// hold, or one on the other side. The caller answers those separately — a 404
/// and a 400 — because they are different mistakes; see
/// `api::practice_editor_add`. The other two positions cannot fail.
pub fn placed_after(
    deck: &[PracticeQuestionRecord],
    new_id: Uuid,
    new_side: &str,
    at: NewPosition,
) -> Option<Vec<Uuid>> {
    // The new row's slot is the LAST one, because `next_sort_order` is MAX + 1.
    let mut slots: Vec<Slot> = deck.iter().map(Slot::of).collect();
    slots.push(Slot {
        id: new_id,
        side: new_side,
    });

    let ids = side_ids(&slots, new_side);
    let before = match at {
        // The top of the side is "above whatever is currently first" — and on a
        // side with no rows yet, that is the end, which is the same place.
        NewPosition::Start => ids.iter().copied().find(|id| *id != new_id),
        NewPosition::After(after) => before_from_after(&ids, new_id, after)?,
        NewPosition::End => None,
    };
    let reordered = side_order(&ids, new_id, before)?;
    Some(with_side_reordered(&slots, new_side, &reordered))
}

/// Where a newly added question goes, among its own side's rows.
///
/// Three places, and they are three because two of them cannot share one absent
/// value: the END is what an add with no position asks for (the bottom box, and
/// every caller that predates the gap control), and the START is what the gap
/// above the first row asks for. See `dto::practice_review::AddQuestionRequest`
/// for the wire shape this is built from and why it needed a flag of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewPosition {
    /// The top of the side — the gap above its first row.
    Start,
    /// Immediately below this row, which must be on the same side.
    After(Uuid),
    /// The end of the side. The historical behaviour, and still the default.
    End,
}

/// One position in the deck: which row is in it, and whose side that row is on.
///
/// ## Rust Learning: a borrowed `&'a str`, not an owned `String`
///
/// Every one of these is built from a record the caller already owns and drops
/// before this function returns, so borrowing the side is free and cloning it
/// would be one allocation per row per drag. The lifetime is elided into the
/// struct's own `'a`, which ties a `Slot` to the deck it came from — the borrow
/// checker then makes it impossible to keep one after the deck is gone.
struct Slot<'a> {
    id: Uuid,
    side: &'a str,
}

impl<'a> Slot<'a> {
    fn of(question: &'a PracticeQuestionRecord) -> Self {
        Slot {
            id: question.id,
            side: question.side.as_str(),
        }
    }
}

/// The ids sitting in one side's slots, in deck order.
fn side_ids(slots: &[Slot<'_>], side: &str) -> Vec<Uuid> {
    slots
        .iter()
        .filter(|slot| slot.side == side)
        .map(|slot| slot.id)
        .collect()
}

/// One side's ids, with `moving` lifted out and put back above `before`.
///
/// The whole of what a drag MEANS, and unchanged since it was written: the
/// module's original `resequenced` body, extracted so the drag and the add place
/// a row by one rule rather than two that can drift.
fn side_order(ids: &[Uuid], moving: Uuid, before: Option<Uuid>) -> Option<Vec<Uuid>> {
    let mut ids = ids.to_vec();
    if !ids.contains(&moving) {
        return None;
    }

    // Lift it out FIRST, then find the target's index in what remains. Reading
    // the index before removing would count the moving row itself, so every
    // drop onto a row BELOW it would land one place too high.
    //
    // A consequence worth knowing: dropping onto the row directly below asks for
    // the order that already exists, because a row is already immediately above
    // its own successor. That is not a bug and the tests pin it — the
    // scenario-facts drag behaves identically.
    ids.retain(|id| *id != moving);
    let at = match before {
        None => ids.len(),
        Some(target) if target == moving => return None,
        Some(target) => ids.iter().position(|id| *id == target)?,
    };
    ids.insert(at, moving);
    Some(ids)
}

/// "Immediately BELOW `after`" expressed as this module's "immediately above".
///
/// The drag names the row it lands on top of; the add names the row it lands
/// under. They are the same statement about one gap, from the two sides of it, so
/// one of them is converted rather than the placement rule being written twice.
///
/// `after` = the side's last row → `before = None`, the end of the side.
/// Otherwise the answer is whichever row currently follows `after`.
///
/// ## Rust Learning: `Option<Option<T>>`, and why it is not a smell here
///
/// The two layers mean genuinely different things and collapsing them would lose
/// one. The OUTER `None` is a refusal — `after` names no row on this side, which
/// the caller answers with a 404 or a 400. The INNER `None` is a real, successful
/// answer: "there is no row to go above, so this is the end of the side". A
/// single `Option` would have to report the end-of-side case as a failure, or the
/// refusal as an end-of-side, and both would put a question somewhere nobody
/// asked for.
fn before_from_after(ids: &[Uuid], moving: Uuid, after: Uuid) -> Option<Option<Uuid>> {
    // The moving row is already in `ids` (it was appended as the deck's last
    // slot), and it must not be its own neighbour — so it is excluded before the
    // successor is read.
    let settled: Vec<Uuid> = ids.iter().copied().filter(|id| *id != moving).collect();
    let at = settled.iter().position(|id| *id == after)?;
    Some(settled.get(at + 1).copied())
}

/// The deck's ids, with one side's rows replaced by `reordered`, slot for slot.
///
/// The other side never moves: a slot that held a Chuck row still holds the same
/// Chuck row, at the same index. See the module doc for why that is the whole
/// point rather than an implementation detail.
///
/// ## Rust Learning: a draining iterator as a cursor
///
/// `reordered.iter()` is walked lazily across the slot loop, so each of the
/// dragged side's slots takes the NEXT id from the new order. It is the shortest
/// honest way to say "fill these positions, in order, from that list". The
/// `unwrap_or(slot.id)` can only fire if the two lists disagree in length, which
/// they cannot — `reordered` is a permutation of exactly the ids `side_ids`
/// returned for this side — and it degrades to leaving the row where it was
/// rather than dropping it from the deck (Standing Rule 1: the impossible case
/// does not silently lose a row).
fn with_side_reordered(slots: &[Slot<'_>], side: &str, reordered: &[Uuid]) -> Vec<Uuid> {
    let mut next = reordered.iter();
    slots
        .iter()
        .map(|slot| {
            if slot.side == side {
                next.next().copied().unwrap_or(slot.id)
            } else {
                slot.id
            }
        })
        .collect()
}

/// Write the deck's `sort_order` values to match `ids`, in order.
///
/// ## Why every row is parked first
///
/// `sort_order` carries a UNIQUE constraint across the whole SCENARIO
/// (`practice_questions_order_unique`), so assigning the new numbers directly
/// would collide the moment two rows swap — the first `UPDATE` would try to take
/// a number the second still holds. Every row moves to a negative parking number,
/// then to its final one. The same two-phase shape `swap_sort_order` uses for two
/// rows, generalised to the deck.
///
/// ## What `ids` must be, and what happens when it is not
///
/// The WHOLE scenario's questions, every one exactly once. Anything less leaves
/// rows holding numbers this loop is about to hand out, and the second phase
/// collides — which is precisely the 500 that made Chuck's drag never save (see
/// the module doc). [`resequenced`] and [`placed_after`] both return whole-deck
/// permutations for that reason; nothing else should call this.
///
/// Inside the caller's transaction, so a failure halfway leaves the deck in the
/// order it had rather than half re-sequenced.
pub async fn write_order(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<(), PipelineRepoError> {
    for (index, id) in ids.iter().enumerate() {
        sqlx::query("UPDATE practice_questions SET sort_order = $2 WHERE id = $1")
            .bind(id)
            .bind(-(index as i32) - 1)
            .execute(&mut **tx)
            .await?;
    }
    for (index, id) in ids.iter().enumerate() {
        sqlx::query(
            "UPDATE practice_questions SET sort_order = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(index as i32)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Where one id sits among its OWN side's rows in a whole-deck order.
///
/// The change log records the position a person can see — "third of Chuck's
/// questions" — and since tonight the order handed around is the whole deck, in
/// which that same row might be twentieth. Without this the log would start
/// reporting a number that matches nothing on screen.
///
/// `side` is handed in rather than looked up, because the ADD path asks about a
/// row that is not in `deck`: the deck was read before the `INSERT`, so the new
/// question's side is known only to its caller. `id` is admitted to its own side
/// by identity for the same reason.
///
/// `None` when the id is not in the order at all, which the caller treats as the
/// broken invariant it is rather than defaulting to zero.
pub fn position_within_side(
    deck: &[PracticeQuestionRecord],
    order: &[Uuid],
    id: Uuid,
    side: &str,
) -> Option<usize> {
    order
        .iter()
        .filter(|other| {
            **other == id
                || deck
                    .iter()
                    .find(|q| q.id == **other)
                    .is_some_and(|q| q.side == side)
        })
        .position(|other| *other == id)
}

#[cfg(test)]
#[path = "practice_reorder_tests.rs"]
mod tests;

// The whole-deck and add-after placement rules have their own sibling — this
// module reached the size limit with them inline, and they are a distinct
// subject from the within-side rule the tests above pin.
#[cfg(test)]
#[path = "practice_place_tests.rs"]
mod place_tests;

// The live-database proof that the save bug is gone — `#[ignore]`d, because the
// only thing that can demonstrate it is a real Postgres enforcing a real unique
// constraint. See that module's doc for why no pure test could have caught this.
#[cfg(test)]
#[path = "practice_reorder_live_tests.rs"]
mod live_tests;
