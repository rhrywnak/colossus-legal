// =============================================================================
// factCard.ts — the witness card's pure display helpers (FACT_CARD_v2 §2, §3)
// =============================================================================
//
// Everything the card decides that does not need React. CLAUDE.md §30: there is
// no component-test infrastructure in this repo, so anything worth asserting has
// to be reachable without rendering — and on this card the things worth asserting
// are exactly the ones that only show up on screen. Which rows carry an em dash.
// Which carry a draft mark. Which cards are open and which are collapsed.
//
// ## The frontend composes nothing
//
// Every value here either PICKS a served string or SUBSTITUTES served numbers
// into a served template. The five sentences themselves — the title, the Backs
// line, the Supports lines — arrive finished from the backend, for the reason the
// language law gives: a sentence assembled in two places reads two ways.
//
// ## ⚑ v2.1 (ruling R37): the machine's PROSE left this card
//
// Until v2.1 `cardRows` returned four rows — Backs, Supports, Watch out, Answer
// — and the block above said "nothing here is hidden for being absent: a card
// with no Answer is work somebody still owes". That reasoning was sound for the
// surface it was written for, a deck a witness reads. This page is not that
// surface: it is Roman's and Chuck's working file, and three of those four rows
// were the machine's suggested WORDS about evidence rather than the evidence.
//
// So the card shows the record — Proof and Supports — and the one row a human
// actually sets from here is a POINTER, not prose: which talking point this fact
// backs, offered as a picker in `FactCardBody` rather than as an editable
// sentence. See `CardFieldName`, which still names all five fields, because the
// API and the tables are untouched: `answer`, `watch_out` and `backs_position`
// are all still stored, still served and still writable. They are not RENDERED
// here any more, which is a display decision and reversible in this one file.

import type {
  FactCardBlock,
  FactCardDrafts,
  ScenarioCard,
} from "../services/scenarioCards";
import type { FactCardWording } from "../services/evidenceLinks";

/** Which field of a card an edit addresses. Mirrors `domain::fact_card`. */
export type CardFieldName =
  | "title"
  | "backs_position"
  | "supports"
  | "watch_out"
  | "answer";

/** One rendered row of the card body. */
export type CardRow = {
  /** Which field this row shows, so the editor knows what to PUT. */
  field: CardFieldName;
  /** The row's served label — "Proof", "Backs", … */
  label: string;
  /**
   * The lines to print. Empty means the row shows the stored em dash; it is
   * NEVER a reason to drop the row.
   *
   * A list rather than a string because Supports can carry two lines, and one
   * shape for all four rows is what keeps the renderer from branching.
   */
  lines: string[];
  /** Whether this field is still the machine's draft. */
  draft: boolean;
};

/**
 * The authored rows the card RENDERS. One, since v2.1: Supports.
 *
 * Proof is not here: it is the record's own words, no human writes it, and it has
 * no `authored_by` to draft-mark. It renders above this from the card's quote —
 * as a `Q:`/`A:` pair when the item is a discovery answer.
 *
 * ## Why this still returns a LIST of one
 *
 * The shape is the contract with `FactCardBody`, which walks it. Returning a bare
 * row would push the "is there anything to draw here" branch into the renderer,
 * which is where it was before this module existed. A list of one costs nothing
 * and means the day a row comes back — or a new one arrives — it arrives here,
 * with a test, rather than as a second `<div>` inside the component.
 *
 * ## What left, and where it went (ruling R37)
 *
 * `backs_position` is now a PICKER in `FactCardBody`, not a sentence: the value a
 * human sets from this page is which talking point the fact backs, and a free
 * text box let them type a position no point occupies. `watch_out` and `answer`
 * are the machine's prose and left the page entirely. All three are still on the
 * payload — see `CardFieldName` and the module header.
 */
export function cardRows(
  block: FactCardBlock,
  wording: FactCardWording,
): CardRow[] {
  return [
    {
      field: "supports",
      label: wording.supports_label,
      lines: block.supports,
      draft: block.drafts.supports,
    },
  ];
}

/**
 * What a row prints when nobody has filled it in.
 *
 * One function rather than an inline `?? wording.empty_value`, so the rule "an
 * empty row shows the stored em dash and STAYS" has one place and one test.
 */
export function rowText(row: CardRow, wording: FactCardWording): string[] {
  return row.lines.length > 0 ? row.lines : [wording.empty_value];
}

/**
 * The card's own title, or the em dash.
 *
 * The title bar is not one of the four rows — it carries the position number and
 * the count tags too — so it gets its own accessor rather than being folded into
 * `cardRows` and then pulled back out.
 */
export function cardTitle(
  card: ScenarioCard,
  wording: FactCardWording,
): { text: string; draft: boolean } {
  const block = card.card;
  if (!block || block.title === null) {
    return { text: wording.empty_value, draft: false };
  }
  return { text: block.title, draft: block.drafts.title };
}

/**
 * The deck's own line — "3 of 10 shown · 7 more collapsed".
 *
 * Returns `null` when nothing is collapsed: a line reporting "10 of 10 shown · 0
 * more collapsed" is noise on a deck that is entirely open.
 */
export function deckLine(
  total: number,
  visible: number,
  wording: FactCardWording,
): string | null {
  const shown = Math.min(Math.max(visible, 0), total);
  const collapsed = total - shown;
  if (collapsed <= 0) return null;
  return wording.deck_template
    .replace("{shown}", String(shown))
    .replace("{total}", String(total))
    .replace("{collapsed}", String(collapsed));
}

/**
 * Whether the card at `index` opens in full.
 *
 * ## Domain note: a fold, never a filter
 *
 * Every card is on the page. This decides only whether one is OPEN — §2's "ten
 * visible, rest collapsed (title bar + source line only; click opens)". A
 * collapsed card is one click from its whole content, which is what makes the
 * fold safe on a surface where a hidden fact is a lost one.
 *
 * A nonsensical served count is clamped rather than trusted: a negative would
 * collapse everything with no way to tell why, and the value is served.
 */
export function opensInFull(index: number, visibleCount: number): boolean {
  return index < Math.max(visibleCount, 0);
}

/**
 * The accusation and stance an Include should be prefilled with (§2).
 *
 * From the card's `supports[0]` — the machine's own first answer to "what does
 * this bear on". `null` when the card names none, in which case the human picks
 * one and the Include is refused until they do.
 */
export function includePrefill(
  card: ScenarioCard,
): { allegation_id: string; stance: "supports" | "rebuts" } | null {
  const first = card.card?.supports_refs[0];
  if (!first) return null;
  return { allegation_id: first.allegation_id, stance: first.stance };
}

/**
 * Whether ANY field on this card is still a draft.
 *
 * Not rendered today. It exists because "is this deck prepared" is the question a
 * readiness check will ask, and deriving it in a second place from five booleans
 * is how two answers start to differ.
 */
export function anyDraft(drafts: FactCardDrafts): boolean {
  return (
    drafts.title ||
    drafts.backs ||
    drafts.supports ||
    drafts.watch_out ||
    drafts.answer
  );
}
