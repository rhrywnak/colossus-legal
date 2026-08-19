// =============================================================================
// PracticeRowEdit.tsx — the inline fields on a row being edited (task B1)
// =============================================================================
//
// Question · tactic (George's rows) · follows (redirect rows) · watch-for ·
// stronger answer, then Save and Cancel.
//
// ## Why each field is saved as its OWN change
//
// `practice_deck_changes` records one row per field with a before and an after,
// and Marie's list says `Q3 re-worded` or `Q3 — watch_for changed`. Saving five
// fields as one blob would make "what changed" a diff somebody computed rather
// than a fact the editor stated — and re-wording, which she must re-read, would
// stop being distinguishable from a tweak to the watch-for, which she need not.
//
// So Save writes only what was actually touched, one call per field. On a row
// where nothing moved it writes nothing at all, which is the honest outcome for
// a Save nobody changed anything before pressing.

import React from "react";

import type { PracticeQuestion, PracticeWording } from "../../services/practice";
import type { PracticeEditor } from "../../pages/usePracticeEditor";
import type { EditableField } from "../../services/practiceEditor";
import { wordingOf } from "../../services/practice";
import * as e from "./practiceEditorStyles";
import * as s from "./practiceStyles";

interface Props {
  question: PracticeQuestion;
  wording: PracticeWording;
  editor: PracticeEditor;
  onClose: () => void;
}

/** One labelled field in the stack. */
const Field: React.FC<{ label: string; children: React.ReactNode }> = ({
  label,
  children,
}) => (
  <div>
    <label style={e.editLabel}>{label}</label>
    {children}
  </div>
);

const PracticeRowEdit: React.FC<Props> = ({ question, wording, editor, onClose }) => {
  const w = (key: string) => wordingOf(wording, key);

  // The stored values as they stand, so Save can tell what actually moved.
  const [text, setText] = React.useState(question.text);
  const [tactic, setTactic] = React.useState(question.tactic ?? "");
  const [follows, setFollows] = React.useState(question.follows_key ?? "");
  const [watchFor, setWatchFor] = React.useState(question.watch_for ?? "");
  const [stronger, setStronger] = React.useState(question.stronger ?? "");

  const save = () => {
    // `null` clears an optional field; the server refuses a blank question text.
    const touched: Array<[EditableField, string, string]> = [
      ["text", text, question.text],
      ["tactic", String(tactic), question.tactic ?? ""],
      ["follows", follows, question.follows_key ?? ""],
      ["watch_for", watchFor, question.watch_for ?? ""],
      ["stronger", stronger, question.stronger ?? ""],
    ];
    for (const [field, next, before] of touched) {
      if (next.trim() === before.trim()) continue;
      editor.edit(question.id, field, next.trim() === "" ? null : next.trim());
    }
    onClose();
  };

  return (
    <div style={e.editRow}>
      <Field label={w("editor_field_question")}>
        <textarea
          style={e.editTextarea}
          value={text}
          onChange={(event) => setText(event.target.value)}
          aria-label={w("editor_field_question")}
        />
      </Field>

      {/* A tactic belongs to a cross question and nowhere else — a Chuck
          question has no trap in it, and the server refuses one on it. */}
      {question.kind === "cross" && (
        <Field label={w("editor_field_tactic")}>
          <input
            style={e.editInput}
            value={tactic}
            onChange={(event) => setTactic(event.target.value)}
            aria-label={w("editor_field_tactic")}
          />
        </Field>
      )}

      {/* Only a redirect follows a George question. */}
      {question.kind === "redirect" && (
        <Field label={w("editor_field_follows")}>
          <input
            style={e.editInput}
            value={follows}
            onChange={(event) => setFollows(event.target.value)}
            aria-label={w("editor_field_follows")}
          />
        </Field>
      )}

      <Field label={w("editor_field_watch_for")}>
        <input
          style={e.editInput}
          value={watchFor}
          onChange={(event) => setWatchFor(event.target.value)}
          aria-label={w("editor_field_watch_for")}
        />
      </Field>

      <Field label={w("editor_field_stronger")}>
        <input
          style={e.editInput}
          value={stronger}
          onChange={(event) => setStronger(event.target.value)}
          aria-label={w("editor_field_stronger")}
        />
      </Field>

      <div style={{ ...s.row, marginTop: 4 }}>
        <button
          type="button"
          style={s.buttonPrimary}
          disabled={!editor.ready}
          onClick={save}
        >
          {w("editor_save_label")}
        </button>
        <button type="button" style={s.button} onClick={onClose}>
          {w("editor_cancel_label")}
        </button>
        <span style={{ ...s.sub, fontSize: 13 }}>
          {w("editor_saved_hint_template").replace("{who}", editor.editingAs)}
        </span>
      </div>
    </div>
  );
};

export default PracticeRowEdit;
