// =============================================================================
// PrepInstanceCard — one time they said it, with your answer under it (task R3)
// =============================================================================
//
// The heart of the prep page, and the reason the separate TIMELINE section is
// gone: this list IS the chronology, oldest first, and the answer lives INSIDE
// the card rather than in a parallel column somewhere else on the page.
//
// ## Why the answer is in the same card and not beside it
//
// A witness under cross gets one question at a time and needs one answer at a
// time. The old page put the statements in one block and the timeline in
// another, so "what did they say" and "what do I say back" were two scrolls
// apart. Here they are one box: their words, then hers, visibly hers.
//
// ## Why an unanswered instance is LOUD
//
// It is the prep list. A quiet gap is one she discovers in the room.

import React from "react";

import { allegationChipStyle, chipStyle } from "./scenarioSectionStyles";
import type { RehearsalInstance, RehearsalWording } from "../services/rehearsal";

const cardStyle: React.CSSProperties = {
  padding: "16px 18px",
  borderRadius: "10px",
  background: "var(--bg-surface)",
  boxShadow: "var(--shadow-card)",
  borderLeft: "4px solid var(--state-danger-strong)",
};

const metaRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "8px",
  flexWrap: "wrap",
  marginBottom: "8px",
  fontSize: "12.5px",
};

/** Them. The colour is the fastest thing on the card. */
const whoStyle: React.CSSProperties = {
  fontWeight: 600,
  color: "var(--v3-red-text)",
};

const whenStyle: React.CSSProperties = { color: "var(--text-muted)" };

const quoteStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "15.5px",
  lineHeight: 1.55,
  color: "var(--text-primary)",
};

const sourceStyle: React.CSSProperties = {
  marginTop: "8px",
  fontSize: "12.5px",
  color: "var(--text-muted)",
};

const sourceLinkStyle: React.CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "none",
};

/** Ours. Green edge, per the mockup — the one visual promise on the card. */
const answerStyle: React.CSSProperties = {
  marginTop: "14px",
  paddingLeft: "14px",
  borderLeft: "4px solid var(--state-success-strong)",
};

const answerLabelStyle: React.CSSProperties = {
  fontSize: "11px",
  fontWeight: 600,
  letterSpacing: "0.08em",
  textTransform: "uppercase",
  color: "var(--state-success-strong)",
  marginBottom: "5px",
};

/** The gap, and it is meant to be the loudest thing in the list. */
const gapStyle: React.CSSProperties = {
  marginTop: "14px",
  padding: "10px 14px",
  borderRadius: "8px",
  background: "var(--state-danger-bg-soft)",
  color: "var(--state-danger-strong)",
  fontSize: "13.5px",
  fontWeight: 600,
};

interface Props {
  instance: RehearsalInstance;
  wording: RehearsalWording;
}

const PrepInstanceCard: React.FC<Props> = ({ instance, wording }) => (
  <li id={`instance-${instance.position}`} style={cardStyle}>
    <div style={metaRowStyle}>
      <span style={whoStyle}>{instance.who}</span>
      {/* The date, or the stored "no date yet" prompt — never a blank where a
          date goes, and never an invented one. 57% of this case's evidence has
          no date, so this is a common and honest state. */}
      <span style={whenStyle}>{instance.when ?? instance.when_gap}</span>
      <span style={chipStyle}>{instance.kind_label}</span>
      {/* The phase, decided server-side by forum first and date second. */}
      <span style={allegationChipStyle}>{instance.phase}</span>
    </div>

    <p style={quoteStyle}>{instance.quote}</p>

    {instance.source.label && (
      <div style={sourceStyle}>
        {instance.source.href ? (
          <a
            href={instance.source.href}
            style={sourceLinkStyle}
            target="_blank"
            rel="noreferrer"
          >
            {instance.source.label}
          </a>
        ) : (
          instance.source.label
        )}
      </div>
    )}

    {instance.answer ? (
      <div style={answerStyle}>
        <div style={answerLabelStyle}>{wording.answer_label}</div>
        <p style={quoteStyle}>{instance.answer.quote}</p>
        {instance.answer.source.label && (
          <div style={sourceStyle}>
            {instance.answer.source.href ? (
              <a
                href={instance.answer.source.href}
                style={sourceLinkStyle}
                target="_blank"
                rel="noreferrer"
              >
                {instance.answer.source.label}
              </a>
            ) : (
              instance.answer.source.label
            )}
          </div>
        )}
      </div>
    ) : (
      // The stored sentence, not a composed one: this is the loudest thing on
      // the page and Roman owns its words.
      <div style={gapStyle}>{instance.answer_banner ?? instance.answer_tag}</div>
    )}
  </li>
);

export default PrepInstanceCard;
