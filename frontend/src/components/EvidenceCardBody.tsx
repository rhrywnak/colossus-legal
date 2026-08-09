// =============================================================================
// EvidenceCardBody — the ONE card body, rendered under both wrappers
// =============================================================================
//
// ONE_CARD_GRAMMAR (approved 2026-08-09). This is the JSX half of the law: the
// candidate wrapper and the fact wrapper both mount this component over the same
// `EvidenceCardView`, so the same piece of evidence looks the same wherever a
// human meets it.
//
// It replaced `CandidateCardBody`, which was the candidate's private body — and
// which the fact card could not use, because the fact row was a thinner
// projection that had already thrown away the elements, the count, the anchor
// highlight, the context, the band and the scan's reason.
//
// ## This component chooses NOTHING
//
// Which pieces exist, where the question folds, how many element chips stand —
// all decided by the pure `evidenceCardView`, which is what makes the
// "same fields under both wrappers" claim a test rather than a hope (Rule 30:
// there is no RTL/jsdom here). This file walks the view and owns only the two
// things a value cannot hold: what is currently unfolded, and the anchor's
// highlight styling.
//
// ## Visual language (§2c, binding)
//
// Pure white surfaces, hairline borders, regular weight with bold reserved for
// true emphasis, one accent plus chips, generous line height.

import React, { useState } from "react";

import { contextPanelStyle, contextStyle, edgeStyle, markStyle } from "./candidateCardStyles";
import { BearsOnRow, ChipRow, ExchangeRow, PinpointRow } from "./EvidenceCardParts";
import type { ChipFilter, EvidenceCardView } from "./evidenceCardModel";
import type { CardGrammarWording } from "../services/evidenceLinks";

// ─── The body ───────────────────────────────────────────────────────────────

/**
 * One evidence card's body — identical under both wrappers.
 *
 * @param view    built by `evidenceCardView` from the payload, by either wrapper
 * @param wording the stored words; the wrapper renders nothing until they load
 * @param onFilterChip narrows the surrounding list (Piece 7). Omitted where the
 *        surrounding list has no filter to narrow.
 */
export const EvidenceCardBody: React.FC<{
  view: EvidenceCardView;
  wording: CardGrammarWording;
  onFilterChip?: (filter: ChipFilter) => void;
  /** Rendered between the chips and the exchange — the wrapper's own extras. */
  children?: React.ReactNode;
}> = ({ view, wording, onFilterChip, children }) => {
  const [questionOpen, setQuestionOpen] = useState(false);
  const [contextOpen, setContextOpen] = useState(false);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.55rem", minWidth: 0 }}>
      <ChipRow chips={view.chips} wording={wording} onFilter={onFilterChip} />

      {children}

      {/* Piece 2a — the question compressed to ONE line, never leading the card.
          The full six-part interrogatory is one click away and is what pushed the
          evidence off a 13-inch screen when it rendered in full. */}
      {view.question && (
        <div>
          <ExchangeRow prefix="Q:" muted>
            {questionOpen ? view.question.full : view.question.short}
            {view.question.truncated && (
              <button
                type="button"
                onClick={(event) => {
                  event.stopPropagation();
                  setQuestionOpen(!questionOpen);
                }}
                style={{
                  border: "none",
                  background: "none",
                  padding: 0,
                  marginLeft: "0.4rem",
                  color: "var(--accent-primary)",
                  cursor: "pointer",
                  fontFamily: "inherit",
                  fontSize: "12.5px",
                }}
              >
                {questionOpen
                  ? wording.question_collapse_label
                  : wording.question_expand_label}
              </button>
            )}
          </ExchangeRow>
          {/* Ruling R2: this badge says who TRANSCRIBED the question. It used to
              read a bare "System" and was taken for the speaker of the answer
              below it — which is why the speaker chip now leads the card and this
              sentence names what it is about. */}
          {view.question.authorship && (
            <div
              style={{
                fontSize: "11.5px",
                color: "var(--text-muted)",
                marginLeft: "2.05em",
                marginTop: "2px",
              }}
            >
              <span aria-hidden="true">{view.question.authorship.human ? "👤" : "⚙"}</span>{" "}
              {view.question.authorship.label}
            </div>
          )}
        </div>
      )}

      {/* Piece 2a — the ANSWER is the star of the card, with the anchor marked.
          A statement that is not a Q&A pair renders here with no Q: line above
          it: the A: block is simply the statement. */}
      <ExchangeRow prefix="A:">
        {view.answer.before}
        <mark style={markStyle}>{view.answer.anchor}</mark>
        {view.answer.after}
      </ExchangeRow>

      <PinpointRow label={view.pinpointLabel} href={view.pinpointHref} />

      {view.bearsOn.map((row, i) => (
        <BearsOnRow key={`${row.accusation}-${i}`} row={row} wording={wording} />
      ))}

      {/* Piece 2e — the scan's reason, on BOTH wrappers. Until ruling R3 it was
          deleted the moment a card was included, taking with it the one sentence
          answering "why does this matter here". */}
      {view.scanReason && (
        <div style={{ ...contextStyle, fontStyle: "italic" }}>
          <span style={{ fontStyle: "normal", fontWeight: 600 }}>
            {view.scanReason.label}
          </span>{" "}
          {view.scanReason.text}
        </div>
      )}

      {/* Piece 2f — context behind ONE click. Nothing on the card repeats the
          A: text, which is what the always-open context panel used to do. */}
      {view.context && (
        <div>
          <button
            type="button"
            onClick={(event) => {
              event.stopPropagation();
              setContextOpen(!contextOpen);
            }}
            style={{
              border: "none",
              background: "none",
              padding: 0,
              color: "var(--accent-primary)",
              cursor: "pointer",
              fontFamily: "inherit",
              fontSize: "12.5px",
            }}
          >
            {contextOpen ? wording.context_hide_label : wording.context_show_label} ▸
          </button>
          {contextOpen && (
            <div style={{ ...contextPanelStyle, marginTop: "6px" }}>
              {view.context.beforeNotice && (
                <>
                  <span style={edgeStyle}>{view.context.beforeNotice}</span>
                  <br />
                </>
              )}
              {view.context.before && <span>{view.context.before} </span>}
              <mark style={markStyle}>{view.answer.anchor}</mark>
              {view.context.after && <span> {view.context.after}</span>}
              {view.context.afterNotice && (
                <>
                  <br />
                  <span style={edgeStyle}>{view.context.afterNotice}</span>
                </>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default EvidenceCardBody;
