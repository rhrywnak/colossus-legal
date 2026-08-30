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



/** Mockup `.modal`: a scrim over the page, the box near the top. */
export const scrim: CSSProperties = {
  position: "fixed",
  inset: 0,
  background: "rgba(17, 24, 39, 0.35)",
  display: "flex",
  alignItems: "flex-start",
  justifyContent: "center",
  paddingTop: "2.5rem",
  zIndex: 50,
};

/** Mockup `.mbox`: 820px wide, 620px tall at most, its BODY scrolling. */
export const box: CSSProperties = {
  width: "820px",
  maxWidth: "calc(100vw - 2rem)",
  maxHeight: "min(620px, calc(100vh - 5rem))",
  background: "var(--bg-surface)",
  borderRadius: "12px",
  boxShadow: "0 20px 50px rgba(0, 0, 0, 0.25)",
  display: "flex",
  flexDirection: "column",
  overflow: "hidden",
  boxSizing: "border-box",
};

export const head: CSSProperties = {
  padding: "0.85rem 1.25rem",
  borderBottom: "1px solid var(--border-default)",
  display: "flex",
  alignItems: "center",
  gap: "0.75rem",
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
  border: "1px solid var(--accent-primary)",
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

/** Mockup `.pk`: `22px 34px 86px 1fr 120px` — tick, order, date, title, note. */
export function pickRow(picked: boolean): CSSProperties {
  return {
    display: "grid",
    gridTemplateColumns: "22px 34px 86px 1fr 120px",
    gap: "0.5rem",
    alignItems: "start",
    border: `1px solid ${picked ? "var(--accent-primary)" : "var(--border-default)"}`,
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

export const pickDate: CSSProperties = {
  color: "var(--text-muted)",
  fontSize: "0.72rem",
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
