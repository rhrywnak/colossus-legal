// =============================================================================
// PracticeDeckList.tsx — the deck, listed on the start card (mockups v3 · v4)
// =============================================================================
//
// Roman's ruling of 2026-08-18: Marie reads the questions BEFORE she starts, and
// the list is open by default. Each row carries her three controls — Practice
// this one ▸, Skip today, and Flag.
//
// Mockup v4 puts a SWITCH in the header: "Edit the deck" turns the same list
// into Chuck's editor — arrows, Edit, Hide, and + Add a question — with
// "Editing as" beside it. The list is the same list; only the controls change.
//
// ## Who sees what
//
// Marie never presses Edit the deck. There is one login, so nothing enforces
// that in the browser — "Editing as" is the honest substitute, and the SERVER
// refuses a change signed by somebody the store does not list as an editor.
//
// ## Every string here comes from the payload
//
// Not one literal sentence. `w()` reads the store and THROWS by name on a
// missing key rather than rendering a blank control.

import React from "react";

import type {
  PracticeAttachOption,
  PracticeQuestion,
  PracticeWording,
} from "../../services/practice";
import type { PracticeDeckControls } from "../../pages/usePracticeDeckControls";
import type { PracticeEditor } from "../../pages/usePracticeEditor";
import { wordingOf } from "../../services/practice";
import { authorsOf } from "./PracticeNotes";
import PracticeAddQuestion from "./PracticeAddQuestion";
import PracticeDeckRow from "./PracticeDeckRow";
import * as d from "./practiceDeckStyles";
import * as e from "./practiceEditorStyles";
import * as s from "./practiceStyles";

interface Props {
  /** This side's questions, in the order the sitting will deal them. */
  questions: PracticeQuestion[];
  wording: PracticeWording;
  /** The row controls' state and handlers — see the hook's header. */
  controls: PracticeDeckControls;
  /** The editor's state and its four writes — see its hook's header. */
  editor: PracticeEditor;
  /** What a new question may attach to. */
  attachOptions: PracticeAttachOption[];
  /** Open a one-question sitting on this question alone (task A2). */
  onPracticeOne: (question: PracticeQuestion) => void;
  /** Open this question's review page (task B3). */
  onReview: (question: PracticeQuestion) => void;
  /** True while that session POST is in flight, so the row can say so. */
  startingOne: boolean;
}

/**
 * The instruction sentence, with the two control labels rendered bold.
 *
 * The stored row carries `{skip}` and `{flag}` rather than the words themselves,
 * so renaming a button cannot leave the sentence naming one that no longer
 * exists. Split on the placeholders rather than injecting HTML: this is React,
 * and `dangerouslySetInnerHTML` over a stored string is how a wording row
 * becomes a script tag.
 */
const Instruction: React.FC<{ wording: PracticeWording }> = ({ wording }) => {
  const w = (key: string) => wordingOf(wording, key);
  const parts = w("deck_instruction_template").split(/(\{skip\}|\{flag\})/);
  return (
    <p style={d.deckInstruction}>
      {parts.map((part, i) => {
        if (part === "{skip}") return <b key={i}>{w("skip_today_label")}</b>;
        if (part === "{flag}") return <b key={i}>{w("flag_label")}</b>;
        return <React.Fragment key={i}>{part}</React.Fragment>;
      })}
    </p>
  );
};

/** "Editing as ⟨Chuck⟩", shown only once the editor is open. */
const EditingAs: React.FC<{ wording: PracticeWording; editor: PracticeEditor }> = ({
  wording,
  editor,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  return (
    <span>
      {w("editor_as_label")}{" "}
      <select
        style={e.editSelect}
        value={editor.editingAs}
        onChange={(event) => editor.setEditingAs(event.target.value)}
        aria-label={w("editor_as_label")}
      >
        <option value="">{w("editor_as_unset")}</option>
        {authorsOf(wording, "editor_authors").map((name) => (
          <option key={name} value={name}>
            {name}
          </option>
        ))}
      </select>
    </span>
  );
};

const PracticeDeckList: React.FC<Props> = ({
  questions,
  wording,
  controls,
  editor,
  attachOptions,
  onPracticeOne,
  onReview,
  startingOne,
}) => {
  const { skippedToday, flagError } = controls;
  const w = (key: string) => wordingOf(wording, key);

  // Open by default — Roman's ruling. Deliberately NOT persisted: the fold is
  // for one page-load, until Chuck rules on whether Marie should see the deck
  // before a drill at all.
  const [open, setOpen] = React.useState(true);
  // Which row has its flag note showing, and what is typed in it. One at a
  // time: two open editors is two half-written complaints and no way to tell
  // which one she meant to save.
  const [editing, setEditing] = React.useState<string | null>(null);
  const [draft, setDraft] = React.useState("");
  // Which row has its EDITOR fields open, and whether the add form is showing.
  const [fieldsFor, setFieldsFor] = React.useState<string | null>(null);
  const [adding, setAdding] = React.useState(false);

  const george = questions.filter((q) => q.side === "george").length;
  const skippedHere = questions.filter((q) => skippedToday.has(q.id)).length;

  const count =
    w("deck_count_template")
      .replace("{n}", String(questions.length))
      .replace("{george}", String(george))
      .replace("{chuck}", String(questions.length - george)) +
    (skippedHere > 0
      ? ` ${w("deck_skipped_suffix_template").replace("{k}", String(skippedHere))}`
      : "");

  const openFlagEditor = (question: PracticeQuestion) => {
    setEditing(question.id);
    setDraft(question.flag_note ?? "");
  };

  return (
    <div style={d.deck}>
      <div style={d.deckHeader}>
        <b>
          {w("deck_heading")} <span style={d.deckCount}>{count}</span>
        </b>
        <span style={e.editBar}>
          {editor.editing && <EditingAs wording={wording} editor={editor} />}
          <button
            type="button"
            style={e.editSwitch}
            data-practice-link
            aria-pressed={editor.editing}
            onClick={editor.toggleEditing}
          >
            {editor.editing ? w("editor_done_label") : w("editor_switch_label")}
          </button>
          <span style={{ color: "var(--practice-separator)" }}>·</span>
          <button
            type="button"
            style={d.deckToggle}
            data-practice-link
            aria-expanded={open}
            onClick={() => setOpen((was) => !was)}
          >
            {open ? w("deck_hide_link") : w("deck_show_link")}
          </button>
        </span>
      </div>

      {open && (
        <>
          <Instruction wording={wording} />
          {flagError !== null && <p style={d.flagged}>{flagError}</p>}
          {/* Standing Rule 1: a failed editor write says so, and says the deck
              is UNCHANGED — an editor who believes an edit landed when it did
              not will not make it again. */}
          {editor.error !== null && (
            <div style={{ ...s.feedback, marginTop: 8 }} role="alert">
              {editor.error}
            </div>
          )}

          {questions.map((question, i) => (
            <PracticeDeckRow
              key={question.id}
              question={question}
              number={i + 1}
              last={i === questions.length - 1}
              wording={wording}
              controls={controls}
              editor={editor}
              editing={editing === question.id}
              draft={draft}
              onDraftChange={setDraft}
              onOpenEditor={() => openFlagEditor(question)}
              onCloseEditor={() => setEditing(null)}
              onPracticeOne={() => onPracticeOne(question)}
              onReview={() => onReview(question)}
              startingOne={startingOne}
              fieldsOpen={fieldsFor === question.id}
              onToggleFields={() =>
                setFieldsFor((was) => (was === question.id ? null : question.id))
              }
            />
          ))}

          {editor.editing &&
            (adding ? (
              <PracticeAddQuestion
                wording={wording}
                attachOptions={attachOptions}
                ready={editor.ready}
                onAdd={(question) => {
                  editor.add(question);
                  setAdding(false);
                }}
                onCancel={() => setAdding(false)}
              />
            ) : (
              <div style={{ ...s.row, marginTop: 10 }}>
                <button type="button" style={s.button} onClick={() => setAdding(true)}>
                  {w("editor_add_label")}
                </button>
              </div>
            ))}
        </>
      )}
    </div>
  );
};

export default PracticeDeckList;
