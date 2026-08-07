// =============================================================================
// trialPrepCardStyles.ts — the two style objects the dashboard grid and the
// timeline views both use
// =============================================================================
//
// Extracted on 2026-08-07 when `ScenarioCard` moved to its own file: both it and
// `TrialPrepViews` need `pillStyle`, and a second copy of a shape token is how
// two pills stop looking alike. Design tokens only — no literal colors (Rule 2).

import React from "react";

/** The dashboard grid's card frame. */
export const scenarioCardStyle: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  padding: "14px 16px",
  display: "flex",
  flexDirection: "column",
  gap: "8px",
};

/** The rounded chip used for status/pattern flags. Callers supply the colors. */
export const pillStyle: React.CSSProperties = {
  display: "inline-block",
  padding: "0.12rem 0.5rem",
  borderRadius: "9999px",
  fontSize: "0.72rem",
  fontWeight: 600,
};
