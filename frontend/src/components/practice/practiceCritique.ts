// practiceCritique.ts — what the critique block shows, and when it shows nothing.
//
// PURE, and its own module for the reason CLAUDE.md rule 30 gives: this project
// tests pure helpers, not components, and a decision exported from a `.tsx` is a
// decision that will not be tested. Every branch below is a state Roman has
// either seen or ruled on.

import type { AnswerResult } from "../../services/practice";

/** What the block under the answer box is showing right now. */
export type CritiqueView =
  /** Nothing has been asked yet. */
  | { kind: "idle" }
  /** The read is in flight. The block is PRESENT and empty — see below. */
  | { kind: "working"; longWait: boolean }
  /** The read produced its three parts. */
  | { kind: "parts"; result: AnswerResult }
  /** An older answer, or one whose parts never arrived: one composed sentence. */
  | { kind: "sentence"; text: string; ok: boolean | null }
  /** The read failed, abstained, or she stopped waiting. Nothing is drawn. */
  | { kind: "none" };

/**
 * What to draw for one answer's read.
 *
 * ## ⚑ PARTS IF PRESENT, ELSE TEXT — never both
 *
 * `read_text` is a LOSSY PROJECTION of `read_parts`, not a second copy: the
 * server derives it with `compose_read_text`, which drops `why` and keeps only
 * the first pointer. Rendering both would print the call and the first pointer
 * twice, and the two could not be reconciled because one is a summary of the
 * other.
 *
 * ## ⚑ TEXT WITHOUT PARTS IS THE COMMON CASE
 *
 * Measured on DEV, 2026-08-23: of 14 stored answers, 12 carry `read_text` and
 * only 2 carry parts. Ten were written before T1 shipped in .404. So the
 * `sentence` arm is the majority path, and rendering an empty three-part
 * scaffold with blank headings for those rows would make ten of her answers
 * look like a broken page. A plain sentence reads as an older answer, which is
 * what it is.
 *
 * ## Domain note: NOTHING, not an empty box, when there was no read
 *
 * She pressed Stop waiting, or the model was down, or the read abstained. Her
 * answer is saved either way — that is what the two-write shape guarantees — and
 * the honest rendering is no block at all. An empty bordered box says "something
 * should be here", which invites her to wait for something that is not coming.
 */
export function critiqueFor(result: AnswerResult | null): CritiqueView {
  if (result === null) return { kind: "idle" };
  if (result.read_parts !== null) return { kind: "parts", result };
  if (result.read_text !== null) {
    return { kind: "sentence", text: result.read_text, ok: result.read_ok };
  }
  return { kind: "none" };
}

/**
 * The keys a critique cites, paired with the words behind them.
 *
 * Only keys the read actually used, in the order it used them — the payload
 * carries every citable source, and listing all of them would bury the two she
 * needs under eleven she does not.
 *
 * ## ⚑ A cited key with NO source is kept, not dropped
 *
 * It should be impossible: the read refuses a key it was not sent. If one
 * appears anyway, showing the key with nothing behind it is how anybody finds
 * out. Dropping it would hide exactly the failure this list exists to expose.
 */
export function citedSources(
  result: AnswerResult,
): Array<{ key: string; text: string | null }> {
  const parts = result.read_parts;
  if (parts === null) return [];
  return parts.keys.map((key) => ({
    key,
    text: result.read_sources.find((source) => source.key === key)?.text ?? null,
  }));
}
