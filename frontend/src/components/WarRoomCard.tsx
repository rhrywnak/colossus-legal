// =============================================================================
// WarRoomCard.tsx — one scenario on the War Room, as a horizontal status card
// =============================================================================
//
// CC_TASK_WAR_ROOM_v1, drawn on WAR_ROOM_CARD_MOCKUP_v1_2026-09-16. It replaces
// `ScenarioCard.tsx`, whose "Ready" pill said the same word on all eleven cards
// and told nobody their next move. This card reports the pipeline instead:
// evidence ruled → linked into the Matrix → prep written → deck built → answered.
//
// Three panes divided by hairlines:
//   left   — code chip + name, the theme statement (3 lines), the action row;
//   middle — EVIDENCE: facts included, candidates to rule, Matrix linked, scan;
//   right  — PREP & REHEARSAL: talking points, watch items, deck, answered, badge.
//
// Props in, JSX out: no fetch, no state, and no decision — every string and
// every warning flag comes from `warRoomCardView`. The delete REQUEST is raised
// to the page, which owns the confirm dialog and the write, exactly as before.
//
// ## Deviations from the mockup, ordered (CC_GO_WAR_ROOM_v1 / v3)
//
// - No ⋯ menu. Delete is VISIBLE, last in the action row and quieter than the
//   other three (Q5 — Roman overruled the kebab on 2026-08-07 and it stands).
// - The changed pill is amber and reads "N new or changed for Marie" (v3); the
//   mockup's blue "Marie has N changed questions" belonged to a withdrawn rule.
// - No "new answers for you" badge: that is CC_TASK_REVIEW_LOOP_v1.

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
  warRoomPaneHeadingStyle,
  warRoomRowStyle,
  warRoomScanLineStyle,
  warRoomSidePaneStyle,
  warRoomThemeStyle,
  warRoomValueStyle,
} from "./trialPrepCardStyles";
import { type CardRow, warRoomCardView } from "./warRoomCardView";
import type { ScenarioSummary, WarRoomWording } from "../pages/trialPrepData";
import { practicePath, scenarioPagePath } from "../utils/routePaths";

// STRUCTURAL: the mockup's two fixed pane widths — geometry transcribed from one
// approved drawing. Not a setting: it cannot legitimately vary by deployment or
// case, and a change is a design ruling, not a knob.
const EVIDENCE_PANE_WIDTH = "250px";
const PREP_PANE_WIDTH = "280px";

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
    <article style={warRoomCardStyle} data-pane-count={3}>
      <section style={warRoomLeftPaneStyle} data-pane="identity">
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <span style={warRoomCodeChipStyle}>{view.code}</span>
          <h2 style={{ margin: 0, fontSize: "1.3rem", color: "var(--text-primary)" }}>
            {view.title}
          </h2>
        </div>
        {view.theme !== null && <p style={warRoomThemeStyle}>{view.theme}</p>}
        {/* The action row. Every control is a real link or button of its own —
            the card itself is no longer one big link, so nothing here is nested
            inside an anchor. Paths come from the guarded builders (Law 8). */}
        <div style={warRoomActionsStyle}>
          <Link to={scenarioPagePath(slug, scenario.id)} style={warRoomActionStyle(true)}>
            {view.actions.open}
          </Link>
          <Link to={practicePath(slug, scenario.id)} style={warRoomActionStyle(false)}>
            {view.actions.practice}
          </Link>
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

      <section style={warRoomSidePaneStyle(PREP_PANE_WIDTH)} data-pane="prep">
        <h3 style={warRoomPaneHeadingStyle}>{view.prepHeading}</h3>
        <Rows rows={view.prepRows} />
        {view.answered !== null && (
          <>
            <div style={warRoomAnsweredStyle}>
              <b style={{ color: "var(--text-primary)" }}>{view.answered.count}</b>{" "}
              {view.answered.split}
            </div>
            <div style={warRoomBarTrackStyle}>
              <div style={warRoomBarFillStyle(view.answered.fraction)} />
            </div>
          </>
        )}
        <span style={warRoomBadgeStyle(view.badge.kind)} data-badge={view.badge.kind}>
          {view.badge.text}
        </span>
      </section>
    </article>
  );
};

export default WarRoomCard;
