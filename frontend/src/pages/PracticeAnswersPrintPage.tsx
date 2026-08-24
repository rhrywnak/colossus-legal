// PracticeAnswersPrintPage.tsx — Chuck's reading copy, at its own address.
//
// ## Why a sibling ADDRESS and not a mode on the questions view
//
// They are two documents for two different acts: one is marked up, one is read.
// Chuck keeps both tabs open and moves between them, and a `?answers=1` would
// make them one page that sometimes shows something else — the shape whose Back
// button lies, and whose tab strip cannot tell you which you are looking at.
//
// ## Domain note: it does NOT print itself
//
// No `window.print()` on load, for the same reason its sibling has none. He
// opens the tab, READS, and then decides. A page that starts printing before he
// has looked at it is a page he cannot review.
//
// ## Two requests, deliberately
//
// The deck for the questions and their order, the answers for the prose. See
// `fetchPracticeAnswers` for why the second is not folded into the first.

import React from "react";
import { useParams } from "react-router-dom";

import PrintAnswers from "../components/practice/PrintAnswers";
import { planAnswers } from "../components/practice/printAnswerPlan";
import * as p from "../components/practice/printStyles";
import { fetchPracticeDeck, wordingOf, type PracticeDeck } from "../services/practice";
import { fetchPracticeAnswers, type PracticeAnswer } from "../services/practiceAnswers";
import { practicePath } from "../utils/routePaths";
import { asPrintedAt, asSheetDate } from "./practicePrintFormat";

// CONST: the ONE string on this page that is not a settings row, because the
// settings store is precisely what failed to load — `wording` arrives INSIDE the
// deck payload, so on the error path there is no row to read. Deliberately
// matches `practice_print_back_label`'s seeded value. Same carve-out, and same
// reasoning, as the questions view's.
const BACK_WITHOUT_WORDING = "◂ Back to the deck";

const PracticeAnswersPrintPage: React.FC = () => {
  const { slug = "", scenarioId = "" } = useParams();
  const [deck, setDeck] = React.useState<PracticeDeck | null>(null);
  const [answers, setAnswers] = React.useState<PracticeAnswer[] | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  // Taken ONCE, so a re-render cannot move the printed-at stamp and leave two
  // sheets of one printout disagreeing about when they were printed.
  const [printedAt] = React.useState(() => asPrintedAt(new Date()));

  React.useEffect(() => {
    let live = true;
    // Both, together: a page showing questions with every answer missing would
    // be indistinguishable from a deck nobody has answered, and Chuck would act
    // on it. Either failing withdraws the whole sheet and says which.
    Promise.all([
      fetchPracticeDeck(slug, scenarioId),
      fetchPracticeAnswers(slug, scenarioId),
    ])
      .then(([loadedDeck, loadedAnswers]) => {
        if (!live) return;
        setDeck(loadedDeck);
        setAnswers(loadedAnswers);
      })
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice answers: the sheet could not be loaded", cause);
        // Standing Rule 1: the message names WHICH deck failed. A person with
        // three print tabs open cannot act on "failed to fetch".
        const detail = cause instanceof Error ? cause.message : String(cause);
        if (live) {
          setError(
            `Could not load the answers for scenario ${scenarioId} (case ${slug}): ${detail}`,
          );
        }
      });
    return () => {
      live = false;
    };
  }, [slug, scenarioId]);

  // The tab's title, once the deck names itself — so two print tabs are
  // tellable apart from the tab strip alone, and an answers tab from a
  // questions tab.
  React.useEffect(() => {
    if (deck === null) return;
    document.title = wordingOf(deck.wording, "print_answers_page_title")
      .split("{code}")
      .join(deck.code);
  }, [deck]);

  if (error !== null) {
    // Standing Rule 1: a load that failed says so. A blank grey page would read
    // as a deck with nothing in it, which is a different fact entirely.
    return (
      <div style={p.page} data-print-desk>
        <div style={p.bar} data-print-chrome>
          <a style={p.backLink} href={practicePath(slug, scenarioId)}>
            {BACK_WITHOUT_WORDING}
          </a>
        </div>
        <div style={{ ...p.sheet, fontFamily: "inherit" }} role="alert">
          {error}
        </div>
      </div>
    );
  }

  if (deck === null || answers === null) return <div style={p.page} data-print-desk />;

  const w = (key: string) => wordingOf(deck.wording, key);
  const plan = planAnswers(deck.questions, deck.code, deck.title, deck.wording, answers);

  return (
    <div style={p.page} data-print-desk>
      <div style={p.bar} data-print-chrome>
        <button type="button" style={p.printButton} onClick={() => window.print()}>
          {w("print_now_label")}
        </button>
        <a style={p.backLink} href={practicePath(slug, scenarioId)}>
          {w("print_back_label")}
        </a>
      </div>
      <style>{p.PRINT_CSS}</style>
      <PrintAnswers
        plan={plan}
        deckAsOf={asSheetDate(deck.deck_as_of)}
        printedAt={printedAt}
        wording={deck.wording}
      />
    </div>
  );
};

export default PracticeAnswersPrintPage;
