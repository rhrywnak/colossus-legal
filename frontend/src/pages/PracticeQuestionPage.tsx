// PracticeQuestionPage.tsx — one question. Marie writes on it; Chuck reads it.
//
// Mockup v7 views 2, 3 and 4 are three states of THIS page, not three pages.
//
// ## The working state exists before the critique does
//
// Pressing Answer relabels and disables the button, locks the box, and renders
// the critique block AT ONCE — empty, shimmering. Roman's defect #1 of
// 2026-08-20 was that nothing appeared until the read returned, so the page
// looked inert while it worked and she pressed again. Everything about that
// state is set before the request is awaited, in `onAnswer` below.
//
// ## Stop waiting abandons the READ, never the answer
//
// Her words are the FIRST write and the read is the second. By the time the
// working state is on screen the row is already on disk, so abandoning costs
// one critique and nothing else. The ten-second line says exactly that.
//
// ## Earlier versions are not editable, and she never has to open them
//
// One quiet line. Chuck's reading of an older version still points at the words
// he read, which is the whole reason they are kept — and the reason nothing on
// this page can edit them.

import React from "react";
import { useParams } from "react-router-dom";

import Critique from "../components/practice/PracticeCritiqueBlock";
import { critiqueFor } from "../components/practice/practiceCritique";
import { answerChrome, LONG_WAIT_MS } from "../components/practice/practiceAnswerPhase";
import * as c from "../components/practice/practiceCritiqueStyles";
import * as q from "../components/practice/practiceQuestionStyles";
import * as s from "../components/practice/practiceStyles";
import {
  fetchPracticeDeck,
  submitPracticeAnswer,
  wordingOf,
  type AnswerResult,
  type PracticeDeck,
} from "../services/practice";
import {
  fetchQuestionAnswers,
  openAnswerSession,
  type QuestionAnswers,
} from "../services/practiceAnswers";
import { practicePath } from "../utils/routePaths";
import ScenarioTimelineDock from "../components/scenario-timeline/ScenarioTimelineDock";
import { PracticeCrumb, PracticeLoadFailure, PracticeLoading } from "./practiceChrome";

/**
 * The one sentence on this page that is NOT a stored row, and cannot be: the
 * wording arrives on the payload this screen is waiting for. Same carve-out, and
 * same reason, as `PracticePage`'s.
 */
const LOADING = "Loading…";

const PracticeQuestionPage: React.FC = () => {
  const { slug = "", scenarioId = "", questionId = "" } = useParams();
  const [deck, setDeck] = React.useState<PracticeDeck | null>(null);
  const [answers, setAnswers] = React.useState<QuestionAnswers | null>(null);
  const [loadError, setLoadError] = React.useState<string | null>(null);

  const [draft, setDraft] = React.useState("");
  const [working, setWorking] = React.useState(false);
  const [longWait, setLongWait] = React.useState(false);
  const [result, setResult] = React.useState<AnswerResult | null>(null);
  const [writeError, setWriteError] = React.useState<string | null>(null);
  const [showEarlier, setShowEarlier] = React.useState(false);

  // `abandoned` and not a cancelled request: Stop waiting must not cancel the
  // POST. The answer is written before the read runs, so aborting mid-flight
  // could kill the request AFTER the row landed and BEFORE the read attached —
  // leaving her answer marked in-flight forever with nothing coming to settle
  // it. Stopping is a decision about the SCREEN.
  const abandoned = React.useRef(false);

  React.useEffect(() => {
    let live = true;
    Promise.all([
      fetchPracticeDeck(slug, scenarioId),
      fetchQuestionAnswers(questionId),
    ])
      .then(([loadedDeck, loadedAnswers]) => {
        if (!live) return;
        setDeck(loadedDeck);
        setAnswers(loadedAnswers);
        // Pre-filled with what stands. She edits it; pressing Answer unchanged
        // re-reads without writing a version.
        setDraft(loadedAnswers.current?.text ?? "");
      })
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice question: the page could not be loaded", cause);
        const detail = cause instanceof Error ? cause.message : String(cause);
        if (live) setLoadError(`Could not load this question: ${detail}`);
      });
    return () => {
      live = false;
    };
  }, [slug, scenarioId, questionId]);

  const crumb = <PracticeCrumb slug={slug} scenarioId={scenarioId} deck={deck} />;

  if (loadError !== null) {
    return <PracticeLoadFailure crumb={crumb} message={loadError} />;
  }
  if (deck === null || answers === null) {
    return <PracticeLoading crumb={crumb} label={LOADING} />;
  }

  const question = deck.questions.find((row) => row.id === questionId) ?? null;
  const w = (key: string) => wordingOf(deck.wording, key);

  if (question === null) {
    // Standing Rule 1: a question that is not in this deck says so. It is a
    // real state and not a defect — Chuck deletes questions and a bookmark
    // outlives them. Her ANSWERS to it are untouched: Delete is a hide.
    return <PracticeLoadFailure crumb={crumb} message={w("deck_question_missing")} />;
  }

  const onAnswer = () => {
    // ⚑ EVERY VISIBLE PART OF THE WORKING STATE IS SET BEFORE THE AWAIT.
    // The button relabels and disables, the box locks, and the critique block
    // appears empty — all synchronously, so the state exists for as long as the
    // request does rather than appearing after it.
    abandoned.current = false;
    setWorking(true);
    setLongWait(false);
    setWriteError(null);
    setResult(null);
    const timer = window.setTimeout(() => setLongWait(true), LONG_WAIT_MS);

    openAnswerSession(slug, scenarioId)
      .then((sessionId) =>
        submitPracticeAnswer({
          sessionId,
          questionId: question.id,
          answerText: draft,
          dontRecall: false,
          pointsTo: null,
        }),
      )
      .then((answered) => {
        window.clearTimeout(timer);
        // She pressed Stop waiting: the answer is saved and the critique is
        // discarded. Showing it now would be the screen contradicting the
        // control she just used.
        if (abandoned.current) return;
        setWorking(false);
        setResult(answered);
      })
      .catch((cause: unknown) => {
        window.clearTimeout(timer);
        // eslint-disable-next-line no-console
        console.error("practice question: the answer could not be recorded", cause);
        if (abandoned.current) return;
        setWorking(false);
        setWriteError(cause instanceof Error ? cause.message : String(cause));
      });
  };

  const earlier = answers.earlier;
  // ⚑ The working state's three visible facts come from ONE pure decision, so
  // that something can test them: nothing in this project can render a
  // component, so a claim living only in this file is a claim nothing checks.
  const chrome = answerChrome(working ? "working" : "idle");
  const view = critiqueFor(result);

  return (
    <div style={s.page} data-surface="practice">
      <style>{c.CRITIQUE_CSS}</style>
      {crumb}

      {/* Mockup Screen 1's button, and the window it opens. Self-contained:
          it fetches its own data and hides itself when this scenario carries
          no subset, so this page's own reads are untouched. */}
      <ScenarioTimelineDock slug={slug} scenarioId={scenarioId} />

      <section style={s.card}>
        <p style={q.question}>{question.text}</p>
        {question.receipt !== null && <p style={q.from}>{question.receipt}</p>}

        <p style={q.label}>{w("answer_label")}</p>
        <textarea
          style={chrome.boxLocked ? { ...q.box, ...q.boxLocked } : q.box}
          value={draft}
          readOnly={chrome.boxLocked}
          aria-label={w("answer_label")}
          onChange={(event) => setDraft(event.target.value)}
        />

        {/* One quiet line. Collapsed, never editable — Chuck's reading of an
            older version points at the words he read. */}
        {earlier.length > 0 && (
          <>
            <button
              type="button"
              style={q.quiet}
              data-practice-link
              aria-expanded={showEarlier}
              onClick={() => setShowEarlier((was) => !was)}
            >
              {earlier.length === 1
                ? w("earlier_version_one")
                : w("earlier_versions_template").replace("{n}", String(earlier.length))}
            </button>
            {showEarlier && (
              <div style={q.earlier}>
                {earlier.map((version) => (
                  <div key={version.answer_id} style={q.earlierRow}>
                    <div style={q.earlierWhen}>{version.answered_on}</div>
                    <p style={q.earlierText}>{version.text}</p>
                  </div>
                ))}
              </div>
            )}
          </>
        )}

        {writeError !== null && (
          <div style={{ ...s.feedback, marginTop: 12 }} role="alert">
            {writeError}
          </div>
        )}

        <div style={q.buttons}>
          <button
            type="button"
            style={
              chrome.buttonDisabled ? { ...s.buttonPrimary, ...q.buttonWorking } : s.buttonPrimary
            }
            disabled={chrome.buttonDisabled}
            onClick={onAnswer}
          >
            {w(chrome.buttonLabelKey)}
          </button>
          {chrome.stopOffered && (
            <button
              type="button"
              style={s.button}
              onClick={() => {
                abandoned.current = true;
                setWorking(false);
              }}
            >
              {w("read_stop_waiting")}
            </button>
          )}
          <a style={q.back} href={practicePath(slug, scenarioId)}>
            {w("back_label")}
          </a>
        </div>

        {/* PRESENT AND EMPTY from the press, not from the resolution. */}
        <Critique
          view={chrome.critiquePresent ? { kind: "working", longWait } : view}
          wording={deck.wording}
        />
      </section>
    </div>
  );
};

export default PracticeQuestionPage;
