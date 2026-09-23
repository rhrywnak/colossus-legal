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
import {
  addAnswerNote,
  addQuestionNote,
  replyToNote,
  strikeNote,
} from "../../services/practiceReviewLoop";
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
  // Which note the box is answering, or `null` for a note of its own. ONE box
  // serves both: a reply is a note, and two boxes would be two places for the
  // same words to be typed and lost (CC_TASK_FOR_YOU_v1 L3).
  const [replyTo, setReplyTo] = React.useState<PracticeNote | null>(null);
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

  /** Start a reply to one note: the same box, addressed. */
  const beginReply = (note: PracticeNote) => {
    setReplyTo(note);
    setText("");
    setOpen(true);
    setFailed(false);
  };

  /** Close the box, whichever kind it was, without writing. */
  const close = () => {
    setOpen(false);
    setReplyTo(null);
  };

  const onSave = () =>
    run(
      () => {
        // A reply names the note it answers and NOTHING else: the question, the
        // attempt and the scenario are read from that note by the server, so a
        // reply cannot land somewhere its parent is not.
        if (replyTo !== null) {
          return replyToNote(replyTo.id, text);
        }
        return answers.current !== null
          ? addAnswerNote(answers.current.answer_id, text)
          : addQuestionNote(questionId, text);
      },
      () => {
        setText("");
        close();
      },
    );

  return (
    <div style={{ marginTop: 16 }} data-answer-notes>
      <PracticeNoteList
        notes={notes}
        wording={wording}
        busy={busy}
        onStrike={(note) => run(() => strikeNote(note.id), () => undefined)}
        onReply={beginReply}
      />
      {open ? (
        <>
          {/* The box says WHO it is answering when it is a reply — otherwise
              the two kinds of write look identical and the wrong one ships. */}
          {replyTo !== null && (
            <div style={r.noteMeta} data-reply-to>
              {w("row_note_reply_label")} · {replyTo.author} · {replyTo.when}
            </div>
          )}
          <textarea
            style={r.noteBox}
            value={text}
            aria-label={replyTo === null ? w("row_note_add_label") : w("row_note_reply_label")}
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
            <button type="button" style={s.button} disabled={busy} onClick={close}>
              {w("row_note_cancel_label")}
            </button>
          </div>
        </>
      ) : (
        <button
          type="button"
          style={s.buttonQuiet}
          data-practice-link
          onClick={() => {
            setReplyTo(null);
            setOpen(true);
          }}
        >
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
