// =============================================================================
// ScenarioCard.tsx — one scenario on the Trial Prep dashboard grid
// =============================================================================
//
// Split out of `TrialPrepViews.tsx` on 2026-08-07, when the card gained its ⋯
// kebab: that file was already over the module limit (Rule 17) and this card is
// the largest thing in it. The kebab is gone (2026-08-08 — Roman overruled D7 for
// Delete and this card carries a visible Delete instead), but the split stays:
// the size argument was the durable half, and moving the card back would put
// `TrialPrepViews` over the limit again for no gain.
//
// This card is props-in → JSX-out: no fetch, no state. The delete REQUEST is
// raised to the page, which owns the confirm dialog and the write.

import React from "react";
import { Link } from "react-router-dom";

import { scenarioCardStyle, pillStyle } from "./trialPrepCardStyles";
import type { ScenarioSummary } from "../pages/trialPrepData";
import { patternFlagText, statusMeta } from "../pages/trialPrepHelpers";
import { scenarioPagePath } from "../utils/routePaths";

/**
 * Delete on the card face.
 *
 * Danger-coloured text, no fill: it has to be findable on a grid of cards without
 * competing with the card's own title. Sized down from the header's copy — this
 * one sits on twelve cards at once, and twelve red buttons would make the
 * dashboard look like a list of problems.
 */
const cardDeleteButtonStyle: React.CSSProperties = {
  border: "none",
  background: "none",
  padding: "4px 6px",
  borderRadius: "6px",
  cursor: "pointer",
  color: "var(--state-danger-strong)",
  fontFamily: "inherit",
  fontSize: "0.78rem",
};

const ScenarioCard: React.FC<{
  scenario: ScenarioSummary;
  slug: string;
  /** Opens the delete confirmation for THIS scenario. The card never deletes
   *  anything itself — it asks the page, which owns the dialog. */
  onRequestDelete: (scenario: ScenarioSummary) => void;
}> = ({ scenario, slug, onRequestDelete }) => {
  const status = statusMeta(scenario.status);
  const flag = patternFlagText(scenario.baseless_repeat_count);
  return (
    // ## Why Delete is a SIBLING of the link, not a child of it
    //
    // The whole card is the navigation target, so the card IS an `<a>`. A
    // `<button>` inside an `<a>` is invalid HTML, and every click on it would
    // also navigate — the "must not hijack the card link" rule, lost. Nesting
    // and then calling `preventDefault`/`stopPropagation` would paper over it:
    // the markup would still be invalid, and keyboard and middle-click paths
    // would still reach the anchor.
    //
    // So the anchor and the button are siblings inside a positioned wrapper, and
    // the button is placed over the card's bottom-right corner. Nothing to
    // suppress, because nothing overlaps: the two controls are genuinely
    // separate. This reasoning survived the kebab it was written for.
    <div style={{ position: "relative", display: "flex" }}>
      <Link
        to={scenarioPagePath(slug, scenario.id)}
        style={{
          ...scenarioCardStyle,
          textDecoration: "none",
          color: "var(--text-primary)",
          flex: 1,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <span
            style={{
              width: "9px",
              height: "9px",
              borderRadius: "50%",
              backgroundColor: status.color,
              flexShrink: 0,
            }}
          />
          <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>{status.label}</span>
        </div>
        {/* The scenario's code prefixes its name everywhere the scenario appears
            (§2a). Rendered as plain text in the existing title line rather than as
            a new chip or row: the code is part of how the scenario is NAMED, so it
            reads as "S-3 · Marie is obstructive", and no layout changes. The string
            arrives fully formatted from the backend — the browser never builds it.
            The right padding is kept: it holds the title's line length steady
            against the card's edge. */}
        <div
          style={{
            fontSize: "0.95rem",
            fontWeight: 600,
            color: "var(--text-primary)",
            paddingRight: "1.5rem",
          }}
        >
          <span style={{ color: "var(--text-muted)", fontVariantNumeric: "tabular-nums" }}>
            {scenario.code}
          </span>
          {" · "}
          {scenario.attack}
        </div>
        <span
          style={{
            ...pillStyle,
            alignSelf: "flex-start",
            backgroundColor: flag.muted ? "var(--bg-page)" : "var(--state-info-bg-soft)",
            color: flag.muted ? "var(--text-muted)" : "var(--accent-primary)",
          }}
        >
          {flag.text}
        </span>
        {/* The "N instances · no speakers yet · N responses" line was removed on
            2026-08-07 with the metric that led it (see `MetricsBand`). */}
        {/* Visual affordance only — the whole card navigates, so this is plain
            text, not a separate link. */}
        <span style={{ fontSize: "0.82rem", color: "var(--accent-primary)", marginTop: "auto" }}>
          Open scenario →
        </span>
      </Link>

      {/* DELETE, VISIBLE (Roman overruled D7 for Delete on 2026-08-07).

          It replaces the ⋯ kebab that stood here, which held Delete and nothing
          else — a menu whose only item is the thing you came for is a click tax,
          and Roman asked for a button twice before getting one.

          The mis-click guard is unchanged and is the same one D7 relied on: this
          opens the page's confirm dialog, which names the scenario and stays open
          if the delete fails. What changed is that the control says what it does.

          PLACEMENT: bottom-right, diagonally opposite the card's title and as far
          as this card can put it from "Open scenario →" at the bottom-LEFT. */}
      <div style={{ position: "absolute", bottom: "8px", right: "10px" }}>
        <button
          type="button"
          style={cardDeleteButtonStyle}
          title={`Delete ${scenario.code} — asks for confirmation first`}
          aria-label={`Delete scenario ${scenario.code}`}
          onClick={() => onRequestDelete(scenario)}
        >
          Delete
        </button>
      </div>
    </div>
  );
};

export default ScenarioCard;
