// =============================================================================
// FactRemoveControl — taking a fact back out, from the row it is on (2.12, G)
// =============================================================================
//
// Until this task the only way out was to return to the candidate card and undo
// there — the same two-pass problem item A fixes on the other surface.
//
// ## Why this is its own file
//
// `WorkingView` was 231 non-comment lines before task 2.12 and the confirmation
// flow would have taken it past the 300-line limit (Rule 17). The seam is real:
// that file is THE TABLE (which rows exist, how they are searched and laid out)
// and this is ONE CONTROL and the question it asks before acting.

import React, { useState } from "react";

import type { WorkingRow } from "./factsTable";
import { fillCode, type LinkPanelWording } from "../services/evidenceLinks";

/**
 * What stands in for a candidate number the row does not have.
 *
 * A code-less row cannot name itself, and the stored question still reads
 * correctly with a dash in the slot. The alternative — silently dropping the
 * confirmation for exactly the rows that are hardest to identify — would be
 * worse. A dash is punctuation, not vocabulary, which is why it is not a stored
 * string.
 */
const CODELESS = "—";

/**
 * The row's Remove control, and its confirmation (task 2.12, item G).
 *
 * ## Why it asks, and why the question is a sentence
 *
 * Removing an evidence fact undoes a ruling the human made deliberately, and the
 * consequence is not obvious from the word "Remove" — the item does not vanish,
 * it goes back to the queue unruled. So the confirmation says both halves in
 * plain words, from the store: what is being removed (by its C-code, which is
 * why `{code}` is a required placeholder) and where it goes.
 *
 * ## Why it is inline rather than a `window.confirm`
 *
 * A native dialog cannot carry stored wording, cannot be styled, and blocks the
 * page — and this sits in a scroll region where the human may want to look at
 * the row again before deciding. Two buttons in the row keep the subject of the
 * question on screen beside the question.
 *
 * A human fact skips it (`confirm` is `null`): its text is the author's own and
 * deleting it has no second meaning to explain.
 */
const RemoveControl: React.FC<{
  row: WorkingRow;
  onRemove: () => void;
  confirm: LinkPanelWording | null;
}> = ({ row, onRemove, confirm }) => {
  const [asking, setAsking] = useState(false);

  // Task 2.13c item 6: Remove reads as a CONTROL, and a destructive one.
  //
  // It was 12px muted grey with no border — Roman's report was that it did not
  // look like a button at all, and at 3.10:1 (before 2.13b) it was barely
  // legible either. Danger tokens say what it does; the transparent border
  // reserves the space so the hover outline cannot shift the row's layout, which
  // is what a border appearing from nothing would do on a list of forty-six.
  const [hovered, setHovered] = useState(false);
  const buttonStyle: React.CSSProperties = {
    border: `1px solid ${hovered ? "var(--state-danger-strong)" : "transparent"}`,
    borderRadius: "6px",
    background: "none",
    color: "var(--state-danger-strong)",
    cursor: "pointer",
    fontFamily: "inherit",
    fontSize: "0.8rem",
    padding: "0.1rem 0.45rem",
  };

  // A HUMAN fact keeps the control it has always had, unchanged: one button, no
  // confirmation, and the literal 1.7D wrote. That word is pre-existing wording
  // and belongs to task 3.16's sweep, not to this task — and deleting a human's
  // own note needs no explanation of where it goes, because it goes nowhere.
  if (!confirm) {
    return (
      <button
        type="button"
        onClick={onRemove}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        style={buttonStyle}
      >
        Remove
      </button>
    );
  }

  // Everything below is the EVIDENCE path, and every word of it comes from the
  // store (R4). It is only ever rendered once the wording has loaded — the table
  // withholds the control entirely until then, because a control that cannot say
  // what it is about must not be offered.
  const asksAbout = fillCode(confirm.fact_remove_confirm_template, row.code ?? CODELESS);

  if (!asking) {
    return (
      <button
        type="button"
        onClick={() => setAsking(true)}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        style={buttonStyle}
        // The accessible name names the row: a table of forty-six buttons all
        // reading "Remove" tells a screen reader nothing about which is which.
        // It is the stored question, so it says the same thing the visible
        // confirmation will.
        aria-label={asksAbout}
      >
        {confirm.fact_remove_label}
      </button>
    );
  }

  return (
    <span style={{ display: "flex", gap: "0.5rem", alignItems: "center", flexWrap: "wrap" }}>
      <span style={{ fontSize: "0.78rem", color: "var(--text-primary)" }}>{asksAbout}</span>
      <button
        type="button"
        onClick={() => {
          setAsking(false);
          onRemove();
        }}
        style={{ ...buttonStyle, color: "var(--state-danger-strong)", fontWeight: 600 }}
      >
        {confirm.fact_remove_confirm_yes}
      </button>
      <button type="button" onClick={() => setAsking(false)} style={buttonStyle}>
        {confirm.fact_remove_confirm_cancel}
      </button>
    </span>
  );
};

export default RemoveControl;
