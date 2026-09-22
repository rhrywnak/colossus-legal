// =============================================================================
// forYouStyles.ts — the "For you" page (CC_TASK_FOR_YOU_v1 L1)
// =============================================================================
//
// Transcribed from the ratified mockup `FOR_YOU_MOCKUP_v1_2026-09-22.html`
// (boards 1, 2, 3 and 5), through the `var(--practice-…)` tokens the practice
// surfaces already define — no hex literal here or in a component (Rule 2).
//
// ## 390px FIRST
//
// Board 5 is the phone, and it is not an afterthought: the list is read on a
// phone between hearings. So the row is a single column that wraps, the time
// drops under the text rather than beside it at narrow widths, and nothing is
// truncated with an ellipsis — a note cut in half is a note that has to be
// opened to be read, which defeats the page.
//
// STRUCTURAL: geometry from one approved drawing — not settings. A margin is
// not something an operator tunes on the Settings page, and the reusability
// checkpoint does not apply to a transcription of a specific mockup.

import type { CSSProperties } from "react";

import { BLUE, INK, LINE, MUTED, PAPER, QUIET_BG } from "../practice/practiceStyles";

/** The column the whole page lives in. Narrower than the review page's 960: a
 *  list of sentences reads badly at full width. */
export const page: CSSProperties = {
  maxWidth: 820,
  margin: "0 auto",
  padding: "8px 16px 48px",
  color: INK,
  fontFamily: '-apple-system, "Segoe UI", Roboto, Helvetica, Arial, sans-serif',
};

export const title: CSSProperties = { fontSize: 26, margin: "4px 0 6px" };

export const subtitle: CSSProperties = {
  color: MUTED,
  fontSize: 15,
  margin: "0 0 18px",
};

/** The two tabs. */
export const tabs: CSSProperties = {
  display: "flex",
  gap: 6,
  borderBottom: `1px solid ${LINE}`,
  marginBottom: 6,
  flexWrap: "wrap",
};

const tabBase: CSSProperties = {
  appearance: "none",
  border: "none",
  background: "transparent",
  padding: "8px 10px",
  fontSize: 14,
  fontWeight: 600,
  color: MUTED,
  cursor: "pointer",
  borderBottom: "2px solid transparent",
  // 44px of touch target at the phone width, without changing how it looks.
  minHeight: 40,
};

export const tab: CSSProperties = tabBase;

export const tabActive: CSSProperties = {
  ...tabBase,
  color: INK,
  borderBottom: `2px solid ${BLUE}`,
};

/** `TODAY` / `YESTERDAY` / `EARLIER`. */
export const dayHeading: CSSProperties = {
  fontSize: 11,
  letterSpacing: ".08em",
  fontWeight: 700,
  color: MUTED,
  margin: "18px 0 6px",
};

/** One row — the whole row is the link. */
export const row: CSSProperties = {
  display: "block",
  textDecoration: "none",
  color: "inherit",
  background: PAPER,
  border: `1px solid ${LINE}`,
  borderRadius: 8,
  padding: "12px 14px",
  marginBottom: 8,
};

/** A row already read, on the Everything tab: present, and plainly quieter. */
export const rowRead: CSSProperties = {
  ...row,
  background: QUIET_BG,
};

/** `S-11 · The $50,000 — “Whose money…”` */
export const deckLine: CSSProperties = {
  fontSize: 13,
  color: MUTED,
  marginBottom: 4,
};

/** The note, the answer or the new wording. Never truncated. */
export const body: CSSProperties = {
  fontSize: 15,
  lineHeight: 1.45,
  margin: "0 0 6px",
  whiteSpace: "pre-wrap",
};

/** The byline and the time, side by side — and stacked at phone width. */
export const foot: CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: "2px 10px",
  alignItems: "baseline",
  justifyContent: "space-between",
  fontSize: 13,
  color: MUTED,
};

export const byline: CSSProperties = { flex: "1 1 auto", minWidth: 0 };

export const when: CSSProperties = { flex: "0 0 auto", whiteSpace: "nowrap" };

/** Nothing waiting. */
export const empty: CSSProperties = {
  background: PAPER,
  border: `1px solid ${LINE}`,
  borderRadius: 8,
  padding: "28px 18px",
  textAlign: "center",
};

export const emptyTitle: CSSProperties = {
  fontSize: 17,
  fontWeight: 600,
  marginBottom: 6,
};

export const emptyLine: CSSProperties = { fontSize: 14, color: MUTED };

/** A failed read, or a failed write. Stated, never swallowed. */
export const alert: CSSProperties = {
  border: `1px solid ${LINE}`,
  borderRadius: 8,
  padding: "12px 14px",
  margin: "12px 0",
  fontSize: 14,
  color: "var(--practice-red)",
  background: PAPER,
};
