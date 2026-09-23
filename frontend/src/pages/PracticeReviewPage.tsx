// =============================================================================
// PracticeReviewPage.tsx — Chuck's reading pass, at its own address
// =============================================================================
//
// CC_TASK_REVIEW_PAGE_v1 §1, from the ruled mockup
// `PRACTICE_REVIEW_MOCKUP_v1_2026-09-19.html`.
//
// ## What this page is, and why it had to exist
//
// Marie answers a deck over a week. Chuck reads it in one sitting before trial
// and writes on what he reads — and until today the only way through a
// 42-answer deck was to open each question's own page in turn, forty-two times.
// This is the loop's READ half: one scrollable page, deck order, her current
// answer under each question with the notes standing on it and a box.
//
// ## It composes; it decides nothing new
//
// Two reads the build already had — the deck (questions, notes, wording, the
// review block) and the scenario's current answers — and the note and strike
// routes the review loop already shipped. No new endpoint, and no business
// logic: whether Done reviewing is offered is the SERVER's answer
// (`review.can_mark_reviewed`), exactly as it is on the deck page.
//
// ## Two requests, not forty-two
//
// The deck carries per-question notes and the answers payload carries every
// current answer, so a 42-row deck costs two requests and three SQL statements.
// An N+1 here would have been the task's named STOP.
//
// ## After any write, BOTH reads are stale
//
// A note changes the deck payload (which carries the notes) and can change the
// review count; a strike does the same. So every write re-reads BOTH — the
// `PracticeAnswerNotes` pattern, widened to this page's two halves. Nothing is
// patched locally: what renders is the server's, or the page says it failed.

import React from "react";
import { useParams } from "react-router-dom";

import PracticeReviewBar from "../components/practice/PracticeReviewBar";
import PracticeReviewCard from "../components/practice/PracticeReviewCard";
import { shownItems } from "../components/practice/deckSweep";
import { noteTarget, planReview, type ReviewRow } from "../components/practice/practiceReviewPlan";
import * as p from "../components/practice/practiceReviewPageStyles";
import { addAnswerNote, addQuestionNote, strikeNote } from "../services/practiceReviewLoop";
import { fetchPracticeDeck, wordingOf, type PracticeDeck } from "../services/practice";
import { fetchPracticeAnswers, type PracticeAnswer } from "../services/practiceAnswers";
import { practiceAnswersPath } from "../utils/routePaths";
import { PracticeCrumb, PracticeFrame } from "./practiceChrome";

/** Everything one load of this page produces. */
type Loaded = { deck: PracticeDeck; answers: PracticeAnswer[] };

const PracticeReviewPage: React.FC = () => {
  const { slug = "", scenarioId = "" } = useParams();
  const [loaded, setLoaded] = React.useState<Loaded | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);

  /**
   * Both reads, together.
   *
   * Either failing withdraws the whole page and says which: a page showing
   * forty-two questions with every answer missing is indistinguishable from a
   * deck nobody has answered, and Chuck would act on it. The same argument the
   * printed answers sheet makes, for the same two reads.
   */
  const load = React.useCallback(
    () =>
      Promise.all([
        fetchPracticeDeck(slug, scenarioId),
        fetchPracticeAnswers(slug, scenarioId),
      ]).then(([deck, answers]) => ({ deck, answers })),
    [slug, scenarioId],
  );

  React.useEffect(() => {
    let live = true;
    load()
      .then((next) => {
        if (live) setLoaded(next);
      })
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice review: the page could not be loaded", cause);
        // Standing Rule 1: the message names WHICH deck failed. A person with
        // three tabs open cannot act on "failed to fetch".
        const detail = cause instanceof Error ? cause.message : String(cause);
        if (live) {
          setError(`scenario ${scenarioId} (case ${slug}): ${detail}`);
        }
      });
    return () => {
      live = false;
    };
  }, [load, slug, scenarioId]);

  /**
   * Run one write, then re-read BOTH halves and keep the server's answer.
   *
   * The failure path leaves what is on screen exactly as it was and raises the
   * stored sentence: the write did not happen, so the page must not look as
   * though it did.
   */
  const run = (write: () => Promise<unknown>) => {
    setBusy(true);
    setError(null);
    write()
      .then(load)
      .then(setLoaded)
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice review: a note write failed", cause);
        const detail = cause instanceof Error ? cause.message : String(cause);
        setError(`scenario ${scenarioId} (case ${slug}): ${detail}`);
      })
      .finally(() => setBusy(false));
  };

  /** Re-read both halves with no write in front — what Done reviewing needs. */
  const reload = () => run(() => Promise.resolve(undefined));

  const saveNote = (row: ReviewRow, text: string) => {
    const target = noteTarget(row);
    run(() =>
      target.kind === "answer"
        ? addAnswerNote(target.answerId, text)
        : addQuestionNote(target.questionId, text),
    );
  };

  // Before the first load resolves: the frame and the crumb, and nothing that
  // claims the deck is empty. `deck` is null, so the crumb shows two links.
  if (loaded === null && error === null) {
    return (
      <PracticeFrame crumb={<PracticeCrumb slug={slug} scenarioId={scenarioId} deck={null} />}>
        {null}
      </PracticeFrame>
    );
  }

  if (loaded === null) {
    // The load failed and there is no wording to speak with — `wording` arrives
    // INSIDE the deck payload, which is precisely what did not arrive. The
    // technical cause is the only thing this page can honestly say, and it is
    // already in the console; the same carve-out the print pages record.
    return (
      <PracticeFrame
        crumb={<PracticeCrumb slug={slug} scenarioId={scenarioId} deck={null} />}
        alert
      >
        {error}
      </PracticeFrame>
    );
  }

  const { deck, answers } = loaded;
  const w = (key: string) => wordingOf(deck.wording, key);
  const rows = planReview(deck.questions, answers);
  // ONE control, rendered twice. The bar writes the mark itself and then asks
  // the page to re-read, so the zero it shows afterwards is the server's.
  const done = (
    <PracticeReviewBar
      slug={slug}
      scenarioId={scenarioId}
      code={deck.code}
      review={deck.review}
      wording={deck.wording}
      // Exactly the rows below, with the answer and the notes each one is
      // showing. `planReview` is what put them on screen, and `shownItems`
      // reads the same payload — the press cannot sweep a row this page did
      // not draw.
      shown={shownItems(deck.questions)}
      servedAt={deck.served_at}
      onReviewed={reload}
    />
  );

  return (
    <div style={{ background: "var(--bg-canvas)" }} data-surface="practice">
      <PracticeCrumb slug={slug} scenarioId={scenarioId} deck={deck} last={w("review_title")} />
      <div style={p.page} data-review-page>
        <div style={p.eyebrow}>{w("review_title")}</div>
        <div style={p.titleRow}>
          <h1 style={p.title}>
            {deck.code} · {deck.title}
          </h1>
          {/* Paper stays one click away. The printed-answers page is untouched
              and keeps its own address; this is the way in now that the deck's
              button row says Review answers instead. A new tab, like the two
              print controls it replaces. */}
          <a
            style={p.buttonGhost}
            href={practiceAnswersPath(slug, scenarioId)}
            target="_blank"
            rel="noopener noreferrer"
            role="button"
          >
            {w("print_answers_label")}
          </a>
        </div>

        {done}

        {/* A failure AFTER the page has loaded: the reads still hold, so the
            deck stays on screen and the sentence says the write did not land. */}
        {error !== null && (
          <div style={{ ...p.notice, color: "var(--practice-red)" }} role="alert">
            {w("review_load_failed")} {error}
          </div>
        )}

        {rows.length === 0 ? (
          <div style={p.notice}>{w("review_empty_deck")}</div>
        ) : (
          rows.map((row) => (
            <PracticeReviewCard
              key={row.question.id}
              question={row.question}
              answer={row.answer}
              wording={deck.wording}
              busy={busy}
              onSaveNote={(text) => saveNote(row, text)}
              onStrikeNote={(noteId) => run(() => strikeNote(noteId))}
            />
          ))
        )}

        {/* Done reviewing, again, at the end of forty-two rows. The SAME
            control — he has just finished reading, and scrolling back to the
            top to say so is the friction that leaves a queue uncleared. */}
        <div style={p.foot}>{done}</div>
      </div>
    </div>
  );
};

export default PracticeReviewPage;
