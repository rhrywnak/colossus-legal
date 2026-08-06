// =============================================================================
// RehearsalInstanceRow — one instance, and our answer under it (task 2.11 B2)
// =============================================================================
//
// Roman's model, one row of it: *"He mentioned it 5 times in 5 different
// documents. Marie rebutted 5 times."* This is one of those times — who said it,
// when, where, in what kind of statement, in their own words — with what we said
// back directly beneath it, and a loud line when there is nothing beneath it yet.
//
// ## Progressive disclosure at the row level (addendum §2)
//
// Collapsed the row is one line: who · when · where · the quote's opening.
// Expanded it is the full quote, the source control, and our answer. Same
// doctrine as the sections — one thing at a time — applied where a witness
// actually works, which is one instance at a time.
//
// The opening is `quote_first_line`, composed by the backend. This component does
// not slice prose: a cut computed here would differ from a cut computed anywhere
// else the moment two components disagreed about the length.
//
// ## Why the source is here at all
//
// §10 keeps impeachment machinery off this page, and this is the one exception,
// ruled 2026-08-06: an examiner or witness who cannot produce the source on the
// spot loses credibility. The label and the link are the whole of it — no
// confidence, no grading, no verdict, none of which the payload even carries.

import React, { useState } from "react";

import { DIVIDER } from "./scenarioSectionStyles";
import type { RehearsalAnswer, RehearsalInstance } from "../services/rehearsal";

interface Props {
  instance: RehearsalInstance;
  /** The stored label for the paired-answer line. */
  answerLabel: string;
}

const rowStyle: React.CSSProperties = {
  borderTop: DIVIDER,
  padding: "0.9rem 0",
};

const metaStyle: React.CSSProperties = {
  fontSize: "0.9rem",
  color: "var(--text-secondary)",
  display: "flex",
  gap: "0.6rem",
  flexWrap: "wrap",
  alignItems: "baseline",
};

// The scale is the one place this surface departs from the rest of the app, and
// it does so deliberately: this text is read aloud, from a distance, under
// stress, by someone who is not looking for a row.
const quoteStyle: React.CSSProperties = { fontSize: "1.25rem", lineHeight: 1.6 };

const positionStyle: React.CSSProperties = {
  fontWeight: 600,
  color: "var(--text-muted)",
};

/** The link that opens the document at the statement's page. */
const SourceLink: React.FC<{ href: string; label: string; open: string }> = ({
  href,
  label,
  open,
}) => (
  <span style={{ fontSize: "0.9rem", color: "var(--text-secondary)" }}>
    {label}
    {/* No link when the record cannot say WHICH document. A link to nowhere is
        worse than no link — a reader clicks it in front of opposing counsel. */}
    {href && (
      <>
        {" · "}
        <a href={href} style={{ color: "var(--accent-primary)" }}>
          {open}
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
  <div style={{ marginTop: "0.75rem", paddingLeft: "1rem" }}>
    <div style={metaStyle}>
      <strong style={{ color: "var(--text-primary)" }}>{label}</strong>
      <span>{answer.who}</span>
      {/* Exactly one of these is ever present — the backend decides, so this
          cannot render both or neither. */}
      <span>{answer.when ?? answer.when_gap}</span>
    </div>
    <p style={quoteStyle}>{answer.quote}</p>
    <SourceLink
      href={answer.source.href}
      label={answer.source.label}
      open={answer.source.open_label}
    />
  </div>
);

const RehearsalInstanceRow: React.FC<Props> = ({ instance, answerLabel }) => {
  const [open, setOpen] = useState(false);

  return (
    <div style={rowStyle}>
      <button
        type="button"
        onClick={() => setOpen(!open)}
        aria-expanded={open}
        style={{
          display: "block",
          width: "100%",
          textAlign: "left",
          border: "none",
          background: "transparent",
          padding: 0,
          cursor: "pointer",
          fontFamily: "inherit",
        }}
      >
        <div style={metaStyle}>
          <span style={positionStyle}>{instance.position}</span>
          <span>{instance.who}</span>
          <span>{instance.when ?? instance.when_gap}</span>
          <span>{instance.source.label}</span>
          {instance.kind_label && <span>{instance.kind_label}</span>}
        </div>
        {/* The opening when folded, the whole statement when open. Both come
            from the payload; neither is cut here. */}
        <p style={quoteStyle}>{open ? instance.quote : instance.quote_first_line}</p>
      </button>

      {open && (
        <SourceLink
          href={instance.source.href}
          label={instance.source.label}
          open={instance.source.open_label}
        />
      )}

      {/* The answer, or the loud line saying there is not one. Rendered whether
          the row is open or shut: the gap is the single most useful thing on this
          page, and hiding it behind a fold is exactly what must not happen. */}
      {instance.answer ? (
        open && <AnswerBlock answer={instance.answer} label={answerLabel} />
      ) : (
        instance.answer_gap && (
          <p
            style={{
              margin: "0.5rem 0 0",
              fontWeight: 600,
              color: "var(--state-danger-strong)",
            }}
          >
            {instance.answer_gap}
          </p>
        )
      )}
    </div>
  );
};

export default RehearsalInstanceRow;
