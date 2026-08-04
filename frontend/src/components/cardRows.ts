// =============================================================================
// cardRows.ts — the §7 card CONTRACT as a descriptor (split out of cardTriage)
// =============================================================================
//
// Every element a candidate card must show, as a list of labelled rows built
// from the payload. §7 makes a card missing any element a defect, so the
// contract is testable by asserting against this list rather than against the
// DOM.
//
// ## Why a descriptor instead of just writing JSX
//
// CLAUDE.md rule 30 records that component-test infrastructure (RTL, jsdom) is
// deliberately not set up, and 1.3 was not the place to reverse that. Pulling the
// contract into a pure function means the assertion "every §7 element is
// present" is a real test rather than a hope — the part the tests cannot reach
// is only that the JSX faithfully walks the list, which is what Roman's DEV
// verify line covers.
//
// ## Why this is its own file (task 1.7E, Rule 17)
//
// `cardTriage.ts` opened by saying "two pure things live here", and 1.7E's
// additions to the second one — the visible set, navigation, the refusal notice —
// took the file to the 300-line limit. The seam was already written in that
// header: this file is WHAT A CARD SHOWS, and `cardTriage` is WHAT A KEY DOES.
// The §7 contract tests move with the contract and are otherwise unchanged.
//
// ## The frontend computes NOTHING
//
// Every string in a row's `value` comes verbatim from the payload. This module
// chooses which rows exist and in what order; it never composes prose, maps a
// vocabulary, formats a number, or builds a URL (v2 §7 item 2).

import type { ScenarioCard } from "../services/scenarioCards";

/** Which §7 element a row implements. The contract test asserts on these. */
export type CardElement =
  | "quote"
  | "pinpoint"
  | "speaker"
  | "statement_kind"
  | "stance"
  | "bears_on"
  | "grounding"
  | "confidence"
  | "status"
  | "code";

/** One rendered line of a card. */
export type CardRow = {
  element: CardElement;
  /** The human-readable text, verbatim from the payload. */
  value: string;
  /** Chip-style values the card renders as separate pills rather than prose. */
  chips?: string[];
  /** Present only on the pinpoint row — the full-viewer link. */
  href?: string;
};

/**
 * Build the descriptor for one card: every §7 element, in display order.
 *
 * ## Why some rows are conditional and what that means for the contract
 *
 * A row is omitted only when the payload genuinely has nothing to say — a
 * documentary item with no speaker, an extraction with no statement kind. The
 * one structural case is §7.5: a card either has a `stance` (verb + object) or
 * it has a `defer_required_reason` explaining why it cannot be ruled on. It is
 * never missing both, and it never shows a bare verb — that was the July defect.
 * The contract test asserts exactly one of those two is present.
 */
export function cardRows(card: ScenarioCard): CardRow[] {
  const rows: CardRow[] = [];

  if (card.code) rows.push({ element: "code", value: card.code });

  // §7.1 — the quote is always present, context and question when they exist.
  rows.push({ element: "quote", value: card.quote.text });

  // §7.2 — the pinpoint, pre-composed by the backend, with its own jump target.
  // The title and page are NOT joined here: composing them would be the browser
  // making a presentation decision about case vocabulary, which is the exact thing
  // `CardStance.summary` ships pre-composed to prevent one field over.
  rows.push({
    element: "pinpoint",
    value: card.pinpoint.label,
    href: card.pinpoint.viewer_href,
  });

  // §7.3 — speaker with its attribution, then the kind of statement.
  if (card.speaker.name) {
    rows.push({
      element: "speaker",
      value: card.speaker.name,
      chips: [card.speaker.attribution],
    });
  }
  if (card.statement_kind) {
    rows.push({ element: "statement_kind", value: card.statement_kind });
  }

  // §7.5 — the stance WITH its object, or the reason there is none.
  if (card.stance) {
    rows.push({ element: "stance", value: card.stance.summary });
  } else if (card.defer_required_reason) {
    rows.push({ element: "stance", value: card.defer_required_reason });
  }

  // §7.6 — every accusation, with its elements and count as chips.
  for (const bears of card.bears_on) {
    rows.push({
      element: "bears_on",
      value: bears.accusation,
      chips: bears.count ? [...bears.elements, bears.count] : bears.elements,
    });
  }

  // §7.7 / §7.8 — grounding and the confidence band.
  if (card.grounding) {
    rows.push({ element: "grounding", value: card.grounding.label });
  }
  rows.push({ element: "confidence", value: card.confidence.label });

  rows.push({ element: "status", value: card.status_label });

  return rows;
}

/**
 * The §7 elements a card MUST carry to be rulable.
 *
 * Deliberately not every element: speaker and statement kind are legitimately
 * absent on documentary evidence, and `code` is absent until gather numbers the
 * candidate. These five are the ones whose absence makes the card a defect.
 */
export const REQUIRED_CARD_ELEMENTS: CardElement[] = [
  "quote",
  "pinpoint",
  "stance",
  "confidence",
  "status",
];

/** Which required §7 elements a card is missing. Empty means the card is whole. */
export function missingElements(card: ScenarioCard): CardElement[] {
  const present = new Set(cardRows(card).map((r) => r.element));
  return REQUIRED_CARD_ELEMENTS.filter((e) => !present.has(e));
}
