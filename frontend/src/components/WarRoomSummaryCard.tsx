// =============================================================================
// WarRoomSummaryCard.tsx — the War Room's one summary card
// =============================================================================
//
// CC_TASK_SIMPLE_COUNTS_v1, drawn on WAR_ROOM_SUMMARY_CARD_RULED_2026-09-17. It
// replaces BOTH stat strips — the four-tile queue strip and the metric band.
//
//   top row    — Scenarios · Ready · Draft, small; "Questions answered", a slim
//                bar and "N of M · P%"
//   bottom row — three cells, one queue each, each with its owner's stripe and
//                chip: Marie's unanswered questions, the reviewer's answers
//                requiring review, Roman's candidates to rule
//
// Props in, JSX out: every string and colour flag comes from `warRoomSummaryView`.
// The mockup's "MOCKUP —" subtitle is an annotation and does not ship.

import React from "react";

import * as st from "./warRoomSummaryStyles";
import { warRoomSummaryView } from "./warRoomSummaryView";
import type { TrialPrepDashboard, WarRoomWording } from "../pages/trialPrepData";

const WarRoomSummaryCard: React.FC<{
  dashboard: Pick<TrialPrepDashboard, "metrics" | "scenarios">;
  wording: WarRoomWording;
}> = ({ dashboard, wording }) => {
  const view = warRoomSummaryView(dashboard, wording);
  return (
    <section style={st.summaryCardStyle} data-summary-card>
      <div style={st.summaryTopStyle} data-summary-top>
        <div style={st.summaryMetricsStyle}>
          {view.metrics.map((m, i) => (
            <span key={m.label} style={st.summaryMetricStyle(i === 0)} data-metric={m.label}>
              <span style={st.summaryMetricValueStyle}>{m.value}</span>
              {m.label}
            </span>
          ))}
        </div>
        <div style={st.summaryAnsweredStyle} data-summary-answered>
          <span>{view.answeredLabel}</span>
          <div style={st.summaryBarTrackStyle}>
            <div style={st.summaryBarFillStyle(view.fraction)} />
          </div>
          <span>
            <b style={{ color: "var(--text-primary)" }}>{view.answered}</b> {view.answeredRest}
          </span>
        </div>
      </div>
      <div style={st.summaryCellsStyle}>
        {view.cells.map((cell, i) => (
          <div key={cell.owner} style={st.summaryCellStyle(cell.owner, i === 0)} data-owner={cell.owner}>
            <div style={st.summaryCellHeadStyle}>
              <span>{cell.label}</span>
              <span style={st.summaryChipStyle(cell.owner)} data-chip>
                {cell.chip}
              </span>
            </div>
            <div style={st.summaryCountStyle(cell.warning)} data-warning={cell.warning}>
              {cell.count}
            </div>
            <div style={st.summaryContextStyle}>{cell.context}</div>
          </div>
        ))}
      </div>
    </section>
  );
};

export default WarRoomSummaryCard;
