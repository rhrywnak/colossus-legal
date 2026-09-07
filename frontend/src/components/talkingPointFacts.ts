// =============================================================================
// talkingPointFacts.ts — which facts sit under which talking point (v2.1, F)
// =============================================================================
//
// Ruling R37 puts the evidence under the argument it backs: a talking point is a
// sentence Marie says, and the facts that make it true belong beneath it rather
// than in a separate list the reader has to join by hand.
//
// ## Why this is a pure module and not thirty lines inside the section
//
// CLAUDE.md Rule 30: there is no component-test tier in this repo, so a rule
// decided inside a component is a rule no test can reach. Everything here is
// something that ONLY shows up on screen — which facts appear under point 2,
// what order they come in, and what each row says when the card has no title —
// which is exactly the class of decision that has to be assertable.
//
// ## Domain note: `backs_position` is the ONLY link, and it is one-way
//
// There is no join table. A fact points at a talking point by POSITION (an
// `Option<i32>` on the card block, `backend/src/dto/fact_card.rs`), and a
// position is the point's ADDRESS as well as its printed number — the same
// number the write route takes (`PUT …/talking-points/:position`). Two
// consequences a reader should have in mind:
//
//   * Deleting or reordering points RE-AIMS every fact that pointed at them.
//     Nothing dangles and nothing errors; the facts simply move. That is a
//     property of addressing by position, and it is why `factsForPoint` is a
//     filter over live data rather than anything cached.
//   * A fact pointing at a position no point occupies appears under NO point.
//     It is not lost — it is in the Scenario facts list like every other fact,
//     with a picker to re-aim it. See `FactCardBody.BacksPicker`.

import { cardTitle } from "./factCard";
import { includedRows, orderedRows, type WorkingRow } from "./factsTable";
import type { ScenarioCard } from "../services/scenarioCards";
import type { FactCardWording } from "../services/evidenceLinks";

/** One collapsed row under a talking point: `C-40 · {title} · {cite}`. */
export type PointFactLine = {
  /** "C-40". `null` on an included card nothing has numbered — see `pointFactLine`. */
  code: string | null;
  /** The card's own title, or the quote's first line when it has none. */
  title: string;
  /** The pinpoint, pre-composed by the backend — "CFS responses at 26". */
  cite: string;
};

/**
 * The included facts that back one talking point, in the facts list's own order.
 *
 * ## Why the order is `orderedRows` and not the payload's
 *
 * The sequence IS the argument — that is the reasoning behind the drag-to-order
 * control on the facts list, and behind the Reset order beside it. A second
 * ordering here would mean the same three facts read in one sequence under the
 * point and another in the list, and a human who dragged them into an order
 * would find it honoured on one surface only.
 *
 * `includedRows` is the same filter the facts list applies, for the same reason:
 * a candidate nobody has ruled on is the queue's business, and a set-aside fact
 * is deliberately out. A point must never show evidence the scenario does not
 * hold.
 *
 * @param cards    the page's whole card pool
 * @param position the talking point's 1-based position, which is its address
 * @returns the rows to render, possibly empty — a point nobody has attached
 *          evidence to renders no rows at all rather than an empty-state line
 */
export function factsForPoint(
  cards: ScenarioCard[],
  position: number,
): WorkingRow[] {
  return orderedRows(includedRows(cards)).filter(
    (row) => row.card?.card?.backs_position === position,
  );
}

/**
 * What one collapsed row says.
 *
 * ## The title fallback, and why it is the QUOTE and not the em dash
 *
 * `cardTitle` returns the stored em dash when no card block exists or its title
 * is null, which is right on the card itself: an empty title there is work
 * somebody owes and the gap should show. It is wrong HERE. This row is the only
 * thing naming the fact, and a list of three rows reading "C-40 · — · CFS
 * responses at 26" tells a reader nothing about what is under them. So an
 * untitled fact is named by the first line of its own quote — the record's
 * words, never the machine's, which is also what makes the fallback safe to show
 * without a draft mark.
 *
 * "First line" is the first LINE, not the first sentence: quotes arrive verbatim
 * and a transcript answer may carry newlines. Splitting on a sentence boundary
 * would need a rule about abbreviations and pinpoint citations ("p. 10"), and
 * getting that wrong truncates evidence mid-thought.
 *
 * @param row     one included fact
 * @param wording the stored card words — the em dash is only consulted to detect
 *                that there was no title, never printed by this function
 */
export function pointFactLine(
  row: WorkingRow,
  wording: FactCardWording,
): PointFactLine {
  const stored = row.card ? cardTitle(row.card, wording) : null;
  const titled = stored !== null && stored.text !== wording.empty_value;

  return {
    code: row.code,
    title: titled ? stored.text : firstLine(row.text),
    cite: row.pinpointLabel,
  };
}

/**
 * The first line of a quote, trimmed.
 *
 * An all-whitespace quote yields an empty string rather than a line of spaces —
 * which renders as a visibly empty title, and is the honest answer: the row is
 * still there, still carries its code and its cite, and the gap is visible
 * rather than papered over with a stored placeholder that would read as content.
 */
function firstLine(text: string): string {
  return text.split("\n")[0].trim();
}
