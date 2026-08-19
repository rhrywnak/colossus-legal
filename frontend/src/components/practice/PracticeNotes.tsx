// =============================================================================
// PracticeNotes.tsx — Chuck, Marie and Roman writing to each other (task B4)
// =============================================================================
//
// One panel, used in two places: the scenario's, at the foot of the start card,
// and the question's, on the review page above the attempts (Roman's amendment
// 2). The per-attempt notes on the review page use the same add line but render
// inline, as the mockup draws them.
//
// ## Collapsed by default (Roman's amendment 3)
//
// The header is `Notes (3)` and it is the fold. The count is of UNSTRUCK notes,
// because the number is asking her to go and read something and a note somebody
// withdrew is not that.
//
// The disclosure arrow is drawn by the control, NOT stored in the heading — the
// A8 lesson, applied before it could be repeated. A `▸` in the string would sit
// frozen beside a marker that turns.
//
// ## Nothing is deleted
//
// `Strike` is the whole of "take it back", and a struck note stays on screen,
// struck through, saying who struck it and when. A note somebody could delete is
// a note nobody can rely on having been read.

import React from "react";

import type { PracticeNote, PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as e from "./practiceEditorStyles";
import * as s from "./practiceStyles";

// ## The author picker is gone (2026-08-19)
//
// This panel used to open with a "who is writing this?" dropdown, filled from a
// stored `practice_note_authors` list, and Save stayed disabled until a name was
// picked. Both are removed. Every request already arrives authenticated, so the
// server writes the signed-in user; and an allow-list of display names would have
// been a way to lock a real signed-in user out of writing a note about a
// witness's testimony, silently, because Authentik spells their name differently
// than a settings row does. The settings rows are DELETED by the hotfix
// migration rather than left lying about unread.

interface AddProps {
  wording: PracticeWording;
  /** The stored placeholder — a different sentence on an attempt. */
  placeholderKey: string;
  onSave: (text: string) => void;
  /** True while a write is in flight. */
  saving: boolean;
}

/**
 * The input and Save. The author is the signed-in user; nothing asks.
 *
 * Save stays DISABLED while a write is in flight or the box is empty — the
 * server refuses a blank note anyway, and disabling is the honest way to say so
 * before the click rather than after it.
 */
export const NoteAdd: React.FC<AddProps> = ({ wording, placeholderKey, onSave, saving }) => {
  const w = (key: string) => wordingOf(wording, key);
  const [text, setText] = React.useState("");

  return (
    <div style={e.noteAdd}>
      <input
        style={e.noteInput}
        placeholder={w(placeholderKey)}
        value={text}
        onChange={(event) => setText(event.target.value)}
        aria-label={w(placeholderKey)}
      />
      <button
        type="button"
        style={e.noteControl}
        disabled={saving || text.trim() === ""}
        title={text.trim() === "" ? w("notes_empty_hint") : undefined}
        onClick={() => {
          onSave(text.trim());
          setText("");
        }}
      >
        {w("notes_save_label")}
      </button>
    </div>
  );
};

interface RowProps {
  wording: PracticeWording;
  note: PracticeNote;
  onStrike: (note: PracticeNote) => void;
  striking: boolean;
}

/** One note: who and when on the left, what they said on the right. */
export const NoteRow: React.FC<RowProps> = ({ wording, note, onStrike, striking }) => {
  const w = (key: string) => wordingOf(wording, key);
  return (
    <div style={e.note}>
      <div style={e.noteWho}>
        <b style={e.noteAuthor}>{note.author}</b>
        {note.when}
        {/* The struck line sits under the author, as the mockup draws it. Its
            PRESENCE is also what strikes the text through — one field, so a
            note cannot render struck without saying when. */}
        {note.struck !== null && <div>{note.struck}</div>}
      </div>
      <div style={note.struck !== null ? e.noteStruck : undefined}>
        {note.text}{" "}
        {note.struck === null && (
          <button
            type="button"
            style={{ ...e.reviewLink, marginLeft: 6 }}
            disabled={striking}
            onClick={() => onStrike(note)}
          >
            {w("notes_strike_label")}
          </button>
        )}
      </div>
    </div>
  );
};

interface Props {
  wording: PracticeWording;
  notes: PracticeNote[];
  /** `notes_scenario_title` or `notes_question_title`. */
  titleKey: string;
  onSave: (text: string) => void;
  onStrike: (note: PracticeNote) => void;
  saving: boolean;
  /** The last write's failure sentence, or null. Never swallowed. */
  error: string | null;
}

const PracticeNotes: React.FC<Props> = ({
  wording,
  notes,
  titleKey,
  onSave,
  onStrike,
  saving,
  error,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  const [open, setOpen] = React.useState(false);
  // Unstruck only: the count is asking her to read something, and a withdrawn
  // note is not something waiting for her.
  const standing = notes.filter((note) => note.struck === null).length;

  return (
    <div style={e.notePanel}>
      <button
        type="button"
        style={e.noteToggle}
        aria-expanded={open}
        data-practice-link
        onClick={() => setOpen((was) => !was)}
      >
        {w("notes_heading_template").replace("{n}", String(standing))} {open ? "▾" : "▸"}
      </button>

      {open && (
        <>
          <div style={{ ...s.row, justifyContent: "space-between", marginTop: 6 }}>
            <b>{w(titleKey)}</b>
            <span style={{ ...s.sub, fontSize: 13 }}>{w("notes_hint")}</span>
          </div>

          {/* A named absence, never a blank panel: an empty box reads as a list
              that failed to load. */}
          {notes.length === 0 && <p style={s.sub}>{w("notes_empty")}</p>}

          {notes.map((note) => (
            <NoteRow
              key={note.id}
              wording={wording}
              note={note}
              onStrike={onStrike}
              striking={saving}
            />
          ))}

          <NoteAdd
            wording={wording}
            placeholderKey="notes_placeholder"
            onSave={onSave}
            saving={saving}
          />

          {/* Standing Rule 1: a failed write says so, beside the control it
              failed for, and says nothing was written. */}
          {error !== null && (
            <div style={{ ...s.feedback, marginTop: 8 }} role="alert">
              {error}
            </div>
          )}
        </>
      )}
    </div>
  );
};

export default PracticeNotes;
