// =============================================================================
// PracticePointsTo.tsx — "I'd point to…" (task A7)
// =============================================================================
//
// A small control under the answer box that opens THIS scenario's receipts —
// her three points' exhibits, and the documents her questions stand on. She
// picks none, one or several; what she picked rides with the answer, shows on
// the reveal, and prints on Chuck's sheet.
//
// ## Mockup v4 makes it CHIPS, not a fold
//
// v1 built a control that opened a checkbox list. v4 draws the receipts inline
// as pressable chips under the answer box, and that is the better shape for the
// same reason the deck list is open by default: the question "what would I point
// to?" arises while she is typing, and a control she has to open first is one
// she answers without opening.
//
// ## Why it is optional and says nothing when she skips it
//
// It is not a step and not a grade. A witness who answers well and names no
// exhibit has done nothing wrong — Chuck hands the documents up, not her. What
// the control is FOR is letting him see whether the receipt she reached for is
// the one that actually answers the question, which is a different thing from
// whether she reached for one at all.
//
// ## The list reads NOTHING live
//
// It arrives on the deck payload, composed from the seeded receipts and the
// deck's own source lines. Design §5: the tool reads the scenario record, the
// deck and the log, never the graph.

import React from "react";

import type { PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as s from "./practiceStyles";
import * as e from "./practiceEditorStyles";

/**
 * What joins two picked receipts wherever they are printed together.
 *
 * CONST (structural): punctuation between two DATA values, not a sentence. It is
 * the same middle dot the deck count, the row status and the sheet's own clauses
 * already use, and it is exported so the three surfaces that print this list —
 * this control, the reveal, and Chuck's printed sheet — cannot drift into using
 * three different separators for the same list. The settings store deliberately
 * does not hold it: a stored value is trimmed, so it could not carry its own
 * surrounding spaces, and the store cannot validate punctuation.
 */
export const RECEIPT_JOIN = " · ";

interface Props {
  wording: PracticeWording;
  /** This scenario's receipts, already de-duplicated and ordered by the server. */
  receipts: string[];
  /** What she has picked so far. */
  picked: string[];
  onChange: (picked: string[]) => void;
  /** True while the answer is being submitted; the picks are settled by then. */
  disabled: boolean;
}

const PracticePointsTo: React.FC<Props> = ({
  wording,
  receipts,
  picked,
  onChange,
  disabled,
}) => {
  const w = (key: string) => wordingOf(wording, key);

  // A scenario with no receipts at all renders NOTHING — not an empty row of
  // chips and not a label over one. An empty picker reads as a list that failed
  // to load, which is the one thing it must not look like.
  if (receipts.length === 0) return null;

  const toggle = (receipt: string) => {
    onChange(
      picked.includes(receipt)
        ? picked.filter((held) => held !== receipt)
        : [...picked, receipt],
    );
  };

  return (
    <div style={e.pointTo}>
      {/* Mockup v4 makes this a LABEL and a row of chips rather than a control
          that opens a list. Nothing to open means nothing to forget to open,
          and every receipt is readable at a glance while she is still typing —
          which is the moment the question "what would I point to?" arises. */}
      <span style={{ ...s.sub, fontSize: 14 }}>{w("points_to_label")}</span>{" "}
      {receipts.map((receipt) => {
        const on = picked.includes(receipt);
        return (
          <button
            key={receipt}
            type="button"
            style={on ? e.chipOn : e.chip}
            aria-pressed={on}
            disabled={disabled}
            onClick={() => toggle(receipt)}
          >
            {receipt}
          </button>
        );
      })}
    </div>
  );
};

export default PracticePointsTo;
