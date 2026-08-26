// =============================================================================
// timelineWriteStyles.ts — TIMELINE_MOCKUP_v2 Screen 3, as typed style objects
// =============================================================================
//
// The form, the picker, the muted card controls and the undo line. A sibling of
// `timelineStyles.ts` rather than a growth of it: that file is already at 307
// non-comment lines against Rule 17's 300, and the seam is honest — everything
// there draws what the chronology SAYS, everything here draws what somebody does
// to it.
//
// ## Where the mockup's hex values went
//
// Nowhere, same as the sibling. Design R5 keeps the CURRENT visual language, so
// these read the product's own tokens (`--text-primary`, `--border-default`,
// `--bg-surface`, `--accent-primary`, `--burden-warning-text`). The only colours
// that are the drawing's are the ones that come from the DATABASE — a tag chip
// in the form wears the tag's own stored colour, exactly as the filter bar's
// does, which is what makes a recolour an UPDATE rather than a build.
//
// ## Why the delete control is not red-on-white
//
// R17: "Edit/Delete are always visible, muted. Small, gray, never competing with
// content." The mockup draws a `danger` button on the event page's action bar
// and muted text links on the cards; both are reproduced, and neither shouts.

import type { CSSProperties } from "react";

// ─── the form (mockup Screen 3) ──────────────────────────────────────────────

/** The form panel, sitting in place above the list or the event's detail. */
export const form: CSSProperties = {
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "10px",
  padding: "1.1rem 1.25rem",
  margin: "0 0 1.5rem",
};

export const formTitle: CSSProperties = {
  margin: "0 0 0.75rem",
  fontSize: "1.05rem",
  fontWeight: 700,
  color: "var(--text-primary)",
};

/** A row of fields that sit side by side — date · precision · approximate. */
export const inline: CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: "0.9rem",
  alignItems: "flex-end",
};

/** One labelled field, stacked. */
export const field: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "0.25rem",
  margin: "0 0 0.75rem",
  flex: "1 1 auto",
  minWidth: 0,
};

/** The same, capped — the mockup's date and precision fields are narrow. */
export function narrowField(maxWidth: string): CSSProperties {
  return { ...field, maxWidth, flex: `0 1 ${maxWidth}` };
}

export const label: CSSProperties = {
  fontSize: "0.76rem",
  fontWeight: 600,
  color: "var(--text-secondary)",
};

export const input: CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  padding: "0.45rem 0.6rem",
  fontSize: "0.85rem",
  fontFamily: "inherit",
  color: "var(--text-primary)",
  background: "var(--bg-surface)",
  // The app sets no global box-sizing; without this a 100%-wide input overflows
  // its own column by its padding and border. Same reason the card carries it.
  boxSizing: "border-box",
  width: "100%",
};

/** The fact box. Serif, because the fact reads as prose everywhere else. */
export const textarea: CSSProperties = {
  ...input,
  fontFamily: "Georgia, 'Times New Roman', serif",
  lineHeight: 1.5,
  resize: "vertical",
};

/** The approximate checkbox and its words, on one line. */
export const checkRow: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.4rem",
  fontSize: "0.8rem",
  color: "var(--text-secondary)",
  margin: "0 0 0.95rem",
};

/** The form's tag picker — the same chips the filter bar draws. */
export const tagPicker: CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: "0.4rem",
};

/** The Save / Cancel bar. */
export const actionBar: CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  alignItems: "center",
  marginTop: "0.9rem",
};

// ─── buttons ─────────────────────────────────────────────────────────────────

const buttonBase: CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  padding: "0.42rem 0.85rem",
  fontSize: "0.81rem",
  fontWeight: 600,
  fontFamily: "inherit",
  cursor: "pointer",
  background: "var(--bg-surface)",
  color: "var(--text-secondary)",
};

/** A plain control — Cancel, Add a note's Add. */
export const button: CSSProperties = { ...buttonBase };

/**
 * The committing control.
 *
 * `disabled` is a STYLE as well as an attribute: a greyed Save that is still
 * clickable, or a live-looking Save that does nothing, are both the dead button
 * Standing Rule 1 forbids. Both are set from the same boolean.
 */
export function primaryButton(disabled: boolean): CSSProperties {
  return {
    ...buttonBase,
    background: disabled ? "var(--bg-subtle, var(--bg-surface))" : "var(--accent-primary)",
    borderColor: disabled ? "var(--border-default)" : "var(--accent-primary)",
    color: disabled ? "var(--text-disabled)" : "var(--bg-surface)",
    cursor: disabled ? "not-allowed" : "pointer",
  };
}

/** The event page's Delete. Muted per R17 — a border, not a block of red. */
export const dangerButton: CSSProperties = {
  ...buttonBase,
  color: "var(--burden-warning-text)",
  borderColor: "var(--burden-warning-text)",
};

/**
 * The always-visible ✎ / 🗑 on an event card (R17).
 *
 * Hover-only controls are a named anti-pattern and CaseFleet's rows carry a
 * visible pencil, so these are drawn at all times — small, grey, and out of the
 * way of the words.
 */
export const cardAction: CSSProperties = {
  border: "none",
  background: "none",
  padding: "0 0.35rem",
  fontSize: "0.74rem",
  fontFamily: "inherit",
  color: "var(--text-muted)",
  cursor: "pointer",
};

/** Where the card's controls sit: the right end of its meta row. */
export const cardActions: CSSProperties = {
  display: "flex",
  gap: "0.15rem",
  marginLeft: "auto",
};

// ─── the undo line (design R10) ──────────────────────────────────────────────

/**
 * What replaces a deleted card, IN PLACE, until the reader navigates away.
 *
 * ⚑ This line is the whole safety. There is no confirm dialog anywhere in the
 * chronology, by ruling — the delete happens, and this is how it is taken back.
 * It is drawn where the card was rather than as a toast in a corner, so it
 * cannot be missed by somebody looking at the row they just deleted.
 */
export const undoLine: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  background: "var(--bg-surface)",
  border: "1px dashed var(--border-default)",
  borderRadius: "10px",
  padding: "0.8rem 1rem",
  margin: "0 0 0.625rem 0.75rem",
  fontSize: "0.82rem",
  color: "var(--text-muted)",
  boxSizing: "border-box",
};

/** The Undo control inside that line. */
export const undoAction: CSSProperties = {
  border: "none",
  background: "none",
  padding: 0,
  fontSize: "0.82rem",
  fontWeight: 700,
  fontFamily: "inherit",
  color: "var(--accent-primary)",
  cursor: "pointer",
  textDecoration: "underline",
};

// ─── the document picker (design R9) ─────────────────────────────────────────

export const picker: CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  padding: "0.6rem 0.7rem",
  background: "var(--bg-surface)",
};

/** One offered document, as a full-width pick target. */
export const pickerRow: CSSProperties = {
  display: "block",
  width: "100%",
  textAlign: "left",
  border: "none",
  background: "none",
  padding: "0.3rem 0.2rem",
  fontSize: "0.82rem",
  fontFamily: "inherit",
  color: "var(--accent-primary)",
  cursor: "pointer",
};

/** The list of offers, capped in height so the form does not walk off-screen. */
export const pickerResults: CSSProperties = {
  maxHeight: "11rem",
  overflowY: "auto",
  marginTop: "0.4rem",
};

/**
 * The line that says the short list was capped.
 *
 * ⚑ Not decoration. A truncated list that looked complete is how somebody links
 * the wrong document with no idea a better match was cut off.
 */
export const pickerCapped: CSSProperties = {
  fontSize: "0.75rem",
  color: "var(--burden-warning-text)",
  marginTop: "0.35rem",
};

/** One link the form has picked but not yet saved. */
export const pickedLink: CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  gap: "0.4rem",
  fontSize: "0.8rem",
  color: "var(--text-secondary)",
  margin: "0.4rem 0",
};

// ─── failure, which is never silent ──────────────────────────────────────────

/**
 * Where a failed write's sentence lands.
 *
 * Every write path renders this or nothing happened. §C3: "a failed write
 * reaches a rendered sentence (never silent, never a dead button)."
 */
export const writeError: CSSProperties = {
  border: "1px solid var(--burden-warning-text)",
  borderRadius: "6px",
  background: "var(--bg-surface)",
  color: "var(--burden-warning-text)",
  padding: "0.5rem 0.7rem",
  fontSize: "0.8rem",
  margin: "0.6rem 0",
};

// ─── the event page's write rows ─────────────────────────────────────────────

/** The Add-a-note box and its button. */
export const addNoteRow: CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  marginTop: "0.6rem",
};

/** The small control that retires one note or one link. */
export const rowAction: CSSProperties = {
  ...cardAction,
  marginLeft: "0.5rem",
};
