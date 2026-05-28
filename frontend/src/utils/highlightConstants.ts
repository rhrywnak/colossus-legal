/**
 * Highlight configuration — single source of truth for all highlight behavior.
 *
 * Domain-agnostic: no legal, evidence, or project-specific concepts.
 * Used by pdfHighlight.ts and PdfViewer.tsx.
 */

/** Default highlight appearance */
export const HIGHLIGHT_DEFAULTS = {
  color: "var(--burden-warning-bg)",
  opacity: 0.4,
} as const;

/** Available highlight colors for future color picker */
export const HIGHLIGHT_COLORS = [
  { name: "Yellow", value: "var(--burden-warning-bg)" },
  { name: "Green", value: "var(--state-success-bg-soft)" },
  { name: "Blue", value: "var(--accent-bg-soft)" },
  { name: "Pink", value: "var(--state-danger-border)" },
  { name: "Orange", value: "var(--burden-warning-bg)" },
] as const;

/** DOM attributes used for highlight manipulation */
export const HIGHLIGHT_ATTRS = {
  dataAttribute: "data-highlight",
  markedValue: "true",
} as const;
