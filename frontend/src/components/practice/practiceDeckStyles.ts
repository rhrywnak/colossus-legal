// =============================================================================
// practiceDeckStyles.ts — the start screen's question list (mockup v3)
// =============================================================================
//
// PRACTICE_MOCKUP_v3_2026-08-18.html's `/* v3 additions */` block, transcribed by
// the same rule as its sibling: the mockup's own values, through
// `var(--practice-…)`, no hex literal in a component.
//
// ## Why these are not in `practiceStyles.ts`
//
// That file transcribes v2 and was already at 344 non-comment lines before this
// task — over Rule 17's limit, and not this task's to split. Adding a hundred
// more would have made a standing problem worse. The seam is by SCREEN REGION,
// not by mockup version: everything here belongs to the deck list and the resume
// box on the start card, which is the one part of the drill Marie reads BEFORE
// she answers anything.

import type { CSSProperties } from "react";

import { BLUE, INK, LINE, MUTED, PAPER } from "./practiceStyles";

/** `.resume` — the blue box offering an unfinished sitting back. */
export const resume: CSSProperties = {
  background: "var(--practice-resume-bg)",
  border: "1px solid var(--practice-resume-border)",
  borderRadius: 8,
  padding: "12px 14px",
  marginTop: 18,
  fontSize: 16,
  display: "flex",
  gap: 12,
  alignItems: "center",
  flexWrap: "wrap",
};

/** `.deck` — the question list under the count pills. */
export const deck: CSSProperties = { marginTop: 22 };

/** `.deck .dh` — the heading row, with the fold link pushed to the right. */
export const deckHeader: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "baseline",
};

/** `.deck .cnt` — the count beside the heading. */
export const deckCount: CSSProperties = { fontSize: 14, color: MUTED, fontWeight: 400 };

/** `.deck .dh a` — "Hide the questions" / "Show the questions". */
export const deckToggle: CSSProperties = {
  fontSize: 14,
  color: BLUE,
  cursor: "pointer",
  background: "none",
  border: "none",
  padding: 0,
  font: "inherit",
  textDecoration: "none",
};

/** The one sentence of instruction under the heading. */
export const deckInstruction: CSSProperties = {
  fontSize: 14,
  color: MUTED,
  margin: "4px 0 8px",
};

/** `.qrow` — one question: number, body, controls. */
export const questionRow: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "34px 1fr auto",
  gap: 10,
  padding: "12px 6px",
  borderTop: `1px solid ${LINE}`,
  alignItems: "start",
};

/** `.qrow.hasflag` — the tint that says this row carries a complaint. */
export const questionRowFlagged: CSSProperties = {
  background: "var(--practice-flagged-row-bg)",
};

/** `.qrow:last-child` — the list closes with a rule, as the mockup draws it. */
export const questionRowLast: CSSProperties = { borderBottom: `1px solid ${LINE}` };

/** `.qrow .n` */
export const questionNumber: CSSProperties = { color: MUTED, fontSize: 14, paddingTop: 3 };

/** `.qrow .qt` — the question, in the serif. */
export const questionText: CSSProperties = {
  fontFamily: 'Georgia, "Times New Roman", serif',
  fontSize: 18,
  lineHeight: 1.35,
};

/** `.qrow .qs` — the source line. */
export const questionSource: CSSProperties = { fontSize: 13, color: MUTED, marginTop: 3 };

/** `.qrow.skipped .qt`, `.qrow.skipped .qs` — struck through at 40%. */
export const questionSkipped: CSSProperties = { opacity: 0.4, textDecoration: "line-through" };

/** `.qrow .ctl` */
export const rowControls: CSSProperties = { display: "flex", gap: 6, whiteSpace: "nowrap" };

/** `.qrow .ctl button` */
export const rowButton: CSSProperties = {
  fontSize: 13,
  padding: "5px 9px",
  borderRadius: 8,
  border: "1px solid var(--practice-control-border)",
  background: PAPER,
  color: INK,
  font: "inherit",
  cursor: "pointer",
};

/** `.qrow.skipped .ctl button.skip` — the control that put the row out. */
export const rowButtonSkipped: CSSProperties = {
  ...rowButton,
  borderColor: "var(--practice-navy)",
  color: "var(--practice-navy)",
  background: "var(--practice-choice-selected-bg)",
};

/** `.flagline` — the inline note editor. */
export const flagLine: CSSProperties = {
  marginTop: 8,
  display: "flex",
  gap: 8,
  alignItems: "center",
  flexWrap: "wrap",
};

/** `.flagline input` */
export const flagInput: CSSProperties = {
  font: "inherit",
  fontSize: 14,
  padding: "6px 8px",
  border: "1px solid var(--practice-control-border)",
  borderRadius: 6,
  flex: "1 1 320px",
};

/** `.flagged` — the stored note. The ⚑ is the mockup's `::before`. */
export const flagged: CSSProperties = {
  fontSize: 13,
  color: "var(--practice-george-text)",
  marginTop: 6,
};

