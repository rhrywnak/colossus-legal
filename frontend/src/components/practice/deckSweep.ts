// deckSweep.ts — what a deck page SHOWED, as the ids "Done reviewing" sweeps.
//
// CC_TASK_FOR_YOU_v1 L2, ruling of 2026-09-22: "Done reviewing marks exactly
// the items that page showed — never an item that arrived after the page
// loaded." A watermark could not keep that promise, because `now()` covers
// everything, including the note that arrived while the deck was being read.
// So the press sends what it rendered, and this composes that list from the
// payload the page rendered FROM.
//
// ## Why a pure function, in its own file
//
// It is the one place that decides what a press writes, and the property worth
// holding down is a NEGATIVE — that nothing the page did not show can get in.
// A negative asserted over a component is only as good as the clicks somebody
// remembered to simulate; asserted over a function it is just a test.

import type { PracticeQuestion } from "../../services/practice";

/**
 * One item, in the shape the sweep endpoint takes.
 */
// STRUCTURAL: `answer`, `note` and `change` are the serialized form of the Rust
// `SweptItem` enum — `#[serde(rename_all = "snake_case", tag = "kind", content
// = "id")]` in `backend/src/dto/practice_review.rs` — and the same three values
// a For You row carries in `kind`. Wire vocabulary, not a deployment value:
// renaming a variant on either side changes the format, and this union must
// change with it.
export type SweptItem =
  | { kind: "answer"; id: string }
  | { kind: "note"; id: string }
  | { kind: "change"; id: string };

/**
 * Every item the deck's rows put on screen: each visible question's CURRENT
 * answer, and every note standing on that question or on that answer.
 *
 * ## Domain note: what is deliberately NOT here
 *
 * - **Deck changes.** A reworded question waits for the WITNESS, and Done
 *   reviewing is the reviewers' act — `can_mark_reviewed` is `may_review`. A
 *   reviewer sweeping her side would clear work he was never shown.
 * - **Struck notes.** They are rendered (struck through, with their date) and
 *   they wait for nobody, so marking them read is harmless — and they are
 *   included for exactly that reason: the page showed them, and the rule is
 *   what the page showed, not what happened to be waiting.
 * - **Superseded answers.** Only `answer_id`, the current one, is on the row.
 * - **A question with no answer.** Nothing to name; its notes still count.
 *
 * Duplicates are impossible by construction (one answer id per question, one
 * note id per note) and would be harmless anyway — the write is
 * `ON CONFLICT DO NOTHING`.
 */
export function shownItems(questions: PracticeQuestion[]): SweptItem[] {
  const items: SweptItem[] = [];
  for (const question of questions) {
    if (question.answer_id !== undefined && question.answer_id !== null) {
      items.push({ kind: "answer", id: question.answer_id });
    }
    for (const note of question.notes) {
      items.push({ kind: "note", id: note.id });
    }
  }
  return items;
}
