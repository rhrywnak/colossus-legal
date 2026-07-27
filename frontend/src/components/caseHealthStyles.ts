// =============================================================================
// caseHealthStyles.ts — shared inline styles for the Case Health pane
// -----------------------------------------------------------------------------
// Split out of CaseHealthViews.tsx so that file stays under the module size limit
// and so the four sections (headline, per-document table, label table, edge
// table) cannot drift into four slightly-different visual languages. Same split
// discipline as proofMatrixColumns.ts / themeScanFormat.ts.
//
// Design tokens ONLY — every color, font and border here is a `var(--…)` custom
// property from the app's theme. No hex literals, no bespoke palette (Rule 2).
// =============================================================================

import type React from "react";

/** Rendered wherever a value is genuinely absent, never as a stand-in for zero. */
export const EMDASH = "—";

export const sectionStyle: React.CSSProperties = {
  marginTop: "2rem",
};

export const sectionHeading: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "0.95rem",
  fontWeight: 700,
  color: "var(--text-primary)",
  marginBottom: "0.25rem",
};

export const sectionNote: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "0.78rem",
  color: "var(--text-muted)",
  marginBottom: "0.75rem",
};

export const headlineRow: React.CSSProperties = {
  display: "flex",
  gap: "1rem",
  flexWrap: "wrap",
  marginBottom: "0.75rem",
};

export const headlineCard: React.CSSProperties = {
  flex: "1 1 220px",
  padding: "1rem 1.25rem",
  backgroundColor: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
};

/**
 * The headline card. Its accent rule is what visually separates the probative
 * rate from the two figures beside it — the pane's defining ruling is that those
 * numbers must never read as interchangeable.
 */
export const primaryCard: React.CSSProperties = {
  ...headlineCard,
  borderLeft: "3px solid var(--state-warning-strong)",
};

export const headlineValue: React.CSSProperties = {
  fontSize: "2rem",
  fontWeight: 700,
  color: "var(--text-primary)",
  lineHeight: 1.1,
};

/** Deliberately smaller and dimmer than {@link headlineValue} — see primaryCard. */
export const secondaryValue: React.CSSProperties = {
  ...headlineValue,
  fontSize: "1.5rem",
  color: "var(--text-secondary)",
};

export const headlineLabel: React.CSSProperties = {
  fontSize: "0.8rem",
  fontWeight: 600,
  color: "var(--text-primary)",
  marginTop: "0.35rem",
};

export const headlineHelp: React.CSSProperties = {
  fontSize: "0.74rem",
  color: "var(--text-muted)",
  marginTop: "0.2rem",
  lineHeight: 1.4,
};

export const tableStyle: React.CSSProperties = {
  width: "100%",
  borderCollapse: "collapse",
  fontFamily: "var(--font-sans)",
  fontSize: "0.8rem",
};

export const thStyle: React.CSSProperties = {
  textAlign: "left",
  padding: "0.4rem 0.6rem",
  borderBottom: "1px solid var(--border-default)",
  color: "var(--text-muted)",
  fontWeight: 600,
  whiteSpace: "nowrap",
};

export const thNumeric: React.CSSProperties = { ...thStyle, textAlign: "right" };

export const tdStyle: React.CSSProperties = {
  padding: "0.4rem 0.6rem",
  borderBottom: "1px solid var(--border-subtle, var(--border-default))",
  color: "var(--text-secondary)",
};

/** `tabular-nums` keeps digits column-aligned so counts are scannable. */
export const tdNumeric: React.CSSProperties = {
  ...tdStyle,
  textAlign: "right",
  fontVariantNumeric: "tabular-nums",
};

export const tdStrong: React.CSSProperties = {
  ...tdNumeric,
  color: "var(--text-primary)",
  fontWeight: 600,
};

/** Wide tables scroll inside their own container rather than the page body. */
export const scrollWrap: React.CSSProperties = { overflowX: "auto" };

export const noticeStyle: React.CSSProperties = {
  padding: "0.5rem 0.75rem",
  marginBottom: "0.75rem",
  borderLeft: "3px solid var(--state-warning-strong)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "6px",
  fontFamily: "var(--font-sans)",
  fontSize: "0.78rem",
  color: "var(--text-secondary)",
};
