// =============================================================================
// PracticeNoteList.tsx — notes as the ruled mockup's bottom panel draws them
// =============================================================================
//
// CC_TASK_REVIEW_LOOP_v1 §4. A blue left border, the author, the words, the day.
// A struck note stays — struck through, with the day it was struck — because a
// note somebody could make vanish is a note nobody can rely on having been read.
//
// Under Marie's deck row it is read-only. On the answers page it is given
// `onStrike`, and each standing note carries a Strike control.
//
// Every date and the "struck …" line arrive COMPOSED from the server; this
// renders them and formats nothing.

import React from "react";

import { wordingOf, type PracticeNote, type PracticeWording } from "../../services/practice";
import * as r from "./practiceReviewLoopStyles";
import * as s from "./practiceStyles";

interface Props {
  notes: PracticeNote[];
  wording: PracticeWording;
  /** When present, each standing note offers Strike. */
  onStrike?: (note: PracticeNote) => void;
  /** True while a strike is in flight — the controls wait. */
  busy?: boolean;
}

const PracticeNoteList: React.FC<Props> = ({ notes, wording, onStrike, busy = false }) => {
  if (notes.length === 0) return null;
  const w = (key: string) => wordingOf(wording, key);
  return (
    <div data-practice-notes>
      {notes.map((note) => (
        <div key={note.id} style={r.note} data-note-struck={note.struck !== null}>
          <div style={r.noteAuthor}>
            {note.author} · {note.when}
          </div>
          <p style={r.noteText(note.struck !== null)}>{note.text}</p>
          {note.struck !== null && <div style={r.noteMeta}>{note.struck}</div>}
          {note.struck === null && onStrike !== undefined && (
            <button
              type="button"
              style={s.buttonQuiet}
              data-practice-link
              disabled={busy}
              onClick={() => onStrike(note)}
            >
              {w("row_note_strike_label")}
            </button>
          )}
        </div>
      ))}
    </div>
  );
};

export default PracticeNoteList;
