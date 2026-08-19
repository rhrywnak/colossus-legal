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

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::practice::PracticeQuestionRecord;
use super::PipelineRepoError;

/// The side's ids in their new order, or `None` if the drop names no position.
///
/// Pure, so the ordering rule is testable without a database — which is the
/// whole reason it is a free function rather than three lines inside the write.
///
/// `before` is the question the dragged one lands ON TOP OF, i.e. immediately
/// above. `None` means "put it last", which is what dropping past the final row
/// means. Returns `None` when the move cannot name a position: the two ids are
/// the same, the dragged question is not in this side, or the target is not.
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
    let side = deck.iter().find(|q| q.id == dragged)?.side.clone();
    let mut ids: Vec<Uuid> = deck
        .iter()
        .filter(|q| q.side == side)
        .map(|q| q.id)
        .collect();
    if !ids.contains(&dragged) {
        return None;
    }

    // Lift it out FIRST, then find the target's index in what remains. Reading
    // the index before removing would count the dragged row itself, so every
    // drop onto a row BELOW the dragged one would land one place too high.
    //
    // A consequence worth knowing: dropping onto the row directly below asks for
    // the order that already exists, because a row is already immediately above
    // its own successor. That is not a bug and the tests pin it — the
    // scenario-facts drag behaves identically.
    ids.retain(|id| *id != dragged);
    let at = match before {
        None => ids.len(),
        Some(target) if target == dragged => return None,
        Some(target) => ids.iter().position(|id| *id == target)?,
    };
    ids.insert(at, dragged);
    Some(ids)
}

/// Write one side's `sort_order` values to match `ids`, in order.
///
/// ## Why every row is parked first
///
/// `sort_order` carries a UNIQUE constraint per side, so assigning the new
/// numbers directly would collide the moment two rows swap — the first `UPDATE`
/// would try to take a number the second still holds. Every row moves to a
/// negative parking number, then to its final one. The same two-phase shape
/// `swap_sort_order` uses for two rows, generalised to a whole side.
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

#[cfg(test)]
#[path = "practice_reorder_tests.rs"]
mod tests;
