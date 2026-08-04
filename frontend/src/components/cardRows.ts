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

  // §7.5 — the stance WITH its object; or what a HUMAN said it bears on; or the
  // reason there is neither.
  //
  // ## Why the middle branch exists (task 2.10, ruling R2)
  //
  // A card the extraction never linked, that a human HAS linked, has no stance
  // (correctly — the machine found none, and inventing one is what R2 forbids)
  // and no defer reason (correctly — the link cleared it). Without this branch it
  // would have all three absent, and the §7 completeness contract below would go
  // red on a card that is working exactly as designed.
  //
  // The order is not arbitrary: the machine's finding outranks the human's note
  // about a card where both somehow exist, because a stance carries a verb the
  // record supports and this sentence deliberately does not.
  if (card.stance) {
    rows.push({ element: "stance", value: card.stance.summary });
  } else if (card.human_link_summary) {
    rows.push({ element: "stance", value: card.human_link_summary });
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

// ─── The metadata chip row (task 2.12, item C) ──────────────────────────────

/** One metadata chip: its words, and whether it is carrying bad news. */
export type MetaChip = { text: string; warning: boolean };

/**
 * Grounding states that are NOT good news, and so earn a chip.
 *
 * §7.7 makes grounding a WARNING — it appears when the quote could not be
 * located. A chip announcing that the quote WAS found is that contract read
 * backwards: it spends a line of the card saying nothing happened. So `exact`
 * and `normalized` are silent and these two speak.
 *
 * `pending` sits with `not_found` because "Not yet checked" is not good news
 * either — it is the absence of verification, which is precisely what a human
 * about to rule needs told.
 *
 * Measured on S-2 (2026-08-04): 83 `exact`, 65 `normalized`, ZERO `not_found`
 * or `pending`. The chip therefore never appears on that scenario. That is
 * warning-only working, not a chip that broke.
 */
export const GROUNDING_WARNING_STATES = ["not_found", "pending"];

/**
 * The card's metadata chips, in display order.
 *
 * ## What this row is for
 *
 * C-116 carried five chips, three of which said nothing was wrong. The row now
 * carries the two facts a human needs while deciding — who said it and what kind
 * of statement it is — plus anything that is actually a problem.
 *
 * ## Why "extracted" stays, however repetitive it looks
 *
 * The §3 sequencing ruling requires Phase-1 surfaces to display raw speaker
 * strings VISIBLY LABELLED as extracted until canonical identity lands in B0
 * (2.1 / 2.2). It is provenance, not clutter, and it retires when that arrives
 * and not before. Deleting it as noise would be removing the one thing that says
 * this name has not been reconciled with anything.
 */
export function metaChips(card: ScenarioCard): MetaChip[] {
  const chips: MetaChip[] = [];

  if (card.speaker.name) {
    chips.push({ text: card.speaker.name, warning: false });
    chips.push({ text: card.speaker.attribution, warning: false });
  }
  if (card.statement_kind) {
    chips.push({ text: card.statement_kind, warning: false });
  }
  if (card.grounding && GROUNDING_WARNING_STATES.includes(card.grounding.state)) {
    chips.push({ text: card.grounding.label, warning: true });
  }
  // A band is information; "unscored" is the ABSENCE of information taking up
  // space, and whether a scan has run is already a filter facet.
  if (card.confidence.band !== "unscored") {
    chips.push({ text: card.confidence.label, warning: false });
  }
  return chips;
}
