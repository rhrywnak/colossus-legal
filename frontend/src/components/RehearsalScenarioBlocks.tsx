// =============================================================================
// RehearsalScenarioBlocks — the prep page, read-only (task R3)
// =============================================================================
//
// One READY scenario, laid out as REHEARSAL_PAGE_MOCKUP_v3_2026-08-10.html has
// it. This is the page Marie preps from with Chuck in the room.
//
// ## What this build removed, and why each one had to go
//
//   * **Every edit control.** The `SentenceEditor` on the theme, the accusation
//     editor, the add/edit buttons on points and watch items. A prep surface that
//     invites editing invites editing under pressure, minutes before a
//     deposition. All of it routes to the working page now, through the one link
//     in the header.
//   * **Authorship lines** — "Written in plain words by roman · Aug 7". True, and
//     nothing a witness needs while rehearsing. It is provenance for the person
//     who wrote it, on the page where they wrote it.
//   * **The separate TIMELINE section.** The instance list IS the chronology now
//     (oldest first, server-sorted). Two renderings of one sequence, one of them
//     without the answers, was the thing that made this page hard to work from.
//   * **"gaps" in the header counts.** A number labelled "gaps" beside a section
//     heading is a defect report. The section says "3 of 5 answered — 2 to
//     prepare", which is the same fact as work.
//   * **The WHAT THIS IS heading.** The theme is the page's first sentence and
//     carries no label above it; the identity line already says what this is.
//
// ## The collapsible sections went too
//
// Folding earned its place on a page that could not fit — this one is built to be
// read top to bottom in one pass. A fold on a prep surface is a place for
// something to be missed.

import React, { useState } from "react";

import PrepInstanceCard from "./PrepInstanceCard";
import PrepTopBlock from "./PrepTopBlock";
import { chipStyle } from "./scenarioSectionStyles";
import type { RehearsalScenario, RehearsalWording } from "../services/rehearsal";

const sectionStyle: React.CSSProperties = { marginTop: "34px" };

const sectionHeadStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  gap: "12px",
  flexWrap: "wrap",
  marginBottom: "14px",
};

const sectionTitleStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "17px",
  fontWeight: 600,
  color: "var(--text-primary)",
};

const sectionCountStyle: React.CSSProperties = {
  fontSize: "13px",
  color: "var(--text-secondary)",
};

const listStyle: React.CSSProperties = {
  listStyle: "none",
  margin: 0,
  padding: 0,
  display: "flex",
  flexDirection: "column",
  gap: "14px",
};

const filterRowStyle: React.CSSProperties = {
  display: "flex",
  gap: "8px",
  flexWrap: "wrap",
  marginBottom: "14px",
};

const filterChipStyle = (active: boolean): React.CSSProperties => ({
  ...chipStyle,
  cursor: "pointer",
  border: `1px solid ${active ? "var(--accent-primary)" : "var(--border-default)"}`,
  background: active ? "var(--state-info-bg-soft)" : "var(--bg-surface)",
  color: active ? "var(--accent-primary)" : "var(--text-secondary)",
  fontWeight: active ? 600 : 500,
});

const pointStyle: React.CSSProperties = {
  fontSize: "17px",
  lineHeight: 1.5,
  color: "var(--text-primary)",
};

const backedByStyle: React.CSSProperties = {
  marginTop: "6px",
  fontSize: "12.5px",
  color: "var(--text-muted)",
};

const watchStyle: React.CSSProperties = {
  padding: "12px 16px",
  borderRadius: "8px",
  background: "var(--state-warning-bg-soft)",
  borderLeft: "4px solid var(--state-warning-strong)",
  fontSize: "14.5px",
  lineHeight: 1.5,
};

const gapTextStyle: React.CSSProperties = {
  fontSize: "15px",
  color: "var(--text-muted)",
  fontStyle: "italic",
};

interface Props {
  scenario: RehearsalScenario;
  wording: RehearsalWording;
}

const RehearsalScenarioBlocks: React.FC<Props> = ({ scenario, wording }) => {
  // Which phase the reader has narrowed to, or `null` for all of them. Tapping
  // the active chip clears it — the mockup's rule, and the one a reader tries
  // first without being told.
  const [phase, setPhase] = useState<string | null>(null);

  const instances = scenario.accusation.instances;
  // The chips offered are the phases PRESENT, in the order the cards appear.
  // Offering all four would put a chip on screen that filters to nothing, which
  // reads as a broken control rather than an empty phase.
  const phases = instances.reduce<string[]>((seen, i) => {
    if (!seen.includes(i.phase)) seen.push(i.phase);
    return seen;
  }, []);
  const visible = phase === null ? instances : instances.filter((i) => i.phase === phase);

  return (
    <>
      <PrepTopBlock scenario={scenario} wording={wording} />

      {/* THE HEART. One card per marked statement, oldest first, each with the
          answer inside it or the gap that says there is none. */}
      <section style={sectionStyle}>
        <div style={sectionHeadStyle}>
          <h2 style={sectionTitleStyle}>{wording.block_accusation_heading}</h2>
          {scenario.accusation.answered_line && (
            <span style={sectionCountStyle}>{scenario.accusation.answered_line}</span>
          )}
        </div>

        {phases.length > 1 && (
          <div style={filterRowStyle}>
            {phases.map((name) => (
              <button
                key={name}
                type="button"
                aria-pressed={phase === name}
                style={filterChipStyle(phase === name)}
                onClick={() => setPhase(phase === name ? null : name)}
              >
                {name}
              </button>
            ))}
          </div>
        )}

        {instances.length === 0 ? (
          <p style={gapTextStyle}>{scenario.accusation.no_instances_notice}</p>
        ) : (
          <ul style={listStyle}>
            {visible.map((instance) => (
              <PrepInstanceCard
                key={instance.position}
                instance={instance}
                wording={wording}
              />
            ))}
          </ul>
        )}
      </section>

      {/* HER POINTS. Each one large, with the exhibit its pairing already
          proposes — never retyped, and never invented when there is none. */}
      <section style={sectionStyle}>
        <div style={sectionHeadStyle}>
          <h2 style={sectionTitleStyle}>{wording.block_points_heading}</h2>
        </div>
        {scenario.points.length === 0 ? (
          <p style={gapTextStyle}>{scenario.points_gap}</p>
        ) : (
          <ul style={listStyle}>
            {scenario.points.map((point) => (
              <li key={point.position}>
                <div style={pointStyle}>{point.text}</div>
                {/* The exhibit is a PROPOSAL from an existing pairing until it is
                    confirmed on the working page, and it says so. A point with
                    nothing proposable says that instead — the stored sentence,
                    not a blank. */}
                <div style={backedByStyle}>
                  {point.exhibit ?? point.exhibit_notice}
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>

      {/* WHAT TO WATCH FOR. */}
      <section style={sectionStyle}>
        <div style={sectionHeadStyle}>
          <h2 style={sectionTitleStyle}>{wording.block_watch_heading}</h2>
        </div>
        {scenario.watch_for.length === 0 ? (
          <p style={gapTextStyle}>{scenario.watch_for_gap}</p>
        ) : (
          <ul style={listStyle}>
            {scenario.watch_for.map((item) => (
              <li key={item.id} style={watchStyle}>
                {item.text}
              </li>
            ))}
          </ul>
        )}
      </section>
    </>
  );
};

export default RehearsalScenarioBlocks;
