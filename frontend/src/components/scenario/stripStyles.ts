// =============================================================================
// stripStyles.ts — the scenario header strip, and the Timeline subsets section
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screens 1 and 4, approved as drawn.
// Light frames only: this app has one palette and no dark theme, settled by the
// T4 ruling. Nothing here reads a `.frame.dark` value and nothing should.
//
// STRUCTURAL: geometry and rhythm, not settings. One approved drawing,
// transcribed. Every mockup pixel value is in the comment beside its rule so a
// screenshot can be diffed against the drawing without opening the HTML.
//
// ## ⚑ THE PALETTE, ROLE BY ROLE — including the one Roman corrected twice
//
//   mockup role     mockup light   app token                value
//   --card          #ffffff        --bg-surface             #ffffff
//   --ink           #111827        --text-primary           #101828
//   --ink2          #374151        --text-secondary         #475467
//   --muted         #6b7280        --text-muted             #667085
//   --line          #e5e7eb        --border-default         #d0d5dd
//   --indigo-line   #c7d2fe        --border-default         #d0d5dd   ← see below
//   --indigo-bg     #eef2ff        --state-info-bg-soft     #e0e7ff
//   --indigo-ink    #3730a3        --accent-primary         #1570ef
//   --blue          #2563eb        --accent-primary         #1570ef
//   --red           #b91c1c        --state-danger-strong / --v3-red-text
//   --red-bg        #fef2f2        --state-danger-bg-soft
//
// ⚑ `--indigo-line` maps to `--border-default`, NOT to `--accent-primary`. That
// mapping was made once as `--accent-primary` in T4, defended in a comment as
// "furniture, no contrast requirement rides on it", and REJECTED by Roman: a
// pale hairline is not a saturated blue outline whatever the contrast maths
// says. `.btn.ghost`'s border is the same `--indigo-line`, so it takes the same
// token, and so does every other `--indigo-line` site this task touches.
//
// `--accent-primary` appears below only as `color:` — the ghost button's text
// and the section's links. That is `--indigo-ink` / `--blue`, a different role.
// A `border` in this file reading `--accent-primary` is that defect returning.

import type { CSSProperties } from "react";

// ─── Screen 1: the strip (mockup `.strip`) ──────────────────────────────────

/** Mockup `.strip`: white card, hairline, 12px radius, 14/20/12 padding. */
export const strip: CSSProperties = {
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "12px",
  padding: "0.875rem 1.25rem 0.75rem",
};

/**
 * Mockup `.strip .r1`: ONE LINE, and `flex-wrap:nowrap` is load-bearing.
 *
 * The drawing puts code · title · role · status · actions on a single row. Let
 * it wrap and a long scenario name drops the two buttons onto their own line,
 * which is the "chaotic" header this replaces. The title carries
 * `white-space:nowrap` and the row lets it overflow rather than reflow.
 */
export const row1: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.75rem",
  flexWrap: "nowrap",
};

/** Mockup `.strip .code`: 14px / 700 / muted / .06em — survives a rename (§2a). */
export const code: CSSProperties = {
  color: "var(--text-muted)",
  fontWeight: 700,
  fontSize: "0.875rem",
  letterSpacing: "0.06em",
  whiteSpace: "nowrap",
};

/** Mockup `.strip h1`: 24px / 800, no wrap, no margin. */
export const title: CSSProperties = {
  margin: 0,
  fontSize: "1.5rem",
  fontWeight: 800,
  whiteSpace: "nowrap",
  color: "var(--text-primary)",
  overflow: "hidden",
  textOverflow: "ellipsis",
};

/** Mockup `.chip.role`: red-soft fill, red text, pill. Direction is read-only. */
export const roleChip: CSSProperties = {
  display: "inline-block",
  background: "var(--state-danger-bg-soft)",
  color: "var(--v3-red-text)",
  borderRadius: "999px",
  padding: "0.125rem 0.625rem",
  fontSize: "0.75rem",
  fontWeight: 600,
  whiteSpace: "nowrap",
};

/** Mockup `.strip .acts`: pushed right, the two primary actions. */
export const actions: CSSProperties = {
  marginLeft: "auto",
  display: "flex",
  gap: "0.5rem",
  alignItems: "center",
};

/** Mockup `.strip .r2`: Edit · Rehearsal view · … · Delete. */
export const row2: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  marginTop: "0.625rem",
  fontSize: "0.84rem",
};

/** Mockup `.strip .r2 .right`: Delete, pushed to the far end. */
export const row2Right: CSSProperties = {
  marginLeft: "auto",
  display: "flex",
  gap: "0.5rem",
  alignItems: "center",
};

// ─── the four button roles (mockup `.btn` and its modifiers) ────────────────

const buttonBase: CSSProperties = {
  borderRadius: "8px",
  padding: "0.5rem 0.875rem",
  fontSize: "0.84rem",
  fontWeight: 600,
  cursor: "pointer",
  whiteSpace: "nowrap",
  fontFamily: "inherit",
  lineHeight: 1.2,
  textDecoration: "none",
  display: "inline-block",
};

/** Mockup `.btn`: the ONE solid primary on the strip. */
export const solidButton: CSSProperties = {
  ...buttonBase,
  background: "var(--accent-primary)",
  color: "#ffffff",
  border: "none",
};

/**
 * Mockup `.btn.ghost`: outlined, accent text, PALE border.
 *
 * View Timeline is outlined precisely so Practice stays the single solid
 * primary — Screen 1's own note. Two solid buttons side by side is two things
 * claiming to be the main action.
 */
export const ghostButton: CSSProperties = {
  ...buttonBase,
  background: "var(--bg-surface)",
  color: "var(--accent-primary)",
  border: "1px solid var(--border-default)",
};

/** Mockup `.btn.quiet`: white ground, secondary ink, hairline. */
export const quietButton: CSSProperties = {
  ...buttonBase,
  background: "var(--bg-surface)",
  color: "var(--text-secondary)",
  border: "1px solid var(--border-default)",
};

/** Mockup `.btn.quiet[disabled]`: half opacity, not-allowed. */
export const quietDisabled: CSSProperties = {
  ...quietButton,
  opacity: 0.5,
  cursor: "not-allowed",
};

/** Mockup `.btn.danger`: red text, NO border, no fill. Distance is the guard. */
export const dangerButton: CSSProperties = {
  ...buttonBase,
  background: "transparent",
  color: "var(--v3-red-text)",
  border: "none",
  padding: "0.5rem 0.5rem",
};

// ─── Screen 4: the Timeline subsets section (mockup `.editsec`) ─────────────

/** Mockup `.editsec`: the section's own card. */
export const section: CSSProperties = {
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "12px",
  padding: "1rem 1.25rem",
  marginTop: "0.875rem",
};

/** Mockup `.editsec h3`: 15px, tight to the hint below it. */
export const sectionTitle: CSSProperties = {
  margin: "0 0 0.125rem",
  fontSize: "0.94rem",
  color: "var(--text-primary)",
};

/** Mockup `.editsec .hint`: what the section teaches, muted. */
export const sectionHint: CSSProperties = {
  fontSize: "0.78rem",
  color: "var(--text-muted)",
  marginBottom: "0.75rem",
  maxWidth: "60rem",
};

/** Mockup `.srow`: `grid-template-columns:1fr 90px 110px 190px`. */
export function subsetRow(attached: boolean): CSSProperties {
  return {
    display: "grid",
    gridTemplateColumns: "1fr 90px 110px 190px",
    gap: "0.75rem",
    alignItems: "center",
    border: `1px solid ${attached ? "var(--accent-primary)" : "var(--border-default)"}`,
    borderRadius: "10px",
    padding: "0.625rem 0.875rem",
    marginTop: "0.5rem",
    // Mockup `.srow.on`: the indigo ground marks what this scenario carries.
    background: attached ? "var(--state-info-bg-soft)" : "var(--bg-surface)",
  };
}

/** Mockup `.srow .nm`. */
export const subsetName: CSSProperties = {
  fontWeight: 700,
  color: "var(--text-primary)",
};

/** Mockup `.srow .ds`: the description in the serif, as the window draws it. */
export const subsetDescription: CSSProperties = {
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: "0.78rem",
  color: "var(--text-secondary)",
  marginTop: "0.125rem",
};

/** Mockup `.srow .c`. */
export const subsetCount: CSSProperties = {
  color: "var(--text-muted)",
  fontSize: "0.81rem",
};

/** Mockup `.srow .st` / `.srow.on .st` / `.srow .st.off`. */
export function subsetState(attached: boolean): CSSProperties {
  return {
    fontSize: "0.75rem",
    fontWeight: 700,
    color: attached ? "var(--accent-primary)" : "var(--text-muted)",
  };
}

/** Mockup `.srow .ac`: Preview + the one button, right-aligned. */
export const subsetActions: CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  justifyContent: "flex-end",
  alignItems: "center",
};

/** Mockup `.srow .ac a`. */
export const subsetLink: CSSProperties = {
  fontSize: "0.78rem",
  color: "var(--accent-primary)",
  textDecoration: "none",
  fontWeight: 600,
  background: "none",
  border: "none",
  cursor: "pointer",
  fontFamily: "inherit",
  padding: 0,
};

/** Mockup `.btn.sm`: the row's compact Attach / Detach. */
export const smallButton: CSSProperties = { padding: "0.3rem 0.7rem", fontSize: "0.78rem" };

/** The "+ Create a new subset on the timeline →" line under the rows. */
export const createRow: CSSProperties = { marginTop: "0.75rem", fontSize: "0.81rem" };

export const createHint: CSSProperties = {
  color: "var(--text-muted)",
  marginLeft: "0.4rem",
};

/** A failed read or write, rendered rather than swallowed (Standing Rule 1). */
export const sectionError: CSSProperties = {
  marginTop: "0.5rem",
  padding: "0.5rem 0.75rem",
  borderRadius: "8px",
  border: "1px solid var(--v3-red-text)",
  background: "var(--state-danger-bg-soft)",
  color: "var(--v3-red-text)",
  fontSize: "0.81rem",
  fontWeight: 600,
};

/** The pending / empty line, muted. */
export const sectionState: CSSProperties = {
  marginTop: "0.5rem",
  fontSize: "0.81rem",
  color: "var(--text-muted)",
};
