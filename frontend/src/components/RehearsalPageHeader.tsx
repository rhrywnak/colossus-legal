// =============================================================================
// RehearsalPageHeader — the way in, the way around, and the way back
// =============================================================================
//
// The mockup's `.crumb`, `.pagehead`, `.purpose` and `.foldrow`: the breadcrumb,
// the scenario's name, the four header controls, the purpose line, and the two
// fold-everything buttons.
//
// ## Two ways back, on purpose (ruling 7 of the signed set)
//
// The breadcrumb links to Trial Prep and to this scenario's working page, and
// the header carries a NAMED "Scenario page ↗" control besides. The browser's
// Back button must never be the only way out of a surface a witness is reading
// from under stress — and a breadcrumb is a convention some readers do not use.
//
// ## Why this is its own file
//
// Rule 17: the page went past 300 lines with it inline. The seam is real — this
// is chrome, and everything left in the page is state and writes.

import React from "react";

import {
  codeStyle,
  crumbStyle,
  foldButtonStyle,
  foldRowStyle,
  linkStyle,
  navButtonStyle,
  navGroupStyle,
  pageHeadStyle,
  pageTitleStyle,
  purposeStyle,
} from "./rehearsalStyles";
import type { RehearsalPayload, RehearsalScenario } from "../services/rehearsal";
import { Link } from "react-router-dom";

interface Props {
  slug: string;
  wording: RehearsalPayload["wording"];
  /** The scenario on screen, or `undefined` when the case has none ready. */
  scenario: RehearsalScenario | undefined;
  /**
   * "Scenario 2 of 5", composed server-side.
   *
   * `null` when the case has no ready scenario at all — there is no position to
   * be at, and the control group renders the two arrows around nothing rather
   * than a made-up "Scenario 0 of 0".
   */
  position: string | null;
  onPrevious: () => void;
  onNext: () => void;
  atFirst: boolean;
  atLast: boolean;
  onOpenAll: () => void;
  onFoldAll: () => void;
}

const RehearsalPageHeader: React.FC<Props> = ({
  slug,
  wording,
  scenario,
  position,
  onPrevious,
  onNext,
  atFirst,
  atLast,
  onOpenAll,
  onFoldAll,
}) => (
  <>
    <div style={crumbStyle}>
      <Link to={`/cases/${slug}/trial-prep`} style={linkStyle}>
        {wording.crumb_trial_prep_label}
      </Link>
      {scenario && (
        <>
          {" › "}
          <Link to={`/cases/${slug}/scenarios/${scenario.scenario_id}`} style={linkStyle}>
            {scenario.code} · {scenario.title}
          </Link>
        </>
      )}
      {" › "}
      {wording.page_heading}
    </div>

    <div style={pageHeadStyle}>
      <h1 style={pageTitleStyle}>
        {scenario ? (
          <>
            <span style={codeStyle}>{scenario.code}</span> · {scenario.title}
          </>
        ) : (
          wording.page_heading
        )}
      </h1>
      <div style={{ flex: 1 }} />
      <div style={navGroupStyle}>
        {scenario && (
          <Link
            to={`/cases/${slug}/scenarios/${scenario.scenario_id}`}
            style={{ ...navButtonStyle, textDecoration: "none" }}
          >
            {wording.scenario_page_label} ↗
          </Link>
        )}
        <button type="button" onClick={onPrevious} disabled={atFirst} style={navButtonStyle}>
          ‹ {wording.previous_label}
        </button>
        <span>{position}</span>
        <button type="button" onClick={onNext} disabled={atLast} style={navButtonStyle}>
          {wording.next_label} ›
        </button>
      </div>
    </div>

    <p style={purposeStyle}>{wording.purpose_line}</p>

    {/* Both directions, because a reader who wants the whole page should not
        click five carets — and one who wants to work through it one block at a
        time should not close them one at a time either. */}
    {scenario && (
      <div style={foldRowStyle}>
        <button type="button" onClick={onOpenAll} style={foldButtonStyle}>
          {wording.expand_all_label}
        </button>
        <button type="button" onClick={onFoldAll} style={foldButtonStyle}>
          {wording.collapse_all_label}
        </button>
      </div>
    )}
  </>
);

export default RehearsalPageHeader;
