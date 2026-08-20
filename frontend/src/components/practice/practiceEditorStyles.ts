// =============================================================================
// practiceEditorStyles.ts — mockup v4's `/* v4 additions */` block
// =============================================================================
//
// The status line's three colours, the question link's dotted underline, the
// badges, the editor's arrows and inline fields, the add form, the notes panel,
// the receipt chips, and the review page's attempt cards.
//
// Transcribed by the same rule as its three siblings: the mockup's own values,
// through `var(--practice-…)`, no hex literal in a component.
//
// ## Why a fourth style file
//
// The seam is by SCREEN REGION, as it is in the others, and this is one region:
// everything Part B drew. `practiceStyles` is v2, `practiceDeckStyles` is v3's
// question list, `practiceFlowStyles` is v3's top bar plus v1's picker, and this
// is v4.
//
// ## The `font` shorthand, once more
//
// Every rule below that sets a size sets `font: "inherit"` FIRST. `font` is a
// shorthand that resets `font-size`, and React writes a style object's
// properties in declaration order — the other way round renders the control at
// the body's 18px, which is what shipped in .401.

import type { CSSProperties } from "react";

import { BLUE, GREEN, INK, LINE, MUTED, PAPER, RED } from "./practiceStyles";

/** `.status` — the line under an answered row. */
export const status: CSSProperties = { fontSize: 13, marginTop: 4 };

/**
 * `.status.fine` / `.status.repeat` / `.status.skipped` — the three colours.
 *
 * Keyed by the RAW stored mark rather than by the rendered word: the word is a
 * Settings row and matching on it would drop the colour the first time somebody
 * edited it.
 */
export const statusColour: Record<string, CSSProperties> = {
  fine: { color: GREEN },
  repeat: { color: RED },
  skipped: { color: MUTED },
};

/** `.qrow .qt a` — the question text as the link that opens it alone. */
export const questionLink: CSSProperties = {
  font: "inherit",
  color: INK,
  background: "none",
  border: "none",
  borderBottom: "1px dotted var(--practice-link-underline)",
  padding: 0,
  margin: 0,
  textAlign: "left",
  textDecoration: "none",
  cursor: "pointer",
};

/** `.status a` — the `review` link at the end of a status line. */
export const reviewLink: CSSProperties = {
  font: "inherit",
  fontSize: 13,
  color: BLUE,
  background: "none",
  border: "none",
  padding: 0,
  cursor: "pointer",
};

/** `.badge` — the small solid tag beside a row's pill. */
export const badge: CSSProperties = {
  display: "inline-block",
  fontSize: 11,
  padding: "1px 6px",
  borderRadius: 4,
  background: BLUE,
  color: "var(--practice-on-fill)",
  marginLeft: 8,
  verticalAlign: "middle",
};

/** `.badge.draft` — olive, so it does not read as "new". */
export const badgeDraft: CSSProperties = {
  ...badge,
  background: "var(--practice-draft-badge)",
};

/** `.badge.redirect` — Chuck's green. */
export const badgeRedirect: CSSProperties = {
  ...badge,
  background: "var(--practice-chuck-text)",
};

/** A hidden row's grey badge. */
export const badgeHidden: CSSProperties = {
  ...badge,
  background: "var(--practice-tag-bg)",
};

/** `.changed` — the blue box above the deck. */
export const changed: CSSProperties = {
  background: "var(--practice-resume-bg)",
  border: "1px solid var(--practice-resume-border)",
  borderRadius: 8,
  padding: "10px 14px",
  marginTop: 18,
  fontSize: 15,
};

/** `.changed summary` — the fold that opens the list. */
export const changedSummary: CSSProperties = {
  font: "inherit",
  fontSize: 14,
  color: BLUE,
  cursor: "pointer",
};

/** `.changed li` */
export const changedItem: CSSProperties = { fontSize: 14, margin: "3px 0" };

/** `.editbar` — the deck header's right-hand controls. */
export const editBar: CSSProperties = {
  display: "flex",
  gap: 10,
  alignItems: "center",
  fontSize: 14,
};

/**
 * Every control edit mode locks — Start, the pills, the side cards, the fold,
 * Resume, Start over.
 *
 * ## Why the cursor and not only the colour
 *
 * A control that is dimmer but still shows a pointer reads as "clickable, just
 * quiet". `not-allowed` is the one cue a person gets from the mouse BEFORE the
 * click, and `title` (the store's `editor_busy_hint`) is what they get on the
 * hover after it. `disabled` on the element is what actually refuses.
 *
 * The `font` shorthand is not used here: this object is SPREAD over another
 * control's style, and `font: "inherit"` would reset the size that control had
 * already set — the .401 defect, in the one place it could still happen.
 */
export const lockedControl: CSSProperties = {
  opacity: 0.4,
  cursor: "not-allowed",
};

/** `.switch` — "Edit the deck" / "Done editing". */
export const editSwitch: CSSProperties = {
  font: "inherit",
  fontSize: 14,
  color: BLUE,
  background: "none",
  border: "none",
  padding: 0,
  cursor: "pointer",
};

/** `.arrows button` — ▲ and ▼, stacked. */
export const arrowButton: CSSProperties = {
  font: "inherit",
  fontSize: 12,
  padding: "2px 6px",
  display: "block",
  margin: "2px 0",
  borderRadius: 6,
  border: "1px solid var(--practice-control-border)",
  background: PAPER,
  color: INK,
  cursor: "pointer",
};

/** `.edrow` — the inline field stack on a row being edited. */
export const editRow: CSSProperties = { marginTop: 8, display: "grid", gap: 6 };

/** `.edrow label` */
export const editLabel: CSSProperties = {
  fontSize: 12,
  color: MUTED,
  textTransform: "uppercase",
  letterSpacing: ".05em",
};

/** `.edrow input`, `.edrow select` */
export const editInput: CSSProperties = {
  font: "inherit",
  fontSize: 15,
  padding: "6px 8px",
  border: "1px solid var(--practice-control-border)",
  borderRadius: 6,
  width: "100%",
};

/** `.edrow textarea` */
export const editTextarea: CSSProperties = { ...editInput, minHeight: 54 };

/** `.hiddenrow .qt` — a hidden question, greyed for the editor only. */
export const hiddenRow: CSSProperties = { opacity: 0.35 };

/** `.addq` — the add form's dashed box. */
export const addBox: CSSProperties = {
  border: "1px dashed var(--practice-separator)",
  borderRadius: 8,
  padding: "12px 14px",
  marginTop: 10,
};

/** `.addq`'s three side-by-side pickers. */
export const addGrid: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "1fr 1fr 1fr",
  gap: 6,
};

/** `.notepanel` — the collapsible panel at the foot of a card. */
export const notePanel: CSSProperties = {
  marginTop: 22,
  borderTop: "1px dashed var(--practice-line)",
  paddingTop: 12,
};

/** The panel's `Notes (n)` header, which is also the fold. */
export const noteToggle: CSSProperties = {
  font: "inherit",
  fontSize: 17,
  fontWeight: 600,
  color: INK,
  background: "none",
  border: "none",
  padding: 0,
  cursor: "pointer",
};

/** `.note` — one note: who on the left, what on the right. */
export const note: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "90px 1fr",
  gap: 10,
  padding: "8px 0",
  borderBottom: `1px solid ${LINE}`,
  fontSize: 15,
  // A grid track's default `min-width: auto` refuses to shrink below its
  // longest unbreakable word, so one pasted URL in a note would push the panel
  // sideways and take the page's horizontal scrollbar with it. `anywhere` lets
  // the break happen inside the word; `minWidth: 0` lets the track allow it.
  minWidth: 0,
  overflowWrap: "anywhere",
};

/** `.note .who` */
export const noteWho: CSSProperties = { fontSize: 13, color: MUTED };

/** `.note .who b` */
export const noteAuthor: CSSProperties = { display: "block", color: INK };

/** `.note.struck .txt` — struck through, and still readable. */
export const noteStruck: CSSProperties = {
  textDecoration: "line-through",
  color: MUTED,
};

/** `.noteadd` — the author picker, the input and Save. */
export const noteAdd: CSSProperties = {
  display: "flex",
  gap: 8,
  marginTop: 8,
  flexWrap: "wrap",
};

/** `.noteadd input` */
export const noteInput: CSSProperties = {
  font: "inherit",
  fontSize: 14,
  padding: "6px 8px",
  border: "1px solid var(--practice-control-border)",
  borderRadius: 6,
  flex: "1 1 320px",
};

/** `.noteadd select`, and the small buttons beside it. */
export const noteControl: CSSProperties = {
  font: "inherit",
  fontSize: 14,
  padding: "6px 8px",
  border: "1px solid var(--practice-control-border)",
  borderRadius: 6,
  background: PAPER,
  color: INK,
  cursor: "pointer",
};

/** `.pointto` — the row of receipt chips under the answer box. */
export const pointTo: CSSProperties = { marginTop: 8, fontSize: 14 };

/** `.pointto .chip` */
export const chip: CSSProperties = {
  display: "inline-block",
  font: "inherit",
  fontSize: 13,
  padding: "3px 9px",
  borderRadius: 999,
  border: "1px solid var(--practice-control-border)",
  margin: "3px 4px 0 0",
  cursor: "pointer",
  background: PAPER,
  color: INK,
};

/** `.pointto .chip.on` — picked. */
export const chipOn: CSSProperties = {
  ...chip,
  background: "var(--practice-choice-selected-bg)",
  borderColor: BLUE,
  color: "var(--practice-navy)",
};

/** `.attempt` — one attempt's card on the review page. */
export const attempt: CSSProperties = {
  border: `1px solid ${LINE}`,
  borderRadius: 8,
  padding: "12px 14px",
  marginTop: 12,
};

/** `.attempt .ah` — its heading row. */
export const attemptHead: CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  fontSize: 14,
  color: MUTED,
};

/** `.attempt .ah b` */
export const attemptNumber: CSSProperties = { color: INK, fontWeight: 600 };
