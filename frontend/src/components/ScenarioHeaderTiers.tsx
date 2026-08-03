// =============================================================================
// ScenarioHeaderTiers — the two-tier header + eyebrow (defect D7, §2.1)
// =============================================================================
//
// The 1.7B header was ONE line carrying the code, the name, two chips and four
// actions. Defect D7 is the evidence that a detail page cannot do that: the line
// wrapped unpredictably, and `Delete` ended up sitting between `Mark ready to
// rehearse` and the rehearsal link.
//
// §2.1's answer, and the deviation from the study is stated per §2c:
//
//   eyebrow  SCENARIO
//   tier 1   S-2 · name · direction chip · status chip · [verdict slot, 2.4]
//   tier 2   ✎ Edit · Mark ready to rehearse · Rehearsal mode → · ⋯
//
// The study's one-line lean header stays on the Trial Prep LIST page, where a row
// carries identity and nothing else. A DETAIL page carries identity PLUS four
// actions, and two tiers is what restores the uniformity the one-liner was for.
//
// ## Renders only
//
// The chips and the status vocabulary arrive from `headerDescriptor` (pure,
// tested). The readiness slot renders NOTHING until task 2.4 computes a verdict —
// not "Unknown", not a grey placeholder. A verdict is a claim about whether this
// scenario can be taken into a courtroom.

import React from "react";
import { Link } from "react-router-dom";

import ReadyToggle from "./ReadyToggle";
import ScenarioKebab from "./ScenarioKebab";
import { headerDescriptor } from "./scenarioHeader";
import type { ScenarioStatus } from "../pages/trialPrepData";

const eyebrowStyle: React.CSSProperties = {
  fontSize: "0.7rem",
  fontWeight: 600,
  letterSpacing: "0.1em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  marginBottom: "0.15rem",
};

const tierOneStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  flexWrap: "wrap",
  gap: "0.6rem",
};

const tierTwoStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  flexWrap: "wrap",
  gap: "0.6rem",
  marginTop: "0.6rem",
};

const codeStyle: React.CSSProperties = {
  fontSize: "1rem",
  fontWeight: 600,
  color: "var(--text-muted)",
  letterSpacing: "0.02em",
};

// Regular-ish weight, not bold: §2c reserves bold for true emphasis, and a page
// title is not it.
const nameStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "1.25rem",
  fontWeight: 500,
  color: "var(--text-primary)",
};

const chipStyle: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "999px",
  padding: "0.1rem 0.6rem",
  fontSize: "0.74rem",
  whiteSpace: "nowrap",
};

const quietButton: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  background: "var(--bg-surface)",
  color: "var(--text-secondary)",
  fontSize: "0.8rem",
  fontFamily: "inherit",
  padding: "0.3rem 0.7rem",
  cursor: "pointer",
};

interface Props {
  slug: string;
  scenarioId: string;
  code: string;
  name: string;
  direction: string;
  status: ScenarioStatus;
  /** Opens the ONE identity modal (§2.1's one-modal law). */
  onEdit: () => void;
  /** Opens the delete confirmation, from inside the kebab and nowhere else. */
  onDelete: () => void;
  /** The ready gate saved; the page re-fetches. */
  onReadyChanged: () => void;
}

const ScenarioHeaderTiers: React.FC<Props> = ({
  slug,
  scenarioId,
  code,
  name,
  direction,
  status,
  onEdit,
  onDelete,
  onReadyChanged,
}) => {
  const header = headerDescriptor({ code, name, direction, status });

  return (
    <div>
      {/* Roman's ruling 2026-08-03: the eyebrow lets the page read "Scenario S-2:
          …" vertically without congesting the title line with the word. */}
      <div style={eyebrowStyle}>Scenario</div>

      <div style={tierOneStyle}>
        {/* The code sits outside the name because it is the one part of this
            header that survives a rename (§2a). */}
        <span style={codeStyle}>{header.code}</span>
        <h1 className="count-header" style={nameStyle}>
          {header.name}
        </h1>
        <span
          style={{ ...chipStyle, color: header.direction.color }}
          title={header.direction.title ?? undefined}
        >
          {header.direction.label}
        </span>
        <span style={{ ...chipStyle, color: header.status.color }}>{header.status.label}</span>

        {/* The readiness verdict's reserved slot (§9 / task 2.4). `readiness` is
            null until then and NOTHING renders — see the module header. */}
        {header.readiness && (
          <span style={{ ...chipStyle, color: header.readiness.color }}>
            {header.readiness.label}
          </span>
        )}
      </div>

      <div style={tierTwoStyle}>
        <button
          type="button"
          style={quietButton}
          title="Edit this scenario's identity — name, definition, attack, theme, motivation, allegations"
          aria-label="Edit scenario identity"
          onClick={onEdit}
        >
          ✎ Edit
        </button>

        {/* The ready gate (task 1.5, v2 §5): the only path that changes status,
            because a readiness declaration is a human act with a name recorded
            against it. The generic update route refuses `status` outright. */}
        <ReadyToggle
          slug={slug}
          scenarioId={scenarioId}
          ready={status === "ready"}
          onChanged={onReadyChanged}
        />

        <Link
          to={`/cases/${encodeURIComponent(slug)}/rehearsal`}
          style={{ fontSize: "0.82rem" }}
        >
          Rehearsal mode →
        </Link>

        {/* Destructive actions live ONLY here (D7). `marginLeft: auto` pushes the
            kebab to the far right, as far from the primary action as the row
            allows. */}
        <span style={{ marginLeft: "auto" }}>
          <ScenarioKebab onDelete={onDelete} />
        </span>
      </div>
    </div>
  );
};

export default ScenarioHeaderTiers;
