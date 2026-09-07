// =============================================================================
// PointFactRows.tsx — the facts under one talking point (v2.1, change F)
// =============================================================================
//
// One row per included fact that backs this point, and the card a row opens.
// Split out of `TalkingPointsSection` because that file crossed the 300-line
// limit when change F landed (CLAUDE.md rule 17), and split HERE because the
// seam is real: the section owns Marie's sentences and their editor, this owns
// the evidence beneath them.
//
// ## What this component decides: nothing
//
// Which facts appear, in what order, and what each row says are all
// `talkingPointFacts`, pure and tested — Rule 30 again: there is no
// component-test tier here, so a rule decided inside a component is a rule no
// test can reach. This file walks the result and owns only what a value cannot
// hold: which rows are open, and the styling.
//
// ## No third level, and no Backs picker
//
// A row expands to its card and stops. The card's `points` prop is deliberately
// NOT passed: a control offering to move a fact out from under the heading it is
// filed beneath is a control that gets clicked by accident, and the Scenario
// facts list below is the surface that owns that choice. See `FactCardBody`.

import React from "react";

import FactCardBody from "./FactCardBody";
import { factsForPoint, pointFactLine } from "./talkingPointFacts";
import type { AllegationOptions } from "../services/evidenceLinks";
import type { ScenarioCard } from "../services/scenarioCards";

// STRUCTURAL: the punctuation between the three parts of a fact row. Not vocabulary
// and therefore not a stored row — it is the same interpunct this section
// already used inline before v2.1, and the same one the source line on every
// card uses. `aria-hidden` on each: a screen reader announcing "middot" three
// times per row is noise, and the gap already separates the parts in speech.
const ROW_SEPARATOR = "·";

/** The wrapper around one fact row and the card it opens. */
const factBlockStyle: React.CSSProperties = { marginTop: "0.35rem" };

/**
 * One collapsed fact row: a full-width button that reads as a line, not a button.
 *
 * Borderless and background-less on purpose — the mockup draws these as text
 * lines under the point, and a stack of three bordered buttons under every
 * talking point would out-shout the sentence they belong to. `textAlign: left`
 * because a `<button>` centres its content by default, which would leave three
 * rows of different lengths ragged on both edges.
 */
const factRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  gap: "0.5rem",
  flexWrap: "wrap",
  width: "100%",
  textAlign: "left",
  border: "none",
  background: "none",
  padding: "0.2rem 0",
  cursor: "pointer",
  fontFamily: "inherit",
};

const factCodeStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "var(--text-secondary)",
  flexShrink: 0,
};

const factTitleStyle: React.CSSProperties = {
  fontSize: "0.82rem",
  color: "var(--text-primary)",
  minWidth: 0,
};

const factCiteStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "var(--text-muted)",
};

const factSepStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "var(--text-muted)",
  flexShrink: 0,
};

interface Props {
  /** The page's whole card pool; this filters it by `backs_position`. */
  cards: ScenarioCard[];
  /** The talking point's 1-based position, which is also its address. */
  position: number;
  /** The card's words and fold thresholds. The caller withholds this component
   *  entirely until they load — there is no fallback vocabulary for a row. */
  options: AllegationOptions;
  /** The case and scenario a card-field edit is written against. */
  slug: string;
  scenarioId: string;
  /** Which rows are open, by graph node id. Owned by the SECTION, because two
   *  facts under two different points may be open at once and each of these
   *  components sees only its own. */
  openFacts: Set<string>;
  onToggleFact: (graphNodeId: string) => void;
  /** Re-read the deck after a card edit, so the screen matches the store. */
  onCardEdited: () => void;
}

/**
 * The evidence under one talking point.
 *
 * Renders NOTHING when no fact backs this point — not an empty-state line. Three
 * points each apologising for having no evidence yet is three lines of apology
 * on a page whose job is to show the argument, and the Scenario facts list below
 * is where a fact is actually aimed at a point.
 */
const PointFactRows: React.FC<Props> = ({
  cards,
  position,
  options,
  slug,
  scenarioId,
  openFacts,
  onToggleFact,
  onCardEdited,
}) => (
  <>
    {factsForPoint(cards, position).map((factRow) => {
      const line = pointFactLine(factRow, options.fact_card);
      const open = openFacts.has(factRow.graphNodeId);
      return (
        <div key={factRow.graphNodeId} style={factBlockStyle}>
          <button
            type="button"
            style={factRowStyle}
            aria-expanded={open}
            onClick={() => onToggleFact(factRow.graphNodeId)}
          >
            {/* The code is the speakable handle — "pull up C-40" — and leads the
                row for the same reason it leads the card. A card nothing has
                numbered shows no code rather than a blank cell reading as a
                loss, and the separator goes with it so no orphaned dot is left. */}
            {line.code && (
              <>
                <b style={factCodeStyle}>{line.code}</b>
                <span style={factSepStyle} aria-hidden="true">
                  {ROW_SEPARATOR}
                </span>
              </>
            )}
            <span style={factTitleStyle}>{line.title}</span>
            <span style={factSepStyle} aria-hidden="true">
              {ROW_SEPARATOR}
            </span>
            <span style={factCiteStyle}>{line.cite}</span>
          </button>

          {/* The SAME card the facts list renders, opened in place. Not a
              summary of it and not a second card body: one piece of evidence
              reads one way wherever a human meets it (ONE_CARD_GRAMMAR). */}
          {open && factRow.card && (
            <FactCardBody
              card={factRow.card}
              wording={options.fact_card}
              grammar={options.card_grammar}
              questionChars={options.card_question_truncate_chars}
              slug={slug}
              scenarioId={scenarioId}
              collapsed={false}
              onToggle={() => onToggleFact(factRow.graphNodeId)}
              onEdited={onCardEdited}
            />
          )}
        </div>
      );
    })}
  </>
);

export default PointFactRows;
