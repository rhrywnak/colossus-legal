// PracticeWalkPage.tsx — practice mode. Mockup v7 views 5, 6 and 7.
//
// ## ⚑ THIS PAGE WRITES NOTHING
//
// No model call. No database write. No session. Nothing is recorded about what
// she practised, how long she took, or whether she looked. Every fetch on this
// page happens ONCE, on mount, and both are reads.
//
// The test for that is not "the screen looks unchanged" — it is that no request
// is made across the whole loop, asserted by spying the network layer. "Nothing
// visible happened" and "nothing happened" are different claims.
//
// ## Skipping is pressing Next
//
// There is no skip control, because there is nothing to explain: moving on
// without revealing IS skipping. Nothing records that she did it, because
// nothing here records anything.

import React from "react";
import { useParams, useSearchParams } from "react-router-dom";

import * as p from "../components/practice/practiceWalkStyles";
import * as s from "../components/practice/practiceStyles";
import { walkAt, walkSteps, type WalkSide } from "../components/practice/practiceWalk";
import { fetchPracticeDeck, wordingOf, type PracticeDeck } from "../services/practice";
import { fetchPracticeAnswers, type PracticeAnswer } from "../services/practiceAnswers";
import { practicePath, practiceQuestionPath } from "../utils/routePaths";
import { PracticeCrumb, PracticeLoadFailure, PracticeLoading } from "./practiceChrome";

/** See `PracticePage` — the one sentence that cannot be a stored row. */
const LOADING = "Loading…";

const PracticeWalkPage: React.FC = () => {
  const { slug = "", scenarioId = "" } = useParams();
  const [params] = useSearchParams();
  // The side rides the URL so a reload lands on the same walk. `george` is the
  // fallback because it is what the bar offers first, not because it is safer.
  const side: WalkSide = params.get("side") === "chuck" ? "chuck" : "george";

  const [deck, setDeck] = React.useState<PracticeDeck | null>(null);
  const [answers, setAnswers] = React.useState<PracticeAnswer[] | null>(null);
  const [loadError, setLoadError] = React.useState<string | null>(null);
  const [at, setAt] = React.useState(0);
  const [revealed, setRevealed] = React.useState(false);

  React.useEffect(() => {
    let live = true;
    // ⚑ THE ONLY TWO REQUESTS THIS PAGE MAKES, both reads, both once.
    Promise.all([fetchPracticeDeck(slug, scenarioId), fetchPracticeAnswers(slug, scenarioId)])
      .then(([loadedDeck, loadedAnswers]) => {
        if (!live) return;
        setDeck(loadedDeck);
        setAnswers(loadedAnswers);
      })
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice walk: the walk could not be loaded", cause);
        const detail = cause instanceof Error ? cause.message : String(cause);
        if (live) setLoadError(`Could not start practice: ${detail}`);
      });
    return () => {
      live = false;
    };
  }, [slug, scenarioId]);

  const crumb = <PracticeCrumb slug={slug} scenarioId={scenarioId} deck={deck} />;
  if (loadError !== null) return <PracticeLoadFailure crumb={crumb} message={loadError} />;
  if (deck === null || answers === null) {
    return <PracticeLoading crumb={crumb} label={LOADING} />;
  }

  const w = (key: string) => wordingOf(deck.wording, key);
  const steps = walkSteps(deck.questions, answers, side);
  const state = walkAt(steps, at, revealed);
  const sideName = side === "chuck" ? w("who_chuck_title") : w("who_george_title");
  const back = (
    <a style={p.back} href={practicePath(slug, scenarioId)}>
      {w("back_label")}
    </a>
  );

  const next = () => {
    setRevealed(false);
    setAt((was) => was + 1);
  };

  if (state.kind === "nothing") {
    return (
      <div style={s.page} data-surface="practice">
        {crumb}
        <section style={{ ...s.card, ...p.centre }}>
          <p style={p.none}>{w("practice_none_answered")}</p>
          <div style={p.buttons}>{back}</div>
        </section>
      </div>
    );
  }

  if (state.kind === "end") {
    return (
      <div style={s.page} data-surface="practice">
        {crumb}
        <section style={{ ...s.card, ...p.centre }}>
          <p style={p.counter}>
            {w("practice_counter_template")
              .replace("{side}", sideName.toUpperCase())
              .replace(" · {n} OF {m}", "")}
          </p>
          <h2 style={p.endTitle}>{w("practice_end_title")}</h2>
          <p style={p.endCount}>
            {w("practice_end_count_template")
              .replace("{n}", String(state.total))
              .replace("{side}", sideName)}
          </p>
          <div style={p.buttons}>
            <a style={{ ...s.buttonPrimary, ...p.bigLink }} href={practicePath(slug, scenarioId)}>
              {w("back_label")}
            </a>
            <button
              type="button"
              style={s.button}
              onClick={() => {
                setAt(0);
                setRevealed(false);
              }}
            >
              {w("practise_again_label")}
            </button>
          </div>
        </section>
      </div>
    );
  }

  return (
    <div style={s.page} data-surface="practice">
      {crumb}
      <section style={{ ...s.card, ...p.centre }}>
        <p style={p.counter}>
          {w("practice_counter_template")
            .replace("{side}", sideName.toUpperCase())
            .replace("{n}", String(state.at + 1))
            .replace("{m}", String(state.total))}
        </p>
        <p style={state.kind === "revealed" ? p.questionSmall : p.question}>
          {state.step.question.text}
        </p>

        {state.kind === "asking" ? (
          <>
            {/* The greyed area CARRIES ITS INSTRUCTION — standing rule of
                2026-08-19: no control on a practice page is dim and silent. */}
            <div style={p.grey}>
              {w("practice_say_aloud")}
              <br />
              <span style={{ fontSize: 13 }}>
                {w("practice_then_press_template").replace("{label}", w("show_answer_label"))}
              </span>
            </div>
            <div style={p.buttons}>
              <button
                type="button"
                style={{ ...s.buttonPrimary, ...s.buttonBig }}
                onClick={() => setRevealed(true)}
              >
                {w("show_answer_label")}
              </button>
              <button type="button" style={s.button} onClick={next}>
                {w("next_question_label")}
              </button>
            </div>
            <div style={p.buttons}>{back}</div>
            {/* There is no skip control; this says so once. */}
            <p style={p.skipHint}>{w("practice_skip_hint")}</p>
          </>
        ) : (
          <>
            <div style={p.mine}>
              <p style={p.mineLabel}>
                {w("your_answer_dated_template").replace("{when}", state.step.answer.answered_on)}
              </p>
              <p style={p.mineText}>{state.step.answer.text}</p>
            </div>
            <div style={p.buttons}>
              <button
                type="button"
                style={{ ...s.buttonPrimary, ...s.buttonBig }}
                onClick={next}
              >
                {w("next_question_label")}
              </button>
              <a
                style={{ ...s.button, ...p.bigLink }}
                href={practiceQuestionPath(slug, scenarioId, state.step.question.id)}
              >
                {w("change_answer_label")}
              </a>
            </div>
            <div style={p.buttons}>{back}</div>
          </>
        )}
      </section>
    </div>
  );
};

export default PracticeWalkPage;
