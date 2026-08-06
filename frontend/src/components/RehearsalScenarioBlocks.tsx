// =============================================================================
// RehearsalScenarioBlocks — one ready scenario, as Marie reads it (task 2.11 B2)
// =============================================================================
//
// The seven blocks of REHEARSAL_VIEW_DESIGN_v2, replacing the four of task 1.5:
//
//   1  What this is                2  The accusation + every instance
//   3  Our answer under each one   4  The timeline
//   5  Her points                  6  Watch for
//   7  Always  (rendered by the page — it never collapses)
//
// ## What died here, and why it was a defect and not a style
//
// The old block labelled "What they say" rendered `attack_text` — a verbatim
// first-person quote from the record, promoted into the summary of all of it. The
// block labelled "Our answer" rendered the THEME, which is not an answer. Both are
// gone at the root: the payload now carries a human's plain-words accusation and
// the record items a human PAIRED as our answers, and this component has no field
// to render the old ones from.
//
// ## Every word is the store's
//
// There is not one user-facing literal below. Headings, gap sentences, counts and
// the answer label all arrive on the payload, composed. A block with nothing in it
// renders its NAMED gap, never a blank — the honest-gap law, which is what the
// page's whole credibility rests on.

import React from "react";

import RehearsalInstanceRow from "./RehearsalInstanceRow";
import RehearsalSection from "./RehearsalSection";
import RehearsalTimelineBlock from "./RehearsalTimelineBlock";
import { absentStyle, DIVIDER } from "./scenarioSectionStyles";
import {
  GAP_NO_ANSWER,
  type RehearsalPayload,
  type RehearsalScenario,
} from "../services/rehearsal";
import type { OpenSections } from "../pages/rehearsalSections";

interface Props {
  scenario: RehearsalScenario;
  wording: RehearsalPayload["wording"];
  open: OpenSections;
  onToggle: (section: keyof OpenSections) => void;
}

// The scale is the one place this surface departs from the rest of the app, and
// it does so deliberately: read aloud, from a distance, under stress.
const bodyStyle: React.CSSProperties = { fontSize: "1.3rem", lineHeight: 1.6 };

const accusationStyle: React.CSSProperties = {
  ...bodyStyle,
  fontSize: "1.5rem",
  fontWeight: 600,
};

const listStyle: React.CSSProperties = { ...bodyStyle, paddingLeft: "1.4rem" };

/**
 * The gap list, beneath the instances.
 *
 * The prep list reads loudest because it is the one a human can act on today; the
 * other three are named plainly. Branching on the TOKEN, never the sentence —
 * Roman is invited to edit these words, and a client matching on "NO ANSWER"
 * would silently stop telling the kinds apart the day he did.
 */
const GapList: React.FC<{ gaps: RehearsalScenario["accusation"]["gaps"] }> = ({
  gaps,
}) => (
  <ul style={{ ...listStyle, marginTop: "1rem" }}>
    {gaps.map((gap, i) => (
      <li
        key={`${gap.kind}:${i}`}
        style={{
          marginBottom: "0.4rem",
          fontWeight: gap.kind === GAP_NO_ANSWER ? 600 : 400,
          color:
            gap.kind === GAP_NO_ANSWER
              ? "var(--state-danger-strong)"
              : "var(--text-secondary)",
        }}
      >
        {gap.message}
      </li>
    ))}
  </ul>
);

const RehearsalScenarioBlocks: React.FC<Props> = ({
  scenario,
  wording,
  open,
  onToggle,
}) => (
  <>
    {/* 1 — What this is. Not collapsible: it is one sentence, and folding a
        sentence saves nothing while costing the reader their bearings. */}
    <div style={{ borderTop: DIVIDER, paddingTop: "1rem", marginTop: "1.5rem" }}>
      <div style={{ fontSize: "0.8rem", letterSpacing: "0.06em", textTransform: "uppercase", color: "var(--text-muted)" }}>
        {wording.block_what_heading}
      </div>
      <p style={bodyStyle}>
        {scenario.what_this_is ?? (
          <span style={absentStyle}>{scenario.what_this_is_gap}</span>
        )}
      </p>
    </div>

    {/* 2 and 3 — the page. */}
    <RehearsalSection
      heading={wording.block_accusation_heading}
      count={scenario.headers.accusation}
      open={open.accusation}
      onToggle={() => onToggle("accusation")}
    >
      <p style={accusationStyle}>
        {scenario.accusation.text ?? (
          <span style={{ ...absentStyle, fontSize: "1.2rem", fontWeight: 400 }}>
            {scenario.accusation.text_gap}
          </span>
        )}
      </p>

      {/* Exactly one of these is ever present — the backend decides. */}
      <p style={{ ...bodyStyle, fontSize: "1.05rem", color: "var(--text-secondary)" }}>
        {scenario.accusation.count_line ?? scenario.accusation.no_instances_notice}
      </p>

      {scenario.accusation.instances.map((instance) => (
        <RehearsalInstanceRow
          key={instance.position}
          instance={instance}
          answerLabel={wording.answer_label}
        />
      ))}

      {scenario.accusation.gaps.length > 0 && (
        <GapList gaps={scenario.accusation.gaps} />
      )}
    </RehearsalSection>

    {/* 4 — the conditional timeline. */}
    <RehearsalSection
      heading={wording.block_timeline_heading}
      count={scenario.headers.timeline}
      open={open.timeline}
      onToggle={() => onToggle("timeline")}
    >
      <RehearsalTimelineBlock timeline={scenario.timeline} />
    </RehearsalSection>

    {/* 5 — her points, in her words. */}
    <RehearsalSection
      heading={wording.block_points_heading}
      count={scenario.headers.points}
      open={open.points}
      onToggle={() => onToggle("points")}
    >
      {scenario.points.length === 0 ? (
        <p style={absentStyle}>{scenario.points_gap}</p>
      ) : (
        <ol style={listStyle}>
          {scenario.points.map((point, i) => (
            <li key={i} style={{ marginBottom: "0.6rem" }}>
              {point.text}
              {/* The paired exhibit, in HER phrasing, when one is authored. A
                  plain label and never a page: the source citation belongs to an
                  instance row, not to a talking point. */}
              {point.exhibit && (
                <span style={{ color: "var(--text-muted)", fontSize: "1rem" }}>
                  {" "}
                  ({point.exhibit})
                </span>
              )}
            </li>
          ))}
        </ol>
      )}
    </RehearsalSection>

    {/* 6 — what they will wave around. */}
    <RehearsalSection
      heading={wording.block_watch_heading}
      count={scenario.headers.watch_for}
      open={open.watchFor}
      onToggle={() => onToggle("watchFor")}
    >
      {scenario.watch_for.length === 0 ? (
        <p style={absentStyle}>{scenario.watch_for_gap}</p>
      ) : (
        <ul style={listStyle}>
          {scenario.watch_for.map((note, i) => (
            <li key={i} style={{ marginBottom: "0.6rem" }}>
              {note}
            </li>
          ))}
        </ul>
      )}
    </RehearsalSection>
  </>
);

export default RehearsalScenarioBlocks;
