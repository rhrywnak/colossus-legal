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

import { BLUE, INK, MUTED, PALE } from "./practiceStyles";

/**
 * The slim bar under the title: amber count left, Done reviewing right.
 *
 * ## Why it grew a GROUND on 2026-09-19 (ruling STOP-A, option 2)
 *
 * It was amber text inside a plain outlined box. The review-page mockup draws
 * it on the amber it means, and Roman ruled the change into the ONE component
 * rather than into a variant — so the deck page's bar moved with it, which is
 * correct: they are the same bar saying the same thing, and two grounds for one
 * sentence is how a person stops reading it as one sentence.
 */
export const reviewBar: CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  justifyContent: "space-between",
  gap: "8px 16px",
  background: "var(--practice-review-amber-bg)",
  border: "1px solid var(--practice-review-amber-border)",
  borderRadius: 8,
  padding: "10px 16px",
  margin: "16px 0 22px",
};

/** "{n} new since you last reviewed" — the amber of owed work. */
export const reviewCount: CSSProperties = {
  color: "var(--burden-warning-text)",
  fontWeight: 700,
  // 14, from the review mockup: the sentence grew an oldest-waiting clause on
  // 2026-09-19 and at 16 it wrapped on a narrow deck page before the button.
  fontSize: 14,
};

/**
 * The board-4 mark on a question's row: "Chuck left a note".
 *
 * ## Drawn to the board, not to the amber family (CC_TASK_FOR_YOU_POLISH_v1)
 *
 * It shipped as an amber chip under the question text. FOR_YOU_MOCKUP_v1 board
 * 4 draws it as bold rust WORDS in the row's right-hand column, over a tinted
 * row — and the difference matters twice over. The column is where a reader
 * running down a deck of seventeen questions is already looking for the row's
 * status, and the colour separates one unread thing on one question from the
 * review bar's amber, which means a reviewer owes work on the whole deck. Two
 * ambers a row apart would read as one fact.
 *
 * A tag and not a chip: the row is already tinted, and a bordered pill on a
 * tinted row is the same fact drawn three times.
 */
export const waitingMark: CSSProperties = {
  color: "var(--practice-waiting-ink)",
  fontSize: 12,
  fontWeight: 700,
  whiteSpace: "nowrap",
};

/**
 * The row the mark sits on — board 4's `background: var(--soft)`.
 *
 * The mark alone is four words at the end of a long row; the tint is what makes
 * the row findable from the top of a deck without reading any of it. It is the
 * same argument the unread row on the For you page makes, and it is the same
 * pairing: a mark for what, a ground for where.
 */
export const waitingRow: CSSProperties = {
  background: "var(--practice-waiting-row-bg)",
};

/**
 * A reply, drawn under the note it answers.
 *
 * Indented and rail-less: the pair is ONE exchange, and a second full frame
 * beside the first reads as two unrelated notes — which is exactly the thing
 * the reply column exists to stop.
 */
export const replyFrame: CSSProperties = {
  marginLeft: 20,
  paddingLeft: 12,
  borderLeft: "2px solid var(--practice-review-amber-border)",
};

/** The failure sentence under the bar. */
export const reviewError: CSSProperties = { color: "var(--practice-red)", fontSize: 14, width: "100%" };

/**
 * The confirmation, inside the bar and under its sentence.
 *
 * ## Why it sits INSIDE the amber bar and not over the page
 *
 * `ScanConfirmBar` draws its own box because it interrupts a header it does not
 * belong to. This question belongs to the bar that asked it: the count it names
 * is the sentence directly above, and lifting it into a floating panel would
 * separate the number from the question about the number. `width: "100%"` puts
 * it on its own row of the bar's wrap, which is also where `reviewError`
 * already goes.
 *
 * It is not a modal. Nothing is trapped and Cancel is not the only way out,
 * because the deck behind it is where somebody checks what they are about to
 * sweep before answering.
 */
export const reviewConfirm: CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  gap: "8px 12px",
  width: "100%",
  paddingTop: 10,
  marginTop: 2,
  borderTop: "1px solid var(--practice-review-amber-border)",
};

/** The question. Ink, not amber: it is a sentence to read, not a warning. */
export const reviewConfirmSentence: CSSProperties = {
  color: INK,
  fontSize: 14,
  flex: 1,
  minWidth: 220,
};

/**
 * The affirmative. Amber FILL — the one control that moves the shared mark.
 *
 * The house pair, same as `ScanConfirmBar`'s accent fill: the colour that means
 * owed work, and the on-fill foreground that goes with it. The retreat below is
 * the same SHAPE with no fill, deliberately — it must be as easy to hit, not as
 * loud.
 */
export const reviewConfirmYes: CSSProperties = {
  fontFamily: "inherit",
  fontSize: 13,
  fontWeight: 600,
  padding: "6px 16px",
  borderRadius: 999,
  border: "1px solid var(--burden-warning-text)",
  background: "var(--burden-warning-text)",
  color: "var(--v3-on-fill)",
  cursor: "pointer",
  flexShrink: 0,
};

/** The retreat. Same shape, no fill. */
export const reviewConfirmCancel: CSSProperties = {
  fontFamily: "inherit",
  fontSize: 13,
  padding: "6px 16px",
  borderRadius: 999,
  border: "1px solid var(--practice-review-amber-border)",
  background: "transparent",
  color: MUTED,
  cursor: "pointer",
  flexShrink: 0,
};

/**
 * The two grounds a note is drawn on.
 *
 * ## Why a note has two looks and not one
 *
 * Under Marie's deck row a note is one thing among many on a page about
 * QUESTIONS, and the blue rail files it beside the answer it belongs to. On the
 * Review answers page it is the only thing Chuck is there to write, and the
 * ruled mockup draws it on the same amber as the review bar above it — the
 * ground this build already uses for owed work. Same component, same behaviour,
 * same strings; a ground chosen by the surface.
 */
export type NoteTone = "blue" | "amber";

/** One note: the blue left border, on the pale ground. */
export const note: CSSProperties = {
  borderLeft: `4px solid ${BLUE}`,
  background: PALE,
  padding: "8px 12px",
  margin: "8px 0 0",
  borderRadius: "0 6px 6px 0",
};

/** The amber note of the Review answers page — the mockup's `.note`. */
export const noteAmber: CSSProperties = {
  background: "var(--practice-review-amber-bg)",
  border: "1px solid var(--practice-review-amber-border)",
  borderRadius: 8,
  padding: "8px 12px",
  margin: "10px 0 0",
  fontSize: 13.5,
};

/** One note's frame, for the tone the surface asked for. */
export function noteFrame(tone: NoteTone): CSSProperties {
  return tone === "amber" ? noteAmber : note;
}

/** "Chuck · 17 Sep" — the author in blue, bold. */
export const noteAuthor: CSSProperties = { color: BLUE, fontWeight: 700, fontSize: 14 };

/** The same line on the amber ground: the mockup's `.note .who`. */
export const noteAuthorAmber: CSSProperties = {
  color: "var(--burden-warning-text)",
  fontWeight: 700,
  fontSize: 12,
};

/** The author line, for the tone the surface asked for. */
export function noteAuthorFor(tone: NoteTone): CSSProperties {
  return tone === "amber" ? noteAuthorAmber : noteAuthor;
}

/**
 * The note's words. Struck: through, and muted.
 *
 * `tone` DEFAULTS to the blue surface, so the two call sites that predate the
 * Review answers page read exactly as they did — a default parameter rather
 * than a second function, because the only difference is a font size.
 */
export function noteText(struck: boolean, tone: NoteTone = "blue"): CSSProperties {
  return {
    color: struck ? MUTED : INK,
    textDecoration: struck ? "line-through" : "none",
    margin: "2px 0 0",
    fontSize: tone === "amber" ? 13.5 : 15,
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
