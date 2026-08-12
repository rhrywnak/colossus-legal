// =============================================================================
// PairCard — one statement, our answer under it, on BOTH pages (task R4, P3)
// =============================================================================
//
// The batch's centrepiece, and the first component this product renders on the
// working page and the rehearsal page from one file.
//
// ## Why one component and not two that look alike
//
// Because they stopped looking alike. The working page's accusation section
// rendered a code and a bare line of text; the prep page rendered a full card
// with speaker, date, kind and source. Same statement, two presentations, and
// the human doing the marking could not see what the human doing the rehearsing
// would read. Roman's ruling: "the rehearsal rendering wins" — so it does, and
// the working page inherits it whole.
//
// ## What the two pages do NOT share
//
// Controls. The working page passes `controls`; the rehearsal page passes
// nothing and renders nothing — no Mark, no Pair, no Unpair, not disabled and
// not greyed. A control Marie cannot use is a control that costs her a glance in
// front of opposing counsel.
//
// That is the only difference, and it is a prop rather than a variant flag on
// purpose: "render these buttons" is a thing a caller supplies, while a
// `mode: "rehearsal"` boolean invites every future difference to hide behind it.
//
// ## The card, top to bottom
//
//   provenance   who (them-coloured) · date · kind chip · source ↗   C-91
//   quote        the operative words highlighted, folded when long
//   OUR ANSWER   green edge, its own provenance line and its own code
//
// The model — which words are highlighted, whether a quote is long enough to
// fold — is decided in `pairCardModel.ts` and tested there. This file renders it.

import React, { useState } from "react";

import { chipStyle } from "./scenarioSectionStyles";
import type {
  PairCardModel,
  PairCardProvenance,
  PairCardSide,
  QuoteFold,
} from "./pairCardModel";

const cardStyle: React.CSSProperties = {
  padding: "16px 18px",
  borderRadius: "10px",
  background: "var(--bg-surface)",
  boxShadow: "var(--shadow-card)",
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

/** Us. The same slot, the same size, the opposite side. */
const ourWhoStyle: React.CSSProperties = {
  fontWeight: 600,
  color: "var(--state-success-strong)",
};

const mutedStyle: React.CSSProperties = { color: "var(--text-muted)" };

const linkStyle: React.CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "none",
};

/**
 * The C-code, small, in the corner.
 *
 * `marginLeft: auto` pushes it to the far edge of the meta row so it lands in
 * the same place on every card — a handle a reader's eye can go straight to
 * rather than hunt for at the end of a variable-length line.
 */
const codeStyle: React.CSSProperties = {
  marginLeft: "auto",
  fontWeight: 600,
  fontSize: "12px",
  color: "var(--text-secondary)",
  flexShrink: 0,
};

const quoteStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "15.5px",
  lineHeight: 1.55,
  color: "var(--text-primary)",
};

/** The words either side of the highlight — present, and visibly secondary. */
const contextStyle: React.CSSProperties = {
  color: "var(--text-muted)",
};

/**
 * The operative words.
 *
 * A background rather than bold: bold inside a quotation reads as emphasis the
 * SPEAKER put there, and this emphasis is ours. A reader must never come away
 * thinking George stressed the words we chose to mark.
 */
const highlightStyle: React.CSSProperties = {
  background: "var(--state-warning-bg-soft, rgba(255, 214, 102, 0.35))",
  borderRadius: "3px",
  padding: "0 2px",
};

/**
 * The visual clamp, applied only when the model says a fold is warranted.
 *
 * `-webkit-line-clamp` is the one property that can cut at a LINE, which is a
 * fact about the rendered box and so cannot be decided in the model. Two lines,
 * per the design.
 */
const clampedStyle: React.CSSProperties = {
  display: "-webkit-box",
  WebkitLineClamp: 2,
  WebkitBoxOrient: "vertical",
  overflow: "hidden",
};

/** Ours. Green edge — the one visual promise on the card. */
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

const foldButtonStyle: React.CSSProperties = {
  border: "none",
  background: "none",
  padding: 0,
  marginTop: "4px",
  color: "var(--accent-primary)",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: "12.5px",
};

/** Who said it, when, in what — and the handle, at the end. */
const Provenance: React.FC<{ of: PairCardProvenance; ours?: boolean }> = ({ of, ours }) => (
  <div style={metaRowStyle}>
    <span style={ours ? ourWhoStyle : whoStyle}>{of.who}</span>
    {/* A card with no date says nothing about dates rather than showing an empty
        slot — 57% of this case's evidence carries none, so this is the ordinary
        state and not a gap in the work. */}
    {of.when && <span style={mutedStyle}>{of.when}</span>}
    {of.kindLabel && <span style={chipStyle}>{of.kindLabel}</span>}
    {of.sourceLabel && (
      <span style={mutedStyle}>
        {/* No link when the record cannot say WHICH document. A link to nowhere
            is worse than no link — a reader clicks it in front of opposing
            counsel. */}
        {of.sourceHref ? (
          <a href={of.sourceHref} style={linkStyle} target="_blank" rel="noreferrer">
            {of.sourceLabel} ↗
          </a>
        ) : (
          of.sourceLabel
        )}
      </span>
    )}
    {of.code && <span style={codeStyle}>{of.code}</span>}
  </div>
);

/**
 * The question a sworn answer answers — one muted line above the quote.
 *
 * ## Why muted, one line, and clamped (task 394, P2)
 *
 * The QUOTE is the evidence and must stay the loudest thing on the card. The
 * question is context: it is what makes "Yes." mean something, and it is not
 * itself the thing Marie reads out. So it sits above, dimmer and smaller, and it
 * is clamped to one line — a four-line interrogatory above every answer would
 * push the actual evidence off the bottom of a card the design keeps compact.
 *
 * The whole question is still in the DOM, so it is selectable, searchable by the
 * browser's own find, and read in full by a screen reader. Only the BOX is
 * shortened — the same treatment `FoldedQuote` gives a long quote, and for the
 * same reason: nothing here cuts the record's words.
 *
 * ## Domain note: the "Q" is the record's, not ours
 *
 * The line renders the question verbatim with no composed prefix. An added "Q:"
 * would be this component putting a word into a transcript, which is the class
 * of thing the highlight rule already forbids for emphasis.
 */
const questionStyle: React.CSSProperties = {
  margin: "0 0 6px",
  fontSize: "13px",
  lineHeight: 1.5,
  color: "var(--text-muted)",
  fontStyle: "italic",
  display: "-webkit-box",
  WebkitLineClamp: 1,
  WebkitBoxOrient: "vertical",
  overflow: "hidden",
};

/**
 * The quote, folded around its highlight.
 *
 * Open and closed are the SAME markup with a clamp added, so the highlight
 * cannot land differently in the two states — the folded view is the full quote
 * with the box shortened, not a second string cut to fit.
 */
const FoldedQuote: React.FC<{
  fold: QuoteFold;
  showLabel: string | null;
  hideLabel: string | null;
}> = ({ fold, showLabel, hideLabel }) => {
  const [open, setOpen] = useState(false);
  const clamped = fold.needsFold && !open;

  return (
    <div>
      <p style={clamped ? { ...quoteStyle, ...clampedStyle } : quoteStyle}>
        {fold.before && <span style={contextStyle}>{fold.before}</span>}
        <mark style={highlightStyle}>{fold.highlight}</mark>
        {fold.after && <span style={contextStyle}>{fold.after}</span>}
      </p>
      {fold.needsFold && (
        <button
          type="button"
          style={foldButtonStyle}
          onClick={() => setOpen(!open)}
          aria-expanded={open}
          // The accessible name when the control has no words of its own. An
          // arrow alone tells a screen reader nothing, and this is the one class
          // of string this codebase already writes in English rather than
          // storing (see `CandidateList`'s region label).
          aria-label={showLabel ? undefined : "Show the whole quotation"}
        >
          {/* WORDS WHEN THE PAGE HAS THEM, an arrow when it does not.
              
              The working page serves `context_show_label` / `context_hide_label`
              — the card grammar's own pair, already used by the evidence card for
              this exact gesture. The prep page serves no equivalent: its
              `expand_all_label` means "expand every instance", and putting "Expand
              all" under ONE quote would be the page saying something untrue to
              save a migration.
              
              So the fold degrades to the chevron the prep page already uses for
              its section folds. A symbol is not vocabulary, so no sentence is
              invented here — and a stored row for these words is filed rather
              than smuggled in. */}
          {showLabel && hideLabel ? (open ? hideLabel : showLabel) : open ? "▾" : "▸"}
        </button>
      )}
    </div>
  );
};

/**
 * The half of a card that is a statement: its question, then its words.
 *
 * Both halves of a pair render through this, which is what stops the answer from
 * quietly missing a field the instance has — the shape of the defect P2 fixes.
 */
const SideQuote: React.FC<{
  side: PairCardSide;
  showLabel: string | null;
  hideLabel: string | null;
}> = ({ side, showLabel, hideLabel }) => (
  <>
    {side.question && (
      // `title` puts the whole question in the browser's own tooltip, so a
      // clamped line is one hover from being read in full without a control —
      // which matters on the prep page, where there are no controls at all.
      <p style={questionStyle} title={side.question} data-pair-question="">
        {side.question}
      </p>
    )}
    <FoldedQuote fold={side.quote} showLabel={showLabel} hideLabel={hideLabel} />
  </>
);

interface Props {
  card: PairCardModel;
  /** The stored "OUR ANSWER" label. */
  answerLabel: string;
  /**
   * Served expand/collapse words for a long quote, or `null`.
   *
   * `null` renders the chevron instead — see `FoldedQuote`. The card never
   * writes these words itself on either page.
   */
  showLabel: string | null;
  hideLabel: string | null;
  /**
   * What to render when nobody has paired an answer — the stored sentence, and
   * `null` on a surface that says it elsewhere.
   *
   * The prep page names its gaps once, in the prep list (ruling C5), so passing
   * `null` there is what keeps the beta.381 duplicate-gap defect closed.
   */
  gapNotice: string | null;
  /**
   * The working page's controls, or nothing.
   *
   * `undefined` on the rehearsal page renders no control area at all — see this
   * file's header for why that is a prop and not a mode flag.
   */
  controls?: React.ReactNode;
  /**
   * A panel this card's own controls opened, rendered INSIDE the card (P1).
   *
   * ## Why the picker lives here and not beside the list
   *
   * Measured in CC_REPORT_PAIRING_PICKER_DEAD_v1: the answer picker rendered
   * every time it was asked for, at one fixed point at the bottom of the
   * accusation panel — 777, 592, 407 and 92 pixels below the four buttons that
   * opened it. It was invisible for every card but the last, and a human
   * clicking "Pair an answer" concluded the control was dead.
   *
   * A panel opened BY a card belongs IN that card. Putting it here rather than
   * after the card means it cannot drift: there is no offset to get wrong,
   * because there is no gap.
   *
   * `undefined` on the rehearsal page, like `controls` and for the same reason —
   * nothing on that page opens anything.
   */
  expansion?: React.ReactNode;
}

const gapStyle: React.CSSProperties = {
  marginTop: "14px",
  padding: "10px 14px",
  borderRadius: "8px",
  background: "var(--state-danger-bg-soft)",
  color: "var(--state-danger-strong)",
  fontSize: "13.5px",
  fontWeight: 600,
};

const PairCard: React.FC<Props> = ({
  card,
  answerLabel,
  showLabel,
  hideLabel,
  gapNotice,
  controls,
  expansion,
}) => (
  <div
    // The anchor a structure test can hold: "the picker renders inside the card
    // whose button opened it" is a claim about containment, and containment
    // needs a marked container to be checkable at all.
    data-pair-card=""
    style={{
      ...cardStyle,
      // The unanswered card carries the red edge; an answered one does not need
      // to shout, because the green block inside it is the answer to the
      // question the edge was asking.
      borderLeft: card.answer ? "none" : "4px solid var(--state-danger-strong)",
    }}
  >
    <Provenance of={card.provenance} />
    <SideQuote side={card} showLabel={showLabel} hideLabel={hideLabel} />

    {card.answer ? (
      <div style={answerStyle}>
        <div style={answerLabelStyle}>{answerLabel}</div>
        {/* OUR ANSWER carries its OWN provenance — who said it, when, and out of
            which document. Until this card, the answer was a line of text with
            no source, which is the half a witness is asked to produce. */}
        <Provenance of={card.answer.provenance} ours />
        {/* …and its own question, through the same leaf. An answer whose
            question rendered only on the accusation half would be the exact
            asymmetry P2 exists to remove. */}
        <SideQuote side={card.answer} showLabel={showLabel} hideLabel={hideLabel} />
      </div>
    ) : (
      gapNotice && <div style={gapStyle}>{gapNotice}</div>
    )}

    {controls && (
      <div
        style={{
          display: "flex",
          gap: "0.5rem",
          flexWrap: "wrap",
          marginTop: "12px",
        }}
      >
        {controls}
      </div>
    )}

    {/* BELOW the controls, inside the card. A panel a button opened sits under
        the button that opened it — see `expansion` for the 777-pixel defect
        this placement ends. */}
    {expansion}
  </div>
);

export default PairCard;
