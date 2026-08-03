// =============================================================================
// scenarioSectionStyles.ts — the shared chrome of a §2 page section (task 1.7C)
// =============================================================================
//
// Design §2 gives the scenario page seven sections with one visual grammar, and
// §4/§2c makes that grammar binding: pure white canvas, hairline borders (one
// token), 10px radius, whisper shadow, one accent, regular weight with bold
// reserved for true emphasis.
//
// Four sections now render that grammar. Defining it once here is not tidiness —
// it is the only way §2c stays true. Four copies of a border radius drift, and a
// page whose sections have three different corner radii reads as three different
// products.

import type React from "react";

/** The one border token. Every §2 section edge uses this and nothing else. */
export const HAIRLINE = "1px solid var(--border-default)";

/** The section title row: `<h2>` plus its muted subtitle/count. */
export const sectionHeaderStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  flexWrap: "wrap",
  gap: "0.7rem",
  marginTop: "2rem",
  marginBottom: "0.6rem",
};

/** The `<h2>`. 0.98rem/600 — a section head is one of §2c's true-emphasis cases. */
export const sectionTitleStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "0.98rem",
  fontWeight: 600,
  color: "var(--text-primary)",
};

/**
 * The muted line beside a title: a count, an empty-state affordance, a subtitle.
 *
 * Muted rather than hidden because several of these carry the §2 copy that tells
 * the human what the section IS — D10's empty-state fix lives in this slot.
 */
export const sectionMetaStyle: React.CSSProperties = {
  fontSize: "0.78rem",
  color: "var(--text-muted)",
};

/** A section's bordered body. */
export const sectionPanelStyle: React.CSSProperties = {
  background: "var(--bg-surface)",
  border: HAIRLINE,
  borderRadius: "10px",
  boxShadow: "0 1px 2px rgba(16,24,40,.05)",
};

/** A section body with its own padding (the panels that hold prose, not tables). */
export const sectionPaddedPanelStyle: React.CSSProperties = {
  ...sectionPanelStyle,
  padding: "1rem 1.25rem",
};

/** The neutral chip. Chips carry the colour in §2c; this is the uncoloured base. */
export const chipStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "999px",
  padding: "0.1rem 0.55rem",
  fontSize: "0.75rem",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

/** The quiet secondary button used for every `+ Add …` affordance. */
export const addButtonStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "6px",
  background: "var(--bg-surface)",
  color: "var(--accent-primary)",
  borderColor: "var(--accent-primary)",
  fontSize: "0.8rem",
  fontFamily: "inherit",
  padding: "0.3rem 0.7rem",
  cursor: "pointer",
};

/** The italic muted line used wherever something is honestly absent. */
export const absentStyle: React.CSSProperties = {
  fontSize: "0.85rem",
  color: "var(--text-muted)",
  fontStyle: "italic",
};
