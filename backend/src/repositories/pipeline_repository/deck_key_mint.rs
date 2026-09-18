//! Minting a `deck_key` for a question nobody wrote in the deck file.
//!
//! ## The two ways this can go wrong, and why the namespace is the fix
//!
//! A `deck_key` is the handle `seed_practice_deck --update` matches on — by key,
//! never by text (`practice::seed_update`), because text is what a re-wording
//! changes. That gives a hand-added question two bad options and one good one:
//!
//! - **No key at all** (what shipped until 2026-09-17). `--update` refuses the
//!   WHOLE run when a stored row has no key and no file question shares its exact
//!   text (`UpdateError::StoredRowUnmatched`) — so an un-keyed hand-added
//!   question freezes the seeding tool for that scenario. Measured on DEV: 35
//!   such rows, all in one scenario. And a redirect cannot anchor to it, because
//!   `follows_key` resolves against a cross question's `deck_key`.
//! - **A key from the file's own namespace** (`g6`, `c9`). Worse, and quietly so:
//!   the architect's next file question takes `g6` too, `--update` matches by key,
//!   and one question's text is rewritten with another's. That is Chuck's edit
//!   landing on a question he did not mean —
//!   `practice_editor::insert_question`'s domain note named this hazard, and it
//!   was right to.
//! - **A namespace the file can never use** — `x1`, `x2`, … Ruled 2026-09-17.
//!   Collision-free by construction rather than by convention: the file's keys
//!   are `g`/`c`/`r` by side and kind, `x` belongs to no side, and a row whose key
//!   the file does not mention is already "LEFT ALONE and listed in the report"
//!   by `--update`. The question gets an anchor a redirect can follow, and the
//!   seeding tool gets a key it can skip.

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::PipelineRepoError;

/// The prefix no deck file may use.
///
/// STRUCTURAL: this is a namespace boundary, not a preference. The deck file's
/// own keys are `g`/`c`/`r` (george-cross, chuck-direct, chuck-redirect); `x` is
/// reserved for keys this build mints, and changing it would make every already
/// minted key collidable. A rename is a migration, not a settings edit.
pub const HAND_ADDED_PREFIX: &str = "x";

/// The next free `x` key, given every key a scenario already holds.
///
/// Pure, so the numbering rule is testable without a database. Keys that are not
/// `x`-prefixed are IGNORED rather than refused: a scenario is expected to hold
/// `g1`, `c2`, `r3` and they are none of this function's business.
///
/// ## Rust Learning: `filter_map` as "parse, don't validate"
///
/// Each key is turned into `Option<u32>` in one pass — `strip_prefix` says
/// whether it is ours, `parse` says whether the remainder is a number — and
/// `filter_map` keeps only the ones that answered yes to both. A key like
/// `xenon` survives `strip_prefix` and fails `parse`, so it never becomes a
/// number, and `max()` over an empty iterator is `None`, which is the
/// "nothing minted yet" case without a special branch.
pub fn next_hand_key<S: AsRef<str>>(existing: &[S]) -> String {
    let highest = existing
        .iter()
        .filter_map(|key| key.as_ref().trim().strip_prefix(HAND_ADDED_PREFIX))
        // best-effort: a key whose remainder is not a number is not one of ours
        // — `xenon` is somebody else's handle, not a malformed mint — so it is
        // skipped rather than refused. The parse error carries nothing a reader
        // would want; which keys were ignored is visible in the result.
        .filter_map(|rest| rest.parse::<u32>().ok())
        .max();
    // `unwrap_or(0) + 1` rather than `map(+1).unwrap_or(1)`: the first minted key
    // on a deck is `x1`, and saying so once is clearer than saying it twice.
    format!("{HAND_ADDED_PREFIX}{}", highest.unwrap_or(0) + 1)
}

/// Every `deck_key` this scenario holds, for [`next_hand_key`].
///
/// Read inside the caller's transaction, immediately before the insert that uses
/// it.
///
/// ## Domain note: the UNIQUE constraint is the real guard
///
/// Two people adding a question to the same deck in the same instant would both
/// read the same highest key and both mint `x7`. Nothing here prevents that —
/// `practice_questions_deck_key_unique (scenario_id, deck_key)` does, and the
/// second INSERT fails loudly and is reported to the person who pressed Add
/// (Standing Rule 1). That is the correct outcome: a refused add they can repeat
/// beats two questions sharing a handle, which is the silent state `--update`
/// would later resolve by guessing.
///
/// # Errors
/// The database's own error when the SELECT cannot run.
pub async fn keys_in_scenario(
    tx: &mut Transaction<'_, Postgres>,
    scenario_id: Uuid,
) -> Result<Vec<String>, PipelineRepoError> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT deck_key FROM practice_questions \
         WHERE scenario_id = $1 AND deck_key IS NOT NULL",
    )
    .bind(scenario_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows.into_iter().map(|row| row.0).collect())
}

#[cfg(test)]
#[path = "deck_key_mint_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "deck_key_mint_live_tests.rs"]
mod live_tests;
