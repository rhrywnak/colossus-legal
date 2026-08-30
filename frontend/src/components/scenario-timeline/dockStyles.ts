// =============================================================================
// dockStyles.ts — the View Timeline button and the Timeline row
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v1_2026-08-30.html Screen 1, approved as drawn. The
// header's half of it; `windowStyles.ts` is the floating window's.
//
// CONST: geometry and rhythm, not settings. There is no frontend config surface
// for a window's furniture and these are not per-deployment values — they are
// one approved drawing, transcribed. The mockup's own pixel values are in the
// comments beside each rule so a screenshot can be diffed against the drawing
// without opening the HTML.
//
// ## ⚑ THE MOCKUP'S PALETTE IS NOT THIS APP'S PALETTE
//
// Same deviation the Subsets section made, for the same reason: the mockup is
// standalone HTML with literal hexes and no dark theme. Every colour here is
// the app token that plays the mockup's role — `--accent-primary` for the
// indigo furniture, `--state-info-bg-soft` for its pale ground,
// `--burden-warning-text` / `--burden-warning-bg` for the amber gap badge.
// Transcribing the hexes would have reproduced the drawing and broken the theme.

import type { CSSProperties } from "react";


// ─── the button (mockup `.btn`, in the scenario header bar) ─────────────────

export const button: CSSProperties = {
  background: "var(--accent-primary)",
  // `--bg-surface` and NOT `#ffffff`: this file's own header promises every
  // colour is the app token that plays the mockup's role, and a literal white
  // broke that promise in the one place it is least visible — text that happens
  // to look right in the light theme and stays white against a dark-theme fill.
  // `--bg-surface` is what the timeline's own primary button and the three
  // pipeline dialogs use for text on an accent ground, and it flips with the
  // theme. Caught by the rules gate, not by the eye.
  color: "var(--bg-surface)",
  border: "none",
  borderRadius: "8px",
  padding: "0.5rem 0.875rem",
  fontSize: "0.84rem",
  fontWeight: 600,
  cursor: "pointer",
  fontFamily: "inherit",
  whiteSpace: "nowrap",
};

// ─── the Timeline row (mockup Screen 1's `.tlrow`, in the scenario header) ──

export const timelineRow: CSSProperties = {
  display: "flex",
  alignItems: "center",
  flexWrap: "wrap",
  gap: "0.4rem",
  fontSize: "0.78rem",
  color: "var(--text-muted)",
  marginTop: "0.5rem",
};

/** Mockup `.chip.sub`: name · count, indigo on a pale indigo ground. */
export const subsetChip: CSSProperties = {
  display: "inline-block",
  background: "var(--state-info-bg-soft)",
  border: "1px solid var(--accent-primary)",
  borderRadius: "999px",
  padding: "0.1rem 0.6rem",
  fontSize: "0.72rem",
  fontWeight: 600,
  color: "var(--accent-primary)",
};

export const attachLink: CSSProperties = {
  color: "var(--accent-primary)",
  fontWeight: 600,
  background: "none",
  border: "none",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: "0.78rem",
  padding: 0,
};

/** The small chooser the Attach link opens — a list, not a modal. */
export const attachList: CSSProperties = {
  position: "absolute",
  zIndex: 45,
  marginTop: "0.25rem",
  minWidth: "16rem",
  maxHeight: "14rem",
  overflowY: "auto",
  background: "var(--bg-surface)",
  border: "1px solid var(--accent-primary)",
  borderRadius: "10px",
  boxShadow: "0 10px 24px rgba(17, 24, 39, 0.16)",
  padding: "0.25rem",
};

export const attachItem: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.4rem",
  width: "100%",
  textAlign: "left",
  background: "none",
  border: "none",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: "0.78rem",
  color: "var(--text-primary)",
  padding: "0.35rem 0.5rem",
  borderRadius: "6px",
};

export const attachCheck: CSSProperties = {
  width: "1rem",
  color: "var(--accent-primary)",
  fontWeight: 700,
};

export const attachFoot: CSSProperties = {
  ...attachItem,
  borderTop: "1px solid var(--border-default)",
  color: "var(--accent-primary)",
  fontWeight: 600,
  marginTop: "0.2rem",
};
