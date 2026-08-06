// =============================================================================
// RehearsalInstanceRow — one instance, and our answer under it
// =============================================================================
//
// Roman's model, one row of it: *"He mentioned it 5 times in 5 different
// documents. Marie rebutted 5 times."* This is one of those times — # · who ·
// when · where · Open ↗ · kind · tag — with what we said back directly beneath
// it when the row is open.
//
// Rebuilt in task 2.11 C to reproduce `.instance` / `.inst-meta` / `.quote` /
// `.answer` / `.nogap` from REHEARSAL_PAGE_MOCKUP_v2_2026-08-06.html.
//
// ## Progressive disclosure at the row level
//
// Compact, the row is its meta line plus the quote's opening in italic.
// Expanded, it is the full quote and our answer. Same doctrine as the sections —
// one thing at a time — applied where a witness actually works, which is one
// instance at a time.
//
// Which state a row ARRIVES in is the server's call (`instances_start_expanded`,
// from the stored `rehearsal_instance_rows_expand_max`): at or under the cap
// every row is open, above it every row is one line. The list is never
// paginated, at any size.
//
// The opening is `quote_first_line`, composed by the backend. This component does
// not slice prose: a cut computed here would differ from a cut computed anywhere
// else the moment two components disagreed about the length.
//
// ## The gap, at two zoom levels and never three (ruling C5)
//
// The meta line carries the small red tag. An OPENED row carries the short
// banner. The sentence naming who/when/where lives once, in the prep list —
// rendering it here as well was the beta.381 duplicate-gap defect, and both
// strings arrive composed so this component could not rebuild it if it tried.
//
// ## Why the source is here at all
//
// §10 keeps impeachment machinery off this page, and this is the one exception,
// ruled 2026-08-06: an examiner or witness who cannot produce the source on the
// spot loses credibility. The label and the link are the whole of it.

import React from "react";

import {
  linkStyle,
} from "./rehearsalStyles";
import {
  answerLabelStyle,
  answerMetaStyle,
  answerQuoteStyle,
  answerStyle,
  chipStyle,
  instanceFirstLineStyle,
  instanceMetaStyle,
  instanceNumberStyle,
  instanceStyle,
  miniTagOkStyle,
  miniTagStyle,
  noAnswerBannerStyle,
  quoteStyle,
} from "./rehearsalRowStyles";
import type { RehearsalAnswer, RehearsalInstance } from "../services/rehearsal";

interface Props {
  instance: RehearsalInstance;
  /** The stored label for the paired-answer block. */
  answerLabel: string;
  open: boolean;
  onToggle: () => void;
  /** Set on the row the prep list's jump link scrolls to. */
  anchorId: string;
}

/** Where a statement was made, and the control that opens it at that page. */
const Source: React.FC<{
  label: string;
  href: string;
  openLabel: string;
  /** Stops a click on the link from also folding the row it sits in. */
  stopPropagation?: boolean;
}> = ({ label, href, openLabel, stopPropagation }) => (
  <span style={{ color: "var(--text-secondary)" }}>
    {label}
    {/* No link when the record cannot say WHICH document. A link to nowhere is
        worse than no link — a reader clicks it in front of opposing counsel. */}
    {href && (
      <>
        {" · "}
        <a
          href={href}
          style={linkStyle}
          onClick={stopPropagation ? (e) => e.stopPropagation() : undefined}
        >
          {openLabel} ↗
        </a>
      </>
    )}
  </span>
);

/** Our answer, directly beneath the instance it answers. */
const AnswerBlock: React.FC<{ answer: RehearsalAnswer; label: string }> = ({
  answer,
  label,
}) => (
  <div style={answerStyle}>
    <div style={answerLabelStyle}>{label}</div>
    <div style={answerMetaStyle}>
      {answer.who}
      {" · "}
      {/* Exactly one of these is ever present — the backend decides, so this
          cannot render both or neither. */}
      {answer.when ?? answer.when_gap}
      {" · "}
      <Source
        label={answer.source.label}
        href={answer.source.href}
        openLabel={answer.source.open_label}
      />
    </div>
    <div style={answerQuoteStyle}>{answer.quote}</div>
  </div>
);

const RehearsalInstanceRow: React.FC<Props> = ({
  instance,
  answerLabel,
  open,
  onToggle,
  anchorId,
}) => (
  <div style={instanceStyle} id={anchorId}>
    <button type="button" onClick={onToggle} aria-expanded={open} style={instanceMetaStyle}>
      <span style={instanceNumberStyle}>{instance.position}</span>
      <span style={{ fontWeight: 600 }}>{instance.who}</span>
      <span style={{ color: "var(--text-secondary)" }}>
        {instance.when ?? instance.when_gap}
      </span>
      <span style={{ color: "var(--text-secondary)" }}>
        <Source
          label={instance.source.label}
          href={instance.source.href}
          openLabel={instance.source.open_label}
          stopPropagation
        />
      </span>
      {instance.kind_label && <span style={chipStyle}>{instance.kind_label}</span>}
      {/* The tag the server chose. Green means answered; red means not. Both
          words are stored rows, and the choice was made where the answer
          actually is — a client picking between two labels can pick wrong. */}
      <span style={instance.answer ? miniTagOkStyle : miniTagStyle}>
        {instance.answer_tag}
      </span>
    </button>

    {open ? (
      <>
        <div style={quoteStyle}>{instance.quote}</div>
        {instance.answer ? (
          <AnswerBlock answer={instance.answer} label={answerLabel} />
        ) : (
          instance.answer_banner && (
            <div style={noAnswerBannerStyle}>{instance.answer_banner}</div>
          )
        )}
      </>
    ) : (
      <div style={instanceFirstLineStyle}>{instance.quote_first_line}</div>
    )}
  </div>
);

export default RehearsalInstanceRow;
