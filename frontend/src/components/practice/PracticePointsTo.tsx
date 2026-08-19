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
          reopening it. The join is the same separator the reveal and the sheet
          use, and it is drawn here rather than stored: it is punctuation between
          two data values, not a sentence. */}
      {!open && picked.length > 0 && (
        <div style={f.pointsToChosen}>
          {w("points_to_reveal_prefix")} {picked.join(" · ")}
        </div>
      )}
    </div>
  );
};

export default PracticePointsTo;
