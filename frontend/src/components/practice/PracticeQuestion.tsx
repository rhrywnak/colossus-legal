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
import * as f from "./practiceFlowStyles";
import * as e from "./practiceEditorStyles";
import * as s from "./practiceStyles";
import PracticePointsTo from "./PracticePointsTo";
import PracticeTopBar from "./PracticeTopBar";

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
  /** This scenario's receipts, for the "I'd point to…" picker. */
  receipts: string[];
  /** What she has picked, and the setter. `[]` until she opens the control. */
  pointsTo: string[];
  onPointsToChange: (picked: string[]) => void;
  /** The three exits at the top of the screen (mockup v3, item B6). */
  onBack: () => void;
  onSkip: () => void;
  onEnd: () => void;
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
      {/* A redirect wears CHUCK'S pill — he is the one asking — plus a tag of
          its own saying why. Two facts, and neither can carry the other: the
          pill answers "who is speaking", the tag answers "what is this for". */}
      {question.kind === "redirect" && (
        <span style={f.redirectTag}>{wordingOf(wording, "redirect_tag")}</span>
      )}
      {/* No tactic tag at all when the question carries none — a Chuck question
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
  receipts,
  pointsTo,
  onPointsToChange,
  onBack,
  onSkip,
  onEnd,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  const [paused, setPaused] = React.useState(false);
  // Whitespace is not an answer. `trim()` and not `=== ""`, because a stray
  // newline in the box would otherwise send a row the sheet prints as blank.
  const blank = answer.trim() === "";

  // The pause note belongs to THIS question: moving on must clear it, or the
  // sentence "the pause is yours" follows her to a question she did not pause on.
  React.useEffect(() => setPaused(false), [question.id]);

  return (
    <section style={s.card}>
      <PracticeTopBar
        wording={wording}
        screen="question"
        onBack={onBack}
        onSkip={onSkip}
        onEnd={onEnd}
        busy={submitting}
      />
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

      <PracticePointsTo
        wording={wording}
        receipts={receipts}
        picked={pointsTo}
        onChange={onPointsToChange}
        disabled={submitting}
      />

      <div style={s.row}>
        {/* Answer refuses an EMPTY box before the click rather than after it.
            The server refuses one too (`api::practice_fences`), so this is the
            polite half of a fence and not the whole of it — but a witness who
            presses a live button and gets a red sentence back has been told off
            for nothing.

            The hint names the OTHER control on purpose: "I don't recall." is a
            complete answer and stays ONE click, so a disabled Answer must not
            read as "you have to type something to go on". */}
        <button
          type="button"
          style={{ ...s.buttonPrimary, ...(blank ? e.lockedControl : {}) }}
          onClick={onSubmit}
          disabled={submitting || blank}
          title={blank ? w("answer_empty_hint") : undefined}
        >
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
