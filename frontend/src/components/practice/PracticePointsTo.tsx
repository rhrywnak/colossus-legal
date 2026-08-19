// =============================================================================
// PracticePointsTo.tsx — "I'd point to…" (task A7)
// =============================================================================
//
// A small control under the answer box that opens THIS scenario's receipts —
// her three points' exhibits, and the documents her questions stand on. She
// picks none, one or several; what she picked rides with the answer, shows on
// the reveal, and prints on Chuck's sheet.
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
import * as f from "./practiceFlowStyles";

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
  const [open, setOpen] = React.useState(false);

  // A scenario with no receipts at all renders NOTHING — not an empty list and
  // not a control that opens onto one. An empty picker reads as a list that
  // failed to load, which is the one thing it must not look like.
  if (receipts.length === 0) return null;

  const toggle = (receipt: string) => {
    onChange(
      picked.includes(receipt)
        ? picked.filter((held) => held !== receipt)
        : [...picked, receipt],
    );
  };

  return (
    <div>
      <button
        type="button"
        style={f.pointsToToggle}
        aria-expanded={open}
        onClick={() => setOpen((was) => !was)}
        disabled={disabled}
      >
        {open ? w("points_to_done_label") : w("points_to_label")}
      </button>

      {open && (
        <div style={f.pointsToBox}>
          {receipts.map((receipt) => (
            <label key={receipt} style={f.pointsToItem}>
              <input
                type="checkbox"
                checked={picked.includes(receipt)}
                onChange={() => toggle(receipt)}
                disabled={disabled}
              />{" "}
              {receipt}
            </label>
          ))}
        </div>
      )}

      {/* Echoed while the list is CLOSED so she can see what she picked without
          reopening it. See `RECEIPT_JOIN` for why the separator is a constant. */}
      {!open && picked.length > 0 && (
        <div style={f.pointsToChosen}>
          {w("points_to_reveal_prefix")} {picked.join(RECEIPT_JOIN)}
        </div>
      )}
    </div>
  );
};

export default PracticePointsTo;
