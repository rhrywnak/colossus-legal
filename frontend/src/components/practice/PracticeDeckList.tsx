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
import type { PracticeDeckControls } from "../../pages/usePracticeDeckControls";
import type { PracticeEditor } from "../../pages/usePracticeEditor";
import { wordingOf } from "../../services/practice";
import PracticeAddQuestion from "./PracticeAddQuestion";
import PracticeDeckRow from "./PracticeDeckRow";
import { dropPosition } from "../dragReorder";
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
  // Which row a drag picked up. Held HERE and not on the row, because a drop is
  // a fact about two rows and only the list knows both.
  const [dragging, setDragging] = React.useState<string | null>(null);

  /**
   * Turn the editor off, asking first if a row's fields are still open.
   *
   * `window.confirm` and not a custom dialog: this is a one-line yes/no about
   * discarding one row's unsaved fields, it must block the state change, and a
   * bespoke modal here would be a second confirm implementation for the same
   * question the browser already answers. The sentence is the STORE'S, with the
   * row's number in it — "the unsaved edit" without saying which row is a
   * question a person cannot answer with two rows on screen.
   */
  const leaveEditing = () => {
    if (fieldsFor !== null) {
      const n = questions.findIndex((q) => q.id === fieldsFor) + 1;
      const asked = w("editor_discard_confirm_template").replace("{n}", String(n));
      if (!window.confirm(asked)) return;
      setFieldsFor(null);
    }
    editor.toggleEditing();
  };

  /**
   * Warn on RELOAD or tab-close while a row's fields are open.
   *
   * Standing Rule 1's shape for a browser event: the only thing a page may do
   * here is set `returnValue`, and the browser prints its own sentence — ours is
   * not allowed through. It covers reload and close, which is where an open edit
   * is actually lost; edit mode itself is deliberately NOT restored on reload
   * (it is a mode, not a place), so there is nothing to come back to.
   */
  React.useEffect(() => {
    if (!editor.editing || fieldsFor === null) return undefined;
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", warn);
    return () => window.removeEventListener("beforeunload", warn);
  }, [editor.editing, fieldsFor]);

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
          <button
            type="button"
            style={e.editSwitch}
            data-practice-link
            aria-pressed={editor.editing}
            onClick={editor.editing ? leaveEditing : editor.toggleEditing}
          >
            {editor.editing ? w("editor_done_label") : w("editor_switch_label")}
          </button>
          <span style={{ color: "var(--practice-separator)" }}>·</span>
          {/* Locked in edit mode: folding the list away with a row's fields
              open is losing the edit without being asked. It carries the
              store's reason rather than refusing in silence. */}
          <button
            type="button"
            style={{ ...d.deckToggle, ...(editor.editing ? e.lockedControl : {}) }}
            data-practice-link
            aria-expanded={open}
            disabled={editor.editing}
            title={editor.editing ? w("editor_busy_hint") : undefined}
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
        </>
      )}
    </div>
  );
};

export default PracticeDeckList;
