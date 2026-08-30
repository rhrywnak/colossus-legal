// =============================================================================
// subsetStyles.ts — the Subsets section, as the mockup draws it
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v1_2026-08-30.html, Screen 2, approved as drawn. Split
// from `timelineStyles.ts` (398 lines) because this is a different screen's
// furniture, and split from `subsetModalStyles.ts` because Screen 3 is another.
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

/** The indigo section bar. Mockup: `.phead` with `border-color:#3730a3`. */
export const sectionHead: CSSProperties = {
  borderLeft: "4px solid var(--accent-primary)",
  paddingLeft: "1rem",
  display: "flex",
  alignItems: "baseline",
  gap: "0.6rem",
  marginTop: "1.9rem",
};

export const sectionTitle: CSSProperties = {
  fontSize: "1.05rem",
  fontWeight: 700,
  color: "var(--text-primary)",
};

export const sectionSubtitle: CSSProperties = {
  fontSize: "0.78rem",
  color: "var(--text-muted)",
};

/** The ghost `+ Add subset` button, right-aligned. Mockup: `.btn.ghost`. */
export const addButton: CSSProperties = {
  marginLeft: "auto",
  background: "var(--bg-surface)",
  color: "var(--accent-primary)",
  border: "1px solid var(--accent-primary)",
  borderRadius: "8px",
  padding: "0.4rem 0.85rem",
  fontSize: "0.8rem",
  fontWeight: 600,
  cursor: "pointer",
  fontFamily: "inherit",
  whiteSpace: "nowrap",
};

/** Mockup `.subrow`: `grid-template-columns:1fr 90px 220px 120px`. */
export const row: CSSProperties = {
  display: "grid",
  gridTemplateColumns: "1fr 90px 220px 120px",
  gap: "0.75rem",
  alignItems: "center",
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "10px",
  padding: "0.75rem 1rem",
  margin: "0.5rem 0 0 0.75rem",
  boxSizing: "border-box",
};

export const rowName: CSSProperties = { fontWeight: 700, color: "var(--text-primary)" };

/** Serif, like an event's fact. Mockup: `.subrow .ds` is Georgia. */
export const rowDescription: CSSProperties = {
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: "0.81rem",
  color: "var(--text-secondary)",
  marginTop: "0.125rem",
};

export const rowCount: CSSProperties = { color: "var(--text-muted)", fontSize: "0.81rem" };

/** The amber gap line under the count. Mockup: `var(--amber)`, 12px, 600. */
export const rowGaps: CSSProperties = {
  color: "var(--burden-warning-text)",
  fontSize: "0.75rem",
  fontWeight: 600,
};

export const rowCarriedBy: CSSProperties = { fontSize: "0.78rem", color: "var(--text-secondary)" };

/** A scenario code chip. Mockup: `.chip.sub`, indigo on a pale indigo ground. */
export const codeChip: CSSProperties = {
  display: "inline-block",
  background: "var(--state-info-bg-soft)",
  border: "1px solid var(--accent-primary)",
  borderRadius: "999px",
  padding: "0.1rem 0.6rem",
  fontSize: "0.72rem",
  fontWeight: 600,
  color: "var(--accent-primary)",
  marginRight: "0.3rem",
};

export const rowBy: CSSProperties = { color: "var(--text-muted)", fontSize: "0.72rem" };

export const rowActions: CSSProperties = {
  display: "flex",
  gap: "0.6rem",
  justifyContent: "flex-end",
  fontSize: "0.8rem",
};

export const rowAction: CSSProperties = {
  color: "var(--accent-primary)",
  fontWeight: 600,
  background: "none",
  border: "none",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: "0.8rem",
  padding: 0,
  whiteSpace: "nowrap",
};

/** Mockup `.subempty`: muted, italic, indented with the rows. */
export const emptyState: CSSProperties = {
  margin: "0.5rem 0 0 0.75rem",
  color: "var(--text-muted)",
  fontSize: "0.81rem",
  fontStyle: "italic",
};

