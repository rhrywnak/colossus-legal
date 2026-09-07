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
// ## Nothing here is hidden for being absent
//
// §2 is explicit, and it is the one place this instruction overrules the mockup
// of record: a card with no Answer is work somebody still owes, and hiding the
// row hides the work rather than the gap.

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
 * The four authored rows, in the order §2 lists them, ALWAYS all four.
 *
 * Proof is not here: it is the record's own words, no human writes it, and it has
 * no `authored_by` to draft-mark. It renders above these from the card's quote.
 */
export function cardRows(
  block: FactCardBlock,
  wording: FactCardWording,
): CardRow[] {
  return [
    {
      field: "backs_position",
      label: wording.backs_label,
      lines: block.backs === null ? [] : [block.backs],
      draft: block.drafts.backs,
    },
    {
      field: "supports",
      label: wording.supports_label,
      lines: block.supports,
      draft: block.drafts.supports,
    },
    {
      field: "watch_out",
      label: wording.watch_out_label,
      lines: block.watch_out === null ? [] : [block.watch_out],
      draft: block.drafts.watch_out,
    },
    {
      field: "answer",
      label: wording.answer_label,
      lines: block.answer === null ? [] : [block.answer],
      draft: block.drafts.answer,
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
