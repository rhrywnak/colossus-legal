// =============================================================================
// RehearsalTimelineBlock — both sides in date order (task 2.11 B2, block 4)
// =============================================================================
//
// The 50,000-foot view of the accusation repeating and our answers to it. The
// design's ruling stands: strictly chronological, because the force of this block
// is the repetition OVER TIME and grouping by speaker buries it — with a person
// filter so all of one person's entries are one click away.
//
// ## The filter is the one computation this file does
//
// A substring-free equality test over a list the payload already carries. That is
// presentation, not logic: the server decided which entries exist, in what order,
// and who appears in the control. This narrows a list; it never composes a
// sentence, never counts anything a header reports, and never decides whether the
// block draws at all.
//
// ## When it draws nothing, it says why
//
// The server decides that too — the threshold is a stored number evaluated over
// what a human PLACED, and the notice names how many placed items carry no date.
// Measured on this case, twelve of forty-six statements are dated across four
// distinct dates, none later than 2010, so this block will honestly refuse for
// most scenarios until the re-extraction work lands.

import React, { useState } from "react";

import { absentStyle, chromeInputStyle, DIVIDER } from "./scenarioSectionStyles";
import { SIDE_OUR_ANSWER, type RehearsalTimeline } from "../services/rehearsal";

interface Props {
  timeline: RehearsalTimeline;
}

const entryStyle: React.CSSProperties = {
  borderTop: DIVIDER,
  padding: "0.7rem 0",
  display: "grid",
  gridTemplateColumns: "9rem 1fr",
  gap: "0.75rem",
  alignItems: "baseline",
};

const whenStyle: React.CSSProperties = {
  fontSize: "0.95rem",
  color: "var(--text-secondary)",
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
        <label style={{ display: "flex", gap: "0.5rem", alignItems: "baseline" }}>
          <span style={{ fontSize: "0.9rem", color: "var(--text-secondary)" }}>
            {timeline.filter_prompt}
          </span>
          <select
            value={person ?? ""}
            onChange={(e) => setPerson(e.target.value || null)}
            style={chromeInputStyle}
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

      {shown.map((entry, i) => (
        <div key={`${entry.when}:${i}`} style={entryStyle}>
          <span style={whenStyle}>
            {entry.when}
            <br />
            {entry.who}
          </span>
          <span
            style={{
              fontSize: "1.15rem",
              lineHeight: 1.55,
              // The two sides read differently so the interleaving is visible at
              // a glance. Branching on the TOKEN, never on prose — and weight
              // rather than colour alone, so it survives a monochrome print of
              // the rehearsal packet.
              fontWeight: entry.side === SIDE_OUR_ANSWER ? 600 : 400,
              color:
                entry.side === SIDE_OUR_ANSWER
                  ? "var(--text-primary)"
                  : "var(--text-secondary)",
            }}
          >
            {entry.quote}
          </span>
        </div>
      ))}
    </>
  );
};

export default RehearsalTimelineBlock;
