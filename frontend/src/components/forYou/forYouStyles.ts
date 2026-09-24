// =============================================================================
// forYouStyles.ts — the "For you" page (CC_TASK_FOR_YOU_POLISH_v1)
// =============================================================================
//
// Transcribed from the ratified mockup `FOR_YOU_MOCKUP_v1_2026-09-22.html`
// (boards 1, 2 and 5), through the `var(--fy-…)` tokens `tokens.css` defines
// under `[data-surface="for-you"]` — no hex literal here or in a component
// (Rule 2).
//
// ## Why this file no longer reads a --practice-… token
//
// It used to, and that is the defect this task exists to repair. Those tokens
// are defined on `[data-surface="practice"]`; this page declares
// `data-surface="for-you"`, so every one of them resolved to NOTHING and every
// declaration that used one was thrown away at computed-value time — no border,
// no background, no muted grey. The page rendered as bare text lines with
// nothing failing anywhere. `tokens.css` carries the measurement; the fence is
// `__tests__/forYouTokens.test.ts`, which fails the moment a style here reads a
// custom property the page's own surface does not define.
//
// ## 390px FIRST
//
// Board 5 is the phone, and it is not an afterthought: the list is read on a
// phone between hearings. So the row wraps rather than squeezes, the time drops
// UNDER the text at narrow widths (ruling §12.5 — the CSS reading, not a
// re-composed sentence), and nothing is truncated with an ellipsis — a note cut
// in half is a note that has to be opened to be read, which defeats the page.
//
// STRUCTURAL: geometry from one approved drawing — not settings. A margin is
// not something an operator tunes on the Settings page, and the reusability
// checkpoint does not apply to a transcription of a specific mockup.

import type { CSSProperties } from "react";

/** The mockup's `:root`, read by name. See `tokens.css`'s for-you block. */
const INK = "var(--fy-ink)";
const MUTED = "var(--fy-muted)";
const LINE = "var(--fy-line)";
const PAPER = "var(--fy-paper)";
const SOFT = "var(--fy-soft)";
const DOT = "var(--fy-dot)";
const NEW_BORDER = "var(--fy-row-new-border)";

/** The column the whole page lives in. Narrower than the review page's 960: a
 *  list of sentences reads badly at full width. */
export const page: CSSProperties = {
  maxWidth: 820,
  margin: "0 auto",
  padding: "8px 16px 48px",
  color: INK,
  // The board's own body rule — 15px on a 1.5 leading. Every size below is a
  // deliberate step away from this one, so it has to be stated here rather than
  // inherited from whatever the surrounding chrome happens to set.
  font: '15px/1.5 -apple-system, "Segoe UI", Roboto, Helvetica, Arial, sans-serif',
};

/** `For you`. The board's `h2`, not a page-title-sized heading: the list is
 *  what the reader came for, and a 26px title pushed the first row down the
 *  screen on a phone. */
export const title: CSSProperties = { fontSize: 19, margin: "0 0 3px" };

export const subtitle: CSSProperties = {
  color: MUTED,
  fontSize: 13,
  margin: "0 0 16px",
};

/** The two tabs — the board's pill row. */
export const tabs: CSSProperties = {
  display: "flex",
  gap: 6,
  marginBottom: 14,
  flexWrap: "wrap",
};

const tabBase: CSSProperties = {
  appearance: "none",
  border: `1px solid ${LINE}`,
  borderRadius: 999,
  background: PAPER,
  color: INK,
  padding: "5px 13px",
  fontSize: 13,
  cursor: "pointer",
  // 40px of touch target at the phone width, without changing how it looks: the
  // board's own padding gives ~28px, which is under every thumb guideline.
  minHeight: 40,
};

export const tab: CSSProperties = tabBase;

/** The chosen list. Filled with the INK and not with the blue — the tabs pick
 *  between two lists, and a blue fill would compete with the unread rows. */
export const tabActive: CSSProperties = {
  ...tabBase,
  background: "var(--fy-tab-on-bg)",
  color: "var(--fy-tab-on-ink)",
  borderColor: "var(--fy-tab-on-bg)",
};

/** `TODAY` / `YESTERDAY` / `EARLIER`. */
export const dayHeading: CSSProperties = {
  fontSize: 12,
  letterSpacing: ".07em",
  fontWeight: 700,
  color: MUTED,
  margin: "18px 0 8px",
};

/**
 * One row — a CARD, and the whole card is the link.
 *
 * Three columns: the dot, the text, and the time. `min-width: 0` on the middle
 * one is what lets a long line wrap instead of pushing the time off the card —
 * a flex item's default minimum size is its content, which is the single most
 * common way a row like this loses its right-hand column.
 */
const rowBase: CSSProperties = {
  display: "flex",
  gap: 12,
  textDecoration: "none",
  color: INK,
  border: `1px solid ${LINE}`,
  borderRadius: 8,
  padding: "13px 14px",
  marginBottom: 9,
  cursor: "pointer",
};

/**
 * An UNREAD row: the soft blue ground and the blue edge, together.
 *
 * Both, deliberately. A tint alone is very quiet on a bright screen at arm's
 * length, and the edge is what survives a phone held in a corridor; the board
 * draws the pair, and the red dot in the first column is the third reading of
 * the same fact for anyone who cannot separate the two blues.
 */
export const rowUnread: CSSProperties = {
  ...rowBase,
  background: SOFT,
  borderColor: NEW_BORDER,
};

/** A row already read: plain paper, plain edge. Still a card — it is still one
 *  thing that happened, and it is still pressable. */
export const rowRead: CSSProperties = {
  ...rowBase,
  background: PAPER,
};

/** The unread mark. 9px, and the first thing in the row's reading order. */
export const dot: CSSProperties = {
  width: 9,
  height: 9,
  borderRadius: "50%",
  background: DOT,
  marginTop: 7,
  flex: "0 0 9px",
};

/** The read mark: the same circle, hollow. Drawn rather than omitted, so the
 *  text of every row starts on the same vertical rule whichever state it is in
 *  — a list whose left edge moves as you scroll is a list that reads as two. */
export const dotRead: CSSProperties = {
  ...dot,
  background: "transparent",
  border: `1px solid ${LINE}`,
};

/** The text column. */
export const main: CSSProperties = { minWidth: 0, flex: 1 };

/** `S-11 · The $50,000 — “Whose money…”` — the row's own headline. */
export const deckLine: CSSProperties = {
  fontWeight: 600,
  marginBottom: 3,
};

/** The note, the answer or the new wording. Never truncated. */
export const body: CSSProperties = {
  color: INK,
  fontSize: 14,
  margin: 0,
  whiteSpace: "pre-wrap",
};

/** `Chuck · on your answer of Mon 21 Sep`. */
export const byline: CSSProperties = {
  color: MUTED,
  fontSize: 12.5,
  marginTop: 5,
};

/** The time, in the row's right-hand column — and on its own line at phone
 *  width, which `PHONE_CSS` below is what actually does. */
export const when: CSSProperties = {
  color: MUTED,
  fontSize: 12.5,
  whiteSpace: "nowrap",
  paddingTop: 2,
};

/** Nothing waiting — board 2. Centred text, and no frame: an empty state inside
 *  a box reads as a row that failed to load. */
export const empty: CSSProperties = {
  textAlign: "center",
  padding: "40px 16px",
  color: MUTED,
};

export const emptyTitle: CSSProperties = {
  display: "block",
  color: INK,
  fontSize: 17,
  fontWeight: 700,
  marginBottom: 6,
};

export const emptyLine: CSSProperties = { fontSize: 15, color: MUTED };

/** A failed read, or a failed write. Stated, never swallowed. */
export const alert: CSSProperties = {
  border: `1px solid ${LINE}`,
  borderRadius: 8,
  padding: "12px 14px",
  margin: "12px 0",
  fontSize: 14,
  color: DOT,
  background: PAPER,
};

/**
 * The two rules a style OBJECT cannot carry: a media query and a hover.
 *
 * Rendered by the page as a real `<style>` element, scoped by the same
 * `data-surface` attribute the palette is — the `LINK_CSS` precedent in
 * `practice/practiceFlowStyles.ts`, for the same reason.
 *
 * ## Board 5, in one rule
 *
 * At 430px and under, the row wraps and the time takes a line of its own,
 * indented to the text column (9px dot + 12px gap = 21px) so it reads as part
 * of the row rather than as a new one. The board itself folds the time into the
 * byline sentence instead; that sentence is composed on the SERVER, and §1 of
 * this task is explicitly "at the existing wording", so the CSS reading is what
 * ships and the difference is recorded in the report (ruling §12.5).
 */
export const PHONE_CSS = `
[data-surface="for-you"] [data-for-you-row]:hover {
  border-color: var(--fy-blue);
}
@media (max-width: 430px) {
  [data-surface="for-you"] [data-for-you-row] {
    flex-wrap: wrap;
  }
  [data-surface="for-you"] [data-for-you-deck-line] {
    font-size: 14px;
  }
  [data-surface="for-you"] [data-for-you-when] {
    flex-basis: 100%;
    padding-left: 21px;
    padding-top: 4px;
  }
}
`;
