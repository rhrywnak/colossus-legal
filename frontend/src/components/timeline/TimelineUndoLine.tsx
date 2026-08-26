// =============================================================================
// TimelineUndoLine.tsx — what stands where a deleted card was (design R10)
// =============================================================================
//
// ## ⚑ THIS LINE IS THE ONLY SAFETY THERE IS
//
// Ruled 2026-08-25: "Delete is soft-delete with an Undo line, no confirm dialog
// — the same pattern ruled on the practice page." So nothing asks "are you
// sure?" anywhere in the chronology. Pressing 🗑 deletes, and this appears in
// the row that card occupied, until the reader navigates away.
//
// That placement is load-bearing rather than decorative. A toast in a corner
// would satisfy the words and miss the point: with no confirmation BEFORE the
// act, the taking-back has to be where the reader is already looking. Which row
// it belongs in is decided by `rowsForPhase`, in the tested pure module.
//
// ## Nothing is ever removed
//
// The event is still in `chronology_events` with `deleted_at` set, and its
// history carries a `deleted` snapshot signed by whoever pressed the button. Undo
// clears the column and appends a `restored` row. Neither act loses anything,
// which is why a delete can be this cheap.

import React from "react";

import type { ChronologyWording, TimelineEvent } from "../../services/caseTimeline";
import { cw } from "../../services/caseTimeline";
import * as w from "./timelineWriteStyles";

type Props = {
  event: TimelineEvent;
  wording: ChronologyWording;
  /**
   * Restore the event.
   *
   * Optional for the same reason the card's controls are: a surface with no
   * writes behind it draws no control rather than an inert one. In practice a
   * line without an Undo cannot occur — nothing puts an event in this list
   * except a delete, and only a page that can delete can do that.
   */
  onUndo?: (event: TimelineEvent) => void;
};

const TimelineUndoLine: React.FC<Props> = ({ event, wording, onUndo }) => (
  <div style={w.undoLine}>
    {/* The em dash is IN the stored words and the joining space is supplied
        here — the repository's rule for a two-part line, so a template can
        never carry a leading or trailing space that a reviewer cannot see. */}
    <span>{cw(wording, "deleted_line_label")}</span>
    <span style={{ fontStyle: "italic" }}>{event.title}</span>
    {onUndo && (
      <button type="button" style={w.undoAction} onClick={() => onUndo(event)}>
        {cw(wording, "undo_label")}
      </button>
    )}
  </div>
);

export default TimelineUndoLine;
