// =============================================================================
// RehearsalPicker — the rehearsal front door (task R1 Piece 1d)
// =============================================================================
//
// The list Marie meets at `/cases/:slug/rehearsal`: every Ready scenario, by
// code and name, each a link to its own rehearsal address.
//
// ## What this replaced, and why a list beats a default
//
// The bare address used to open on the FIRST ready scenario and say nothing
// about having chosen one. On a case with exactly one ready scenario that looked
// identical to a considered answer — which is what made the .389 defect so hard
// to see from the screen. A Draft scenario's "Rehearsal view" control (which
// carried no scenario at all) landed here, and here rendered S-2's entire
// rehearsal under S-2's title, with no notice of any kind. The reader had asked
// for S-5.
//
// Roman's ruling of 2026-08-10: nothing is ever shown that Marie did not pick.
// A list of one is still a pick — and the day a second scenario is declared
// ready, "the only one" and "the one you chose" stop being the same sentence.
//
// ## Every word here is a stored row
//
// The heading and the empty-state sentence are settings rows. The code and the
// name are the scenario's own, composed by the backend and rendered verbatim.
// This component invents no vocabulary — the only characters it contributes are
// the separator between a code and a name, and the arrow on the link.

import React from "react";
import { Link } from "react-router-dom";

import type { RehearsalScenario } from "../services/rehearsal";
import { rehearsalScenarioPath } from "../utils/routePaths";

const listStyle: React.CSSProperties = {
  listStyle: "none",
  margin: "18px 0 0",
  padding: 0,
  display: "flex",
  flexDirection: "column",
  gap: "10px",
  maxWidth: "44rem",
};

/**
 * One row: a whole-width target, because this is read under stress.
 *
 * The card is the link rather than carrying one, so there is no small text to
 * aim at — the same reasoning `ScenarioCard` states for the dashboard's tiles.
 */
const rowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  gap: "10px",
  padding: "14px 18px",
  borderRadius: "10px",
  background: "var(--bg-surface)",
  boxShadow: "var(--shadow-card)",
  textDecoration: "none",
  color: "var(--text-primary)",
};

const codeStyle: React.CSSProperties = {
  fontSize: "15px",
  fontWeight: 600,
  color: "var(--text-muted)",
  flexShrink: 0,
};

const nameStyle: React.CSSProperties = {
  fontSize: "17px",
  fontWeight: 500,
};

const headingStyle: React.CSSProperties = {
  fontSize: "17px",
  fontWeight: 600,
  margin: "24px 0 0",
};

interface Props {
  slug: string;
  /** The list's title — a stored row. */
  heading: string;
  /** Shown when no scenario is ready. The SAME row the page has always used for
   *  this state, reused rather than duplicated: one question, one sentence. */
  emptyNotice: string;
  /** The ready scenarios, in the order the server sent them. Never re-sorted
   *  here — the backend owns the ordering, as it does for the card pool. */
  scenarios: RehearsalScenario[];
}

const RehearsalPicker: React.FC<Props> = ({ slug, heading, emptyNotice, scenarios }) => {
  // Nobody has declared anything ready. A real state with a stated remedy, not a
  // failure and not an empty list rendered under a heading that promises one.
  if (scenarios.length === 0) {
    return <p style={{ fontSize: "17px", marginTop: "24px" }}>{emptyNotice}</p>;
  }

  return (
    <>
      <h2 style={headingStyle}>{heading}</h2>
      <ul style={listStyle}>
        {scenarios.map((s) => (
          <li key={s.code}>
            <Link to={rehearsalScenarioPath(slug, s.code)} style={rowStyle}>
              {/* The code first, because it is the handle that survives a rename
                  and the thing a human says out loud (§2a). */}
              <span style={codeStyle}>{s.code}</span>
              <span style={nameStyle}>{s.title}</span>
            </Link>
          </li>
        ))}
      </ul>
    </>
  );
};

export default RehearsalPicker;
