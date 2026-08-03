// =============================================================================
// ScenarioHeaderTiers — the v3 header (task 1.7D; supersedes 1.7C's two tiers)
// =============================================================================
//
// Mockup `.hd`: identity on the left, actions on the right, one row each side.
//
//   eyebrow   SCENARIO
//   left      S-2 · name · direction chip · [status segmented control]
//   right     ✎ Edit · Rehearsal view → · ⋯
//
// ## What changed from 1.7C
//
// * The STATUS CHIP IS GONE (ruling R4). The segmented control is the single
//   status surface on the page; a chip beside it would be two renderings of one
//   fact, which is the defect item 3 exists to close.
// * `ReadyToggle`'s two-directional BUTTON is gone with it. A button states the
//   action available, not the state you are in, so the reader had to invert its
//   label to learn the status — see `ScenarioStatusControl` for the full reasoning.
// * The status control sits in the IDENTITY row, not the actions row. It is a
//   statement about what this scenario IS, and v3 groups it with the name.
// * "Rehearsal mode →" reads "Rehearsal view →", the mockup's wording, with its
//   explanatory title attribute.
// * `align-items: center` (v3) rather than `baseline` (1.7C) — the control and the
//   chip are boxes now, and baseline alignment made them sit low against the title.
//
// ## Renders only
//
// The chips and status vocabulary arrive from `headerDescriptor` (pure, tested).
// The readiness VERDICT slot still renders nothing until task 2.4 computes one —
// that is a different thing from the draft/ready status, and conflating them is
// exactly what §9 warns against.

import React from "react";
import { Link } from "react-router-dom";

import ScenarioKebab from "./ScenarioKebab";
import ScenarioStatusControl from "./ScenarioStatusControl";
import { headerDescriptor } from "./scenarioHeader";
import { chipStyle, ghostButtonStyle } from "./scenarioSectionStyles";
import type { ScenarioStatus } from "../pages/trialPrepData";

/** Mockup `.eyebrow`: 11px/600, .1em tracking, uppercase, --text-3. */
const eyebrowStyle: React.CSSProperties = {
  fontSize: "11px",
  fontWeight: 600,
  letterSpacing: "0.1em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  marginBottom: "4px",
};

/** Mockup `.hd`: the two sides, top-aligned, 24px apart. */
const headerRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "flex-start",
  justifyContent: "space-between",
  gap: "24px",
  // NO wrap: the mockup's `.hd` keeps the actions on the title's row. With wrap on,
  // a long scenario name pushed Edit / Rehearsal / ⋯ onto a second line, which read
  // as a stray toolbar — the first thing the screenshot comparison caught.
};

/** Mockup `.hd-title`: centre-aligned, 12px gaps. */
const identityRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "12px",
  flexWrap: "wrap",
};

/** Mockup `.scode`: 19px/600, --text-3. */
const codeStyle: React.CSSProperties = {
  fontSize: "19px",
  fontWeight: 600,
  color: "var(--text-muted)",
};

/** Mockup `h1`: 23px/600, -.01em. */
const nameStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "23px",
  fontWeight: 600,
  letterSpacing: "-0.01em",
  color: "var(--text-primary)",
};

/** Mockup `.hd-actions`: centre-aligned, 10px gaps, 14px top pad. */
const actionsRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "10px",
  flexShrink: 0,
  paddingTop: "14px",
};

/** Mockup `.link`: accent, 13.5px, no underline. */
const linkStyle: React.CSSProperties = {
  color: "var(--accent-primary)",
  fontSize: "13.5px",
  textDecoration: "none",
  whiteSpace: "nowrap",
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
  /** The status control saved; the page re-fetches. */
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
    <div style={headerRowStyle}>
      <div>
        {/* Roman's ruling 2026-08-03: the eyebrow lets the page read "Scenario
            S-2: …" vertically without congesting the title line with the word. */}
        <div style={eyebrowStyle}>Scenario</div>

        <div style={identityRowStyle}>
          {/* The code sits outside the name because it is the one part of this
              header that survives a rename (§2a). */}
          <span style={codeStyle}>{header.code}</span>
          <h1 className="count-header" style={nameStyle}>
            {header.name}
          </h1>

          {/* Mockup `.chip.dir`: red-soft fill, red text. Direction is read-only —
              flipping it would make this a different scenario, and the backend
              refuses it on the update route. The title says so. */}
          <span
            style={{
              ...chipStyle,
              background: "var(--state-danger-bg-soft)",
              color: "var(--v3-red-text)",
            }}
            title={header.direction.title ?? undefined}
          >
            {header.direction.label}
          </span>

          {/* The single status surface (item 3 / R4). No status chip beside it. */}
          <ScenarioStatusControl
            slug={slug}
            scenarioId={scenarioId}
            status={status}
            onChanged={onReadyChanged}
          />

          {/* The readiness VERDICT slot (§9 / task 2.4) — a different claim from
              draft/ready status: whether this scenario can be taken into a
              courtroom. `readiness` is null until 2.4 computes one and NOTHING
              renders. Not "Unknown", not a grey placeholder. */}
          {header.readiness && (
            <span style={{ ...chipStyle, color: header.readiness.color }}>
              {header.readiness.label}
            </span>
          )}
        </div>
      </div>

      <div style={actionsRowStyle}>
        <button
          type="button"
          style={ghostButtonStyle}
          title="Edit this scenario's identity — name, definition, attack, theme, motivation, allegations"
          aria-label="Edit scenario identity"
          onClick={onEdit}
        >
          ✎ Edit
        </button>

        <Link
          to={`/cases/${encodeURIComponent(slug)}/rehearsal`}
          style={linkStyle}
          title="Marie's testimony-prep view — shows scenarios marked Ready"
        >
          Rehearsal view →
        </Link>

        {/* Destructive actions live ONLY here (D7). */}
        <ScenarioKebab onDelete={onDelete} />
      </div>
    </div>
  );
};

export default ScenarioHeaderTiers;
