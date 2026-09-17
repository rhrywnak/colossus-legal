// =============================================================================
// practiceReviewLoopStyles.ts — the review bar and the notes (CC_TASK_REVIEW_LOOP_v1)
// =============================================================================
//
// Transcribed from the ruled mockup v3's bottom panel, through `var(--practice-…)`
// tokens and the house warning pair — no hex literal in a component (Rule 2).
// Its own file: every sibling practice style file is near Rule 17's limit.
//
// STRUCTURAL: geometry from one approved drawing — not settings.

import type { CSSProperties } from "react";

import { BLUE, INK, LINE, MUTED, PALE } from "./practiceStyles";

/** The slim bar under the title: amber count left, Done reviewing right. */
export const reviewBar: CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  justifyContent: "space-between",
  gap: "8px 16px",
  border: `1px solid ${LINE}`,
  borderRadius: 10,
  padding: "10px 16px",
  margin: "10px 0 14px",
};

/** "{n} new since you last reviewed" — the amber of owed work. */
export const reviewCount: CSSProperties = {
  color: "var(--burden-warning-text)",
  fontWeight: 700,
  fontSize: 16,
};

/** The failure sentence under the bar. */
export const reviewError: CSSProperties = { color: "var(--practice-red)", fontSize: 14, width: "100%" };

/** One note: the blue left border, on the pale ground. */
export const note: CSSProperties = {
  borderLeft: `4px solid ${BLUE}`,
  background: PALE,
  padding: "8px 12px",
  margin: "8px 0 0",
  borderRadius: "0 6px 6px 0",
};

/** "Chuck · 17 Sep" — the author in blue, bold. */
export const noteAuthor: CSSProperties = { color: BLUE, fontWeight: 700, fontSize: 14 };

/** The note's words. Struck: through, and muted. */
export function noteText(struck: boolean): CSSProperties {
  return {
    color: struck ? MUTED : INK,
    textDecoration: struck ? "line-through" : "none",
    margin: "2px 0 0",
    fontSize: 15,
    whiteSpace: "pre-wrap",
  };
}

/** The date / "struck 18 Sep" line. */
export const noteMeta: CSSProperties = { color: MUTED, fontSize: 13, marginTop: 2 };

/** The note box on the answers page. */
export const noteBox: CSSProperties = {
  width: "100%",
  boxSizing: "border-box",
  minHeight: 70,
  font: "inherit",
  fontSize: 15,
  padding: 8,
  border: "1px solid var(--practice-control-border)",
  borderRadius: 6,
  marginTop: 8,
};

/** The notes panel's row of controls. */
export const noteButtons: CSSProperties = { display: "flex", gap: 10, marginTop: 8, alignItems: "center" };
