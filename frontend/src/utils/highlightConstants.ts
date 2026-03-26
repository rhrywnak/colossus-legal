/**
 * Highlight configuration — single source of truth for all highlight behavior.
 *
 * Domain-agnostic: no legal, evidence, or project-specific concepts.
 * Used by pdfHighlight.ts and PdfViewer.tsx.
 */

/** Default highlight appearance */
export const HIGHLIGHT_DEFAULTS = {
  color: "#FFEB3B",
  opacity: 0.4,
} as const;

/** Available highlight colors for future color picker */
export const HIGHLIGHT_COLORS = [
  { name: "Yellow", value: "#FFEB3B" },
  { name: "Green", value: "#A5D6A7" },
  { name: "Blue", value: "#90CAF9" },
  { name: "Pink", value: "#F48FB1" },
  { name: "Orange", value: "#FFCC80" },
] as const;

/** DOM attributes used for highlight manipulation */
export const HIGHLIGHT_ATTRS = {
  dataAttribute: "data-highlight",
  markedValue: "true",
} as const;
