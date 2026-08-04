// =============================================================================
// ScenarioNotice — one banner shape for the page's warnings (2.12 extraction)
// =============================================================================
//
// The scenario page carries two of these: the re-read notice (task 1.7F Part A)
// and the accusation-catalogue failure (task 2.12). They had identical markup
// and identical styling written out twice, which is one edit away from drifting
// into two different-looking warnings on one screen.
//
// Extracted when 2.12's banner took `ScenarioDetailPage` past the 300-line limit
// (Rule 17). The seam is the ordinary one: this is WHAT A WARNING LOOKS LIKE and
// the page is WHICH WARNINGS EXIST.
//
// ## Why the actions are children rather than props
//
// The two banners offer different actions — one has Try again and Dismiss, the
// other has none, because a catalogue that failed to load is not retried from
// here and cannot be dismissed without hiding the reason two controls are
// missing. Modelling that as `onRetry?` / `onDismiss?` would bake this page's
// two cases into a component that has no opinion about either.

import React from "react";

/**
 * A page-level warning that leaves every section on screen and working.
 *
 * `role="alert"` because these appear in response to something the human just
 * did (or something that just failed under them), and a warning nobody is told
 * about is the silent-failure defect in visual form.
 */
export const ScenarioNotice: React.FC<{
  message: string;
  children?: React.ReactNode;
}> = ({ message, children }) => (
  <div
    role="alert"
    style={{
      display: "flex",
      gap: "0.75rem",
      alignItems: "baseline",
      flexWrap: "wrap",
      border: "1px solid var(--state-warning-strong, var(--border-default))",
      borderRadius: "8px",
      padding: "0.6rem 0.8rem",
      margin: "0.5rem 0",
      fontSize: "0.85rem",
      color: "var(--text-secondary)",
    }}
  >
    <span>{message}</span>
    {children}
  </div>
);

export default ScenarioNotice;
