// =============================================================================
// PracticeReviewCard.tsx — one question on the Review answers page
// =============================================================================
//
// CC_TASK_REVIEW_PAGE_v1 §1, from the ruled mockup: the two pills, the question,
// Marie's CURRENT answer with the line saying when and who, the notes standing
// on it, and a box to add one.
//
// ## Domain note: WHERE A NOTE LANDS, which is the whole card
//
// With an answer, the note lands on the ANSWER THAT STANDS NOW — that is what
// Chuck is reading, and a note there counts toward Marie's pill until she
// answers again. With no answer it lands on the QUESTION, and she sees it
// before she writes. Two different writes to two different rows, and the only
// thing on screen that distinguishes them is the wording of the box and of the
// block above it. Both are stored rows; neither is composed here.
//
// ## Why this component writes nothing itself
//
// It hands the text up. The page owns the re-read (`PracticeAnswerNotes`'s
// pattern: write, then fetch, never patch a local array), because after any
// write BOTH of the page's reads are stale — the deck carries the notes and the
// answers payload carries the answer. A component that re-read only its own
// half would leave the other half wrong.

import React from "react";

import { wordingOf, type PracticeQuestion, type PracticeWording } from "../../services/practice";
import type { PracticeAnswer } from "../../services/practiceAnswers";
import PracticeNoteList from "./PracticeNoteList";
import * as p from "./practiceReviewPageStyles";
import * as s from "./practiceStyles";

/**
 * The pill on a card: George, Chuck, or the braid's third colour.
 *
 * The SAME three pills the deck row draws (`PracticeDeckRow`'s `sidePill`),
 * deliberately: a question that is green on one page and a different green on
 * the next reads as a different question. The mockup's own greens differ
 * slightly; consistency inside the product wins, and the deviation is on the
 * record in the report's table.
 */
export function sidePill(question: PracticeQuestion, wording: PracticeWording) {
  const w = (key: string) => wordingOf(wording, key);
  if (question.braid) return { style: s.pillBraid, label: w("pill_braid") };
  if (question.side === "george") return { style: s.pillGeorge, label: w("pill_george") };
  return { style: s.pillChuck, label: w("pill_chuck") };
}

interface Props {
  question: PracticeQuestion;
  /** The current answer, or `null` when she has not written one. */
  answer: PracticeAnswer | null;
  wording: PracticeWording;
  /** Write a note on this card. The page decides the target from `answer`. */
  onSaveNote: (text: string) => void;
  /** Withdraw a standing note. */
  onStrikeNote: (noteId: string) => void;
  /** True while any write on this card is in flight — its controls wait. */
  busy: boolean;
}

const PracticeReviewCard: React.FC<Props> = ({
  question,
  answer,
  wording,
  onSaveNote,
  onStrikeNote,
  busy,
}) => {
  const [text, setText] = React.useState("");
  const w = (key: string) => wordingOf(wording, key);
  const pill = sidePill(question, wording);

  // The placeholder names the thing the note will land on. Two rows, because
  // the two boxes write to two different places and this is the only warning.
  const placeholder = w(
    answer === null ? "review_note_placeholder_question" : "review_note_placeholder_answer",
  );

  const save = () => {
    const trimmed = text.trim();
    if (trimmed === "") return;
    onSaveNote(trimmed);
    // Cleared optimistically because the PAGE owns the outcome: on failure it
    // raises the stored failure sentence, and a box that kept its text would
    // invite a second press writing the note twice.
    setText("");
  };

  return (
    <article style={p.card} data-review-card data-question-id={question.id}>
      <div style={p.pills}>
        <span style={pill.style}>{pill.label}</span>
        {/* Withheld rather than emptied: a question carrying no tactic shows no
            tag, exactly as it does on the deck row. */}
        {question.tactic !== null && <span style={s.tacticTag}>{question.tactic}</span>}
      </div>

      <div style={p.question}>{question.text}</div>

      {answer === null ? (
        <div style={p.unanswered} data-review-unanswered>
          {w("review_unanswered")}
        </div>
      ) : (
        <div style={p.answer} data-review-answer>
          {answer.text}
          <div style={p.answerMeta}>{answer.answered_meta}</div>
        </div>
      )}

      {/* Struck notes stay, struck through — Roman's ruling of 2026-09-19 and
          the standing law of this table. The amber ground is the mockup's. */}
      <PracticeNoteList
        notes={question.notes}
        wording={wording}
        tone="amber"
        busy={busy}
        onStrike={(note) => onStrikeNote(note.id)}
      />

      <div style={p.addNote}>
        <input
          style={p.noteInput}
          value={text}
          placeholder={placeholder}
          aria-label={placeholder}
          disabled={busy}
          onChange={(event) => setText(event.target.value)}
          // Enter saves: Chuck is typing forty-two short notes in one pass, and
          // reaching for the mouse between each is the difference between a
          // page he uses and one he abandons.
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              save();
            }
          }}
        />
        <button
          type="button"
          style={p.buttonGhost}
          disabled={busy || text.trim() === ""}
          onClick={save}
        >
          {w("row_note_save_label")}
        </button>
      </div>
    </article>
  );
};

export default PracticeReviewCard;
