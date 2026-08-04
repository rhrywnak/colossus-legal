// =============================================================================
// candidateCardStyles — the §2c visual language of one candidate card (2.10 split)
// =============================================================================
//
// Extracted from `CandidateCard.tsx` when task 2.10's link panel took that file
// past the 300-line limit (Rule 17). Nothing here changed in the move.
//
// The seam is the conventional one and the repo already has its sibling
// (`scenarioSectionStyles`): this file is WHAT A CARD LOOKS LIKE, and what
// remains in `CandidateCard` is WHAT IT SHOWS. A v3 visual tweak now touches one
// file that contains no logic at all.
//
// ## Visual language (§2c, binding)
//
// Pure white surfaces, no card borders, regular weight with bold reserved for the
// pinpoint page, one accent, generous line height.

import React from "react";

const SURFACE = "var(--bg-surface)"; // --card #ffffff

/**
 * The card. Mockup `.card`: white, radius 12, `--shadow-raised`, padding
 * 18px 24px 20px. NO BORDER — v3 removed every card border (see the
 * `--bg-canvas` comment in tokens.css for why the canvas moved with them).
 */
export const cardStyle: React.CSSProperties = {
  background: SURFACE,
  borderRadius: "var(--radius-card)",
  padding: "18px 24px 20px",
  display: "flex",
  flexDirection: "column",
  gap: "0.6rem",
  fontWeight: 400,
  // The unselected card still lifts off the page, just less. A flat card on a
  // tinted canvas reads as a hole rather than a surface.
  boxShadow: "var(--shadow-card)",
};

/**
 * The SELECTED card — the one the keyboard is aimed at.
 *
 * v3 marks selection with ELEVATION (`--shadow-raised`) rather than 1.7C's accent
 * border. That is not only a style change: the card is the thing a human is about
 * to rule on, and lifting it above its siblings says "this one" in a way a
 * coloured outline competes with the ruling buttons to say.
 */
export const selectedCardStyle: React.CSSProperties = {
  ...cardStyle,
  boxShadow: "var(--shadow-raised)",
};

/** The compact row a ruled card collapses to. Same surface, one line of it. */
export const compactCardStyle: React.CSSProperties = {
  ...cardStyle,
  flexDirection: "row",
  alignItems: "center",
  gap: "12px",
  padding: "10px 16px",
  cursor: "pointer",
};

/** Mockup `.pin` / neutral chips inside the card body. */
export const chipStyle: React.CSSProperties = {
  borderRadius: "6px",
  padding: "3px 10px",
  fontSize: "12px",
  fontWeight: 500,
  color: "var(--text-secondary)",
  whiteSpace: "nowrap",
};

/**
 * A metadata chip in the one-row metadata strip (task 1.7E, item 8).
 *
 * ## Why it truncates instead of being shortened
 *
 * The payload's words are the payload's words: "Grounded — found on the page" is
 * the sentence the backend composed, and rewriting it to "✓ grounded" in the
 * browser would be this file inventing vocabulary. So the chip renders the
 * sentence verbatim and lets CSS clip what does not fit, with the full text on the
 * element's `title` — truncation is presentation, paraphrase would be authorship.
 */
export const metaChipStyle: React.CSSProperties = {
  ...chipStyle,
  background: "var(--v3-chrome)",
  maxWidth: "24ch",
  overflow: "hidden",
  textOverflow: "ellipsis",
  display: "inline-block",
  verticalAlign: "middle",
};

/**
 * The quote-in-context panel. Mockup `.ctx`: its OWN soft surface (#fafbfc,
 * radius 10, padding 14px 18px) inset within the white card.
 *
 * 1.7C rendered the context as bare text. Giving it a panel does a job: it makes
 * visually obvious where the SOURCE PAGE's words start and stop, so the reader can
 * tell the document's voice from the card's own labels above and below it.
 */
/**
 * A metadata chip that is carrying BAD NEWS (task 2.12, item C).
 *
 * The grounding chip only appears when the quote could not be located (§7.7), so
 * on the rare occasion it is there it must not look like the neutral chips
 * beside it. Amber rather than red: the quote not being found is a reason to
 * look before ruling, not a failure — and red is reserved on this surface for
 * Exclude and for refusals.
 *
 * Colour never stands alone here either: the chip's own words say what is wrong.
 */
export const warningChipStyle: React.CSSProperties = {
  background: "var(--state-warning-bg-soft)",
  color: "var(--v3-amber-text)",
};

export const contextPanelStyle: React.CSSProperties = {
  background: "var(--v3-context-panel)",
  borderRadius: "10px",
  padding: "14px 18px",
  color: "var(--text-secondary)",
  lineHeight: 1.7,
  fontSize: "13.5px",
};

export const contextStyle: React.CSSProperties = {
  color: "var(--text-secondary)",
  fontSize: "13.5px",
  lineHeight: 1.7,
};

/** Mockup `.edge`: the honest page-edge marker — italic, muted, 12.5px. */
export const edgeStyle: React.CSSProperties = {
  color: "var(--text-muted)",
  fontStyle: "italic",
  fontSize: "12.5px",
};

/** Mockup `.ctx mark`: #ffec9e, radius 3, padding 1px 3px, weight 500. */
export const markStyle: React.CSSProperties = {
  background: "var(--highlight-quote-soft)",
  color: "var(--text-primary)",
  padding: "1px 3px",
  borderRadius: "3px",
  fontWeight: 500,
};

/** The one-line quote in a compact row: whatever fits, then an ellipsis. */
export const compactQuoteStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
  fontSize: "13px",
  color: "var(--text-secondary)",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};
