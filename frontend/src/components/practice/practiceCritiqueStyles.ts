// practiceCritiqueStyles.ts — the block under the answer box.
//
// Its own module rather than more exports in `practiceStyles`: that file serves
// the whole practice surface and is already the largest of them, and a reader
// looking for why the working state shimmers should not walk past the deck row's
// styles to find it.
//
// Every colour is a `--practice-*` token declared in `styles/tokens.css`; the
// palette test enforces that per file, and this file is on its list.

import type { CSSProperties } from "react";

/** The block itself. One rail colour per state — red, green, or waiting. */
const base: CSSProperties = {
  borderRadius: 8,
  padding: "14px 16px",
  margin: "16px 0 0",
  borderLeftWidth: 4,
  borderLeftStyle: "solid",
};

/** A read that named a fault. */
export const fault: CSSProperties = {
  ...base,
  background: "var(--practice-read-bad-bg)",
  borderLeftColor: "var(--practice-red)",
};

/** A read that found nothing wrong. */
export const fine: CSSProperties = {
  ...base,
  background: "var(--practice-read-ok-bg)",
  borderLeftColor: "var(--practice-green)",
};

/**
 * The block WHILE the read is running.
 *
 * It exists from the moment she presses Answer — Roman's defect #1 of
 * 2026-08-20 was that nothing appeared until the read came back, so the page
 * looked inert while it worked. A state not drawn is a state not built.
 */
export const working: CSSProperties = {
  ...base,
  background: "var(--practice-pale)",
  borderLeftColor: "var(--practice-control-border)",
};

/** The call — the one line naming what happened. */
export const call: CSSProperties = { fontWeight: 700, fontSize: 15.5, margin: "0 0 9px" };

/** A labelled part: WHY, WHAT TO DO INSTEAD. */
export const partLabel: CSSProperties = {
  fontSize: 10.5,
  letterSpacing: ".09em",
  textTransform: "uppercase",
  color: "var(--practice-muted)",
  fontWeight: 700,
  display: "block",
  margin: "0 0 3px",
};

export const part: CSSProperties = { margin: "0 0 9px" };

/** A citation key, inline in the prose. */
export const cite: CSSProperties = {
  background: "var(--practice-paper)",
  border: "1px solid var(--practice-pill-border)",
  borderRadius: 4,
  padding: "0 5px",
  fontSize: 12.5,
  color: "var(--practice-blue)",
  fontWeight: 600,
  whiteSpace: "nowrap",
};

/** The footnote list under the parts. */
export const sources: CSSProperties = {
  margin: "10px 0 0",
  paddingTop: 10,
  borderTop: "1px dashed var(--practice-line)",
};

export const sourceRow: CSSProperties = {
  display: "flex",
  gap: 8,
  fontSize: 12.5,
  color: "var(--practice-muted)",
  margin: "0 0 5px",
};

export const sourceKey: CSSProperties = {
  color: "var(--practice-blue)",
  fontWeight: 700,
  flex: "none",
  minWidth: 28,
};

/** A cited key whose source is missing — shown, never hidden. */
export const sourceMissing: CSSProperties = { fontStyle: "italic", color: "var(--practice-red)" };

/** The foot of the block: who has not reviewed this, and the way to flag it. */
export const foot: CSSProperties = {
  fontSize: 11.5,
  color: "var(--practice-muted)",
  margin: "11px 0 0",
  borderTop: "1px solid var(--practice-line)",
  paddingTop: 8,
};

/** The line under a one-sentence critique, pointing at the fuller read. */
export const plainHint: CSSProperties = { ...foot, fontStyle: "italic" };

/** One shimmering bar, standing in for a line of text that has not arrived. */
export const shimmer: CSSProperties = {
  height: 11,
  borderRadius: 5,
  margin: "0 0 8px",
  background: "var(--practice-shimmer)",
};

/** The spinner beside "Reading your answer". */
export const spinner: CSSProperties = {
  width: 15,
  height: 15,
  border: "2px solid var(--practice-pill-border)",
  borderTopColor: "var(--practice-blue)",
  borderRadius: "50%",
  display: "inline-block",
};

/** The CSS a style object cannot carry: the two animations. */
export const CRITIQUE_CSS = `
@keyframes practice-shimmer { 0% { background-position: 100% 50% } 100% { background-position: 0 50% } }
@keyframes practice-spin { to { transform: rotate(360deg) } }
[data-critique-shimmer] { background-size: 400% 100%; animation: practice-shimmer 1.4s ease infinite; }
[data-critique-spinner] { animation: practice-spin .8s linear infinite; }
@media (prefers-reduced-motion: reduce) {
  /* The shimmer and the spin are decoration; the WORDS beside them carry the
     state. A person who has asked for less motion still learns the read is
     running, from the button, the locked box and the line under the bars. */
  [data-critique-shimmer], [data-critique-spinner] { animation: none; }
}
`;
