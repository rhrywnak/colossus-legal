//! How `--update` WRITES: park the order, then assign it.
//!
//! Split from `seed_update` when the .403 order fix arrived and pushed that
//! module past Rule 17. The split is not arbitrary — it is the seam the module
//! already had. `seed_update` decides WHAT the run will do (which file key maps
//! to which stored row, what is an update and what is an insert), and every part
//! of that is a pure function with a unit test. This module is the half that
//! opens a transaction and touches the database, and it is the half only a real
//! Postgres can check.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::deck_file::DeckFile;
use super::seed_rows::{insert_question, set_deck_key, update_question};
use super::seed_update::UpdateError;

/// Write the plan: the one-time keys, then every update and insert, in one
/// transaction.
///
/// One transaction for the reason the first seed gives: a half-updated deck is
/// worse than an un-updated one, because the page renders it confidently.
pub(super) async fn apply_update(
    pool: &PgPool,
    scenario_id: Uuid,
    deck: &DeckFile,
    refs: &[Option<String>],
    assignments: &[(Uuid, &str, String)],
    key_of: &[(Uuid, String)],
) -> Result<(), UpdateError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|source| UpdateError::Database { source })?;

    // PHASE 1 — park every existing row's `sort_order` out of the way.
    //
    // See `park_sort_orders` for why. Without it a deck whose file order is a
    // PERMUTATION of the stored order fails mid-transaction, and only ever on a
    // real database: the dry run never writes, so it cannot see it.
    park_sort_orders(&mut tx, scenario_id)
        .await
        .map_err(|source| UpdateError::Database { source })?;

    for (id, key, _) in assignments {
        set_deck_key(&mut tx, *id, key)
            .await
            .map_err(|source| UpdateError::Database { source })?;
    }

    write_questions(&mut tx, scenario_id, deck, refs, key_of).await?;

    tx.commit()
        .await
        .map_err(|source| UpdateError::Database { source })
}

/// PHASE 2 — write every question its FINAL number, updating or inserting.
///
/// The file's position IS the order: question `i` gets `sort_order = i + 1`. A
/// key the stored deck already holds is updated in place; a key it does not is
/// inserted. Nothing is ever deleted — a stored row the file dropped refuses the
/// whole run long before this point.
///
/// Safe to assign final numbers one at a time only because [`park_sort_orders`]
/// has emptied the range first. Called from inside that same transaction.
async fn write_questions(
    tx: &mut Transaction<'_, Postgres>,
    scenario_id: Uuid,
    deck: &DeckFile,
    refs: &[Option<String>],
    key_of: &[(Uuid, String)],
) -> Result<(), UpdateError> {
    for (i, question) in deck.questions.iter().enumerate() {
        let key = question.key.as_deref().map(str::trim).unwrap_or_default();
        let sort_order = i32::try_from(i + 1).unwrap_or(i32::MAX);
        let existing = key_of
            .iter()
            .find(|(_, held)| held == key)
            .map(|(id, _)| *id);
        let written = match existing {
            Some(id) => update_question(tx, id, question, refs[i].as_deref(), sort_order).await,
            None => {
                insert_question(tx, scenario_id, question, refs[i].as_deref(), sort_order).await
            }
        };
        written.map_err(|source| UpdateError::Database { source })?;
    }
    Ok(())
}

/// Move every one of this scenario's rows to a distinct NEGATIVE `sort_order`.
///
/// ## The defect this exists for (found on .403, S-5)
///
/// `practice_questions` carries `UNIQUE (scenario_id, sort_order)`, and the write
/// loop below assigns each question its FINAL number one row at a time. That is
/// fine while the file order matches the stored order — every row writes the
/// number it already had. It breaks the moment the file is a re-ordering:
///
/// ```text
///   stored:  g1=1  g2=2  g3=3  g4=4  g5=5
///   file:    g3    g4    g2    g1    g5      (the .403 ruling)
///   write 1: g3 := 1   ← g1 still holds 1 → practice_questions_order_unique
/// ```
///
/// The row being written and the row in its way are both mid-transaction, so
/// there is no order of writes that avoids this in general — a permutation with
/// a cycle always has a first write that collides. The fix is to empty the range
/// first.
///
/// ## Why parking, and not a DEFERRABLE constraint
///
/// Both would work. Parking wins on three counts:
///
/// 1. **It is already the house pattern for this exact column.**
///    `swap_sort_order` parks one row to a negative sentinel to exchange two
///    numbers, and `practice_reorder::write_order` parks a whole side before
///    re-sequencing it. A third mechanism for the same problem on the same
///    column is a thing the next reader has to learn.
/// 2. **No schema change.** Making the constraint deferrable is a migration that
///    has to reach every environment BEFORE the binary that relies on it — and
///    `--update` is run by hand against DEV and PROD, so the ordering of those
///    two deploys becomes something a person has to get right.
/// 3. **It keeps the invariant immediate for everyone else.** `SET CONSTRAINTS
///    … DEFERRED` moves the check to COMMIT for the whole transaction, so a
///    genuine duplicate written by some other code path in the same transaction
///    would surface later and further from its cause. The constraint is worth
///    more checking immediately for every writer than it costs this one.
///
/// ## Why `row_number()` and not `sort_order = -sort_order - 1`
///
/// The arithmetic form is shorter and is correct as long as every current value
/// is distinct and non-negative — which the constraint guarantees today. But it
/// is correct BECAUSE of a fact held somewhere else, and a row that somehow held
/// a negative number would map back into the positive range and collide with a
/// real row. `row_number()` numbers the rows 1..n from nothing, so the parked
/// values are distinct by construction whatever the input was.
///
/// Nobody ever sees a parked value: the park, the writes and the commit are one
/// transaction. A reader who finds a negative `sort_order` in the table is
/// looking at a bug, not at this.
async fn park_sort_orders(
    tx: &mut Transaction<'_, Postgres>,
    scenario_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE practice_questions q \
            SET sort_order = -p.n \
           FROM (SELECT id, row_number() OVER (ORDER BY sort_order, id) AS n \
                   FROM practice_questions WHERE scenario_id = $1) p \
          WHERE q.id = p.id",
    )
    .bind(scenario_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
