// =============================================================================
// PracticePage.tsx — Marie's practice drill (PRACTICE v0, mockup v2)
// =============================================================================
//
// Four screens, one page, one payload: S0 start · S1 question · S2 reveal ·
// S3 Chuck's sheet. Everything the drill renders arrives from
// `GET /api/cases/:slug/scenarios/:id/practice` on mount; the only calls made
// mid-session are the writes.
//
// ## Why one page and not four routes
//
// A witness moving from a question to its reveal must never wait on a network,
// and must never see a screen fail between them. Four routes would mean four
// mounts, four chances to lose her typed answer to a re-render, and an address
// bar that changes under her while she is reading.
//
// ## Where the judging is NOT
//
// Nowhere in this file. The red/green sentence comes from the server or does not
// come at all. The mockup's `judge()` was a mockup; this build has no local
// fallback, no heuristic and no scoring — the four boxes are Marie's own and the
// one sentence is the model's.
//
// ## Standing Rule 1 on this page
//
// Three distinct failures, three distinct screens: the deck failed to load (the
// stored load-failure sentence), the deck is EMPTY (the stored "seed it"
// sentence — not a failure at all), and the answer write failed (the stored
// answer-failure sentence, on the question screen, saying nothing was saved).

import React from "react";
import { useParams } from "react-router-dom";

import Breadcrumb from "../components/Breadcrumb";
import PracticeQuestionScreen from "../components/practice/PracticeQuestion";
import PracticeReveal from "../components/practice/PracticeReveal";
import PracticeSheetScreen from "../components/practice/PracticeSheet";
import PracticeStart, { type PracticeWho } from "../components/practice/PracticeStart";
import * as s from "../components/practice/practiceStyles";
import {
  closePracticeAnswer,
  endPracticeSession,
  fetchPracticeDeck,
  markHelpOpened,
  startPracticeSession,
  submitPracticeAnswer,
  wordingOf,
  type AnswerResult,
  type PracticeDeck,
  type PracticeQuestion,
  type PracticeSheet,
  type SelfCheck,
} from "../services/practice";
import { scenarioPagePath, trialPrepPath } from "../utils/routePaths";
import { buildQueue, availableFor, requeue } from "./practiceQueue";

/** Which of the four screens is showing. */
type Screen = "start" | "question" | "reveal" | "sheet";

/** All four boxes unticked — the state every question starts in. */
const NO_SELF_CHECK: SelfCheck = {
  only_asked: false,
  accepted_premise: false,
  explained_unasked: false,
  guessed: false,
};

const PracticePage: React.FC = () => {
  const { slug = "", scenarioId = "" } = useParams<{ slug: string; scenarioId: string }>();

  const [deck, setDeck] = React.useState<PracticeDeck | null>(null);
  const [loadError, setLoadError] = React.useState<string | null>(null);
  const [screen, setScreen] = React.useState<Screen>("start");

  const [who, setWho] = React.useState<PracticeWho>("george");
  const [sessionId, setSessionId] = React.useState<string | null>(null);
  const [starting, setStarting] = React.useState(false);

  const [queue, setQueue] = React.useState<PracticeQuestion[]>([]);
  const [index, setIndex] = React.useState(0);
  const [answer, setAnswer] = React.useState("");
  const [submitting, setSubmitting] = React.useState(false);
  const [answerError, setAnswerError] = React.useState<string | null>(null);
  const [result, setResult] = React.useState<AnswerResult | null>(null);
  const [selfCheck, setSelfCheck] = React.useState<SelfCheck>(NO_SELF_CHECK);
  const [sheet, setSheet] = React.useState<PracticeSheet | null>(null);
  const [helpNotRecorded, setHelpNotRecorded] = React.useState(false);
  const [markError, setMarkError] = React.useState<string | null>(null);

  // One fetch on mount. Every failure has an explicit `.catch` and an explicit
  // screen; nothing here can reject silently.
  React.useEffect(() => {
    let cancelled = false;
    setLoadError(null);
    fetchPracticeDeck(slug, scenarioId)
      .then((payload) => {
        if (!cancelled) setDeck(payload);
      })
      .catch((error: unknown) => {
        if (cancelled) return;
        // eslint-disable-next-line no-console
        console.error("practice: the deck could not be loaded", error);
        setLoadError(error instanceof Error ? error.message : String(error));
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId]);

  const crumb = (
    <Breadcrumb
      items={[
        { label: "Dashboard", to: "/" },
        { label: "Trial Prep", to: trialPrepPath(slug) },
        ...(deck === null
          ? []
          : [{ label: `${deck.code} · ${deck.title}`, to: scenarioPagePath(slug, scenarioId) }]),
        { label: "Practice" },
      ]}
    />
  );

  if (loadError !== null) {
    // The stored sentence is unavailable when the payload itself failed, so this
    // one notice is composed here — and it names the underlying failure rather
    // than replacing it with a friendlier lie.
    return (
      <div style={s.page} data-surface="practice">
        {crumb}
        <section style={s.card} role="alert">
          <div style={s.feedback}>{loadError}</div>
        </section>
      </div>
    );
  }

  if (deck === null) {
    return (
      <div style={s.page} data-surface="practice">
        {crumb}
        <section style={s.card}>
          <span style={s.progress}>Loading…</span>
        </section>
      </div>
    );
  }

  const w = (key: string) => wordingOf(deck.wording, key);

  // An empty deck is NOT a failure. S-6 is in exactly this state until somebody
  // runs the seed, and the page says so in the store's own words.
  if (deck.questions.length === 0) {
    return (
      <div style={s.page} data-surface="practice">
        {crumb}
        <section style={s.card}>
          <div style={s.kicker}>{w("kicker")}</div>
          <h1 style={s.h1}>
            {deck.code} · {deck.title}
          </h1>
          <p style={s.sub}>{w("empty_deck")}</p>
        </section>
      </div>
    );
  }

  const current = queue[index] ?? null;

  const handleStart = () => {
    setStarting(true);
    startPracticeSession(slug, scenarioId, who)
      .then((id) => {
        setSessionId(id);
        setQueue(buildQueue(deck.questions, who));
        setIndex(0);
        setAnswer("");
        setAnswerError(null);
        setScreen("question");
      })
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the session could not be started", error);
        setLoadError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => setStarting(false));
  };

  const submit = (text: string, dontRecall: boolean) => {
    if (current === null || sessionId === null) return;
    setSubmitting(true);
    setAnswerError(null);
    // "(nothing typed)" is not composed here — an empty box records as the empty
    // string, and the sheet prints what she actually typed. Trimming to a
    // placeholder would put words in a witness's mouth on Chuck's sheet.
    submitPracticeAnswer({ sessionId, questionId: current.id, answerText: text, dontRecall })
      .then((answered) => {
        setResult(answered);
        setSelfCheck(NO_SELF_CHECK);
        setScreen("reveal");
      })
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the answer was not recorded", error);
        setAnswerError(w("answer_failed"));
      })
      .finally(() => setSubmitting(false));
  };

  /**
   * Settle the answer she just read, then move on.
   *
   * The close write is what puts her four boxes and her mark on the row Chuck's
   * sheet renders, so it is AWAITED: a `repeat` that failed to land would print
   * "fine" on the sheet against the one question she asked him to run the mock
   * cross on.
   *
   * ## Why a failure keeps her HERE
   *
   * The queue, her answer and the read all live in this component's state. Ending
   * the session — or replacing the page with a failure card — over a network blip
   * would cost her the whole sitting. So a failed close leaves everything
   * standing, says so above the two buttons, and lets her press again.
   */
  const advance = async (nextQueue: PracticeQuestion[], mark: "fine" | "repeat") => {
    if (result !== null) {
      setMarkError(null);
      try {
        await closePracticeAnswer(result.answer_id, mark, selfCheck);
      } catch (error: unknown) {
        // eslint-disable-next-line no-console
        console.error("practice: the mark was not recorded", error);
        setMarkError(w("mark_not_recorded"));
        return;
      }
    }
    setQueue(nextQueue);
    setResult(null);
    setHelpNotRecorded(false);
    setMarkError(null);
    setAnswer("");
    setAnswerError(null);
    if (index + 1 >= nextQueue.length) {
      finish();
      return;
    }
    setIndex(index + 1);
    setScreen("question");
  };

  const finish = () => {
    if (sessionId === null) return;
    endPracticeSession(sessionId)
      .then((composed) => {
        setSheet(composed);
        setScreen("sheet");
      })
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the session could not be closed", error);
        setLoadError(error instanceof Error ? error.message : String(error));
      });
  };

  /**
   * Record that she opened the drawer, and SAY SO if that write fails.
   *
   * The notice is scoped to the drawer rather than raised over the page: what
   * failed is one cell on Chuck's sheet, and her answer, her boxes and her mark
   * are all unaffected. Saying nothing would leave the sheet quietly wrong on
   * the one column Chuck reads to decide where to spend his mock cross.
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
    // `data-surface="practice"` is what resolves every `var(--practice-…)` in
    // `practiceStyles`. Without it the whole palette falls back to nothing and
    // the screen renders unstyled — which is why `practiceStyles.test.ts` pins
    // that this attribute and that CSS block stay in step.
    <div style={s.page} data-surface="practice">
      {/* The print stylesheet. Inline styles cannot express a media query, so
          this one rule set is a real <style> element, scoped by data attribute. */}
      <style>{s.PRINT_CSS}</style>
      <div data-practice-no-print>{crumb}</div>

      {screen === "start" && (
        <PracticeStart
          code={deck.code}
          title={deck.title}
          wording={deck.wording}
          lastSessionLine={deck.last_session_line}
          who={who}
          onWhoChange={setWho}
          onStart={handleStart}
          starting={starting}
          available={availableFor(deck.questions, who)}
          deckSize={deck.questions.length}
        />
      )}

      {screen === "question" && current !== null && (
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
        />
      )}

      {screen === "reveal" && current !== null && result !== null && (
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
          // Fire-and-forget at the call site is correct here: `advance` owns its
          // own failure (it shows the notice and returns), so there is nothing
          // for the click handler to await or catch.
          onNext={() => void advance(queue, "fine")}
          onAgainLater={() => void advance(requeue(queue, current), "repeat")}
        />
      )}

      {screen === "sheet" && sheet !== null && (
        <PracticeSheetScreen
          sheet={sheet}
          wording={deck.wording}
          onPracticeAgain={() => {
            setScreen("start");
            setSessionId(null);
            setSheet(null);
            setQueue([]);
            setIndex(0);
          }}
        />
      )}
    </div>
  );
};

export default PracticePage;
