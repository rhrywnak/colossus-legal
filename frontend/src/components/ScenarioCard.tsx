// =============================================================================
// ScenarioCard.tsx — one scenario on the Trial Prep dashboard grid
// =============================================================================
//
// Split out of `TrialPrepViews.tsx` on 2026-08-07, when the card gained its ⋯
// kebab. Two reasons, and the second is the real one:
//
//   - Size. That file was already over the module limit (Rule 17) and this card
//     is the largest thing in it.
//   - Truth. `TrialPrepViews` opens by declaring "no fetch, no state, no
//     business logic". `ScenarioKebab` holds open/closed state and a document
//     listener. Leaving the card there would have made that header a comment
//     nobody could trust — and the whole point of that file is that a reader can.
//
// This card is still props-in → JSX-out. What it now COMPOSES has state, which
// is a different claim, and it is made here rather than quietly broken there.

import React from "react";
import { Link } from "react-router-dom";

import ScenarioKebab from "./ScenarioKebab";
import { scenarioCardStyle, pillStyle } from "./trialPrepCardStyles";
import type { ScenarioSummary } from "../pages/trialPrepData";
import { patternFlagText, statusMeta } from "../pages/trialPrepHelpers";
import { scenarioPagePath } from "../utils/routePaths";

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
    // ## Why the kebab is a SIBLING of the link, not a child of it
    //
    // The whole card is the navigation target, so the card IS an `<a>`. A
    // `<button>` inside an `<a>` is invalid HTML, and every click on it would
    // also navigate — the "must not hijack the card link" rule, lost. Nesting
    // and then calling `preventDefault`/`stopPropagation` would paper over it:
    // the markup would still be invalid, and keyboard and middle-click paths
    // would still reach the anchor.
    //
    // So the anchor and the kebab are siblings inside a positioned wrapper, and
    // the kebab is placed over the card's top-right corner. Nothing to suppress,
    // because nothing overlaps: the two controls are genuinely separate.
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
            The right padding keeps the title clear of the kebab above it. */}
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

      {/* Destructive actions live ONLY behind the kebab (the D7 ruling) — there
          is deliberately no bare Delete on the card face. Same component the
          scenario page's header uses, so the two menus cannot drift apart. */}
      <div style={{ position: "absolute", top: "6px", right: "6px" }}>
        <ScenarioKebab onDelete={() => onRequestDelete(scenario)} />
      </div>
    </div>
  );
};

export default ScenarioCard;
