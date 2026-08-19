// =============================================================================
// PracticeChanged.tsx — "Changed since your last sitting" (task B2)
// =============================================================================
//
// The blue box above the deck: a heading naming how many questions changed, who
// changed the newest one and when, plus any new notes; and behind a fold, the
// list in plain words.
//
// ## Why the heading arrives composed
//
// It names a COUNT, a person and a day, and the browser holds no template and no
// date format. Everything after "Changed since" is one string the server built.
//
// ## Why the list is folded
//
// She needs to know whether to re-read the deck. That is the heading. The list
// of `Q3 re-worded · Q6 moved` is what she wants only once she has decided to,
// and an unfolded audit trail on the start card is a box people stop reading.

import React from "react";

import type { PracticeChanged as Changed, PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as e from "./practiceEditorStyles";

interface Props {
  wording: PracticeWording;
  changed: Changed;
}

const PracticeChanged: React.FC<Props> = ({ wording, changed }) => {
  const [open, setOpen] = React.useState(false);

  return (
    <div style={e.changed}>
      <b style={{ color: "var(--practice-navy)" }}>{changed.heading}</b>

      {/* The fold is withdrawn when there is nothing behind it — a sitting where
          only NOTES arrived raises this box with an empty item list, and a
          "what changed" link opening onto nothing reads as a list that failed. */}
      {changed.items.length > 0 && (
        <div style={{ marginTop: 6 }}>
          <button
            type="button"
            style={e.changedSummary}
            aria-expanded={open}
            data-practice-link
            onClick={() => setOpen((was) => !was)}
          >
            {wordingOf(wording, "changed_summary")} {open ? "▾" : "▸"}
          </button>
          {open && (
            <ul style={{ margin: "4px 0 0", paddingLeft: 22 }}>
              {changed.items.map((item, i) => (
                // The list is composed sentences with no id of their own, and it
                // is rebuilt whole on every load — the index is stable for as
                // long as the list exists, which is the life of one render.
                // eslint-disable-next-line react/no-array-index-key
                <li key={i} style={e.changedItem}>
                  {item}
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
};

export default PracticeChanged;
