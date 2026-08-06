// =============================================================================
// RehearsalTimelineBlock — the timeline, drawn
// =============================================================================
//
// A vertical spine, oldest first, one dated row per placed item, with THEY SAY /
// OUR ANSWER marked on the side and a person filter above. Rebuilt in task 2.11
// C to reproduce `.tl` / `.tl-row` / `.tl-dot` / `.tl-side` from
// REHEARSAL_PAGE_MOCKUP_v2_2026-08-06.html; B2 rendered a two-column list.
//
// ## What is decided elsewhere, and stays decided elsewhere
//
// WHETHER the block draws at all is the backend's call, from the stored
// `rehearsal_timeline_min_distinct_dates` threshold measured over what a human
// actually PLACED. When it cannot draw, `notice` carries the honest-gap line
// naming how many placed items have no date, and that sentence is the whole
// content of the block. This component never re-evaluates the threshold and
// never draws a partial spine.
//
// The ORDER is the backend's too, and it is the design's point: strictly
// chronological, because the force of the block is the repetition over time,
// which grouping by speaker would bury.
//
// ## Why the side is styled from a TOKEN and labelled from a row
//
// `side` is `their_words` / `our_answer` — matched here for the dot and the
// chip's colour. `side_label` is the word a human reads. Branching on the label
// would silently stop telling the sides apart the day Roman reworded one, on the
// block whose whole point is that the two are distinguishable at a glance.

import React, { useState } from "react";

import {
  linkStyle,
} from "./rehearsalStyles";
import {
  timelineBodyStyle,
  timelineDateStyle,
  timelineDotStyle,
  timelineFilterStyle,
  timelineRowStyle,
  timelineSelectStyle,
  timelineSideStyle,
  timelineSourceStyle,
  timelineSpineStyle,
} from "./rehearsalRowStyles";
import { absentStyle } from "./scenarioSectionStyles";
import {
  SIDE_OUR_ANSWER,
  type RehearsalTimeline,
  type RehearsalTimelineEntry,
} from "../services/rehearsal";

interface Props {
  timeline: RehearsalTimeline;
}

/** One dated row on the spine. */
const Row: React.FC<{ entry: RehearsalTimelineEntry }> = ({ entry }) => {
  const ours = entry.side === SIDE_OUR_ANSWER;

  return (
    <div style={timelineRowStyle}>
      <span
        style={{
          ...timelineDotStyle,
          background: ours ? "var(--v3-green-text)" : "var(--text-secondary)",
        }}
      />
      <span style={timelineDateStyle}>{entry.when}</span>
      <span
        style={{
          ...timelineSideStyle,
          color: ours ? "var(--v3-green-text)" : "var(--text-secondary)",
          background: ours ? "var(--state-success-bg-soft)" : "var(--v3-chrome)",
        }}
      >
        {entry.side_label}
      </span>
      <div style={timelineBodyStyle}>
        <span style={{ fontWeight: 600 }}>{entry.who}</span>
        {" — "}
        {entry.quote}
      </div>
      <div style={timelineSourceStyle}>
        {entry.source.label}
        {/* No link when the record cannot say WHICH document — the same rule the
            instance rows follow. */}
        {entry.source.href && (
          <>
            {" · "}
            <a href={entry.source.href} style={linkStyle}>
              {entry.source.open_label} ↗
            </a>
          </>
        )}
      </div>
    </div>
  );
};

const RehearsalTimelineBlock: React.FC<Props> = ({ timeline }) => {
  const [person, setPerson] = useState<string | null>(null);

  // The server said the block cannot draw. It also said WHY, and that sentence is
  // the whole content of the block in that state — never a blank, which would be
  // indistinguishable from a section that failed to load.
  if (timeline.notice) {
    return <p style={absentStyle}>{timeline.notice}</p>;
  }

  const shown = person
    ? timeline.entries.filter((entry) => entry.who === person)
    : timeline.entries;

  return (
    <>
      {/* The filter is offered only when there is more than one person to choose
          between — a control with one option cannot do anything, and a control
          that cannot do anything reads as a broken one. */}
      {timeline.people.length > 1 && (
        <label style={timelineFilterStyle}>
          {timeline.filter_prompt}
          <select
            value={person ?? ""}
            onChange={(e) => setPerson(e.target.value || null)}
            style={timelineSelectStyle}
          >
            <option value="">{timeline.filter_all_label}</option>
            {timeline.people.map((name) => (
              <option key={name} value={name}>
                {name}
              </option>
            ))}
          </select>
        </label>
      )}

      <div style={timelineSpineStyle}>
        {shown.map((entry, i) => (
          <Row key={`${entry.when}:${i}`} entry={entry} />
        ))}
      </div>
    </>
  );
};

export default RehearsalTimelineBlock;
