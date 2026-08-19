//! Writing ONE deck row: the insert both paths share, and the update one uses.
//!
//! Split from [`super::seed`] on 2026-08-19: that module was one line under Rule
//! 17's limit before this task added five columns and a second write path to it.
//! The seam is by SUBJECT rather than by arithmetic — `seed` decides what a run
//! will do and proves it, `seed_update` works out which rows a re-run touches,
//! and this is the only place either of them spells the columns.
//!
//! ## Why the insert lives here rather than in each caller
//!
//! There are now two ways a question reaches the table — a first seed and an
//! `--update` that inserts a key the deck has gained — and a column added to one
//! and not the other is a row that renders differently depending on which
//! command wrote it. One statement, two callers.

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use super::deck_file::DeckQuestion;

/// Who the `created_by` / `updated_by` columns name for rows this tool writes.
///
// CONST: structural — the name of the PROGRAM, not a deployment value. It answers
// "what wrote this row" in an audit, and it changes only when this binary is
// renamed — which is a code change by definition.
pub(super) const WRITER: &str = "seed_practice_deck";

/// Insert one question.
///
/// `sort_order` is passed rather than derived from the question's place in the
/// file, because the update path assigns it from the file's order while the
/// first seed assigns it from the loop index — the same number, decided by
/// different callers, and neither of them should be re-deriving it here.
pub(super) async fn insert_question(
    tx: &mut Transaction<'_, Postgres>,
    scenario_id: Uuid,
    question: &DeckQuestion,
    source_ref: Option<&str>,
    sort_order: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO practice_questions \
         (scenario_id, side, text, tactic, braid_rows, source_kind, source_ref, receipt, \
          watch_for, stronger, stronger_lean, pair_said, pair_admitted, sort_order, created_by, \
          deck_key, kind, follows_key, source_line, draft_by) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20)",
    )
    .bind(scenario_id)
    .bind(question.side.as_column())
    .bind(question.text.trim())
    .bind(question.tactic)
    .bind(question.braid_rows.as_deref().map(str::trim))
    .bind(question.source_kind.as_column())
    .bind(source_ref)
    .bind(question.receipt.as_deref().map(str::trim))
    .bind(question.watch_for.as_deref().map(str::trim))
    .bind(question.stronger.as_deref().map(str::trim))
    .bind(question.stronger_lean.as_deref().map(str::trim))
    .bind(question.pair_said.as_deref().map(str::trim))
    .bind(question.pair_admitted.as_deref().map(str::trim))
    .bind(sort_order)
    .bind(WRITER)
    .bind(question.key.as_deref().map(str::trim))
    .bind(question.resolved_kind().as_column())
    .bind(question.follows.as_deref().map(str::trim))
    .bind(question.source_line.as_deref().map(str::trim))
    .bind(question.draft_by.as_deref().map(str::trim))
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// Overwrite one stored question from the file, in place.
///
/// ## Domain note: what this deliberately does NOT touch
///
/// `id`, `scenario_id`, `created_by` and the flag columns. The id is what
/// `practice_answers.question_id` cites under `ON DELETE RESTRICT`, so keeping
/// it is the whole reason this is an UPDATE and not a delete-and-reinsert —
/// Chuck's sheet stays readable and no answer is orphaned. The flag is Marie's,
/// not the file's, and a re-seed that silently cleared her complaint would
/// destroy the one thing the flag exists to carry.
///
/// `deck_key` is not assigned here either: the caller has already decided which
/// stored row this key belongs to, and re-writing it would let a plan that
/// matched the wrong row rename it as well as overwrite it.
pub(super) async fn update_question(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    question: &DeckQuestion,
    source_ref: Option<&str>,
    sort_order: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE practice_questions SET \
           side = $2, text = $3, tactic = $4, braid_rows = $5, source_kind = $6, \
           source_ref = $7, receipt = $8, watch_for = $9, stronger = $10, \
           stronger_lean = $11, pair_said = $12, pair_admitted = $13, sort_order = $14, \
           kind = $15, follows_key = $16, source_line = $17, draft_by = $18, \
           updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(id)
    .bind(question.side.as_column())
    .bind(question.text.trim())
    .bind(question.tactic)
    .bind(question.braid_rows.as_deref().map(str::trim))
    .bind(question.source_kind.as_column())
    .bind(source_ref)
    .bind(question.receipt.as_deref().map(str::trim))
    .bind(question.watch_for.as_deref().map(str::trim))
    .bind(question.stronger.as_deref().map(str::trim))
    .bind(question.stronger_lean.as_deref().map(str::trim))
    .bind(question.pair_said.as_deref().map(str::trim))
    .bind(question.pair_admitted.as_deref().map(str::trim))
    .bind(sort_order)
    .bind(question.resolved_kind().as_column())
    .bind(question.follows.as_deref().map(str::trim))
    .bind(question.source_line.as_deref().map(str::trim))
    .bind(question.draft_by.as_deref().map(str::trim))
    .execute(&mut **tx)
    .await
    .map(|_| ())
}

/// Give one stored row the key the file says it has.
///
/// Written ONCE, on the pass that matches pre-key rows by their exact text. A
/// row that already carries a key is never re-keyed by this tool: the key is the
/// identity, and re-keying is the one edit that could silently point Chuck's
/// edits at a different question.
pub(super) async fn set_deck_key(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE practice_questions SET deck_key = $2, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .bind(key.trim())
        .execute(&mut **tx)
        .await
        .map(|_| ())
}
