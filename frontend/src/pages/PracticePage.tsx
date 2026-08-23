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
import { useNavigate, useParams } from "react-router-dom";

import PracticeStart, { type PracticeWho } from "../components/practice/PracticeStart";
import * as f from "../components/practice/practiceFlowStyles";
import * as s from "../components/practice/practiceStyles";
import {
  fetchPracticeDeck,
  startPracticeSession,
  wordingOf,
  type PracticeDeck,
  type PracticeQuestion,
} from "../services/practice";
import { resumeSitting, startOverSitting } from "../services/practiceFlow";
import { saveNote, strikeNote } from "../services/practiceEditor";
import {
  practicePath,
  practicePrintPath,
  practiceQuestionPath,
  practiceSessionPath,
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
  const navigate = useNavigate();

  const [deck, setDeck] = React.useState<PracticeDeck | null>(null);
  // A ref beside the state so the failure-sentence callbacks can read the
  // CURRENT wording without being rebuilt on every render — which would rebuild
  // the editor hook's handlers with them.
  const deckRef = React.useRef<PracticeDeck | null>(null);
  const [loadError, setLoadError] = React.useState<string | null>(null);
  const [who, setWho] = React.useState<PracticeWho>("george");
  const [starting, setStarting] = React.useState(false);
  const [startingOne, setStartingOne] = React.useState(false);
  const [resuming, setResuming] = React.useState(false);
  const [resumeError, setResumeError] = React.useState<string | null>(null);
  const [reloads, setReloads] = React.useState(0);
  const [savingNote, setSavingNote] = React.useState(false);
  const [noteError, setNoteError] = React.useState<string | null>(null);

  const rowControls = usePracticeDeckControls(setDeck);

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
  const writeNote = (text: string) => {
    setSavingNote(true);
    setNoteError(null);
    saveNote(slug, scenarioId, { questionId: null, answerId: null }, text)
      .then(() => reload())
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the note was not saved", error);
        setNoteError(w("notes_failed"));
      })
      .finally(() => setSavingNote(false));
  };

  /** Strike one note through. Never a delete. */
  const strike = (note: { id: string }) => {
    setSavingNote(true);
    setNoteError(null);
    strikeNote(note.id)
      .then(() => reload())
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the note was not struck", error);
        setNoteError(w("notes_failed"));
      })
      .finally(() => setSavingNote(false));
  };

  /**
   * Open a sitting and go to its address.
   *
   * The queue is settled BEFORE the call, so the sitting that is STORED and the
   * sitting that is dealt are the same list — not two slices taken a moment
   * apart from state that could have moved between them.
   */
  const open = (
    sideOf: PracticeWho,
    dealt: PracticeQuestion[],
    busy: (value: boolean) => void,
  ) => {
    busy(true);
    startPracticeSession(slug, scenarioId, {
      who: sideOf,
      queue: dealt.map((q) => q.id),
      count: dealt.length,
      skippedToday: [...rowControls.skippedToday],
    })
      .then((id) => navigate(practiceSessionPath(slug, scenarioId, id)))
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the session could not be started", error);
        setLoadError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => busy(false));
  };

  /**
   * A ONE-question sitting on the question she clicked (task A2).
   *
   * `who` is the question's own side rather than the filter she happens to be
   * looking at, because the sitting contains nothing else — recording it as a
   * "George" sitting holding one of Chuck's would put a wrong word on the row
   * this evening leaves behind.
   */
  const practiceOne = (question: PracticeQuestion) => {
    open(question.side === "george" ? "george" : "chuck", [question], setStartingOne);
  };

  const resume = () => {
    if (deck.open_session === null) return;
    const sessionId = deck.open_session.session_id;
    setResuming(true);
    setResumeError(null);
    resumeSitting(sessionId)
      .then(() => navigate(practiceSessionPath(slug, scenarioId, sessionId)))
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the sitting could not be resumed", error);
        setResumeError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => setResuming(false));
  };

  const startOver = () => {
    if (deck.open_session === null) return;
    setResuming(true);
    setResumeError(null);
    startOverSitting(deck.open_session.session_id)
      .then(() => {
        // Re-read rather than patching the payload in place: closing a sitting
        // changes the last-session line too, and that sentence is composed on
        // the server. Guessing at it here would be the browser writing a
        // sentence it holds no template for.
        setReloads((n) => n + 1);
        navigate(practicePath(slug, scenarioId), { replace: true });
      })
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the sitting could not be closed", error);
        setResumeError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => setResuming(false));
  };

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
      <PracticeStart
        code={deck.code}
        title={deck.title}
        printHref={practicePrintPath(slug, scenarioId)}
        wording={deck.wording}
        lastSessionLine={deck.last_session_line}
        who={who}
        onWhoChange={setWho}
        onStart={() => open(who, view.available.slice(0, view.count), setStarting)}
        starting={starting}
        controls={rowControls}
        view={view}
        openSession={deck.open_session}
        onResume={resume}
        onStartOver={startOver}
        resuming={resuming}
        resumeError={resumeError}
        onPracticeOne={practiceOne}
        startingOne={startingOne}
        onReview={(question) =>
          navigate(practiceQuestionPath(slug, scenarioId, question.id))
        }
        editor={editor}
        attachOptions={deck.attach_options}
        changed={deck.changed}
        notes={deck.notes}
        onSaveNote={writeNote}
        onStrikeNote={strike}
        savingNote={savingNote}
        noteError={noteError}
      />
    </div>
  );
};

export default PracticePage;
