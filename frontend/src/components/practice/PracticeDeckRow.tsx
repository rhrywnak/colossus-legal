// =============================================================================
// PracticeDeckRow.tsx — one question on the start card (mockup v4)
// =============================================================================
//
// Split out of `PracticeDeckList` when Part B gave the row a second mode. The
// row now renders two different control strips — Marie's (Practice this one ▸ ·
// Skip today · Flag) and the editor's (▲▼ · Edit · Hide) — plus the status line,
// the badges, and the inline field stack when a row is being edited.
//
// ## The two things that stop being links in edit mode (hotfix §2)
//
// The question TEXT and the status line's `review` link both leave the page: one
// opens a one-question sitting, the other navigates to the review page. In .402
// they stayed live inside the editor, so Roman's first click on a question he
// meant to EDIT threw away the edit and started a drill.
//
// Both become plain text while `editor.editing` is on — not a disabled button
// but no button at all. A disabled control here would be a fourth greyed thing
// on a row that already has three, saying "you may not read this question",
// which is not true; she may, just not from inside the editor.
//
// ## Every string comes from the payload
//
// Not one literal sentence. `w()` reads the store and THROWS by name on a
// missing key rather than rendering a blank control.

import React from "react";

import type { PracticeQuestion, PracticeWording } from "../../services/practice";
import type { PracticeDeckControls } from "../../pages/usePracticeDeckControls";
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

/**
 * The badges beside the pill, in the mockup's order.
 *
 * Three different facts, three colours, and none of them can carry another:
 * `changed` is about her (re-read this), `draft` is about Chuck (edit this), and
 * `redirect` is about the question itself.
 */
const Badges: React.FC<{ question: PracticeQuestion; wording: PracticeWording }> = ({
  question,
  wording,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  return (
    <>
      {question.changed && <span style={e.badge}>{w("badge_changed")}</span>}
      {question.draft_by !== null && <span style={e.badgeDraft}>{w("badge_draft")}</span>}
      {question.kind === "redirect" && (
        <span style={e.badgeRedirect}>{w("redirect_tag")}</span>
      )}
      {question.hidden && <span style={e.badgeHidden}>{w("editor_hidden_badge")}</span>}
    </>
  );
};

interface Props {
  question: PracticeQuestion;
  number: number;
  /** The last row draws the rule that closes the list. */
  last: boolean;
  wording: PracticeWording;
  controls: PracticeDeckControls;
  editor: PracticeEditor;
  editing: boolean;
  draft: string;
  onDraftChange: (draft: string) => void;
  onOpenEditor: () => void;
  onCloseEditor: () => void;
  onPracticeOne: () => void;
  onReview: () => void;
  startingOne: boolean;
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
  number,
  last,
  wording,
  controls,
  editor,
  editing,
  draft,
  onDraftChange,
  onOpenEditor,
  onCloseEditor,
  onPracticeOne,
  onReview,
  startingOne,
  fieldsOpen,
  onToggleFields,
  dragging,
  onPickUp,
  onDropHere,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  const skipped = controls.skippedToday.has(question.id);
  const flagged = question.flag_note !== null && question.flag_note !== "";
  const pill = sidePill(question, wording);
  // Struck through at 40%, not hidden: a skipped row she cannot see is one she
  // cannot put back. A HIDDEN row is greyed instead — only the editor sees it
  // at all, and it is not struck because nothing about it is crossed out.
  const muted = skipped ? d.questionSkipped : question.hidden ? e.hiddenRow : undefined;
  const [dropOver, setDropOver] = useDropTarget();
  // Drag is an EDIT-MODE affordance only. Outside it the row's controls are
  // Marie's, and a deck that re-ordered itself under her hand while she was
  // reading it would be the page rewriting the questions she is about to face.
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
        ...(editor.editing ? { gridTemplateColumns: "34px 56px 1fr auto" } : {}),
        ...(flagged ? d.questionRowFlagged : {}),
        ...(last ? d.questionRowLast : {}),
        // Where the drop would land. Only on OTHER rows: highlighting the row
        // being dragged would say it is about to move onto itself.
        ...(dropOver && dragging !== null && dragging !== question.id
          ? { borderTop: "2px solid var(--practice-navy)" }
          : {}),
      }}
    >
      <div style={d.questionNumber}>{number}</div>

      {/* The arrows column, which only exists in the editor — the mockup adds a
          fourth grid column for it rather than overlaying the number. */}
      {editor.editing && (
        <div>
          {/* The grip, and then the arrows under it. Both do the same job: the
              drag is faster with a mouse, the arrows are the KEYBOARD path and
              stay for exactly that reason — a re-order that only a mouse can
              perform is one Chuck cannot do from the keyboard at all. */}
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
        <Badges question={question} wording={wording} />

        {/* The question text is the link that opens it alone (task A2). A
            `<button>` and not an `<a>`: it runs a handler that opens a session,
            it does not navigate to a URL — and an anchor with no href is not
            reachable by keyboard, which on a witness surface is not a detail. */}
        {editor.editing ? (
          <div style={{ ...d.questionText, ...muted }} data-practice-question>
            {question.text}
          </div>
        ) : (
          <button
            type="button"
            style={{ ...e.questionLink, ...d.questionText, ...muted }}
            data-practice-question
            onClick={onPracticeOne}
            disabled={startingOne}
          >
            {question.text}
          </button>
        )}
        {question.receipt !== null && (
          <div style={{ ...d.questionSource, ...muted }}>{question.receipt}</div>
        )}

        {/* What happened to this question, composed by the server, coloured by
            the RAW mark rather than by the rendered word — the word is a
            Settings row, and matching on it would drop the colour the first
            time somebody edited it. NOTHING renders on a question nobody has
            answered: an empty status line reads as one that failed to load. */}
        {question.status !== null && (
          <div
            style={{
              ...e.status,
              ...(e.statusColour[question.status_mark ?? ""] ?? {}),
            }}
          >
            {question.status}
            {/* The status stays; only the way OUT of the page goes. */}
            {!editor.editing && (
              <>
                {" "}
                <button
                  type="button"
                  style={e.reviewLink}
                  data-practice-link
                  onClick={onReview}
                >
                  {w("row_review_link")}
                </button>
              </>
            )}
          </div>
        )}

        {flagged && (
          <div style={d.flagged}>
            ⚑ {w("flag_shown_template").replace("{note}", question.flag_note ?? "")}
          </div>
        )}

        {editing && (
          <div style={d.flagLine}>
            <input
              style={d.flagInput}
              placeholder={w("flag_placeholder")}
              value={draft}
              onChange={(event) => onDraftChange(event.target.value)}
              aria-label={w("flag_placeholder")}
            />
            <button
              type="button"
              style={d.rowButton}
              disabled={controls.savingFlagFor === question.id}
              onClick={() => {
                controls.saveFlag(question.id, draft);
                onCloseEditor();
              }}
            >
              {w("flag_save_label")}
            </button>
            <button type="button" style={d.rowButton} onClick={onCloseEditor}>
              {w("flag_cancel_label")}
            </button>
          </div>
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
        {editor.editing ? (
          <>
            <button
              type="button"
              style={d.rowButton}
              disabled={!editor.ready}
              onClick={onToggleFields}
            >
              {w("editor_edit_label")}
            </button>
            <button
              type="button"
              style={d.rowButton}
              disabled={!editor.ready}
              onClick={() => editor.hide(question.id, !question.hidden)}
            >
              {question.hidden ? w("editor_unhide_label") : w("editor_hide_label")}
            </button>
          </>
        ) : (
          <>
            {/* Roman's amendment 1: a BUTTON, the same style and size as Skip
                today, and to its LEFT. The question text stays a link as well;
                both do the same thing. */}
            <button
              type="button"
              style={d.rowButton}
              onClick={onPracticeOne}
              disabled={startingOne}
            >
              {w("row_practice_this_label")}
            </button>
            <button
              type="button"
              style={skipped ? d.rowButtonSkipped : d.rowButton}
              aria-pressed={skipped}
              onClick={() => controls.toggleSkip(question.id)}
            >
              {skipped ? w("skipped_today_label") : w("skip_today_label")}
            </button>
            <button
              type="button"
              style={d.rowButton}
              onClick={() => (editing ? onCloseEditor() : onOpenEditor())}
            >
              {flagged ? w("flag_edit_label") : w("flag_label")}
            </button>
          </>
        )}
      </div>
    </div>
  );
};

export default PracticeDeckRow;
