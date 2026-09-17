// =============================================================================
// PracticeAnswerNotes.tsx — the notes panel on one question's answers page
// =============================================================================
//
// CC_TASK_REVIEW_LOOP_v1 §4: "Chuck writes a note from the question's answers
// view." Shows the notes on the question and on its CURRENT answer, a box to add
// one, and Strike on each standing note.
//
// ## Where a new note lands
//
// On the CURRENT answer when one exists — that is what Chuck is reading, and a
// note there counts toward Marie's pill until she answers again. With no answer
// yet it lands on the question itself.
//
// ## Always re-read, never patched
//
// After a write or a strike the panel re-reads the question's answers and hands
// them up, so what renders is the server's list — never a local array that could
// disagree with what Marie will see under her row.

import React from "react";

import { wordingOf, type PracticeNote, type PracticeWording } from "../../services/practice";
import { fetchQuestionAnswers, type QuestionAnswers } from "../../services/practiceAnswers";
import { addAnswerNote, addQuestionNote, strikeNote } from "../../services/practiceReviewLoop";
import PracticeNoteList from "./PracticeNoteList";
import * as r from "./practiceReviewLoopStyles";
import * as s from "./practiceStyles";

interface Props {
  questionId: string;
  answers: QuestionAnswers;
  wording: PracticeWording;
  /** The fresh answers after a write, for the page to keep. */
  onChanged: (answers: QuestionAnswers) => void;
}

const PracticeAnswerNotes: React.FC<Props> = ({ questionId, answers, wording, onChanged }) => {
  const [open, setOpen] = React.useState(false);
  const [text, setText] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [failed, setFailed] = React.useState(false);
  const w = (key: string) => wordingOf(wording, key);

  const notes = [...answers.question_notes, ...(answers.current?.notes ?? [])];

  /** Run one write, re-read, and hand the fresh answers up — or say it failed. */
  const run = (write: () => Promise<PracticeNote>, after: () => void) => {
    setBusy(true);
    setFailed(false);
    write()
      .then(() => fetchQuestionAnswers(questionId))
      .then((fresh) => {
        onChanged(fresh);
        after();
      })
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: a note write failed", cause);
        setFailed(true);
      })
      .finally(() => setBusy(false));
  };

  const onSave = () =>
    run(
      () =>
        answers.current !== null
          ? addAnswerNote(answers.current.answer_id, text)
          : addQuestionNote(questionId, text),
      () => {
        setText("");
        setOpen(false);
      },
    );

  return (
    <div style={{ marginTop: 16 }} data-answer-notes>
      <PracticeNoteList
        notes={notes}
        wording={wording}
        busy={busy}
        onStrike={(note) => run(() => strikeNote(note.id), () => undefined)}
      />
      {open ? (
        <>
          <textarea
            style={r.noteBox}
            value={text}
            aria-label={w("row_note_add_label")}
            onChange={(event) => setText(event.target.value)}
          />
          <div style={r.noteButtons}>
            <button
              type="button"
              style={s.buttonPrimary}
              disabled={busy || text.trim() === ""}
              onClick={onSave}
            >
              {w("row_note_save_label")}
            </button>
            <button type="button" style={s.button} disabled={busy} onClick={() => setOpen(false)}>
              {w("row_note_cancel_label")}
            </button>
          </div>
        </>
      ) : (
        <button type="button" style={s.buttonQuiet} data-practice-link onClick={() => setOpen(true)}>
          {w("row_note_add_label")}
        </button>
      )}
      {failed && (
        <div style={{ ...s.feedback, marginTop: 8 }} role="alert">
          {w("row_note_failed")}
        </div>
      )}
    </div>
  );
};

export default PracticeAnswerNotes;
