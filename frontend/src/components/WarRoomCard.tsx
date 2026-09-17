// =============================================================================
// WarRoomCard.tsx — one scenario on the War Room, as a horizontal status card
// =============================================================================
//
// CC_TASK_WAR_ROOM_v1, reshaped by CC_TASK_REVIEW_LOOP_v1 to the ruled mockup v3.
//
// Three panes divided by hairlines:
//   left   — code chip + the title (the scenario link), the theme, Timeline/Delete;
//   middle — EVIDENCE: facts included, candidates to rule, Matrix linked, scan day;
//   right  — PREP & REHEARSAL: "Practice →", the answered headline, one muted
//            line, the bar, and the pills.
//
// Props in, JSX out: no fetch, no state, and no decision — every string and
// every colour flag comes from `warRoomCardView`. The delete REQUEST is raised
// to the page, which owns the confirm dialog and the write.
//
// ## Click zones (the accessible-card pattern — see `WAR_ROOM_CARD_CSS`)
//
// The title link is stretched over the LEFT pane; "Practice →" over the PREP
// pane. Delete and Timeline sit above both. The EVIDENCE pane stays inert this
// build. There is NO hint text anywhere: the affordance is the hover tint and the
// pointer (the mockup's grey "click anywhere here…" is an annotation, and the
// ruling removed the other one).

import React from "react";
import { Link } from "react-router-dom";

import ScenarioTimelineDock from "./scenario-timeline/ScenarioTimelineDock";
import {
  warRoomActionStyle,
  warRoomActionsStyle,
  warRoomAnsweredStyle,
  warRoomBadgeStyle,
  warRoomBarFillStyle,
  warRoomBarTrackStyle,
  warRoomCardStyle,
  warRoomCodeChipStyle,
  warRoomDeleteStyle,
  warRoomLeftPaneStyle,
  warRoomMetaStyle,
  warRoomPaneHeadingStyle,
  warRoomPrepHeaderStyle,
  warRoomRowStyle,
  warRoomScanLineStyle,
  warRoomSidePaneStyle,
  warRoomThemeStyle,
  warRoomTitleStyle,
  warRoomValueStyle,
} from "./trialPrepCardStyles";
import { type CardRow, type WarRoomCardView, warRoomCardView } from "./warRoomCardView";
import type { ScenarioSummary, WarRoomWording } from "../pages/trialPrepData";
import { practicePath, scenarioPagePath } from "../utils/routePaths";

// STRUCTURAL: the mockup's two fixed pane widths — geometry transcribed from one
// approved drawing. Not a setting: it cannot legitimately vary by deployment or
// case, and a change is a design ruling, not a knob.
const EVIDENCE_PANE_WIDTH = "250px";
const PREP_PANE_WIDTH = "300px";

/** A pane's label/value rows. */
const Rows: React.FC<{ rows: CardRow[] }> = ({ rows }) => (
  <>
    {rows.map((r) => (
      <div key={r.label} style={warRoomRowStyle} data-row={r.label}>
        <span>{r.label}</span>
        <span style={warRoomValueStyle(r.warning)} data-warning={r.warning}>
          {r.value}
        </span>
      </div>
    ))}
  </>
);

/** The prep pane below its header: headline, muted line, bar, pills. */
const PrepBody: React.FC<{ view: WarRoomCardView }> = ({ view }) => (
  <>
    {view.answered === null ? (
      <div style={warRoomAnsweredStyle}>{view.deckNone}</div>
    ) : (
      <>
        <div style={warRoomAnsweredStyle}>
          <b>{view.answered.count}</b> {view.answered.word}
        </div>
        <div style={warRoomMetaStyle}>{view.answered.meta}</div>
        <div style={warRoomBarTrackStyle}>
          <div style={warRoomBarFillStyle(view.answered.fraction)} />
        </div>
      </>
    )}
    {view.badges.map((badge) => (
      <span key={badge.kind} style={warRoomBadgeStyle(badge.kind)} data-badge={badge.kind}>
        {badge.text}
      </span>
    ))}
  </>
);

const WarRoomCard: React.FC<{
  scenario: ScenarioSummary;
  slug: string;
  wording: WarRoomWording;
  /** Whether a timeline subset carries this scenario — the page's one read. */
  hasTimeline: boolean;
  /** Opens the page's delete confirmation for THIS scenario. */
  onRequestDelete: (scenario: ScenarioSummary) => void;
}> = ({ scenario, slug, wording, hasTimeline, onRequestDelete }) => {
  const view = warRoomCardView(scenario, wording, hasTimeline);

  return (
    <article style={warRoomCardStyle} data-pane-count={3} data-war-room-card>
      <section style={warRoomLeftPaneStyle} data-pane="identity" data-zone>
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <span style={warRoomCodeChipStyle}>{view.code}</span>
          <h2 style={warRoomTitleStyle}>
            {/* The zone's one link. Paths come from the guarded builders (Law 8). */}
            <Link to={scenarioPagePath(slug, scenario.id)} data-zone-link="scenario">
              {view.title}
            </Link>
          </h2>
        </div>
        {view.theme !== null && <p style={warRoomThemeStyle}>{view.theme}</p>}
        {/* Raised above the stretched link: the whole row is dead space. */}
        <div style={warRoomActionsStyle} data-zone-above>
          {view.actions.timeline !== null && (
            <ScenarioTimelineDock
              slug={slug}
              scenarioId={scenario.id}
              buttonLabel={view.actions.timeline}
              buttonStyle={warRoomActionStyle(false)}
            />
          )}
          <button
            type="button"
            style={warRoomDeleteStyle}
            aria-label={`${view.actions.delete} ${view.code}`}
            onClick={() => onRequestDelete(scenario)}
          >
            {view.actions.delete}
          </button>
        </div>
      </section>

      <section style={warRoomSidePaneStyle(EVIDENCE_PANE_WIDTH)} data-pane="evidence">
        <h3 style={warRoomPaneHeadingStyle}>{view.evidenceHeading}</h3>
        <Rows rows={view.evidenceRows} />
        <div style={warRoomScanLineStyle(view.scanWarning)} data-warning={view.scanWarning}>
          {view.scanLine}
        </div>
      </section>

      <section style={warRoomSidePaneStyle(PREP_PANE_WIDTH)} data-pane="prep" data-zone>
        <div style={warRoomPrepHeaderStyle}>
          <h3 style={warRoomPaneHeadingStyle}>{view.prepHeading}</h3>
          <Link
            to={practicePath(slug, scenario.id)}
            style={warRoomActionStyle(true)}
            data-zone-link="practice"
          >
            {view.actions.practice}
          </Link>
        </div>
        <PrepBody view={view} />
      </section>
    </article>
  );
};

export default WarRoomCard;
