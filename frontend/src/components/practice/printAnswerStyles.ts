// printAnswerStyles.ts — the answer block on Chuck's reading copy.
//
// Its own module rather than four more exports in `printStyles`: that file is
// the QUESTIONS sheet's, it is already the larger half of the print surface, and
// a reader looking for why an answer is boxed should not have to walk past the
// ruled-line styles that deliberately do not apply here.
//
// Every colour is a `--print-*` token, declared in `styles/tokens.css` under
// `[data-print-desk]` — the same rule the palette test enforces on its sibling.

import type { CSSProperties } from "react";

/** Marie's answer, set apart from the question above it. */
export const answer: CSSProperties = {
  borderLeft: "3px solid var(--print-answer-rule)",
  background: "var(--print-answer-bg)",
  padding: "9px 13px",
  margin: "6px 0 0",
};

/**
 * The same block where she has NOT answered.
 *
 * Deliberately a different shape, not merely different words: a reader
 * skimming eight sheets for the gaps should find them without reading. No fill
 * and a dashed rule — the visual grammar of "nothing here yet" that the greyed
 * practice box uses on screen.
 */
export const absent: CSSProperties = {
  borderLeft: "3px dashed var(--print-hairline)",
  padding: "9px 13px",
  margin: "6px 0 0",
};

/** The day she gave it. Above the words, small — it qualifies them. */
export const when: CSSProperties = {
  fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, sans-serif",
  fontSize: 10.5,
  letterSpacing: ".04em",
  textTransform: "uppercase",
  color: "var(--print-faint)",
  margin: "0 0 4px",
};

/** Her words. Serif and full size: this is the thing on the page to be read. */
export const text: CSSProperties = {
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: 14,
  lineHeight: 1.5,
  color: "var(--print-ink)",
  margin: 0,
};
