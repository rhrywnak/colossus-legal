// =============================================================================
// practiceReviewPageStyles.ts — the Review answers page (CC_TASK_REVIEW_PAGE_v1)
// =============================================================================
//
// Transcribed from the ruled mockup
// `PRACTICE_REVIEW_MOCKUP_v1_2026-09-19.html`, through `var(--practice-…)`
// tokens — no hex literal in a component or in this file (Rule 2). Its own file
// for the reason every sibling practice style file has one: each of them is
// near Rule 17's limit, and these belong to one page.
//
// ## Why this page is 960px and the drill is 860px
//
// The mockup's `.wrap` is `max-width: 960px`, and it is not an accident of
// drawing. The drill shows ONE question at a time at 26px in Georgia — a column
// that reads like a book. This shows forty-two questions with their answers and
// the notes on them, at 17px, and Chuck scrolls the whole deck in one pass. A
// narrower column would add rows to scroll without adding anything to read.
//
// ## Light only, deliberately
//
// The app has no dark theme (the task's own Law 3). Every value here resolves
// through a token whose one definition lives in `tokens.css`, so the day a
// theme arrives this file does not change.
//
// STRUCTURAL: geometry from one approved drawing — not settings. A margin is
// not something an operator tunes on the Settings page, and the reusability
// checkpoint does not apply to a transcription of a specific mockup.

import type { CSSProperties } from "react";

import { BLUE, INK, LINE, MUTED, PAPER, QUIET_BG } from "./practiceStyles";

/** The amber ground of owed work — the bar, and a note. See `tokens.css`. */
const AMBER_BG = "var(--practice-review-amber-bg)";
const AMBER_BORDER = "var(--practice-review-amber-border)";
const AMBER_INK = "var(--burden-warning-text)";

/** `.wrap` — the column the whole page lives in. */
export const page: CSSProperties = {
  maxWidth: 960,
  margin: "0 auto",
  padding: "8px 32px 48px",
  color: INK,
  fontFamily: '-apple-system, "Segoe UI", Roboto, Helvetica, Arial, sans-serif',
};

/** `.eyebrow` — REVIEW ANSWERS, over the title. */
export const eyebrow: CSSProperties = {
  fontSize: 11,
  letterSpacing: ".08em",
  color: MUTED,
  fontWeight: 700,
  marginTop: 18,
  // Upper-cased HERE and not in the store: the same row names the button in the
  // deck's row and the crumb above this page, and neither of those shouts.
  textTransform: "uppercase",
};

/** `h1` — `S-1 · Marie is obstructive and uncooperative`. */
export const title: CSSProperties = {
  fontSize: "1.4rem",
  fontWeight: 800,
  margin: "14px 0 2px",
};

/** `.qcard` — one question, its answer, its notes and the box. */
export const card: CSSProperties = {
  background: PAPER,
  border: `1px solid ${LINE}`,
  borderRadius: 10,
  padding: "18px 20px",
  marginBottom: 16,
};

/** `.pills` — the row above the question text. */
export const pills: CSSProperties = { marginBottom: 8 };

/** `.qtext` — the question, in Georgia at 17px. */
export const question: CSSProperties = {
  fontFamily: 'Georgia, "Times New Roman", serif',
  fontSize: 17,
  lineHeight: 1.4,
  margin: "2px 0 12px",
};

/** `.answer` — her words, behind the blue rail. */
export const answer: CSSProperties = {
  borderLeft: `3px solid ${BLUE}`,
  background: QUIET_BG,
  borderRadius: "0 8px 8px 0",
  padding: "10px 14px",
  fontSize: 14.5,
  lineHeight: 1.5,
  // Her words, exactly as typed — including the line breaks she typed.
  whiteSpace: "pre-wrap",
};

/** `.answer .meta` — `Answered 14 Sep · Marie`. */
export const answerMeta: CSSProperties = {
  fontSize: 12,
  color: MUTED,
  marginTop: 6,
  // The meta line is composed prose and must not inherit the answer's
  // pre-wrap: a stored template carries no line breaks to preserve.
  whiteSpace: "normal",
};

/**
 * `.unanswered` — the muted block where there is no answer.
 *
 * A GREY rail rather than the answer's blue, and italic: the difference must be
 * visible at scrolling speed, because it is the difference between a note that
 * lands on an answer and one that lands on a question.
 */
export const unanswered: CSSProperties = {
  borderLeft: `3px solid ${LINE}`,
  background: QUIET_BG,
  borderRadius: "0 8px 8px 0",
  padding: "10px 14px",
  fontSize: 13.5,
  color: MUTED,
  fontStyle: "italic",
};

/** `.addnote` — the box and its button, on one line. */
export const addNote: CSSProperties = {
  display: "flex",
  gap: 8,
  marginTop: 12,
  // The mockup draws one line. At a phone's width the button drops under the
  // box rather than squeezing the field to nothing.
  flexWrap: "wrap",
};

/** `.addnote input` */
export const noteInput: CSSProperties = {
  flex: "1 1 240px",
  minWidth: 0,
  boxSizing: "border-box",
  font: "inherit",
  fontSize: 13.5,
  border: "1px solid var(--practice-control-border)",
  borderRadius: 8,
  padding: "8px 12px",
};

/** `.btn-ghost` — Save note, and the Print link at the top right. */
export const buttonGhost: CSSProperties = {
  background: PAPER,
  color: BLUE,
  border: "1px solid var(--practice-control-border)",
  borderRadius: 8,
  padding: "7px 14px",
  fontSize: 13,
  fontWeight: 600,
  cursor: "pointer",
  font: "inherit",
  textDecoration: "none",
  // An anchor and a button must sit on the same baseline in the same row.
  display: "inline-block",
  lineHeight: 1.2,
};

/** The title line: the heading left, the Print link hard right. */
export const titleRow: CSSProperties = {
  display: "flex",
  alignItems: "flex-end",
  justifyContent: "space-between",
  gap: 16,
  flexWrap: "wrap",
};

/** `.foot` — the second Done reviewing, at the end of the page. */
export const foot: CSSProperties = {
  display: "flex",
  justifyContent: "flex-end",
  marginTop: 6,
};

/** The failure sentence, and the empty-deck sentence. */
export const notice: CSSProperties = {
  ...card,
  color: MUTED,
  fontSize: 14.5,
};

/** The amber trio, exported for the note list's amber tone. */
export const amber = {
  background: AMBER_BG,
  border: AMBER_BORDER,
  ink: AMBER_INK,
};
