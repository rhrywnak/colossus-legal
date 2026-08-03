// =============================================================================
// scenarioSectionStyles.ts — the shared chrome of a §2 page section
// =============================================================================
//
// Design §2 gives the scenario page seven sections with one visual grammar. Task
// 1.7D replaces that grammar wholesale, per Roman's 2026-08-03 ruling and the
// signed SCENARIO_PAGE_MOCKUP_v3: values here are TRANSCRIBED from the mockup.
//
// ## What changed from 1.7C, and why it is a system not a repaint
//
// 1.7C: white canvas · cards separated by a 1px hairline · radius 10 · a whisper
// shadow that was decoration.
// 1.7D: tinted canvas (#f4f5f7) · cards with NO BORDER AT ALL, separated from the
// page by two layered shadows · radius 12 · dividers only INSIDE a card.
//
// The two halves of each system are not interchangeable. A borderless white card
// is invisible on a white page, and a hairline border on a tinted page with a
// layered shadow reads as a double edge. So the canvas and the card treatment
// moved together — see the `--bg-canvas` comment in tokens.css for the full chain.
//
// ## Every colour here is a token, and the tokens are SCOPED
//
// The v3 palette lives under `[data-surface="v3"]` (tokens.css), which the
// scenario page and the rehearsal view carry. So `var(--text-primary)` below
// resolves to the mockup's #1a202c on those surfaces and to the app's #101828
// everywhere else — the cascade does the scoping, which is why this file names no
// hex values. Ruling R1; app-wide adoption is task 3.14.

import type React from "react";

/**
 * The internal divider.
 *
 * INTERNAL only. A card's edge is its shadow now, so if you find this on the
 * outside of a panel it is a bug — that is the "double edge" the v3 ruling
 * removed.
 */
export const DIVIDER = "1px solid var(--border-default)";

/**
 * Kept as a name so 1.7C call sites do not silently mean something new.
 *
 * @deprecated Use {@link DIVIDER}. There are no hairline card borders in v3.
 */
export const HAIRLINE = DIVIDER;

/** The section title row: `<h2>` plus its muted count/subtitle. */
export const sectionHeaderStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  flexWrap: "wrap",
  gap: "0.75rem",
  // Mockup `.sec-h`: margin-top 32px.
  marginTop: "2rem",
  marginBottom: "0.75rem",
};

/** The `<h2>`. Mockup `.sec-h h2`: 15px/600. */
export const sectionTitleStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "15px",
  fontWeight: 600,
  color: "var(--text-primary)",
};

/** The muted line beside a title. Mockup `.sec-h .count`: 12.5px, --text-3. */
export const sectionMetaStyle: React.CSSProperties = {
  fontSize: "12.5px",
  color: "var(--text-muted)",
};

/**
 * A card. Mockup `.panel`: white, radius 12, `--shadow-card`. NO BORDER.
 *
 * The shadow is two layers on purpose — a tight one for the contact edge and a
 * wide soft one for the lift. One layer alone reads as a border, which is the
 * thing v3 removed.
 */
export const sectionPanelStyle: React.CSSProperties = {
  background: "var(--bg-surface)",
  borderRadius: "var(--radius-card)",
  boxShadow: "var(--shadow-card)",
  // Mockup `.panel` + `.facts`: content is clipped to the radius so a table's
  // first row cannot square off the card's corners.
  overflow: "hidden",
};

/** A card with the mockup's `.pad` (18px 24px) for panels holding prose. */
export const sectionPaddedPanelStyle: React.CSSProperties = {
  ...sectionPanelStyle,
  padding: "18px 24px",
};

/**
 * The neutral chip. Mockup `.chip`: 12px/500, radius 999, padding 3px 11px.
 *
 * Colourless by itself — in v3 the chip's colour comes from its variant
 * (`.chip.dir` red-soft, `.chip.alleg` green-soft), never from this base.
 */
export const chipStyle: React.CSSProperties = {
  fontSize: "12px",
  fontWeight: 500,
  padding: "3px 11px",
  borderRadius: "999px",
  whiteSpace: "nowrap",
  color: "var(--text-secondary)",
  background: "var(--v3-chrome)",
};

/** The allegation chip. Mockup `.chip.alleg`: green-soft fill, green text. */
export const allegationChipStyle: React.CSSProperties = {
  ...chipStyle,
  background: "var(--v3-chip-alleg-bg)",
  color: "var(--v3-chip-alleg-text)",
  // The mockup's chips are short ("¶41 — refused amicable division"), but the real
  // complaint sentences are not — S-2's ¶41 is 140 characters, and with the base
  // chip's `nowrap` it ran clean off the right edge of the identity card. The
  // mockup cannot rule on data it does not contain, so the chip wraps: same fill,
  // same radius, readable at any length.
  whiteSpace: "normal",
  lineHeight: 1.45,
};

/**
 * The ghost button. Mockup `.btn.ghost`: white, NO border, its own `--shadow-card`,
 * radius 9, 13.5px/500.
 *
 * A white button on a white card is only visible because of that shadow — which is
 * why the shadow is not optional here the way a decorative one would be.
 */
export const ghostButtonStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13.5px",
  fontWeight: 500,
  padding: "8px 15px",
  borderRadius: "9px",
  border: "none",
  cursor: "pointer",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
  boxShadow: "var(--shadow-card)",
};

/** Every `+ Add …` affordance is a ghost button in v3. */
export const addButtonStyle: React.CSSProperties = ghostButtonStyle;

/** The italic muted line used wherever something is honestly absent. */
export const absentStyle: React.CSSProperties = {
  fontSize: "13px",
  color: "var(--text-muted)",
  fontStyle: "italic",
};

/** A `kbd` key hint. Mockup `kbd`: 11.5px, chrome fill, radius 5, NO border. */
export const kbdStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "11.5px",
  padding: "0 6px",
  borderRadius: "5px",
  background: "var(--v3-chrome-strong)",
  color: "var(--text-secondary)",
};

/** A borderless input on the chrome fill. Mockup `input[type=search]` / `select`. */
export const chromeInputStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13.5px",
  padding: "8px 12px",
  border: "none",
  borderRadius: "8px",
  background: "var(--v3-chrome)",
  color: "var(--text-primary)",
};
