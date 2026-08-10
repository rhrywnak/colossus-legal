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
// Both of those pointed at `/cases/:slug/scenarios/:id` until the .382 fix — a
// route App.tsx has never declared, so BOTH ways out of this surface 404'd and
// the redundancy ruling 7 bought protected nothing. They compose through
// `utils/routePaths` now, and a test pins what it emits against what App.tsx
// declares. Nothing on this page spells a route by hand.
//
// ## Why this is its own file
//
// Rule 17: the page went past 300 lines with it inline. The seam is real — this
// is chrome, and everything left in the page is state and writes.

import React from "react";

import {
  codeStyle,
  crumbStyle,
  linkStyle,
  navButtonDisabledStyle,
  navButtonStyle,
  navGroupStyle,
  pageHeadStyle,
  pageTitleStyle,
  purposeStyle,
} from "./rehearsalStyles";
import type { RehearsalPayload, RehearsalScenario } from "../services/rehearsal";
import { scenarioPagePath, trialPrepPath } from "../utils/routePaths";
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
}) => (
  <>
    <div style={crumbStyle}>
      <Link to={trialPrepPath(slug)} style={linkStyle}>
        {wording.crumb_trial_prep_label}
      </Link>
      {scenario && (
        <>
          {" › "}
          <Link to={scenarioPagePath(slug, scenario.scenario_id)} style={linkStyle}>
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
            to={scenarioPagePath(slug, scenario.scenario_id)}
            style={{ ...navButtonStyle, textDecoration: "none" }}
          >
            {wording.scenario_page_label} ↗
          </Link>
        )}
        {/* The style follows the attribute, so an inert control never reads as
            an alive one (see `navButtonDisabledStyle`). With one ready scenario
            BOTH arrows are at a bound, which is the case Roman measured. */}
        <button
          type="button"
          onClick={onPrevious}
          disabled={atFirst}
          style={atFirst ? navButtonDisabledStyle : navButtonStyle}
        >
          ‹ {wording.previous_label}
        </button>
        <span>{position}</span>
        <button
          type="button"
          onClick={onNext}
          disabled={atLast}
          style={atLast ? navButtonDisabledStyle : navButtonStyle}
        >
          {wording.next_label} ›
        </button>
      </div>
    </div>

    <p style={purposeStyle}>{wording.purpose_line}</p>

    {/* THE FOLD-EVERYTHING CONTROLS ARE GONE (task R3).

        They opened and closed five collapsible sections; the prep page has no
        collapsible sections. It is built to be read top to bottom in one pass,
        and a fold on a surface a witness works from is a place for something to
        be missed. */}
  </>
);

export default RehearsalPageHeader;
