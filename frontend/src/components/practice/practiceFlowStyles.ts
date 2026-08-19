// =============================================================================
// practiceFlowStyles.ts — the top bar, the row status, and the receipt picker
// =============================================================================
//
// Mockup v3's `.topbar` block, transcribed by the same rule as its two siblings:
// the mockup's own values, through `var(--practice-…)`, no hex literal in a
// component. Plus the two things v1 (the Chuck review) adds — the status under a
// deck row, and the list the "I'd point to…" control opens under the answer box.
//
// ## Why a third style file
//
// `practiceStyles.ts` transcribes mockup v2 and was over Rule 17's limit before
// this task; `practiceDeckStyles.ts` transcribes the start card's question list.
// The seam here is by SCREEN REGION, as it is there: everything below belongs to
// the QUESTION and REVEAL screens — the way out of a sitting, and the one
// control under the answer box — except the row status, which is filed here
// because it is the deck row's report on a SITTING rather than part of the list.
//
// ## The `font` shorthand, once more
//
// Every rule below that sets a size sets `font: "inherit"` FIRST. React writes a
// style object's properties in order and `font` is a shorthand that resets
// `font-size`, so the other order silently renders the control at the body's
// 18px. That is what shipped in .401; see `practiceDeckStyles.deckToggle`.

import type { CSSProperties } from "react";

import { BLUE, INK, LINE, MUTED, PAPER } from "./practiceStyles";

/** `.topbar` — the row of exits above the question. */
export const topBar: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  margin: "-8px 0 10px",
  fontSize: 14,
};

/**
 * `.topbar a` — a blue link that is really a button.
 *
 * A `<button>` and not an `<a>`: these do not navigate to a URL, they run a
 * handler that settles a row and closes a sitting. An anchor with no `href` is
 * not reachable by keyboard, which on a witness surface is not a detail.
 */
export const topBarLink: CSSProperties = {
  font: "inherit",
  fontSize: 14,
  color: BLUE,
  background: "none",
  border: "none",
  padding: 0,
  cursor: "pointer",
  textDecoration: "none",
};

/** `.topbar .sep` — the dot between two controls. */
export const topBarSeparator: CSSProperties = {
  color: "var(--practice-separator)",
  margin: "0 8px",
};

/** The grey sentence beside Back, saying what it costs. 13px, as drawn. */
export const topBarHint: CSSProperties = { fontSize: 13, color: MUTED };

/**
 * The status under a deck row — `answered today · repeat`.
 *
 * 13px muted, matching the source line it sits under. The sentence itself is
 * composed on the server; nothing here decides what it says.
 */
export const rowStatus: CSSProperties = { fontSize: 13, color: MUTED, marginTop: 3 };

/**
 * The question text on a deck row, as the link that opens it alone.
 *
 * Inherits the row's serif and colour rather than turning blue: the mockup's
 * row is a question, not a menu, and a blue serif question reads as a hyperlink
 * to somewhere else rather than as the question itself. The pointer and the
 * hover underline are what say it is clickable, and `Practice this one ▸` on
 * the control side is the visible half for a reader who never hovers.
 */
export const questionLink: CSSProperties = {
  font: "inherit",
  color: "inherit",
  background: "none",
  border: "none",
  padding: 0,
  margin: 0,
  textAlign: "left",
  cursor: "pointer",
};

/** `redirect` — the small tag beside Chuck's pill on a redirect question. */
export const redirectTag: CSSProperties = {
  display: "inline-block",
  fontSize: 12,
  padding: "2px 8px",
  borderRadius: 4,
  background: "var(--practice-chuck-bg)",
  color: "var(--practice-chuck-text)",
  border: "1px solid var(--practice-chuck-border)",
  marginLeft: 8,
  verticalAlign: "middle",
};

/** The control under the answer box that opens this scenario's receipts. */
export const pointsToToggle: CSSProperties = {
  font: "inherit",
  fontSize: 14,
  color: BLUE,
  background: "none",
  border: "none",
  padding: 0,
  marginTop: 10,
  cursor: "pointer",
};

/** The opened list of receipts. */
export const pointsToBox: CSSProperties = {
  marginTop: 8,
  border: `1px solid ${LINE}`,
  borderRadius: 8,
  padding: "10px 14px",
  background: PAPER,
};

/** One receipt, as a checkbox row. */
export const pointsToItem: CSSProperties = {
  display: "block",
  fontSize: 15,
  color: INK,
  margin: "6px 0",
  cursor: "pointer",
};

/** The picked receipts, echoed under the control and on the reveal. */
export const pointsToChosen: CSSProperties = { fontSize: 14, color: MUTED, marginTop: 8 };

/**
 * The one rule inline styles cannot express: `.topbar a:hover`.
 *
 * The mockup underlines a top-bar link on hover, and that is not decoration —
 * it is the only thing on the bar that says these blue words are pressable
 * before you press them. A style OBJECT has no `:hover`, so this is a real
 * `<style>` element, scoped by the same `data-surface` attribute the palette
 * is, and keyed on a data attribute rather than a tag so it cannot leak onto
 * any other link on the page.
 *
 * Rendered by both practice pages beside `PRINT_CSS`, for the same reason that
 * one is: a media query and a pseudo-class are the two things a React style
 * object cannot carry.
 */
export const LINK_CSS = `
[data-surface="practice"] [data-practice-link]:hover {
  text-decoration: underline;
}
[data-surface="practice"] [data-practice-link]:disabled {
  cursor: default;
  opacity: 0.5;
  text-decoration: none;
}
`;
