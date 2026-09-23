// =============================================================================
// PracticePage.tsx — the start card (screen S0)
// =============================================================================
//
// The card Marie opens on: the accusation by name, who is asking, how many, the
// deck listed with its per-row controls, the sitting she walked out of, and
// Start. Nothing on this page answers a question — pressing Start (or a row's
// own `Practice this one ▸`) opens a sitting and NAVIGATES to its address.
//
// ## Why the sitting is a different page now (Section B, item B10)
//
// It was four screens at one address, and that is exactly what .401 got wrong:
// Roman answered question 1, left the page, came back — and started at question
// 1 again with no sign his answer had been kept. It had been kept; the screen
// had no address to return him to. A sitting is a thing you can be in the middle
// of, so it has a URL, and `PracticeSessionPage` renders it.
//
// ## Standing Rule 1 on this page
//
// Four distinct failures, four distinct observables: the deck failed to load
// (the underlying sentence, since the wording came with the payload that
// failed), the deck is EMPTY (the stored "seed it" sentence — not a failure at
// all), the session could not be opened (the failure card), and the resume /
// start-over write failed (a notice inside the blue box, with the sitting
// untouched).

import React from "react";
import { useParams } from "react-router-dom";

import PracticeStart, { type PracticeWho } from "../components/practice/PracticeStart";
import {
  browserStore,
  readAnalysis,
  writeAnalysis,
} from "../components/practice/answerAnalysis";
import * as f from "../components/practice/practiceFlowStyles";
import * as s from "../components/practice/practiceStyles";
import {
  fetchPracticeDeck,
  wordingOf,
  type PracticeDeck,
  type PracticeQuestion,
} from "../services/practice";
import { hideQuestion } from "../services/practiceEditor";
import {
  practicePrintPath,
  practiceQuestionPath,
  practiceReviewPath,
  practiceWalkPath,
} from "../utils/routePaths";
import { PracticeCrumb, PracticeFrame, PracticeLoadFailure, PracticeLoading } from "./practiceChrome";
import { usePracticeDeckControls } from "./usePracticeDeckControls";
import { usePracticeEditor } from "./usePracticeEditor";

/**
 * What the loading card says before the store has been read.
 *
 * The one sentence on these two pages that is NOT a stored row, and it cannot
 * be: the wording arrives on the payload this card is waiting for. Every other
 * literal was removed in v0 for the reason the wording law gives; this one has
 * nowhere to come from.
 */
const LOADING = "Loading…";

const PracticePage: React.FC = () => {
  const { slug = "", scenarioId = "" } = useParams<{ slug: string; scenarioId: string }>();

  const [deck, setDeck] = React.useState<PracticeDeck | null>(null);
  // A ref beside the state so the failure-sentence callbacks can read the
  // CURRENT wording without being rebuilt on every render — which would rebuild
  // the editor hook's handlers with them.
  const deckRef = React.useRef<PracticeDeck | null>(null);
  const [loadError, setLoadError] = React.useState<string | null>(null);
  // Which side the list shows. The *Who's asking?* selector is gone, so this is
  // fixed at the whole deck rather than read from a control — `view` still takes
  // it because the same hook serves the editor's "all questions" view.
  // Which side the LIST and the practice bar are showing — two controls on one
  // value, so choosing a side to read also aims the button that practises it.
  //
  // ⚑ CHUCK'S SIDE OPENS THE PAGE (Roman, 2026-09-15, from Marie's first week).
  // She practises the direct far more often than the cross — it is the half she
  // will actually be led through — and the page opened on the defense's every
  // time, which is one control to change before every single sitting. ONE state
  // and therefore one default: the picker and the tabs cannot disagree about
  // which side she is looking at.
  const [side, setSide] = React.useState<"george" | "chuck">("chuck");

  // The Answer-analysis switch. Its VALUE lives in the browser, not here — see
  // `answerAnalysis.ts` — and this state is the copy the current screen renders.
  // Read once on mount rather than on every render: reading storage is a
  // side-effecting call that can throw, and a render is not the place for one.
  const analysisStore = React.useMemo(() => browserStore(), []);
  const [analysisOn, setAnalysisOn] = React.useState(() => readAnalysis(analysisStore));

  /**
   * Flick the switch: remember it, then show it.
   *
   * The write is best-effort (a browser may refuse storage) and the screen
   * follows the state EITHER WAY — a switch that would not move because the
   * browser would not remember it is a control that appears broken over a
   * preference nobody can see. The warning is `writeAnalysis`'s own.
   */
  const onAnalysis = React.useCallback(
    (on: boolean) => {
      writeAnalysis(analysisStore, on);
      setAnalysisOn(on);
    },
    [analysisStore],
  );
  // `view` stays on `mixed` deliberately. It feeds the title row's print lock
  // and the editor's own deck, both of which are about the WHOLE deck: a print
  // button that locked itself because the side currently on screen happens to
  // be empty would be this change breaking a control it has no business
  // touching. The LIST does its own side selection, from the raw payload order,
  // through `sideSections`.
  const who: PracticeWho = "mixed";
  const [reloads, setReloads] = React.useState(0);
  // Delete and its undo. `deletingId` names WHICH row is in flight, not merely
  // that one is: with several rows on screen a single boolean would grey the
  // wrong one.
  const [deletingId, setDeletingId] = React.useState<string | null>(null);
  const [deleteError, setDeleteError] = React.useState<string | null>(null);

  const rowControls = usePracticeDeckControls();

  /**
   * Re-read the deck.
   *
   * Every editor write and every note write ends here rather than patching the
   * payload: a move re-orders two rows, an edit re-writes what the change log
   * says, an add changes the deck's length and Marie's badges, and a note
   * changes the "new notes" clause. All of those are sentences the SERVER
   * composes, and the browser holds no template for any of them.
   */
  const reload = React.useCallback(() => setReloads((n) => n + 1), []);

  // The stored failure sentences are read at FAILURE time rather than captured:
  // the wording arrives on the payload, and these hooks are constructed before
  // it exists. A literal here would be the one sentence on this page the store
  // could not change.
  const editorFailure = React.useCallback(
    () => (deckRef.current === null ? "" : wordingOf(deckRef.current.wording, "editor_failed")),
    [],
  );
  const editor = usePracticeEditor(slug, scenarioId, reload, editorFailure);

  // One fetch on mount, and again after Start over — which changes what the
  // payload says (the open sitting is gone) and nothing else. `reloads` is the
  // whole mechanism: bumping it re-runs this effect.
  React.useEffect(() => {
    let cancelled = false;
    setLoadError(null);
    fetchPracticeDeck(slug, scenarioId)
      .then((payload) => {
        if (cancelled) return;
        setDeck(payload);
        deckRef.current = payload;
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
  }, [slug, scenarioId, reloads]);

  const crumb = <PracticeCrumb slug={slug} scenarioId={scenarioId} deck={deck} />;

  if (loadError !== null) return <PracticeLoadFailure crumb={crumb} message={loadError} />;
  if (deck === null) return <PracticeLoading crumb={crumb} label={LOADING} />;

  const w = (key: string) => wordingOf(deck.wording, key);

  // An empty deck is NOT a failure. S-6 is in exactly this state until somebody
  // runs the seed, and the page says so in the store's own words.
  if (deck.questions.length === 0) {
    return (
      <PracticeFrame crumb={crumb}>
        <div style={s.kicker}>{w("kicker")}</div>
        <h1 style={s.h1}>
          {deck.code} · {deck.title}
        </h1>
        <p style={s.sub}>{w("empty_deck")}</p>
      </PracticeFrame>
    );
  }

  const view = rowControls.view(deck.questions, who);

  /** Write one note on this scenario, and re-read so the counts follow. */
  /**
   * Delete a question, or put it back — one call, two directions.
   *
   * ## Domain note: the mechanism is the existing HIDE, unchanged
   *
   * Nothing is deleted. `practice_answers.question_id` keeps its
   * `ON DELETE RESTRICT`, so a question Marie has answered can never be orphaned
   * from her answers, and the user's contract — "I will not see this again" —
   * is what the hide actually delivers.
   *
   * Standing Rule 1: a failure sets a sentence naming the question, and the row
   * stays exactly where it is. A row that vanished on a failed write would tell
   * Chuck the deck had changed when it had not.
   */
  const setHidden = (question: PracticeQuestion, hidden: boolean) => {
    setDeletingId(question.id);
    setDeleteError(null);
    hideQuestion(question.id, hidden)
      .then(() => setReloads((n) => n + 1))
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the question could not be hidden", error);
        const detail = error instanceof Error ? error.message : String(error);
        setDeleteError(`“${question.text.slice(0, 60)}…” — ${detail}`);
      })
      .finally(() => setDeletingId(null));
  };
  const remove = (question: PracticeQuestion) => setHidden(question, true);
  const putBack = (question: PracticeQuestion) => setHidden(question, false);

  return (
    // `data-surface="practice"` is what resolves every `var(--practice-…)` in
    // `practiceStyles`. Without it the whole palette falls back to nothing and
    // the screen renders unstyled — which is why `practiceStyles.test.ts` pins
    // that this attribute and that CSS block stay in step.
    <div style={s.page} data-surface="practice">
      {/* The hover rule the top bar and the deck links need; a style object
          cannot carry a pseudo-class. See `LINK_CSS`. */}
      <style>{f.LINK_CSS}</style>
      {crumb}

      {/* ⚑ THE STRIP IS GONE FROM THIS PAGE (Roman, 2026-09-07, from a
          screenshot). It rendered above the card below and the two said the same
          thing: the same code, the same title, an inch apart.

          Its three LIVE pieces moved onto that card's own title line — the
          direction chip and the Draft/Ready switch (`ScenarioIdentityControls`)
          and the View Timeline dock. Nothing else on it was live here: Practice
          was hidden (`hidePractice` — this IS the practice page), and Edit and
          Delete were never drawn, because this page passed no handler for either
          and the strip draws no control it cannot operate. No `onStatusChanged`
          was passed either, so there is no page-level refresh to re-wire; the
          switch's own re-read moved with it.

          `ScenarioHeaderStrip` itself is untouched and still renders on the
          scenario detail page. */}

      <PracticeStart
        slug={slug}
        scenarioId={scenarioId}
        code={deck.code}
        title={deck.title}
        printHref={practicePrintPath(slug, scenarioId)}
        reviewHref={practiceReviewPath(slug, scenarioId)}
        wording={deck.wording}
        view={view}
        servedAt={deck.served_at}
        editor={editor}
        attachOptions={deck.attach_options}
        tacticCards={deck.tactic_cards}
        analysisOn={analysisOn}
        onAnalysis={onAnalysis}
        onDelete={remove}
        onUndoDelete={putBack}
        deletingId={deletingId}
        deleteError={deleteError}
        questionHref={(question) => practiceQuestionPath(slug, scenarioId, question.id)}
        walkHref={(forSide) => practiceWalkPath(slug, scenarioId, forSide)}
        side={side}
        onSide={setSide}
        allQuestions={deck.questions}
        review={deck.review}
        onReviewed={() => setReloads((n) => n + 1)}
      />
    </div>
  );
};

export default PracticePage;
