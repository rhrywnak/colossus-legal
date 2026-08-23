// =============================================================================
// PracticeDeckRow.tsx — one question on the list (mockup v7, view 1)
// =============================================================================
//
// ## What a row IS, after the cuts
//
// The side and tactic pills · the question text · the `Built from:` line ·
// `Answered on 22 Aug` when an answer exists and NOTHING when it does not · a
// Delete button. That is the whole surface.
//
// ## What it stopped being, and why
//
// It carried a sequential number, three controls of Marie's (Practice this
// one ▸ · Skip today · Flag), a composed status line with a `review` link, a
// re-read badge, and — inside the editor — a fourth control called Hide. Roman
// named the four things anyone actually does on this page, and none of those
// served any of them. `CC_TASK_PRACTICE_ONE_PAGE` §3 removes them.
//
// ## Domain note: Delete IS the old Hide, relabelled and brought into the open
//
// The mechanism underneath is unchanged — the question is hidden, never
// deleted — so a question Marie has answered can never be orphaned from her
// answers, and `practice_answers.question_id` keeps its `ON DELETE RESTRICT`
// meaning. What changed is who can reach it: Hide lived inside edit mode, which
// lived behind a text link Chuck could not find. The user's contract is "I will
// not see this again", and that is what is kept.
//
// The editor's own Hide button is therefore GONE — two controls for one
// mechanism, one of them hidden, is how the feature went unused for a week.
//
// ## Every string comes from the payload
//
// Not one literal sentence. `w()` reads the store and THROWS by name on a
// missing key rather than rendering a blank control.

import React from "react";

import type { PracticeQuestion, PracticeWording } from "../../services/practice";
import type { PracticeEditor } from "../../pages/usePracticeEditor";
import { wordingOf } from "../../services/practice";
import { DragHandle, reorderProps, useDropTarget } from "../dragReorder";
import * as d from "./practiceDeckStyles";
import * as e from "./practiceEditorStyles";
import PracticeRowEdit from "./PracticeRowEdit";
import * as s from "./practiceStyles";

/**
 * The pill on a row: George, Chuck, or the braid's third colour.
 *
 * A braid is answered differently from either side, which is why the mockup
 * gives it a colour of its own rather than a George pill with a note.
 */
const sidePill = (question: PracticeQuestion, wording: PracticeWording) => {
  const w = (key: string) => wordingOf(wording, key);
  if (question.braid) return { style: s.pillBraid, label: w("pill_braid") };
  if (question.side === "george") return { style: s.pillGeorge, label: w("pill_george") };
  return { style: s.pillChuck, label: w("pill_chuck") };
};

interface Props {
  question: PracticeQuestion;
  /** The last row draws the rule that closes the list. */
  last: boolean;
  wording: PracticeWording;
  editor: PracticeEditor;
  /** Remove this question from the deck. The mechanism is the existing hide. */
  onDelete: () => void;
  /** True while this row's delete is in flight. */
  deleting: boolean;
  /** True when this row's inline field stack is open. */
  fieldsOpen: boolean;
  onToggleFields: () => void;
  /** The row a drag picked up, or null. The list owns it — a drop needs both ends. */
  dragging: string | null;
  onPickUp: () => void;
  onDropHere: () => void;
}

const PracticeDeckRow: React.FC<Props> = ({
  question,
  last,
  wording,
  editor,
  onDelete,
  deleting,
  fieldsOpen,
  onToggleFields,
  dragging,
  onPickUp,
  onDropHere,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  const pill = sidePill(question, wording);
  const [dropOver, setDropOver] = useDropTarget();
  // Drag is an EDIT-MODE affordance only. Outside it the row is Marie's, and a
  // deck that re-ordered itself under her hand while she was reading it would
  // be the page rewriting the questions she is about to face.
  const canDrag = editor.editing && editor.ready;

  return (
    <div
      {...reorderProps({
        enabled: canDrag,
        onPickUp,
        onDropHere,
        onHover: setDropOver,
      })}
      style={{
        ...d.questionRow,
        // Three columns outside the editor, four inside it — the arrows get a
        // column of their own rather than overlaying anything.
        gridTemplateColumns: editor.editing ? "56px 1fr auto" : "1fr auto",
        ...(last ? d.questionRowLast : {}),
        // Where the drop would land. Only on OTHER rows: highlighting the row
        // being dragged would say it is about to move onto itself.
        ...(dropOver && dragging !== null && dragging !== question.id
          ? { borderTop: "2px solid var(--practice-navy)" }
          : {}),
      }}
    >
      {editor.editing && (
        <div>
          {/* The grip, and the arrows under it. Both do the same job: the drag
              is faster with a mouse, the arrows are the KEYBOARD path and stay
              for exactly that reason — a re-order only a mouse can perform is
              one Chuck cannot do from the keyboard at all. */}
          <DragHandle hint={w("editor_drag_hint")} style={{ fontSize: 13 }} />
          <button
            type="button"
            style={e.arrowButton}
            aria-label={w("editor_up_label")}
            title={w("editor_up_label")}
            disabled={!editor.ready}
            onClick={() => editor.move(question.id, "up")}
          >
            ▲
          </button>
          <button
            type="button"
            style={e.arrowButton}
            aria-label={w("editor_down_label")}
            title={w("editor_down_label")}
            disabled={!editor.ready}
            onClick={() => editor.move(question.id, "down")}
          >
            ▼
          </button>
        </div>
      )}

      <div>
        <span style={{ ...pill.style, fontSize: 12 }}>{pill.label}</span>
        {question.tactic !== null && <span style={s.tacticTag}>{question.tactic}</span>}

        {/* Plain text in L2. The question text becomes the click target when
            the page it opens exists — building the link first would ship a row
            whose one affordance leads nowhere. */}
        <div style={d.questionText} data-practice-question>
          {question.text}
        </div>

        {question.receipt !== null && <div style={d.questionSource}>{question.receipt}</div>}

        {/* The ONE status a row carries, composed by the server. A question
            nobody has answered renders NOTHING here: an empty line under a
            question reads as a status that failed to load, which is a different
            fact and the wrong one to show. */}
        {question.answered_on !== null && (
          <div style={e.status}>{question.answered_on}</div>
        )}

        {fieldsOpen && (
          <PracticeRowEdit
            question={question}
            wording={wording}
            editor={editor}
            onClose={onToggleFields}
          />
        )}
      </div>

      <div style={d.rowControls}>
        {editor.editing && (
          <button
            type="button"
            style={d.rowButton}
            disabled={!editor.ready}
            onClick={onToggleFields}
          >
            {w("editor_edit_label")}
          </button>
        )}
        {/* Delete is NOT inside edit mode. Roman: "do not force users into more
            steps than is required." It works whether or not the question has
            been answered, because the mechanism is a hide and her answers are
            never touched. */}
        <button
          type="button"
          style={d.rowDeleteButton}
          disabled={deleting}
          onClick={onDelete}
        >
          {w("row_delete_label")}
        </button>
      </div>
    </div>
  );
};

export default PracticeDeckRow;
