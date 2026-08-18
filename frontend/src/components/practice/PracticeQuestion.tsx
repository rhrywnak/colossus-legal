// =============================================================================
// PracticeQuestion.tsx — screen S1 of PRACTICE_MOCKUP_v2
// =============================================================================
//
// One question, large, in serif; its pill and tactic tag; the source line under
// it; the box she types into; Answer, "I don't recall." and Pause.
//
// ## The pause is a control, not a decoration
//
// It is the fifth rule on the ALWAYS card and the hardest one to believe on the
// stand. Pressing it shows a sentence and nothing else happens — no timer starts,
// nothing is recorded. That is the point.

import React from "react";

import type { PracticeQuestion as Question, PracticeWording } from "../../services/practice";
import { wordingOf } from "../../services/practice";
import * as s from "./practiceStyles";

interface Props {
  question: Question;
  wording: PracticeWording;
  /** 1-based, and the queue's length — both already counted by the page. */
  position: number;
  total: number;
  answer: string;
  onAnswerChange: (text: string) => void;
  onSubmit: () => void;
  onDontRecall: () => void;
  /** True while the answer POST is in flight. */
  submitting: boolean;
  /** The stored failure sentence, or null. Rendered beneath the controls. */
  error: string | null;
}

/**
 * The pill and the tactic tag, shared by S1 and S2.
 *
 * One component because the mockup renders the identical pair on both screens,
 * and two copies is where the braid's purple pill eventually appears on only one
 * of them.
 */
export const QuestionPills: React.FC<{ question: Question; wording: PracticeWording }> = ({
  question,
  wording,
}) => {
  const pillStyle = question.braid
    ? s.pillBraid
    : question.side === "george"
      ? s.pillGeorge
      : s.pillChuck;
  const pillText = question.braid
    ? wordingOf(wording, "pill_braid")
    : question.side === "george"
      ? wordingOf(wording, "pill_george")
      : wordingOf(wording, "pill_chuck");

  return (
    <span>
      <span style={pillStyle}>{pillText}</span>
      {/* No tag at all when the question carries no tactic — a Chuck question
          has no trap in it, and an empty grey box would imply one. */}
      {question.tactic !== null && <span style={s.tacticTag}>{question.tactic}</span>}
    </span>
  );
};

/**
 * The "Built from: …" line, with a braid's rows in bold after it.
 *
 * Renders NOTHING when there is no receipt. That is the honest-gap law: a
 * question with no traceable source says nothing rather than showing an empty
 * line that reads like a source that failed to load.
 */
export const SourceLine: React.FC<{ question: Question }> = ({ question }) => {
  if (question.receipt === null) return null;
  return (
    <div style={s.from}>
      {question.receipt}
      {question.braid_rows !== null && (
        <>
          {" · "}
          <b style={{ color: s.INK, fontWeight: 600 }}>{question.braid_rows}</b>
        </>
      )}
    </div>
  );
};

const PracticeQuestion: React.FC<Props> = ({
  question,
  wording,
  position,
  total,
  answer,
  onAnswerChange,
  onSubmit,
  onDontRecall,
  submitting,
  error,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  const [paused, setPaused] = React.useState(false);

  // The pause note belongs to THIS question: moving on must clear it, or the
  // sentence "the pause is yours" follows her to a question she did not pause on.
  React.useEffect(() => setPaused(false), [question.id]);

  return (
    <section style={s.card}>
      <div style={{ ...s.row, justifyContent: "space-between", marginTop: 0 }}>
        <span style={s.progress}>
          {w("progress_template")
            .replace("{n}", String(position))
            .replace("{total}", String(total))}
        </span>
        <QuestionPills question={question} wording={wording} />
      </div>

      <div style={s.question}>{question.text}</div>
      <SourceLine question={question} />

      <p style={{ marginTop: 22, marginBottom: 6 }}>
        <b>{w("answer_label")}</b> {w("answer_hint")}
      </p>
      <textarea
        style={s.textarea}
        value={answer}
        placeholder={w("answer_placeholder")}
        onChange={(e) => onAnswerChange(e.target.value)}
        aria-label={w("answer_label")}
      />

      <div style={s.row}>
        <button type="button" style={s.buttonPrimary} onClick={onSubmit} disabled={submitting}>
          {w("answer_button")}
        </button>
        <button type="button" style={s.button} onClick={onDontRecall} disabled={submitting}>
          {w("dont_recall_button")}
        </button>
        <button type="button" style={s.buttonQuiet} onClick={() => setPaused(true)}>
          {w("pause_button")}
        </button>
      </div>

      {paused && (
        <div style={{ ...s.sub, marginTop: 10 }}>
          {w("pause_note_prefix")} <i>{w("pause_note_emphasis")}</i>
        </div>
      )}

      {/* The write failed and NOTHING was saved. Said in those words, because a
          witness who believes an answer was logged when it was not is the worst
          outcome this screen has. */}
      {error !== null && (
        <div style={{ ...s.feedback, marginTop: 14 }} role="alert">
          {error}
        </div>
      )}
    </section>
  );
};

export default PracticeQuestion;
