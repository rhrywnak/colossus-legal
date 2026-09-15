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

import type {
  PracticeQuestion,
  PracticeWording,
  TacticCard,
} from "../../services/practice";
import type { PracticeEditor } from "../../pages/usePracticeEditor";
import type { EditableField } from "../../services/practiceEditor";
import { useAuth } from "../../context/AuthContext";
import { signedInAs } from "../../services/practiceEditor";
import { wordingOf } from "../../services/practice";
import * as e from "./practiceEditorStyles";
import * as s from "./practiceStyles";

interface Props {
  question: PracticeQuestion;
  wording: PracticeWording;
  editor: PracticeEditor;
  /** The seven cards, from the payload — see `PracticeDeck.tactic_cards`. */
  tacticCards: TacticCard[];
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

const PracticeRowEdit: React.FC<Props> = ({
  question,
  wording,
  editor,
  tacticCards,
  onClose,
}) => {
  const { user } = useAuth();
  const w = (key: string) => wordingOf(wording, key);

  // The stored values as they stand, so Save can tell what actually moved.
  const [text, setText] = React.useState(question.text);
  // ⚑ THE CARD NUMBER, as a string because that is what a `<select>` value is.
  //
  // It used to be seeded from `question.tactic`, which is the RESOLVED NAME —
  // `false premise`, or `compound · braid` on a braid row. The box therefore
  // opened showing a word in a control the server only accepts a number in, and
  // any edit of it was a 400. The number is its own field on the wire now, for
  // exactly this: a form sets cards, a pill shows names.
  const [tactic, setTactic] = React.useState(
    question.tactic_card === null ? "" : String(question.tactic_card),
  );
  const [follows, setFollows] = React.useState(question.follows_key ?? "");
  const [watchFor, setWatchFor] = React.useState(question.watch_for ?? "");
  const [stronger, setStronger] = React.useState(question.stronger ?? "");
  const [receipt, setReceipt] = React.useState(question.receipt ?? "");

  const save = () => {
    // `null` clears an optional field; the server refuses a blank question text.
    const touched: Array<[EditableField, string, string]> = [
      ["text", text, question.text],
      // Compared against the NUMBER it was, not the name it displays — the two
      // are different strings for the same card, and comparing across them would
      // post an edit every time the form was opened and saved untouched.
      [
        "tactic",
        tactic,
        question.tactic_card === null ? "" : String(question.tactic_card),
      ],
      ["follows", follows, question.follows_key ?? ""],
      ["watch_for", watchFor, question.watch_for ?? ""],
      ["stronger", stronger, question.stronger ?? ""],
      // Blank CLEARS it, like every other optional field: the loop below sends
      // `null`, and the server writes SQL NULL rather than the empty string the
      // column's CHECK refuses.
      ["receipt", receipt, question.receipt ?? ""],
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
          {/* A DROPDOWN of the seven cards, not a number box. The options are
              the payload's list — the store's own vocabulary, resolved once on
              the server for both the pills and this. The VALUE is the card
              number, which is what the column holds; the blank option clears
              the tag. */}
          <select
            style={e.editInput}
            value={tactic}
            onChange={(event) => setTactic(event.target.value)}
            aria-label={w("editor_field_tactic")}
          >
            <option value="">{w("editor_tactic_none")}</option>
            {tacticCards.map((card) => (
              <option key={card.card} value={String(card.card)}>
                {card.name}
              </option>
            ))}
          </select>
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

      {/* The `Built from: …` line the row prints under its question. A TEXTAREA
          and not an input, like the question above it: a receipt cites a page of
          the record and runs longer than a line, and a box that shows a fifth of
          what it holds is a box nobody can proof-read. */}
      <Field label={w("editor_field_receipt")}>
        <textarea
          style={e.editTextarea}
          value={receipt}
          onChange={(event) => setReceipt(event.target.value)}
          aria-label={w("editor_field_receipt")}
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
        {/* The signature is the LOGIN's, and this only prints it — the server
            writes it from the session either way, so a slow /api/me cannot
            leave a change unsigned, only this sentence briefly nameless. */}
        <span style={{ ...s.sub, fontSize: 13 }}>
          {w("editor_saved_hint_template").replace("{who}", signedInAs(user))}
        </span>
      </div>
    </div>
  );
};

export default PracticeRowEdit;
