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

/** `.deck .dh a` — "Hide the questions" / "Show the questions".
 *
 * ## Why `font` comes FIRST and `fontSize` after it
 *
 * `font` is a SHORTHAND: setting it resets `font-size` along with everything
 * else. React writes a style object's properties in declaration order, so
 * `{ fontSize: 14, …, font: "inherit" }` sets 14 and then throws it away — the
 * control renders at the body's 18px. That is exactly what shipped in .401 and
 * what the mockup check called out as "larger than drawn". The order below is
 * the fix, and it is the whole fix.
 */
export const deckToggle: CSSProperties = {
  font: "inherit",
  fontSize: 14,
  color: BLUE,
  cursor: "pointer",
  background: "none",
  border: "none",
  padding: 0,
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

/** `.qrow .ctl button` — 13px / 5×9, as the mockup draws it.
 *
 * `font: "inherit"` FIRST, then `fontSize`. See [`deckToggle`] for why the order
 * is the whole of it: the shorthand resets the size, and in .401 these buttons
 * rendered at 18px for that reason.
 */
/**
 * `Delete` on a row — the one control a row carries outside edit mode.
 *
 * ## Why it is quiet and not red
 *
 * It is a destructive-sounding word for an act that destroys nothing: the
 * mechanism underneath is the existing hide, and Marie's answers are untouched.
 * A red button would promise a consequence the code does not deliver, and an
 * undo sits under it either way. Muted, with the border carrying the warmth.
 */
export const rowDeleteButton: CSSProperties = {
  background: PAPER,
  border: "1px solid var(--practice-delete-border)",
  color: "var(--practice-delete-ink)",
  borderRadius: 6,
  padding: "6px 12px",
  fontSize: 13,
  whiteSpace: "nowrap",
  cursor: "pointer",
};

export const rowButton: CSSProperties = {
  font: "inherit",
  fontSize: 13,
  padding: "5px 9px",
  borderRadius: 8,
  border: "1px solid var(--practice-control-border)",
  background: PAPER,
  color: INK,
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

/**
 * The break above Chuck's redirects in the deck list.
 *
 * A rule and a small caps-ish label rather than a heading: it separates two
 * kinds of question inside one side's list, and a real heading would imply a
 * third side. `font` is NOT used as a shorthand here — the .401 defect was a
 * shorthand resetting a size set on the line above it.
 */
export const redirectsSubheader: CSSProperties = {
  marginTop: 14,
  marginBottom: 2,
  paddingTop: 10,
  borderTop: "1px solid var(--practice-line)",
  fontSize: 12,
  fontWeight: 600,
  letterSpacing: ".04em",
  color: "var(--practice-separator)",
};

/**
 * The line that stands where a deleted row stood, carrying its undo.
 *
 * Same vertical rhythm as a row so the list does not jump when one goes: a list
 * that reflowed under Chuck's hand as he deleted three questions would move the
 * next one out from under his cursor.
 */
export const deletedLine: CSSProperties = {
  padding: "14px 0",
  borderBottom: `1px solid ${LINE}`,
  fontSize: 13.5,
  color: MUTED,
};

/** `Undo`, beside that line. A link, because it is a way back and not an act. */
export const undoLink: CSSProperties = {
  background: "none",
  border: 0,
  padding: 0,
  font: "inherit",
  color: BLUE,
  textDecoration: "underline",
  cursor: "pointer",
};

/**
 * The small line under the list explaining why a row can carry no date.
 *
 * It exists only because the marks were removed. Right-aligned and italic, as
 * the mockup has it — an aside about the page rather than a fact about a
 * question.
 */
export const statusFootnote: CSSProperties = {
  fontSize: 11.5,
  color: MUTED,
  textAlign: "right",
  fontStyle: "italic",
  margin: "10px 0 0",
};
