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

// ─── FACT_CARD_v2 §3 styles (tokens only) ────────────────────────────────────

const cardRowStyle: React.CSSProperties = {
  padding: "12px 0",
  borderTop: "1px solid var(--border-default)",
};

// The source line, small and muted: on this page the WORDS lead and the
// provenance follows, which is the reverse of the working surface.
const cardHeadStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  flexWrap: "wrap",
  gap: "8px",
  fontSize: "12.5px",
  color: "var(--text-muted)",
};

const cardCodeStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "11.5px",
  color: "var(--text-muted)",
};

const cardTitleStyle: React.CSSProperties = {
  marginTop: "4px",
  fontSize: "16px",
  fontWeight: 600,
  color: "var(--text-primary)",
};

const cardQuoteStyle: React.CSSProperties = {
  marginTop: "4px",
  fontSize: "14.5px",
  lineHeight: 1.45,
  color: "var(--text-secondary)",
};

// Her reply is the one line on this page she says out loud, so it is the one
// thing given a panel of its own.
const cardAnswerStyle: React.CSSProperties = {
  marginTop: "8px",
  padding: "10px 12px",
  borderRadius: "6px",
  backgroundColor: "var(--bg-page)",
  fontSize: "15px",
  lineHeight: 1.5,
  color: "var(--text-primary)",
};

const proofStyle: React.CSSProperties = {
  marginTop: "8px",
  paddingLeft: "12px",
  borderLeft: "2px solid var(--state-success-strong)",
};

const proofTitleStyle: React.CSSProperties = {
  fontSize: "14px",
  fontWeight: 600,
  color: "var(--text-primary)",
};


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

/** The number and the point, on one line (task R4, P5). */
const pointRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  gap: "10px",
};

/** Quiet, fixed-width, and the same number the working page prints. */
const pointNumberStyle: React.CSSProperties = {
  fontSize: "13px",
  fontWeight: 600,
  color: "var(--text-muted)",
  flexShrink: 0,
  minWidth: "1.1rem",
};

const backedByStyle: React.CSSProperties = {
  marginTop: "4px",
  marginLeft: "calc(1.1rem + 10px)",
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

        {/* FACT_CARD_v2 §3: the other side's own statements, oldest first, each
            with her Answer. Read off the scenario's CARDS rather than off the
            marked instances — a card is written for every statement in the deck,
            and the instance list only ever held the ones somebody had paired.

            A card with no Answer is SHOWN, with the stored gap sentence beneath
            it. That is the one thing this section must never do quietly: an
            unanswered accusation is the most important thing on the page, and
            hiding it would hide the work rather than the gap. */}
        {scenario.accusation_cards.length > 0 && (
          <ul style={listStyle}>
            {scenario.accusation_cards.map((card) => (
              <li key={card.code ?? card.quote ?? card.title} style={cardRowStyle}>
                <div style={cardHeadStyle}>
                  {card.when && <span>{card.when}</span>}
                  {card.who && <span>{card.who}</span>}
                  {card.code && <span style={cardCodeStyle}>{card.code}</span>}
                </div>
                {card.title && <div style={cardTitleStyle}>{card.title}</div>}
                {card.quote && <div style={cardQuoteStyle}>{card.quote}</div>}
                <div style={card.answer ? cardAnswerStyle : gapTextStyle}>
                  {card.answer ?? card.answer_gap}
                </div>
              </li>
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
                {/* NUMBER AND TEXT ON ONE LINE (task R4, P5). The number is the
                    same one the working page prints beside this point, so a
                    human can say "point three" in either place and mean one
                    thing. */}
                <div style={pointRowStyle}>
                  <span style={pointNumberStyle}>{point.position}</span>
                  <span style={pointStyle}>{point.text}</span>
                </div>
                {/* THE EXHIBIT, ONLY WHEN THERE IS ONE (task R4, P5).
                    
                    `exhibit_notice` — the stored "nothing paired yet" sentence —
                    is deliberately NOT rendered here. It is a note to the person
                    doing the preparing, and this is the page Marie reads in the
                    room: a line under every point saying it has no exhibit is
                    five lines telling her about work that did not happen, at the
                    moment she can least act on it. The working page says it, next
                    to the control that fixes it.
                    
                    Nothing renders when there is nothing — not a blank line, not
                    a dash. */}
                {point.exhibit && <div style={backedByStyle}>{point.exhibit}</div>}
                {/* FACT_CARD_v2 §3: the Proof of every card that backs this
                    point — the 3.9 pairing, read off `backs_position` rather
                    than authored. Nothing renders when nothing backs it, on the
                    same rule as the exhibit line above: a note about work that
                    did not happen belongs on the page where it can be done. */}
                {point.backing.map((proof) => (
                  <div key={proof.code ?? proof.quote ?? proof.title} style={proofStyle}>
                    {proof.title && <div style={proofTitleStyle}>{proof.title}</div>}
                    {proof.quote && <div style={cardQuoteStyle}>{proof.quote}</div>}
                    <div style={cardHeadStyle}>
                      {proof.when && <span>{proof.when}</span>}
                      {proof.who && <span>{proof.who}</span>}
                      {proof.code && <span style={cardCodeStyle}>{proof.code}</span>}
                    </div>
                  </div>
                ))}
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
        {/* FACT_CARD_v2 §3: every card's warning, with its C-code, BEFORE the
            free-form items. A warning tied to a statement she can cite leads —
            "they will say the judge approved it, and that is C-116" is something
            she can answer; a standing note is something to remember. */}
        {scenario.card_watch_for.length > 0 && (
          <ul style={listStyle}>
            {scenario.card_watch_for.map((item) => (
              <li key={item.code ?? item.text} style={watchStyle}>
                {item.code && <span style={cardCodeStyle}>{item.code}</span>}
                {item.text}
              </li>
            ))}
          </ul>
        )}

        {scenario.watch_for.length === 0 ? (
          // The gap is shown only when BOTH halves are empty: a section carrying
          // ten card warnings and no standing notes is not an empty section, and
          // saying so would report a gap that is not there.
          scenario.card_watch_for.length === 0 && (
            <p style={gapTextStyle}>{scenario.watch_for_gap}</p>
          )
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
