// =============================================================================
// PracticeQuestionReviewPage.tsx — one question, every attempt (task B3)
// =============================================================================
//
// `…/practice/:scenarioId/question/:questionId`. The question as she was asked
// it, then the attempts newest first — each with its read, her four boxes, what
// she said she would point to, and the notes written on it — then the study
// material, then Practice this one again ▸.
//
// ## Read-only, by Roman's ruling
//
// Nothing on this page edits an answer. An answer is a MOMENT: what she said at
// 12:01 on Tuesday is a fact about Tuesday, and letting her improve it would
// make Chuck's note on that attempt point at words she never wrote. She answers
// again instead, and the attempts stack.
//
// ## What "attempt 1" means, and where that is decided
//
// On the server. The attempts are numbered in the order they HAPPENED and then
// reversed for a page that says "newest first" out loud, so attempt 1 is always
// her first however the list is sorted. This page renders the order it is given.

import React from "react";
import { useNavigate, useParams } from "react-router-dom";

import { NoteAdd, NoteRow } from "../components/practice/PracticeNotes";
import PracticeNotes from "../components/practice/PracticeNotes";
import { QuestionPills, SourceLine } from "../components/practice/PracticeQuestion";
import * as e from "../components/practice/practiceEditorStyles";
import * as f from "../components/practice/practiceFlowStyles";
import * as s from "../components/practice/practiceStyles";
import { RECEIPT_JOIN } from "../components/practice/PracticePointsTo";
import {
  startPracticeSession,
  wordingOf,
  type PracticeNote,
  type PracticeWording,
} from "../services/practice";
import {
  fetchQuestionReview,
  saveNote,
  strikeNote,
  type PracticeAttempt,
  type PracticeReview,
} from "../services/practiceEditor";
import { practicePath, practiceSessionPath } from "../utils/routePaths";
import { PracticeCrumb, PracticeLoadFailure, PracticeLoading } from "./practiceChrome";

/** See `PracticePage` for why this one sentence is not a stored row. */
const LOADING = "Loading…";

/** One attempt's card: the heading, her words, the read, the detail, the notes. */
const Attempt: React.FC<{
  attempt: PracticeAttempt;
  wording: PracticeWording;
  onSaveNote: (text: string) => void;
  onStrikeNote: (note: PracticeNote) => void;
  saving: boolean;
}> = ({ attempt, wording, onSaveNote, onStrikeNote, saving }) => {
  const w = (key: string) => wordingOf(wording, key);
  // The same three states the reveal uses, and the same neutral rail for the
  // third: green would congratulate her for a call that never happened, red
  // would accuse her of something nobody judged.
  const rail =
    attempt.read_ok === null
      ? { ...s.feedback, borderLeftColor: s.LINE, background: s.QUIET_BG }
      : attempt.read_ok
        ? s.feedbackOk
        : s.feedback;

  return (
    <div style={e.attempt}>
      <div style={e.attemptHead}>
        <span style={e.attemptNumber}>{attempt.heading}</span>
        <span style={e.statusColour[attempt.mark_key] ?? undefined}>{attempt.mark}</span>
      </div>

      <div style={s.yours}>{attempt.answer}</div>

      <div style={{ ...rail, marginTop: 6 }}>
        {attempt.read_text ?? w("read_unavailable")}
        <small style={s.feedbackNote}>
          <span style={s.tag}>{w("read_tag")}</span>
        </small>
      </div>

      {/* What she said she would point to, then help and boxes. The pointed-to
          clause is withdrawn entirely when she named nothing: a prefix with an
          empty list after it reads as data that went missing. */}
      <div style={{ ...s.sub, fontSize: 14, marginTop: 6 }}>
        {attempt.points_to.length > 0 && (
          <>
            {w("points_to_reveal_prefix")} {attempt.points_to.join(RECEIPT_JOIN)}
            {" · "}
          </>
        )}
        {attempt.detail}
      </div>

      {/* What she was ACTUALLY asked, when the question has been re-worded
          since. Absent — not empty — when the wording has not changed, so this
          renders nothing rather than an empty line. The header above shows the
          question as it reads today, which is the one she will be asked next
          time; this is the one her answer answers. */}
      {attempt.asked_as !== undefined && attempt.asked_as !== null && (
        <div style={{ ...s.sub, fontSize: 13, marginTop: 4, fontStyle: "italic" }}>
          {attempt.asked_as}
        </div>
      )}

      {attempt.notes.map((note) => (
        <NoteRow
          key={note.id}
          wording={wording}
          note={note}
          onStrike={onStrikeNote}
          striking={saving}
        />
      ))}

      <NoteAdd
        wording={wording}
        placeholderKey="notes_attempt_placeholder"
        onSave={onSaveNote}
        saving={saving}
      />
    </div>
  );
};

const PracticeQuestionReviewPage: React.FC = () => {
  const { slug = "", scenarioId = "", questionId = "" } = useParams<{
    slug: string;
    scenarioId: string;
    questionId: string;
  }>();
  const navigate = useNavigate();

  const [review, setReview] = React.useState<PracticeReview | null>(null);
  const [loadError, setLoadError] = React.useState<string | null>(null);
  const [saving, setSaving] = React.useState(false);
  const [noteError, setNoteError] = React.useState<string | null>(null);
  const [starting, setStarting] = React.useState(false);
  const [reloads, setReloads] = React.useState(0);

  React.useEffect(() => {
    let cancelled = false;
    setLoadError(null);
    fetchQuestionReview(slug, scenarioId, questionId)
      .then((payload) => {
        if (!cancelled) setReview(payload);
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        // eslint-disable-next-line no-console
        console.error("practice: the review could not be loaded", error);
        setLoadError(error instanceof Error ? error.message : String(error));
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId, questionId, reloads]);

  const crumb = <PracticeCrumb slug={slug} scenarioId={scenarioId} deck={null} />;

  if (loadError !== null) return <PracticeLoadFailure crumb={crumb} message={loadError} />;
  if (review === null) return <PracticeLoading crumb={crumb} label={LOADING} />;

  const w = (key: string) => wordingOf(review.wording, key);
  const question = review.question;

  /** Write one note, anywhere on this page, and re-read so the panel follows. */
  const writeNote = (answerId: string | null) => (text: string) => {
    setSaving(true);
    setNoteError(null);
    saveNote(slug, scenarioId, { questionId, answerId }, text)
      .then(() => setReloads((n) => n + 1))
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the note was not saved", error);
        setNoteError(w("notes_failed"));
      })
      .finally(() => setSaving(false));
  };

  const strike = (note: PracticeNote) => {
    setSaving(true);
    setNoteError(null);
    strikeNote(note.id)
      .then(() => setReloads((n) => n + 1))
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the note was not struck", error);
        setNoteError(w("notes_failed"));
      })
      .finally(() => setSaving(false));
  };

  /** Practice this one again ▸ — a one-question sitting, as the row's link is. */
  const practiceAgain = () => {
    setStarting(true);
    startPracticeSession(slug, scenarioId, {
      who: question.side === "george" ? "george" : "chuck",
      queue: [question.id],
      count: 1,
      skippedToday: [],
    })
      .then((id) => navigate(practiceSessionPath(slug, scenarioId, id)))
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the session could not be started", error);
        setLoadError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => setStarting(false));
  };

  return (
    <div style={s.page} data-surface="practice">
      <style>{f.LINK_CSS}</style>
      {crumb}
      <section style={s.card}>
        <div style={f.topBar}>
          <button
            type="button"
            style={f.topBarLink}
            data-practice-link
            onClick={() => navigate(practicePath(slug, scenarioId))}
          >
            {w("back_label")}
          </button>
          <button
            type="button"
            style={f.topBarLink}
            data-practice-link
            disabled={starting}
            onClick={practiceAgain}
          >
            {w("review_practice_again")}
          </button>
        </div>

        <div style={{ ...s.row, justifyContent: "space-between", marginTop: 0 }}>
          <span style={s.progress}>{review.progress}</span>
          <QuestionPills question={question} wording={review.wording} />
        </div>
        <div style={{ ...s.question, fontSize: 22 }}>{question.text}</div>
        <SourceLine question={question} />

        {/* Roman's amendment 2: the question's own notes, ABOVE the attempts,
            collapsed like the scenario panel. */}
        <PracticeNotes
          wording={review.wording}
          notes={review.notes}
          titleKey="notes_question_title"
          onSave={writeNote(null)}
          onStrike={strike}
          saving={saving}
          error={noteError}
        />

        <div style={{ ...s.kicker, marginTop: 18 }}>{w("review_attempts_kicker")}</div>
        {/* A question reached by a typed address with nothing behind it. No row
            offers the link, so this is rare — and it is a named absence rather
            than an empty stretch of page, with the study material still below. */}
        {review.attempts.length === 0 && <p style={s.sub}>{w("review_no_attempts")}</p>}
        {review.attempts.map((attempt) => (
          <Attempt
            key={attempt.answer_id}
            attempt={attempt}
            wording={review.wording}
            onSaveNote={writeNote(attempt.answer_id)}
            onStrikeNote={strike}
            saving={saving}
          />
        ))}

        <div style={{ ...s.kicker, marginTop: 22 }}>{w("points_kicker")}</div>
        <ol>
          {review.points.map((point) => (
            <li key={point.position} style={s.pointItem}>
              {point.text}
              <div style={s.receipt}>
                {point.exhibit === null
                  ? w("point_no_receipt")
                  : `${w("receipt_prefix")} ${point.exhibit}`}
              </div>
            </li>
          ))}
        </ol>

        {question.pair_said !== null && question.pair_admitted !== null && (
          <>
            <div style={{ ...s.kicker, marginTop: 18 }}>{w("pair_kicker")}</div>
            <div style={s.pair}>
              <div style={s.pairCell}>
                <div style={s.pairLabel}>{w("pair_said_label")}</div>
                {question.pair_said}
              </div>
              <div style={s.pairCell}>
                <div style={s.pairLabel}>{w("pair_admitted_label")}</div>
                {question.pair_admitted}
              </div>
            </div>
          </>
        )}

        {question.watch_for !== null && <div style={s.watch}>{question.watch_for}</div>}

        {/* OPEN on this page, unlike the drill's — she is studying, not being
            drilled, and there is nothing to withhold. The heading says so: "A
            stronger answer" rather than "Show a stronger answer". */}
        <details style={s.stronger} open>
          <summary style={s.strongerSummary}>{w("review_stronger_heading")}</summary>
          <div style={s.strongerExample}>
            {question.stronger ??
              (question.kind === "redirect"
                ? w("redirect_stronger_line")
                : w("stronger_no_receipt"))}
          </div>
          {question.stronger !== null && question.stronger_lean !== null && (
            <div style={s.strongerLean}>{question.stronger_lean}</div>
          )}
          <div style={s.strongerNote}>
            {w("stronger_note_prefix")} <i>{w("stronger_note_emphasis")}</i>
            {w("stronger_note_suffix")}
          </div>
        </details>
      </section>
    </div>
  );
};

export default PracticeQuestionReviewPage;
