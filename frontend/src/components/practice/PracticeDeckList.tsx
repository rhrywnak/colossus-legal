// =============================================================================
// PracticeDeckList.tsx — the deck, listed on the start card (mockups v3 · v4)
// =============================================================================
//
// Roman's ruling of 2026-08-18: Marie reads the questions BEFORE she starts, and
// the list is open by default. Each row carries her three controls — Practice
// this one ▸, Skip today, and Flag.
//
// Mockup v4 puts a SWITCH in the header: "Edit the deck" turns the same list
// into Chuck's editor — arrows, Edit, Hide, and + Add a question. The list is
// the same list; only the controls change.
//
// ## Who signs a change (changed 2026-08-19)
//
// Nobody is asked. The "Editing as ⟨Chuck⟩" dropdown that used to sit beside the
// switch is gone: every write already arrives authenticated and the server signs
// the change from the session. The picker's real cost was not the pixels — the
// editor hook refused every write while it was unset, silently, so Edit appeared
// to work and did nothing.
//
// ## Edit mode is a MODE
//
// While the switch is on, this list's own fold is disabled and the row text
// stops being a link (see `PracticeDeckRow`), because both leave a half-finished
// edit behind. Turning the switch OFF with a row's fields still open asks first,
// naming the row — saved changes are already written and are not at risk; the
// one still in the fields is.
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
import type { PracticeEditor } from "../../pages/usePracticeEditor";
import { wordingOf } from "../../services/practice";
import PracticeAddQuestion from "./PracticeAddQuestion";
import PracticeDeckRow from "./PracticeDeckRow";
import { dropPosition } from "../dragReorder";
import * as d from "./practiceDeckStyles";
import * as s from "./practiceStyles";

interface Props {
  /** This side's questions, in the order the sitting will deal them. */
  questions: PracticeQuestion[];
  wording: PracticeWording;
  /** The editor's state and its four writes — see its hook's header. */
  editor: PracticeEditor;
  /** What a new question may attach to. */
  attachOptions: PracticeAttachOption[];
  /** Remove a question from the deck. The mechanism is the existing hide. */
  onDelete: (question: PracticeQuestion) => void;
  /** Put back the question the last Delete removed. */
  onUndoDelete: (question: PracticeQuestion) => void;
  /** The question whose delete is in flight, or null. */
  deletingId: string | null;
  /** A delete or undo that failed, already composed. */
  deleteError: string | null;
  /** Which row has its editor field stack open. Owned above — see the note by
      `adding` for why. */
  fieldsFor: string | null;
  setFieldsFor: React.Dispatch<React.SetStateAction<string | null>>;
}

const PracticeDeckList: React.FC<Props> = ({
  questions,
  wording,
  editor,
  attachOptions,
  onDelete,
  onUndoDelete,
  deletingId,
  deleteError,
  fieldsFor,
  setFieldsFor,
}) => {
  const w = (key: string) => wordingOf(wording, key);

  // The rows deleted on THIS page-load, in the order they went, so each can
  // leave an undo line where it stood. Deliberately not persisted and
  // deliberately not a restore path: it lives until the page is left or
  // reloaded, which is the whole of what Roman ruled — "no restore path beyond
  // that undo. Do not invent a state."
  const [deleted, setDeleted] = React.useState<PracticeQuestion[]>([]);

  // Whether the add form is showing. `fieldsFor` is NOT here: the control that
  // guards an open field stack — Edit the deck — moved to the title row above,
  // and state must live where the thing that guards it lives, or the guard is
  // reading a copy.
  const [adding, setAdding] = React.useState(false);
  // Which row a drag picked up. Held HERE and not on the row, because a drop is
  // a fact about two rows and only the list knows both.
  const [dragging, setDragging] = React.useState<string | null>(null);

  const george = questions.filter((q) => q.side === "george").length;

  const count =
    w("deck_count_template")
      // `{n}` — the deck total — was dropped from the template on 2026-08-23:
      // the two side counts already add up to it, and a third number is a third
      // thing to read on a line whose whole job is "how many, from whom".
      .replace("{george}", String(george))
      .replace("{chuck}", String(questions.length - george));

  return (
    <div style={d.deck}>
      <div style={d.deckHeader}>
        <b>
          {w("deck_heading")} <span style={d.deckCount}>{count}</span>
        </b>
      </div>

      <>
          {/* Standing Rule 1: a delete or undo that failed says so. A row that
              silently stayed put reads as a control that does nothing. */}
          {deleteError !== null && (
            <div style={{ ...s.feedback, marginTop: 8 }} role="alert">
              {deleteError}
            </div>
          )}
          {/* Standing Rule 1: a failed editor write says so, and says the deck
              is UNCHANGED — an editor who believes an edit landed when it did
              not will not make it again. */}
          {editor.error !== null && (
            <div style={{ ...s.feedback, marginTop: 8 }} role="alert">
              {editor.error}
            </div>
          )}

          {questions.map((question, i) => (
            <React.Fragment key={question.id}>
              {/* The Chuck-view break. A redirect wears Chuck's pill because
                  Chuck asks it — but it is not a question he OPENS with, it is
                  one he asks to repair what the defense just did. Ten rows run
                  together would read as ten opening questions.

                  Only where the kind CHANGES, and only on a list that actually
                  holds both: in Mixed the redirects are interleaved with their
                  defense questions, and a header before each one would fire five
                  times. `questions[i - 1]` is the row above as RENDERED, so this
                  follows whatever order the deck is in rather than assuming one. */}
              {question.kind === "redirect" &&
                (i === 0 || questions[i - 1].kind !== "redirect") &&
                questions.some((q) => q.kind !== "redirect") && (
                  <div style={d.redirectsSubheader}>{w("redirects_subheader")}</div>
                )}
            {deleted.some((q) => q.id === question.id) ? (
              // The undo line, exactly where the row was. It replaces a confirm
              // dialog: a dialog costs a step every time to guard against the
              // rare case; this costs nothing in the normal case and still
              // covers the misclick.
              <div style={d.deletedLine}>
                {w("row_deleted_notice")}{" "}
                <button
                  type="button"
                  style={d.undoLink}
                  data-practice-link
                  onClick={() => {
                    onUndoDelete(question);
                    setDeleted((was) => was.filter((q) => q.id !== question.id));
                  }}
                >
                  {w("row_undo_label")}
                </button>
              </div>
            ) : (
            <PracticeDeckRow
              key={question.id}
              question={question}
              last={i === questions.length - 1}
              wording={wording}
              editor={editor}
              onDelete={() => {
                onDelete(question);
                setDeleted((was) => [...was, question]);
              }}
              deleting={deletingId === question.id}
              dragging={dragging}
              onPickUp={() => setDragging(question.id)}
              onDropHere={() => {
                if (dragging === null) return;
                // The browser computes NEIGHBOURS, never an ordinal — the
                // position is the server's, derived from what is stored. Same
                // rule the scenario-facts drag follows.
                const landing = dropPosition(questions, (q) => q.id, dragging, question.id);
                setDragging(null);
                if (landing !== null) editor.reorder(dragging, landing.before);
              }}
              fieldsOpen={fieldsFor === question.id}
              onToggleFields={() =>
                setFieldsFor((was) => (was === question.id ? null : question.id))
              }
            />
            )}
            </React.Fragment>
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

          {/* Why a row can be blank. It exists because the marks were REMOVED:
              a reader who remembers "answered today · repeat · attempt 2" needs
              telling once that their absence is not a fault in the page. */}
          <p style={d.statusFootnote}>{w("deck_status_footnote")}</p>
      </>
    </div>
  );
};

export default PracticeDeckList;
