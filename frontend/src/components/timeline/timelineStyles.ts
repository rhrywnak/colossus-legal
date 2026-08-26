// =============================================================================
// timelineStyles.ts — TIMELINE_MOCKUP_v2's stylesheet, as typed style objects
// =============================================================================
//
// The mockup's `<style>` block, transcribed. Same move, and the same reasons, as
// `practiceStyles.ts`: every other page in this product styles inline from a
// `*Styles.ts` sibling, so a CSS module here would be the only one.
//
// ## Where the mockup's hex values went
//
// The mockup draws its own palette (`--ink`, `--line`, `--muted`, …). This page
// does NOT adopt those literals: design R5 says keep the CURRENT visual
// language, and the page being replaced already reads the product's own tokens
// (`--text-primary`, `--border-default`, `--bg-surface`, `--accent-primary`).
// Adopting the drawing's hexes would be a silent re-theme of a live page.
//
// The colours that ARE the mockup's are the ones that come from the DATABASE —
// every phase bar and every tag dot and chip reads its own stored `color`, which
// is exactly what makes a recolour an UPDATE rather than a build.

import type { CSSProperties } from "react";

/** The page's own column. */
export const page: CSSProperties = { paddingTop: "2rem", paddingBottom: "4rem" };

// ─── title row ───────────────────────────────────────────────────────────────

export const titleRow: CSSProperties = { marginBottom: "1.5rem" };

export const h1: CSSProperties = {
  fontSize: "1.5rem",
  fontWeight: 700,
  color: "var(--text-primary)",
  margin: 0,
  marginBottom: "0.3rem",
};

export const subCount: CSSProperties = {
  fontSize: "0.84rem",
  color: "var(--text-muted)",
  margin: 0,
};

// ─── filter bar ──────────────────────────────────────────────────────────────

export const filters: CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: "0.5rem",
  alignItems: "center",
  margin: "1.1rem 0 0.4rem",
};

export const search: CSSProperties = {
  flex: "1 1 230px",
  maxWidth: "320px",
  display: "flex",
  alignItems: "center",
  gap: "0.375rem",
  border: "1px solid var(--border-default)",
  borderRadius: "9999px",
  padding: "0.375rem 0.75rem",
  background: "var(--bg-surface)",
  fontSize: "0.81rem",
};

export const searchInput: CSSProperties = {
  border: "none",
  outline: "none",
  flex: 1,
  fontSize: "0.81rem",
  background: "transparent",
  color: "var(--text-primary)",
  fontFamily: "inherit",
};

/** One filter chip. `color` and `active` come from the stored tag row. */
export function chip(color: string, active: boolean): CSSProperties {
  return {
    border: `1px solid ${active ? color : "var(--border-default)"}`,
    background: active ? `${color}1a` : "var(--bg-surface)",
    color,
    borderRadius: "9999px",
    padding: "0.3rem 0.8rem",
    fontSize: "0.81rem",
    fontWeight: 600,
    cursor: "pointer",
    fontFamily: "inherit",
  };
}

/** The "All" chip, which is the page's own ink rather than a tag's colour. */
export function allChip(active: boolean): CSSProperties {
  return {
    ...chip("var(--text-secondary)", false),
    background: active ? "var(--text-primary)" : "var(--bg-surface)",
    color: active ? "var(--bg-surface)" : "var(--text-secondary)",
    borderColor: active ? "var(--text-primary)" : "var(--border-default)",
  };
}

export const dateControl: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.375rem",
  border: "1px solid var(--border-default)",
  borderRadius: "9999px",
  padding: "0.3rem 0.8rem",
  fontSize: "0.79rem",
  color: "var(--text-secondary)",
  background: "var(--bg-surface)",
};

export const dateInput: CSSProperties = {
  border: "none",
  outline: "none",
  background: "transparent",
  fontSize: "0.79rem",
  color: "var(--text-primary)",
  fontFamily: "inherit",
};

// ─── phase section ───────────────────────────────────────────────────────────

export const phase: CSSProperties = { marginBottom: "2rem", scrollMarginTop: "80px" };

/** The phase header's coloured bar reads the phase's own stored colour. */
export function phaseHead(color: string): CSSProperties {
  return {
    borderLeft: `4px solid ${color}`,
    paddingLeft: "1rem",
    display: "flex",
    alignItems: "baseline",
    gap: "0.6rem",
  };
}

export const phaseLabel: CSSProperties = {
  fontSize: "1.05rem",
  fontWeight: 700,
  color: "var(--text-primary)",
};

export const phaseMeta: CSSProperties = { fontSize: "0.78rem", color: "var(--text-muted)" };

/** The muted italic subtitle under a phase header (design R14). */
export const phaseDesc: CSSProperties = {
  color: "var(--text-muted)",
  fontSize: "0.81rem",
  fontStyle: "italic",
  margin: "0.15rem 0 0.6rem 1rem",
};

export const expandControl: CSSProperties = {
  marginLeft: "auto",
  fontSize: "0.78rem",
  fontWeight: 600,
  color: "var(--accent-primary)",
  whiteSpace: "nowrap",
  background: "none",
  border: "none",
  cursor: "pointer",
  fontFamily: "inherit",
  padding: 0,
};

export const scrollHint: CSSProperties = {
  fontSize: "0.69rem",
  color: "var(--text-disabled)",
  margin: "0 0 0.375rem 1rem",
};

/**
 * The scroll window (design R6).
 *
 * ## Why the height is a multiple and not the mockup's 332px
 *
 * The mockup fixes `max-height: 332px` for its four entries. Four is a STORED
 * number here, so the height has to follow it — hard-coding 332 would show four
 * rows' worth of window however many rows the setting asked for. `ROW_HEIGHT` is
 * that 332 ÷ 4, kept as one named constant so the arithmetic is visible.
 */
const ROW_HEIGHT_PX = 83;

export function scrollWindow(visibleEvents: number): CSSProperties {
  return {
    maxHeight: `${visibleEvents * ROW_HEIGHT_PX}px`,
    overflowY: "auto",
    padding: "2px 4px 8px 2px",
    scrollbarGutter: "stable",
  };
}

// ─── event card ──────────────────────────────────────────────────────────────

export const card: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "96px 14px 1fr",
  gap: "0.625rem",
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "10px",
  padding: "0.8rem 1rem",
  margin: "0 0 0.625rem 0.75rem",
  textAlign: "left",
  // border-box, and the width follows from the margin rather than a calc: this
  // app sets no global box-sizing, so a content-box card at 100% width overflows
  // its own scroll window by its padding and border and the text is clipped.
  boxSizing: "border-box",
  width: "auto",
  cursor: "pointer",
  fontFamily: "inherit",
};

export const cardDate: CSSProperties = {
  color: "var(--text-muted)",
  fontSize: "0.78rem",
  paddingTop: "0.125rem",
  whiteSpace: "nowrap",
};

export function dot(color: string): CSSProperties {
  return {
    width: "9px",
    height: "9px",
    borderRadius: "50%",
    marginTop: "0.375rem",
    background: color,
  };
}

export const cardTitle: CSSProperties = {
  margin: 0,
  fontSize: "0.9rem",
  fontWeight: 600,
  display: "inline",
  color: "var(--text-primary)",
};

export function tagChip(color: string): CSSProperties {
  return {
    display: "inline-block",
    fontSize: "0.69rem",
    fontWeight: 600,
    borderRadius: "9999px",
    padding: "0.09rem 0.56rem",
    marginLeft: "0.5rem",
    verticalAlign: "1px",
    background: `${color}18`,
    color,
  };
}

export const fact: CSSProperties = {
  margin: "0.3rem 0 0",
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: "0.84rem",
  color: "var(--text-secondary)",
  lineHeight: 1.55,
};

export const rowMeta: CSSProperties = {
  marginTop: "0.44rem",
  display: "flex",
  gap: "0.625rem",
  flexWrap: "wrap",
  alignItems: "center",
  fontSize: "0.75rem",
};

export const docLink: CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "none",
  fontWeight: 600,
};

export const pinpoint: CSSProperties = { color: "var(--text-disabled)", fontWeight: 400 };

/** The amber "no document yet" mark (design R12) — a MARK, never a link. */
export const noDoc: CSSProperties = {
  color: "var(--burden-warning-text)",
  background: "var(--burden-warning-bg)",
  borderRadius: "9999px",
  padding: "0.06rem 0.56rem",
  fontWeight: 600,
};

/** The neutral "not checked" mark — visibly NOT the amber one. */
export const unchecked: CSSProperties = {
  color: "var(--text-muted)",
  background: "var(--bg-page, transparent)",
  border: "1px solid var(--border-default)",
  borderRadius: "9999px",
  padding: "0.06rem 0.56rem",
  fontWeight: 600,
};

export const noteCount: CSSProperties = { color: "var(--text-muted)" };

// ─── states ──────────────────────────────────────────────────────────────────

export const state: CSSProperties = {
  padding: "2rem",
  textAlign: "center",
  color: "var(--text-muted)",
};

export const errorState: CSSProperties = {
  ...state,
  color: "var(--status-dropped-text)",
  fontWeight: 600,
};

/** The loud row for an event whose phase has no row (design B4). */
export const unknownPhase: CSSProperties = {
  border: "1px solid var(--status-dropped-text)",
  borderRadius: "10px",
  padding: "0.8rem 1rem",
  margin: "0 0 0.625rem 0",
  color: "var(--status-dropped-text)",
  fontSize: "0.82rem",
  fontWeight: 600,
};

// ─── event page ──────────────────────────────────────────────────────────────

export const crumb: CSSProperties = {
  fontSize: "0.81rem",
  color: "var(--text-muted)",
  marginBottom: "0.9rem",
};

export const crumbLink: CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "none",
};

export const eventTitle: CSSProperties = {
  margin: "0.125rem 0",
  fontSize: "1.35rem",
  color: "var(--text-primary)",
};

export const when: CSSProperties = { color: "var(--text-muted)", fontSize: "0.84rem" };

export const fullFact: CSSProperties = {
  fontFamily: "Georgia, serif",
  fontSize: "0.97rem",
  margin: "0.9rem 0",
  color: "var(--text-primary)",
  maxWidth: "760px",
  lineHeight: 1.6,
};

export const panel: CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "10px",
  padding: "0.9rem 1.1rem",
  marginTop: "0.9rem",
};

export const panelHeading: CSSProperties = {
  margin: "0 0 0.5rem",
  fontSize: "0.75rem",
  letterSpacing: "0.1em",
  color: "var(--text-muted)",
  textTransform: "uppercase",
};

export const linkRow: CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  alignItems: "center",
  fontSize: "0.84rem",
  margin: "0.375rem 0",
};

export const note: CSSProperties = {
  borderLeft: "3px solid var(--border-default)",
  padding: "0.125rem 0 0.125rem 0.75rem",
  margin: "0.625rem 0",
  fontSize: "0.84rem",
  color: "var(--text-primary)",
};

export const noteBy: CSSProperties = { fontSize: "0.75rem", color: "var(--text-muted)" };

export const history: CSSProperties = {
  fontSize: "0.78rem",
  color: "var(--text-muted)",
  margin: "0.25rem 0",
};

export const panelEmpty: CSSProperties = {
  fontSize: "0.82rem",
  color: "var(--text-muted)",
  fontStyle: "italic",
};
