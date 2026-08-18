// =============================================================================
// ScenarioHeaderTiers — the v3 header (task 1.7D; supersedes 1.7C's two tiers)
// =============================================================================
//
// Mockup `.hd`: identity on the left, actions on the right, one row each side.
//
//   eyebrow   SCENARIO
//   left      S-2 · name · direction chip · [status segmented control]
//   right     ✎ Edit · Rehearsal view → · Practice ▸ · | · Delete
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

import ScenarioStatusControl from "./ScenarioStatusControl";
import { headerDescriptor } from "./scenarioHeader";
import { chipStyle, ghostButtonStyle } from "./scenarioSectionStyles";
import { statusMeta } from "../pages/trialPrepHelpers";
import type { ScenarioStatus } from "../pages/trialPrepData";
import { fillSlots } from "../services/evidenceLinks";
import { practicePath, rehearsalScenarioPath } from "../utils/routePaths";

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

/**
 * The gap between the routine actions and the destructive one.
 *
 * Not decoration: it is half of what replaced the kebab. D7's concern was
 * ADJACENCY, and a visible separator is what keeps Delete from reading as the
 * next item in a list of harmless controls.
 */
const actionSeparatorStyle: React.CSSProperties = {
  width: "1px",
  height: "18px",
  background: "var(--border-default)",
  margin: "0 4px",
};

/**
 * Delete, as a button that says what it does.
 *
 * Danger-coloured TEXT on a plain background rather than a filled red button: it
 * must be findable and unmistakable, not the loudest thing on the page. The
 * confirm dialog is the commitment step; this is the request.
 */
const deleteButtonStyle: React.CSSProperties = {
  border: "none",
  background: "none",
  padding: "6px 8px",
  borderRadius: "8px",
  cursor: "pointer",
  color: "var(--state-danger-strong)",
  fontFamily: "inherit",
  fontSize: "13.5px",
};

/** Mockup `.link`: accent, 13.5px, no underline. */
const linkStyle: React.CSSProperties = {
  color: "var(--accent-primary)",
  fontSize: "13.5px",
  textDecoration: "none",
  whiteSpace: "nowrap",
};

/**
 * The same control, inert, on a scenario that is not Ready.
 *
 * The style follows the fact, exactly as `navButtonDisabledStyle` does on the
 * rehearsal page's bounded arrows: muted text and `not-allowed`, so a control
 * that will not go anywhere does not read as one that will. An accent-coloured
 * link that silently does nothing is worse than either state alone.
 */
const blockedLinkStyle: React.CSSProperties = {
  ...linkStyle,
  color: "var(--text-disabled)",
  cursor: "not-allowed",
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
  /** Opens the delete confirmation. The header never deletes anything itself —
   *  the page owns the dialog, which names the scenario and stays open on
   *  failure (the mis-click guard, since Roman overruled D7 for Delete). */
  onDelete: () => void;
  /** The status control saved; the page re-fetches. */
  onReadyChanged: () => void;
  /**
   * Why the rehearsal control is inert, as a `{status}` template — or `null`
   * while the identity payload has not loaded.
   *
   * `null` renders NO rehearsal control at all, in either state. That is the
   * honest-gap law and it is the same rule `AccusationSection` follows with
   * `if (!panel) return null`: there is no literal to fall back to (v2 §2b), and
   * a control that cannot say why it is inert is worse than an absent one. The
   * page's own augmentation-failure notice is what says why it is missing.
   */
  rehearsalBlockedTemplate: string | null;
  /**
   * The stored label on the practice control — or `null` while the identity
   * payload has not loaded, which withdraws the control entirely.
   *
   * The same honest-gap rule its neighbour follows: there is no literal to fall
   * back to (v2 §2b), and a control whose own name this build had to invent is
   * one Roman cannot rename in Settings.
   */
  practiceLabel: string | null;
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
  rehearsalBlockedTemplate,
  practiceLabel,
}) => {
  const header = headerDescriptor({ code, name, direction, status });

  // THE .389 DEFECT, and the whole reason this control changed (audit defects
  // 1-4; Roman's round trip of 2026-08-09).
  //
  // This link used to be `rehearsalPath(slug)` — the CASE-level address, with no
  // scenario on it — and it rendered identically whatever the status was. Clicked
  // on S-5 (Draft) it landed on the rehearsal page with no code in the URL, which
  // made that page's "did the address name something?" test false, which left its
  // index clamped at 0, which rendered S-2 — the only Ready scenario — under S-2's
  // own title with no notice of any kind. From there both ways out composed their
  // address from the scenario on screen, so "Scenario page →" returned to S-2 as
  // well. One missing argument, two silent substitutions.
  //
  // Two things fix it and they only work together: the address now CARRIES the
  // code (so the receiving page can refuse by name), and the control is inert
  // unless the scenario is actually Ready (so the refusal is normally never
  // reached). `rehearsalAddress.test.ts` pins both halves.
  const ready = status === "ready";
  // Filled from the status this header is already rendering, so the sentence
  // names the state the reader can see rather than assuming "Draft". The column
  // still permits `needs_evidence` (ruling 6), and `statusMeta` is the one place
  // that turns a status into a word — the same one the dashboard card reads.
  const blockedReason = rehearsalBlockedTemplate
    ? fillSlots(rehearsalBlockedTemplate, { status: statusMeta(status).label })
    : null;

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

        {/* The address carries THIS scenario's code (§2a's stable handle, and
            what the rehearsal route declares) rather than the case. A Ready
            scenario goes to its own rehearsal; nothing else can be reached from
            here at all. */}
        {blockedReason !== null &&
          (ready ? (
            <Link
              to={rehearsalScenarioPath(slug, code)}
              style={linkStyle}
              title="Marie's testimony-prep view — shows scenarios marked Ready"
            >
              Rehearsal view →
            </Link>
          ) : (
            // A `<span>`, not a disabled `<button>` or an `<a>` with no href: it
            // is not a control that failed, it is a destination that does not
            // exist yet. The reason sits in the title AND beside it, because the
            // hover-tooltip-as-only-explanation is exactly what let the broken
            // version look alive for eleven days.
            <span style={blockedLinkStyle} title={blockedReason}>
              Rehearsal view →
            </span>
          ))}
        {blockedReason !== null && !ready && (
          <span style={{ fontSize: "12px", color: "var(--text-muted)", maxWidth: "22rem" }}>
            {blockedReason}
          </span>
        )}

        {/* PRACTICE v0 (2026-08-17): the way into Marie's drill, in the
            rehearsal area of the header where the task placed it.

            ## Why this control is NOT gated on Ready, when its neighbour is

            "Rehearsal view →" is inert on a drafted scenario because rehearsal is
            the surface a witness is TAKEN TO — showing her a scenario nobody
            declared finished is the defect that ruling closed. The drill is the
            opposite kind of destination: it is where Roman and Chuck find out
            whether a deck is any good, on scenarios that are still being built,
            and the page it opens says in the store's own words when a scenario
            has no deck yet. Gating it would hide the only screen that can report
            that state.

            The label's ▸ is the mockup's, and it is a right-pointing triangle
            rather than an arrow because the drill OPENS rather than navigates
            away — four screens live behind it. */}
        {practiceLabel !== null && (
          <Link
            to={practicePath(slug, scenarioId)}
            style={linkStyle}
            title="Twenty minutes, one accusation — Marie answers, and Chuck gets the sheet"
          >
            {practiceLabel}
          </Link>
        )}

        {/* DELETE, VISIBLE (Roman overruled D7 for Delete on 2026-08-07).

            D7 put Delete behind a ⋯ kebab because a bare Delete had sat one
            mis-click from the ready-to-rehearse control. Roman asked twice for a
            button and got a menu twice; his ruling is that the confirm dialog —
            which names the scenario and stays open on failure — is the guard, and
            distance does the rest.

            DISTANCE, concretely: the readiness control lives in the IDENTITY row
            above, not here, and this button sits last in the actions row behind a
            separator. Nothing destructive is adjacent to anything routine. */}
        <span style={actionSeparatorStyle} aria-hidden="true" />
        <button
          type="button"
          style={deleteButtonStyle}
          title="Delete this scenario — asks for confirmation first"
          onClick={onDelete}
        >
          Delete
        </button>
      </div>
    </div>
  );
};

export default ScenarioHeaderTiers;
