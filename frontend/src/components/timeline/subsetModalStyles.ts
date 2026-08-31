// =============================================================================
// subsetModalStyles.ts — the Add/Edit subset modal and its picker
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v1_2026-08-30.html, Screen 3, approved as drawn. The
// sibling of `subsetStyles.ts`: that is the section on the page, this is the
// modal over it.
//
// CONST: geometry and rhythm, not settings. There is no frontend config surface
// for a grid template, and these are not per-deployment values — they are one
// approved drawing, transcribed. The mockup's own pixel values are kept in the
// comments beside them so the next reader can diff a screenshot against the
// drawing without opening the HTML.
//
// ## ⚑ THE MOCKUP'S PALETTE IS NOT THIS APP'S PALETTE
//
// The mockup is a standalone HTML file with its own literal hex colours
// (`#3730a3` indigo, `#b45309` amber). This app themes from CSS custom
// properties and supports a dark theme, so every colour below is the app token
// that plays the mockup's role — `--accent-primary` for the indigo bar,
// `--burden-warning-text` for the amber gap count. Transcribing the hexes would
// have reproduced the drawing and broken the theme, which is the one deviation
// the mockup cannot rule on because it has no dark mode to be wrong in.

import type { CSSProperties } from "react";



/**
 * Mockup `.mbox` geometry — the three numbers T6.3 names (defect D7).
 *
 * CONST and not config: an approved drawing, transcribed. `MODAL_TOP` is 48px
 * because that is one app-header's height, so the box opens clear of it; the
 * `MODAL_MARGIN` of 96 is that gap top AND bottom, which is what keeps the
 * footer — and therefore Save — on screen at 700px as well as at 900.
 */
export const MODAL_WIDTH = 860;
export const MODAL_TOP = 48;
export const MODAL_MARGIN = 96;

/** Over the scrim, and over the page's own sticky furniture. */
export const MODAL_Z_INDEX = 50;

/** Mockup `.scrim`. Fixed, full-viewport, and it does not scroll with anything. */
export const scrim: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "rgba(17, 24, 39, 0.42)",
  zIndex: MODAL_Z_INDEX,
};

/**
 * Mockup `.mbox`: 860 wide, capped at the viewport less 96px, its BODY scrolling.
 *
 * The height is `max-height` and never `height`: a three-event subset draws a
 * short box rather than a tall one with white space under the list, which is
 * what the mockup draws and what a dialog that can be dragged should do.
 */
export const box: CSSProperties = {
  width: "100%",
  maxHeight: `calc(100vh - ${MODAL_MARGIN}px)`,
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "12px",
  boxShadow: "0 20px 50px rgba(0, 0, 0, 0.35)",
  display: "flex",
  flexDirection: "column",
  overflow: "hidden",
  boxSizing: "border-box",
};

/**
 * Mockup `.mhead`: the title bar, and the DRAG HANDLE (T6.3).
 *
 * `cursor: move` and `userSelect: none` are what make it read as a handle and
 * stop a drag from selecting the title text as it goes.
 */
export const head: CSSProperties = {
  padding: "0.75rem 1.125rem",
  borderBottom: "1px solid var(--border-default)",
  display: "flex",
  alignItems: "center",
  gap: "0.75rem",
  background: "var(--bg-page)",
  cursor: "move",
  userSelect: "none",
};

/** Mockup `.mhead .grip`: the ⠿, muted, tight. Its NAME is a stored row. */
export const grip: CSSProperties = {
  color: "var(--text-muted)",
  fontSize: "1rem",
  letterSpacing: "-2px",
  cursor: "move",
};

export const headTitle: CSSProperties = {
  margin: 0,
  fontSize: "1.05rem",
  fontWeight: 700,
  color: "var(--text-primary)",
};

/** The picked-count pill. Mockup `.mhead .cnt`: indigo on pale indigo. */
export const pill: CSSProperties = {
  marginLeft: "auto",
  fontSize: "0.8rem",
  fontWeight: 700,
  color: "var(--accent-primary)",
  background: "var(--state-info-bg-soft)",
  // STANDING RULING (rejected three times across T4 and T5, 2026-08-31):
  // `--accent-primary` is an INK and a FILL, never a hairline. It stays the
  // pill's text and the soft indigo stays its ground; the outline is the pale
  // one every other bordered thing in this app wears.
  border: "1px solid var(--border-default)",
  borderRadius: "999px",
  padding: "0.15rem 0.75rem",
  whiteSpace: "nowrap",
};

/** Mockup `.mform`: `grid-template-columns:220px 1fr` — name beside description. */
export const form: CSSProperties = {
  padding: "0.75rem 1.25rem",
  borderBottom: "1px solid var(--border-default)",
  display: "grid",
  gridTemplateColumns: "220px 1fr",
  gap: "0.75rem",
};

export const label: CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--text-muted)",
  fontWeight: 600,
  display: "block",
  marginBottom: "0.2rem",
};

export const input: CSSProperties = {
  width: "100%",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  padding: "0.42rem 0.6rem",
  font: "inherit",
  fontSize: "0.82rem",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
  boxSizing: "border-box",
};

export const textarea: CSSProperties = { ...input, resize: "vertical", minHeight: "3.2rem" };

/** Mockup `.mbody`: the ONLY thing that scrolls; the page under it does not. */
export const body: CSSProperties = {
  flex: 1,
  overflowY: "auto",
  padding: "0.5rem 1.25rem 0.85rem",
};

export const hint: CSSProperties = {
  fontSize: "0.78rem",
  color: "var(--text-muted)",
  margin: "0.4rem 0 0.125rem",
};

/** The amber "gaps are not on the chronology" line. Mockup draws it in-list. */
export const gapHint: CSSProperties = {
  margin: "0.25rem 0 0 0.5rem",
  fontSize: "0.75rem",
  color: "var(--burden-warning-text)",
  fontWeight: 600,
};

/** Mockup `.mph`: the picker's own phase header, thinner than the page's. */
export function pickerPhaseHead(color: string): CSSProperties {
  return {
    display: "flex",
    alignItems: "baseline",
    gap: "0.6rem",
    borderLeft: `4px solid ${color}`,
    paddingLeft: "0.65rem",
    margin: "0.9rem 0 0.4rem",
  };
}

/** The picker phase header's count line. Mockup `.mph .pmeta`. */
export const phaseMeta: CSSProperties = {
  fontSize: "0.74rem",
  color: "var(--text-muted)",
};

export const pickerPhaseLabel: CSSProperties = {
  margin: 0,
  fontSize: "0.88rem",
  fontWeight: 700,
  color: "var(--text-primary)",
};

/** Mockup `.pk`: `22px 34px 104px 1fr 150px` — tick, order, date, title, note. */
export function pickRow(picked: boolean): CSSProperties {
  return {
    display: "grid",
    gridTemplateColumns: "22px 34px 104px 1fr 150px",
    gap: "0.5rem",
    alignItems: "start",
    // Both states wear the SAME pale hairline — see the standing ruling on the
    // pill above. What tells a picked row from an unpicked one is its ground,
    // its order number and its live note field, which is three signals where
    // the mockup's indigo outline was a fourth.
    border: "1px solid var(--border-default)",
    // Pale INDIGO, not amber. `--burden-warning-bg` was the first reading of
    // the mockup's `#f5f7ff` and it was wrong: a picked row is a chosen row, and
    // dressing it in the app's warning colour tells the author something is
    // wrong with every event they pick. `--state-info-bg-soft` (#e0e7ff) is the
    // token that plays the mockup's role.
    background: picked ? "var(--state-info-bg-soft)" : "var(--bg-surface)",
    borderRadius: "8px",
    padding: "0.5rem 0.625rem",
    margin: "0 0 0.375rem 0.5rem",
    fontSize: "0.8rem",
    boxSizing: "border-box",
  };
}

/** The story number. Mockup `.pk .ord`: indigo, bold, centred. */
export const order: CSSProperties = {
  fontWeight: 700,
  color: "var(--accent-primary)",
  fontSize: "0.72rem",
  textAlign: "center",
  paddingTop: "0.125rem",
};

export const orderControls: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  alignItems: "center",
  lineHeight: 1,
};

/** ▲ / ▼ beside the number — the reorder the mockup draws as a drag. */
export const orderButton: CSSProperties = {
  background: "none",
  border: "none",
  cursor: "pointer",
  color: "var(--accent-primary)",
  fontSize: "0.58rem",
  padding: 0,
  lineHeight: 1,
  fontFamily: "inherit",
};

/**
 * Mockup `.pk .d`: the date, and it is the STRONGEST text in the row.
 *
 * Weight 800 at 12.5px in full ink, against a title in normal weight. That is
 * the mockup's emphasis and it is the right one: this is a screen for putting
 * events in order, so the thing being ordered by is the thing to read first.
 * Before T6.2 it was muted 11.5px raw ISO, which read as metadata.
 *
 * `approximate` turns it amber — a claim about the DATE, not about the event.
 * The same token, for the same reason, as the floating window's `eventDate`.
 */
export function pickDate(approximate: boolean): CSSProperties {
  return {
    fontWeight: 800,
    fontSize: "0.78rem",
    whiteSpace: "nowrap",
    color: approximate ? "var(--burden-warning-text)" : "var(--text-primary)",
  };
}

/** Mockup `.pk .d i`: the precision or the ⚑, on its own line under the date. */
export const pickDateCaption: CSSProperties = {
  display: "block",
  fontStyle: "normal",
  fontWeight: 600,
  fontSize: "0.66rem",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

export const pickTitle: CSSProperties = { color: "var(--text-primary)" };

/** A picked event whose chronology row is gone (design R1). */
export const removedTitle: CSSProperties = {
  color: "var(--text-muted)",
  textDecoration: "line-through",
};

export const removedNote: CSSProperties = {
  display: "block",
  fontSize: "0.68rem",
  fontWeight: 600,
  color: "var(--burden-warning-text)",
  textDecoration: "none",
};

export const noteInput: CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  fontSize: "0.72rem",
  padding: "0.2rem 0.4rem",
  width: "100%",
  font: "inherit",
  fontFamily: "inherit",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
  boxSizing: "border-box",
};

/** Mockup `.mfoot`: the size line left, the buttons right. */
export const foot: CSSProperties = {
  borderTop: "1px solid var(--border-default)",
  background: "var(--bg-page)",
  padding: "0.75rem 1.25rem",
  display: "flex",
  gap: "0.625rem",
  alignItems: "center",
  fontSize: "0.8rem",
  flexWrap: "wrap",
};

/** Mockup `.mfoot .warn`: the 12–20 sentence, amber. */
export const sizeWarning: CSSProperties = {
  color: "var(--burden-warning-text)",
  fontWeight: 600,
};

export const footCount: CSSProperties = { color: "var(--text-muted)" };

export const footSpacer: CSSProperties = { marginLeft: "auto" };

// ─── the honest banner (T6.4, defect D2) ─────────────────────────────────────

/**
 * Mockup `.banner`: ONE box, red-bordered, holding both halves.
 *
 * ## ⚑ One banner and not two, and that is the point of the drawing
 *
 * Two stacked boxes — a green one and a red one — would read as two events that
 * happened to occur together. One box with two sentences in it reads as what it
 * is: a single save, partly landed. The green half leads because it is the
 * reassurance, and the red half is the instruction that follows it.
 *
 * The pale red border here is `--state-danger-border` rather than the strong
 * red: this is a hairline, and the app's hairlines are pale. The strong red is
 * the TEXT, where the contrast is actually load-bearing — the same split the
 * standing colour ruling makes about `--accent-primary`.
 */
export const banner: CSSProperties = {
  margin: "0 1.125rem 0.5rem",
  border: "1px solid var(--state-danger-border)",
  background: "var(--state-danger-bg-soft)",
  color: "var(--state-danger-strong)",
  borderRadius: "8px",
  padding: "0.5rem 0.75rem",
  fontSize: "0.8rem",
  lineHeight: 1.45,
};

/** Mockup `.banner .ok`: the green half, in front of the red one, on one line. */
export const bannerSaved: CSSProperties = {
  color: "var(--state-success-strong)",
  fontWeight: 700,
  marginRight: "0.3rem",
};
