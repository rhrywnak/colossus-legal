// =============================================================================
// AccusationFactPicker — choosing one INCLUDED fact (task 2.11 B1)
// =============================================================================
//
// The one control both write concerns share: marking a statement as an instance
// and pairing an answer to one are the same gesture — "pick a fact from this
// scenario" — differing only in what the choice means afterwards.
//
// ## Why the choice is limited to the INCLUDED set
//
// Not a convenience. The rehearsal page is built ON the facts a human ruled in,
// and nothing on that page may come from anywhere else. Marking a candidate
// nobody has ruled on would put an unreviewed statement into the accusation's
// count, and pairing one would put it in Marie's mouth as our answer. The picker
// cannot offer what the section may not hold.
//
// ## Every word here is the store's
//
// The prompt and the cancel label arrive on the payload. The only strings this
// file contains are the fact's OWN — its handle and its quote — which are the
// case's data, not this surface's vocabulary.

import React, { useState } from "react";

import {
  absentStyle,
  chromeInputStyle,
  DIVIDER,
  ghostButtonStyle,
} from "./scenarioSectionStyles";

/** One choosable fact, already reduced to what the picker shows. */
export type PickableFact = {
  graphNodeId: string;
  /** "C-14", or `null` for a candidate nothing has numbered yet. */
  code: string | null;
  /** The statement's own words, as the record has them. */
  text: string;
};

interface Props {
  facts: PickableFact[];
  /** The stored prompt over the list. */
  prompt: string;
  /** The stored label on the close control. */
  cancelLabel: string;
  /** Shown when the filter leaves nothing. */
  noMatchNotice: string;
  /**
   * Shown when there was nothing to offer in the first place.
   *
   * Two stored sentences rather than one, because the remedies differ: the first
   * means type something else, the second means there is nothing left to choose.
   * Collapsing them would leave a human retyping against an empty list.
   */
  emptyNotice: string;
  /**
   * Node ids to leave OUT — the fact being answered, and anything already used
   * for this purpose. Passing them in rather than filtering here keeps the rule
   * with the caller that knows what it means.
   */
  excluded: string[];
  onChoose: (graphNodeId: string) => void;
  onCancel: () => void;
}

const listStyle: React.CSSProperties = {
  maxHeight: "18rem",
  overflowY: "auto",
  border: DIVIDER,
  borderRadius: "8px",
};

const rowStyle: React.CSSProperties = {
  display: "block",
  width: "100%",
  textAlign: "left",
  fontFamily: "inherit",
  fontSize: "13px",
  lineHeight: 1.5,
  padding: "10px 12px",
  border: "none",
  borderBottom: DIVIDER,
  background: "transparent",
  cursor: "pointer",
  color: "var(--text-primary)",
};

const codeStyle: React.CSSProperties = {
  fontWeight: 600,
  marginRight: "0.5rem",
  color: "var(--text-secondary)",
};

const AccusationFactPicker: React.FC<Props> = ({
  facts,
  prompt,
  cancelLabel,
  noMatchNotice,
  emptyNotice,
  excluded,
  onChoose,
  onCancel,
}) => {
  const [filter, setFilter] = useState("");

  const offered = facts.filter((fact) => !excluded.includes(fact.graphNodeId));
  const needle = filter.trim().toLowerCase();
  // A substring test and nothing else — the browser does no ranking and no
  // interpretation. Matching the handle as well as the words is what lets a human
  // who already knows "C-14" type it instead of hunting.
  const shown = needle
    ? offered.filter(
        (fact) =>
          fact.text.toLowerCase().includes(needle) ||
          (fact.code ?? "").toLowerCase().includes(needle),
      )
    : offered;

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "0.5rem",
        padding: "12px 0 4px",
      }}
    >
      <p style={{ margin: 0, fontSize: "13px", color: "var(--text-secondary)" }}>
        {prompt}
      </p>

      <input
        type="search"
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
        // The placeholder is the PROMPT the store already supplies, not a second
        // sentence compiled in here. One stored string, used where it fits.
        placeholder={prompt}
        style={{ ...chromeInputStyle, width: "100%" }}
      />

      {/* An empty list that says nothing reads as a broken control. Which of the
          two empty states it is decides what a human should do next, so the two
          are never the same sentence (Standing Rule 1). */}
      {shown.length === 0 && (
        <p style={absentStyle}>{offered.length === 0 ? emptyNotice : noMatchNotice}</p>
      )}

      {/* The bordered box is hidden when it would be empty: an empty outline
          beneath a notice reads as a list that failed to load. */}
      <div style={{ ...listStyle, display: shown.length === 0 ? "none" : "block" }}>
        {shown.map((fact) => (
          <button
            key={fact.graphNodeId}
            type="button"
            style={rowStyle}
            onClick={() => onChoose(fact.graphNodeId)}
          >
            {/* The handle, when the fact has one. A candidate nothing has
                numbered is still choosable — its words are what identify it. */}
            {fact.code && <span style={codeStyle}>{fact.code}</span>}
            {fact.text}
          </button>
        ))}
      </div>

      <div>
        <button type="button" onClick={onCancel} style={ghostButtonStyle}>
          {cancelLabel}
        </button>
      </div>
    </div>
  );
};

export default AccusationFactPicker;
