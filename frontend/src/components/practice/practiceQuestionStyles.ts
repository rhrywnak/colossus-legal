// practiceQuestionStyles.ts — the question page (mockup v7, views 2–4).
//
// Its own module, beside `practiceCritiqueStyles`, for the reason every style
// module here has one: a block belongs beside the surface that reads it. Every
// colour is a `--practice-*` token; the palette test enforces that per file and
// this file is on its list.

import type { CSSProperties } from "react";

/** The question, at the top. Serif and large — it is the thing being answered. */
export const question: CSSProperties = {
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: 19,
  lineHeight: 1.4,
  margin: "0 0 8px",
  color: "var(--practice-ink)",
};

/** Where it was built from. */
export const from: CSSProperties = {
  color: "var(--practice-muted)",
  fontSize: 12,
  margin: "0 0 16px",
};

export const label: CSSProperties = {
  fontSize: 10.5,
  letterSpacing: ".1em",
  textTransform: "uppercase",
  color: "var(--practice-muted)",
  fontWeight: 700,
  margin: "16px 0 7px",
};

/** Her answer, editable. */
export const box: CSSProperties = {
  width: "100%",
  minHeight: 92,
  border: "1px solid var(--practice-control-border)",
  borderRadius: 6,
  padding: "11px 13px",
  font: "16px/1.5 Georgia, serif",
  color: "var(--practice-ink)",
  background: "var(--practice-paper)",
};

/**
 * The same box while the read runs.
 *
 * Locked rather than disabled: `readOnly` keeps it selectable and readable, so
 * she can still SEE what she wrote while waiting. A `disabled` textarea greys
 * its own text, which would hide her answer at the moment she is being judged
 * on it.
 */
export const boxLocked: CSSProperties = {
  background: "var(--practice-pale)",
  color: "var(--practice-muted)",
};

/** `▸ 2 earlier versions` — one quiet line she never has to open. */
export const quiet: CSSProperties = {
  background: "none",
  border: 0,
  padding: 0,
  font: "inherit",
  fontSize: 12.5,
  color: "var(--practice-blue)",
  cursor: "pointer",
  display: "inline-block",
  margin: "9px 0 0",
};

/** The earlier versions, opened. NOT editable — see the page's header. */
export const earlier: CSSProperties = {
  margin: "9px 0 0",
  borderLeft: "2px solid var(--practice-line)",
  paddingLeft: 12,
};

export const earlierRow: CSSProperties = { margin: "0 0 10px" };

export const earlierWhen: CSSProperties = {
  fontSize: 10.5,
  letterSpacing: ".04em",
  textTransform: "uppercase",
  color: "var(--practice-muted)",
  margin: "0 0 3px",
};

/**
 * An earlier version's words.
 *
 * Rendered as TEXT, never in a control. There is no textarea here and no edit
 * path to one: Chuck's reading of an older version points at the words he read,
 * and a box would invite her to change words somebody has already acted on.
 */
export const earlierText: CSSProperties = {
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: 15,
  lineHeight: 1.45,
  color: "var(--practice-muted)",
  margin: 0,
};

export const buttons: CSSProperties = {
  margin: "18px 0 0",
  display: "flex",
  gap: 9,
  alignItems: "center",
  flexWrap: "wrap",
};

/** The primary button while the read runs — relabelled and disabled. */
export const buttonWorking: CSSProperties = {
  background: "var(--practice-control-border)",
  cursor: "default",
};

export const back: CSSProperties = {
  color: "var(--practice-blue)",
  textDecoration: "none",
  fontSize: 14,
};
