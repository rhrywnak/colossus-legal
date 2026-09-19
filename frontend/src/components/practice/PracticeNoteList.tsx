// =============================================================================
// PracticeNoteList.tsx — notes as the ruled mockup's bottom panel draws them
// =============================================================================
//
// CC_TASK_REVIEW_LOOP_v1 §4. A blue left border, the author, the words, the day.
// A struck note stays — struck through, with the day it was struck — because a
// note somebody could make vanish is a note nobody can rely on having been read.
//
// Under Marie's deck row it is read-only. On the answers page and the Review
// answers page it is given `onStrike`, and each standing note carries a Strike
// control.
//
// ## Two tones, one behaviour (CC_TASK_REVIEW_PAGE_v1)
//
// The Review answers page draws notes on the amber of owed work, per its ruled
// mockup, rather than on the blue rail Marie's deck row uses. That is the whole
// of the difference: the same strings, the same strike, and struck notes stay
// visible and struck through on BOTH — Roman's ruling of 2026-09-19, and the
// standing law of this table (`practice_notes.rs`: a note somebody could make
// vanish is a note nobody can rely on having been read).
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
  /** Which ground to draw on. Defaults to the blue rail the deck row uses. */
  tone?: r.NoteTone;
}

const PracticeNoteList: React.FC<Props> = ({
  notes,
  wording,
  onStrike,
  busy = false,
  tone = "blue",
}) => {
  if (notes.length === 0) return null;
  const w = (key: string) => wordingOf(wording, key);
  return (
    <div data-practice-notes>
      {notes.map((note) => (
        <div key={note.id} style={r.noteFrame(tone)} data-note-struck={note.struck !== null}>
          <div style={r.noteAuthorFor(tone)}>
            {note.author} · {note.when}
          </div>
          <p style={r.noteText(note.struck !== null, tone)}>{note.text}</p>
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
