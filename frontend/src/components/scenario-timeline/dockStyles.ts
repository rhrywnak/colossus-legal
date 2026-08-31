// =============================================================================
// dockStyles.ts — the View Timeline button
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 1, approved as drawn. The
// header's half of it; `windowStyles.ts` is the floating window's.
//
// CONST: geometry and rhythm, not settings. There is no frontend config surface
// for a window's furniture and these are not per-deployment values — they are
// one approved drawing, transcribed. The mockup's own pixel values are in the
// comments beside each rule so a screenshot can be diffed against the drawing
// without opening the HTML.
//
// ## ⚑ ONE STYLE LEFT, AND IT IS THE MOCKUP'S GHOST BUTTON
//
// This file dressed a button AND a row. The row is gone (see the note at the
// foot), so what remains is the View Timeline button, which T5 restyles from
// the solid primary it was to the `.btn.ghost` Screen 1 draws.
//
// Palette: `--accent-primary` for the mockup's `--blue` text, and
// `--border-default` for its `--indigo-line` hairline — the mapping Roman ruled
// in the T4 follow-up, where `--accent-primary` on a BORDER was rejected.
// This app has one palette; there is no dark frame to reconcile.

import type { CSSProperties } from "react";


// ─── the button (mockup `.btn`, in the scenario header bar) ─────────────────

export const button: CSSProperties = {
  // ⚑ THE MOCKUP'S `.btn.ghost` (T5, Screen 1): outlined, accent text, PALE
  // border — NOT the solid primary this was until T5. View Timeline sits beside
  // Practice in the strip's action slot, and Screen 1's own note says why the
  // two differ: "View Timeline is the outlined button so Practice stays the one
  // solid primary". Two filled buttons side by side is two things claiming to be
  // the main action.
  //
  // The border is `--border-default`, the mapping Roman ruled for the mockup's
  // `--indigo-line` in the T4 follow-up.
  background: "var(--bg-surface)",
  color: "var(--accent-primary)",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  padding: "0.5rem 0.875rem",
  fontSize: "0.84rem",
  fontWeight: 600,
  cursor: "pointer",
  whiteSpace: "nowrap",
  fontFamily: "inherit",
  lineHeight: 1.2,
};

// ⚑ EVERYTHING BELOW THIS LINE WAS DELETED IN T5.
//
// `timelineRow`, `subsetChip`, `attachLink`, `attachList`, `attachItem`,
// `attachCheck` and `attachFoot` dressed `ScenarioTimelineRow.tsx` — the
// "Timeline: [chips] Attach…" row that rendered under this button on all five
// scenario surfaces. That row WAS defect D6: an attach control on five reading
// surfaces, which is editing done from a page nobody came to edit on. T5 moves
// attaching to `ScenarioSubsetsSection` and deletes the row, so its styles go
// with it rather than sitting here as furniture for a component that no longer
// exists.
