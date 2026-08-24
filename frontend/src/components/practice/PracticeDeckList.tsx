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
// ## One side at a time (mockup v8, 2026-08-23)
//
// A picker above the list chooses a side and the list shows THAT SIDE ONLY, in
// the order the deck is authored in. The interleave is gone: it paired each
// defense trap with the redirect that repairs it, which is a courtroom moment
// and not anybody's job here. Chuck's half is drawn in two labelled runs —
// his directs, then his redirects, each redirect quoting the defense question
// it repairs so it can be judged at all.
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
import PracticeSidePicker, { type SideCounts } from "./PracticeSidePicker";
import PrintAntecedent from "./PrintAntecedent";
import { antecedentOf } from "./printSheetPlan";
import { type DeckSectionLabel, sideSections } from "../../pages/practiceQueue";
import { dropPosition } from "../dragReorder";
import * as d from "./practiceDeckStyles";
import * as s from "./practiceStyles";

interface Props {
  /** The WHOLE deck. The list picks its own side out of it, through
      `sideSections`, so the ordering the list draws and the ordering the
      practice walk deals cannot come from two different places. */
  questions: PracticeQuestion[];
  /** Which side is showing. Owned by the page — the practice bar reads the
      same value, so choosing a side to READ also aims the button that
      practises it. Two adjacent controls with the same two labels that
      disagreed would practise the side she is not looking at. */
  side: "george" | "chuck";
  onSide: (side: "george" | "chuck") => void;
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
  /** Where one question's own page lives. The PAGE composes it — no component
      below this one holds route knowledge. */
  questionHref: (question: PracticeQuestion) => string;
  /** Which row has its editor field stack open. Owned above — see the note by
      `adding` for why. */
  fieldsFor: string | null;
  setFieldsFor: React.Dispatch<React.SetStateAction<string | null>>;
}

const PracticeDeckList: React.FC<Props> = ({
  questions,
  side,
  onSide,
  wording,
  editor,
  attachOptions,
  onDelete,
  onUndoDelete,
  deletingId,
  deleteError,
  questionHref,
  fieldsFor,
  setFieldsFor,
}) => {
  const w = (key: string) => wordingOf(wording, key);

  /**
   * A section's heading, chosen from LITERAL keys.
   *
   * `sideSections` names its runs, and passing that name straight to `w()` would
   * hide both keys from the reach scan described at the call site below. So the
   * name is matched here and each key is written out where the scanner can read
   * it. The `else` is the redirects and not a fallback: `DeckSection.labelKey`
   * holds exactly these two values or null, and null never reaches this call.
   */
  const sectionHeading = (labelKey: DeckSectionLabel): string => {
    switch (labelKey) {
      case "directs_subheader":
        return w("directs_subheader");
      case "redirects_subheader":
        return w("redirects_subheader");
      default: {
        // Standing Rule 1. `never` makes a new section label a COMPILE error
        // here rather than a run that quietly wears the redirects' heading;
        // the throw covers the same value arriving from untyped data.
        const unreachable: never = labelKey;
        throw new Error(
          `practice: no heading is defined for the section label ${String(unreachable)}`,
        );
      }
    }
  };

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

  // Both counts, off the WHOLE deck, so the button for the side that is not
  // showing still says how much is behind it.
  const sections = sideSections(questions, side);
  const counts: SideCounts = {
    george: sideSections(questions, "george").reduce(
      (n, part) => n + part.questions.length,
      0,
    ),
    chuck: sideSections(questions, "chuck").reduce(
      (n, part) => n + part.questions.length,
      0,
    ),
  };
  // The rows as RENDERED, flattened — `last` draws the closing rule, and the
  // antecedent lookup resolves against every question the deck still holds
  // rather than only this side's, because a redirect's target is on the other.
  const rows = sections.flatMap((part) => part.questions);
  const visible = questions.filter((q) => !q.hidden);

  return (
    <div style={d.deck}>
      <div style={d.deckHeader}>
        <b>{w("deck_heading")}</b>
      </div>
      {/* The two-sided count line that used to sit beside the heading is gone:
          the picker below prints those same two numbers, and a page that showed
          them twice three inches apart invites a reader to work out why they
          might differ. Its row is untouched in the store. */}
      <PracticeSidePicker
        side={side}
        onSide={onSide}
        counts={counts}
        wording={wording}
      />
      {/* ⚑ BOTH KEYS ARE WRITTEN OUT AS LITERALS, and the choice is made around
          the two calls rather than inside one. `practice_wording_reach_tests`
          scans this file for a literal key inside the helper's parentheses and
          requires each one it finds to be a field on the backend's mirror —
          that scan is the ONLY thing standing between a mistyped key and the
          blank practice page of .407, and a key computed inside the parens is a
          key it cannot see.

          Note the wording of this comment. It does not spell the call shape it
          is describing, because the scanner reads JSX block comments — it only
          strips line comments — and an example written out here is picked up as
          a real call to a key that does not exist. That is the sixth time prose
          about a parser has been eaten by the parser it was about. */}
      <p style={d.countLine}>
        {side === "george"
          ? w("deck_defense_countline")
          : w("deck_chuck_countline")}
      </p>

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

        {sections.map((part) => (
          <React.Fragment key={part.labelKey ?? "only"}>
            {/* The run's heading, when the side has more than one run. Chuck's
                  directs tell the jury the story; his redirects repair what the
                  defense has just asked. Ten rows run together would read as ten
                  opening questions. `sideSections` withholds both labels when
                  there is only one run — a heading above the only section on
                  screen tells a reader a second one exists somewhere. */}
            {part.labelKey !== null && (
              <div style={d.sectionLabel}>{sectionHeading(part.labelKey)}</div>
            )}
            {part.questions.map((question) => (
              <React.Fragment key={question.id}>
                {/* The defense question this one repairs, quoted above it. Drawn
                  by the SHARED component the printed sheets use — a redirect
                  read on its own means nothing, and this is the same judgement
                  Chuck makes on paper. Resolved against the whole visible deck,
                  because the question it follows is on the other side. */}
                {question.kind === "redirect" && (
                  <PrintAntecedent
                    after={antecedentOf(question, visible)}
                    wording={wording}
                    style={d.ante}
                    quoteStyle={d.anteQuote}
                  />
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
                        setDeleted((was) =>
                          was.filter((q) => q.id !== question.id),
                        );
                      }}
                    >
                      {w("row_undo_label")}
                    </button>
                  </div>
                ) : (
                  <PracticeDeckRow
                    key={question.id}
                    question={question}
                    last={question.id === rows[rows.length - 1]?.id}
                    wording={wording}
                    editor={editor}
                    questionHref={questionHref(question)}
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
                      const landing = dropPosition(
                        rows,
                        (q) => q.id,
                        dragging,
                        question.id,
                      );
                      setDragging(null);
                      if (landing !== null)
                        editor.reorder(dragging, landing.before);
                    }}
                    fieldsOpen={fieldsFor === question.id}
                    onToggleFields={() =>
                      setFieldsFor((was) =>
                        was === question.id ? null : question.id,
                      )
                    }
                  />
                )}
              </React.Fragment>
            ))}
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
              <button
                type="button"
                style={s.button}
                onClick={() => setAdding(true)}
              >
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
