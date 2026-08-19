// =============================================================================
// PracticeSessionPage.tsx — one SITTING, at its own address (item B10)
// =============================================================================
//
// `…/practice/:scenarioId/session/:sessionId`. Three screens: the question, its
// reveal, and Chuck's sheet when the sitting ends.
//
// ## Why the sitting has an address (the measured defect)
//
// .401 held all four screens at the start card's address. Roman answered
// question 1, left the page, came back — and started at question 1 again with no
// sign his answer had been kept. It HAD been kept. An address is what lets the
// browser's Back button and the reload key mean something, and it is what makes
// "resume at the next undealt question" a thing this page can do on its own,
// from the server's own record, rather than from state a reload destroys.
//
// ## Where the position comes from
//
// Not from state, and not from a cursor on the server. The sitting carries the
// QUEUE it was dealt and the questions already answered; the next undealt one is
// the difference. A `skipped` row counts as dealt — she was shown that question
// and set it aside, and dealing it again is the one thing the control she
// pressed asked not to happen.
//
// ## Where the judging is NOT
//
// Nowhere in this file. The red/green sentence comes from the server or does not
// come at all: no local fallback, no heuristic, no score.

import React from "react";
import { useNavigate, useParams } from "react-router-dom";

import PracticeQuestionScreen from "../components/practice/PracticeQuestion";
import PracticeReveal from "../components/practice/PracticeReveal";
import PracticeSheetScreen from "../components/practice/PracticeSheet";
import * as f from "../components/practice/practiceFlowStyles";
import * as s from "../components/practice/practiceStyles";
import {
  closePracticeAnswer,
  endPracticeSession,
  fetchPracticeDeck,
  markHelpOpened,
  submitPracticeAnswer,
  wordingOf,
  type AnswerResult,
  type PracticeDeck,
  type PracticeQuestion,
  type PracticeSheet,
  type SelfCheck,
} from "../services/practice";
import { fetchSitting, skipPracticeQuestion, type Sitting } from "../services/practiceFlow";
import { practicePath } from "../utils/routePaths";
import { PracticeCrumb, PracticeLoadFailure, PracticeLoading } from "./practiceChrome";
import { requeue } from "./practiceQueue";

/** What the loading card says before the store has been read. See `PracticePage`. */
const LOADING = "Loading…";

/** All four boxes unticked — the state every question starts in. */
const NO_SELF_CHECK: SelfCheck = {
  only_asked: false,
  accepted_premise: false,
  explained_unasked: false,
  guessed: false,
};

/**
 * The sitting's queue, as deck questions, and where in it she is.
 *
 * ## Why a question the deck no longer holds is DROPPED and reported
 *
 * The queue is a list of ids stored when the sitting opened. Between then and
 * now the deck can change — that is the whole of Part B. An id nothing matches
 * cannot be rendered, and rendering a placeholder for it would be the screen
 * inventing a question. It is left out, and the resulting queue is shorter,
 * which the progress line then says truthfully.
 */
export function resumeAt(
  deck: PracticeQuestion[],
  sitting: Sitting,
): { queue: PracticeQuestion[]; index: number } {
  const queue = sitting.queue
    .map((id) => deck.find((q) => q.id === id))
    .filter((q): q is PracticeQuestion => q !== undefined);

  // Answered ids are consumed one at a time so a question she answered TWICE
  // (the repeat re-queues it) advances past both of its places, not just the
  // first. Counting occurrences rather than membership is the whole of it.
  const outstanding = [...sitting.answered];
  const index = queue.findIndex((question) => {
    const at = outstanding.indexOf(question.id);
    if (at === -1) return true;
    outstanding.splice(at, 1);
    return false;
  });
  return { queue, index: index === -1 ? queue.length : index };
}

const PracticeSessionPage: React.FC = () => {
  const { slug = "", scenarioId = "", sessionId = "" } = useParams<{
    slug: string;
    scenarioId: string;
    sessionId: string;
  }>();
  const navigate = useNavigate();

  const [deck, setDeck] = React.useState<PracticeDeck | null>(null);
  const [loadError, setLoadError] = React.useState<string | null>(null);
  const [queue, setQueue] = React.useState<PracticeQuestion[]>([]);
  const [index, setIndex] = React.useState(0);
  const [oneQuestion, setOneQuestion] = React.useState(false);

  const [answer, setAnswer] = React.useState("");
  const [pointsTo, setPointsTo] = React.useState<string[]>([]);
  const [submitting, setSubmitting] = React.useState(false);
  const [answerError, setAnswerError] = React.useState<string | null>(null);
  const [result, setResult] = React.useState<AnswerResult | null>(null);
  const [selfCheck, setSelfCheck] = React.useState<SelfCheck>(NO_SELF_CHECK);
  const [sheet, setSheet] = React.useState<PracticeSheet | null>(null);
  const [helpNotRecorded, setHelpNotRecorded] = React.useState(false);
  const [markError, setMarkError] = React.useState<string | null>(null);

  // Both reads on mount, together: the deck (every screen's words and every
  // question's text) and the sitting (which of them were dealt, and how far she
  // got). `Promise.all` rather than two effects so a half-loaded page — a queue
  // with no wording to render it — is not a state this component can be in.
  React.useEffect(() => {
    let cancelled = false;
    setLoadError(null);
    Promise.all([fetchPracticeDeck(slug, scenarioId), fetchSitting(sessionId)])
      .then(([payload, sitting]) => {
        if (cancelled) return;
        setDeck(payload);
        const { queue: dealt, index: at } = resumeAt(payload.questions, sitting);
        setQueue(dealt);
        setIndex(at);
        setOneQuestion(dealt.length === 1);
        // A sitting that is already over, or one whose stored queue is empty,
        // has nothing to resume. Back to the start card rather than an empty
        // question screen, which would read as a deck that failed to load.
        if (sitting.ended || dealt.length === 0 || at >= dealt.length) {
          navigate(practicePath(slug, scenarioId), { replace: true });
        }
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        // eslint-disable-next-line no-console
        console.error("practice: the sitting could not be opened", error);
        setLoadError(error instanceof Error ? error.message : String(error));
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId, sessionId, navigate]);

  const crumb = <PracticeCrumb slug={slug} scenarioId={scenarioId} deck={deck} />;

  if (loadError !== null) return <PracticeLoadFailure crumb={crumb} message={loadError} />;
  if (deck === null) return <PracticeLoading crumb={crumb} label={LOADING} />;

  const w = (key: string) => wordingOf(deck.wording, key);
  const current = queue[index] ?? null;
  const toStart = () => navigate(practicePath(slug, scenarioId));

  const submit = (text: string, dontRecall: boolean) => {
    if (current === null) return;
    setSubmitting(true);
    setAnswerError(null);
    // "(nothing typed)" is not composed here — an empty box records as the empty
    // string, and the sheet prints what she actually typed. Trimming to a
    // placeholder would put words in a witness's mouth on Chuck's sheet.
    submitPracticeAnswer({
      sessionId,
      questionId: current.id,
      answerText: text,
      dontRecall,
      // `[]` and "never opened it" are different facts and are sent
      // differently. She has opened the control iff she picked something —
      // this build has no third state to distinguish, and it says so by
      // sending null rather than an empty array she never saw.
      pointsTo: pointsTo.length > 0 ? pointsTo : null,
    })
      .then((answered) => {
        setResult(answered);
        setSelfCheck(NO_SELF_CHECK);
      })
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the answer was not recorded", error);
        setAnswerError(w("answer_failed"));
      })
      .finally(() => setSubmitting(false));
  };

  /**
   * Settle the answer she just read — her four boxes and her mark.
   *
   * AWAITED, because a `repeat` that failed to land would print "fine" on
   * Chuck's sheet against the one question she asked him to run the mock cross
   * on. A failure keeps her HERE, with everything standing and the notice above
   * the two buttons, rather than carrying her forward as if it had worked.
   */
  const settle = async (mark: "fine" | "repeat"): Promise<boolean> => {
    if (result === null) return true;
    setMarkError(null);
    try {
      await closePracticeAnswer(result.answer_id, mark, selfCheck);
      return true;
    } catch (error: unknown) {
      // eslint-disable-next-line no-console
      console.error("practice: the mark was not recorded", error);
      setMarkError(w("mark_not_recorded"));
      return false;
    }
  };

  /** Clear everything that belongs to the question she has just left. */
  const clearQuestion = () => {
    setResult(null);
    setHelpNotRecorded(false);
    setMarkError(null);
    setAnswer("");
    setPointsTo([]);
    setAnswerError(null);
  };

  const advance = async (nextQueue: PracticeQuestion[], mark: "fine" | "repeat") => {
    if (!(await settle(mark))) return;
    clearQuestion();
    // A ONE-question sitting ends on the start card, not on a sheet (task A2).
    // A Chuck's sheet holding a single row for every question she drilled alone
    // would bury the sitting he actually wants to read.
    if (oneQuestion) {
      await close(false);
      return;
    }
    setQueue(nextQueue);
    if (index + 1 >= nextQueue.length) {
      await close(true);
      return;
    }
    setIndex(index + 1);
  };

  /**
   * Close the sitting. `showSheet` decides whether she sees Chuck's sheet or the
   * start card — the same write either way, so the sitting is closed even when
   * she is on her way somewhere else.
   */
  const close = async (showSheet: boolean) => {
    try {
      const composed = await endPracticeSession(sessionId);
      if (showSheet) {
        setSheet(composed);
        return;
      }
    } catch (error: unknown) {
      // eslint-disable-next-line no-console
      console.error("practice: the session could not be closed", error);
      setLoadError(error instanceof Error ? error.message : String(error));
      return;
    }
    toStart();
  };

  /** ◂ Back to start. The sitting stays OPEN; the start card offers it back. */
  const back = async () => {
    // On the reveal the row already exists, so it is settled exactly as
    // "Got it — next question" would settle it: boxes as ticked, mark fine
    // unless she pressed "again later". Nothing is lost and nothing duplicated.
    if (result !== null && !(await settle("fine"))) return;
    toStart();
  };

  /** End session ▸ — settle if there is a row, close, and show the sheet. */
  const end = async () => {
    if (result !== null && !(await settle("fine"))) return;
    clearQuestion();
    await close(true);
  };

  /** Skip this one — doesn't fit. A row marked `skipped`, no read, no tokens. */
  const skip = () => {
    if (current === null) return;
    setSubmitting(true);
    setAnswerError(null);
    skipPracticeQuestion(sessionId, current.id)
      .then(async () => {
        clearQuestion();
        if (index + 1 >= queue.length) {
          await close(true);
          return;
        }
        setIndex(index + 1);
      })
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the skip was not recorded", error);
        setAnswerError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => setSubmitting(false));
  };

  /**
   * Record that she opened the drawer, and SAY SO if that write fails.
   *
   * Scoped to the drawer rather than raised over the page: what failed is one
   * cell on Chuck's sheet, and her answer, her boxes and her mark are all
   * unaffected.
   */
  const handleHelpOpened = () => {
    if (result === null) return;
    setHelpNotRecorded(false);
    markHelpOpened(result.answer_id).catch((error: unknown) => {
      // eslint-disable-next-line no-console
      console.error("practice: opening the stronger answer was not recorded", error);
      setHelpNotRecorded(true);
    });
  };

  return (
    <div style={s.page} data-surface="practice">
      {/* The print stylesheet. Inline styles cannot express a media query, so
          this one rule set is a real <style> element, scoped by data attribute. */}
      <style>{s.PRINT_CSS}</style>
      <style>{f.LINK_CSS}</style>
      <div data-practice-no-print>{crumb}</div>

      {sheet !== null ? (
        <PracticeSheetScreen
          sheet={sheet}
          wording={deck.wording}
          onPracticeAgain={toStart}
        />
      ) : current === null ? (
        <PracticeLoading crumb={null} label={LOADING} />
      ) : result === null ? (
        <PracticeQuestionScreen
          question={current}
          wording={deck.wording}
          position={index + 1}
          total={queue.length}
          answer={answer}
          onAnswerChange={setAnswer}
          onSubmit={() => submit(answer, false)}
          onDontRecall={() => {
            const text = w("dont_recall_text");
            setAnswer(text);
            submit(text, true);
          }}
          submitting={submitting}
          error={answerError}
          receipts={deck.receipts}
          pointsTo={pointsTo}
          onPointsToChange={setPointsTo}
          onBack={() => void back()}
          onSkip={skip}
          onEnd={() => void end()}
        />
      ) : (
        <PracticeReveal
          question={current}
          wording={deck.wording}
          position={index + 1}
          total={queue.length}
          answer={answer}
          readText={result.read_text}
          readOk={result.read_ok}
          points={deck.points}
          selfCheck={selfCheck}
          onSelfCheckChange={setSelfCheck}
          onHelpOpened={handleHelpOpened}
          helpNotRecorded={helpNotRecorded}
          markError={markError}
          pointsTo={pointsTo}
          // Fire-and-forget at the call site is correct: each of these owns its
          // own failure (it shows the notice and returns), so there is nothing
          // for the click handler to await or catch.
          onNext={() => void advance(queue, "fine")}
          onAgainLater={() =>
            void advance(oneQuestion ? queue : requeue(queue, current), "repeat")
          }
          onBack={() => void back()}
          onEnd={() => void end()}
          busy={submitting}
        />
      )}
    </div>
  );
};

export default PracticeSessionPage;
