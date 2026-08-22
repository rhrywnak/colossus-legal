// PracticePrintPage.tsx — Chuck's review sheets, at their own address.
//
// ## Why this is a PAGE and not a print stylesheet on the practice page
//
// Roman's ruling of 2026-08-22, on a measured finding: the practice page's deck
// list is rendered `{open && (…)}`, so a folded deck is **not in the DOM** and no
// stylesheet can reveal it; and it renders `editor.editing ? view.all :
// view.ordered`, so the question set on screen follows the *Who's asking?*
// selector and edit mode. "Print the whole deck, ignoring the selector" is not
// something that DOM can be asked for. This page reads the payload instead.
//
// ## Domain note: it does NOT print itself
//
// No `window.print()` on load. Chuck opens the tab, READS the sheets, and then
// decides — a page that starts printing before he has looked at it is a page he
// cannot review, and reviewing is the entire purpose. The Print button is his.

import React from "react";
import { useParams } from "react-router-dom";

import PrintSheets from "../components/practice/PrintSheets";
import { planSheets } from "../components/practice/printSheetPlan";
import * as p from "../components/practice/printStyles";
import {
  fetchPracticeDeck,
  wordingOf,
  type PracticeDeck,
} from "../services/practice";
import { practicePath } from "../utils/routePaths";
import { asPrintedAt, asSheetDate } from "./practicePrintFormat";

// CONST: the ONE string on this page that is not a settings row, because the
// settings store is precisely what failed to load — `wording` arrives INSIDE the
// deck payload, so on the error path there is no row to read. It deliberately
// matches `practice_print_back_label`'s seeded value; if an operator re-words that
// row, this fallback keeps the older words on the one screen where no stored word
// is reachable. A second fetch just for one label was considered and rejected: it
// is a second thing to fail on the path that has already failed, and the
// alternative — no way back at all from a failed load — is worse than both.
const BACK_WITHOUT_WORDING = "◂ Back to the deck";

const PracticePrintPage: React.FC = () => {
  const { slug = "", scenarioId = "" } = useParams();
  const [deck, setDeck] = React.useState<PracticeDeck | null>(null);
  const [error, setError] = React.useState<string | null>(null);

  // Taken ONCE, when the page loads, rather than at render: a re-render must not
  // move the printed-at stamp, or the sheets in one printout disagree about when
  // they were printed.
  const [printedAt] = React.useState(() => asPrintedAt(new Date()));

  React.useEffect(() => {
    let live = true;
    fetchPracticeDeck(slug, scenarioId)
      .then((loaded) => {
        if (live) setDeck(loaded);
      })
      .catch((cause: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice print: the deck could not be loaded", cause);
        // Standing Rule 1: the message names WHICH deck failed. A person with
        // three print tabs open cannot act on "failed to fetch".
        const detail = cause instanceof Error ? cause.message : String(cause);
        if (live) {
          setError(
            `Could not load the deck for scenario ${scenarioId} (case ${slug}): ${detail}`,
          );
        }
      });
    return () => {
      live = false;
    };
  }, [slug, scenarioId]);

  // The tab's title, once the deck names itself. A person with three of these
  // open needs to tell them apart from the tab strip alone.
  React.useEffect(() => {
    if (deck === null) return;
    document.title = wordingOf(deck.wording, "print_page_title")
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

  if (deck === null) return <div style={p.page} data-print-desk />;

  const w = (key: string) => wordingOf(deck.wording, key);
  const plan = planSheets(deck.questions, deck.code, deck.title, deck.wording);

  return (
    <div style={p.page} data-print-desk>
      {/* `data-print-chrome` is the whole of what this page hides in print. The
          practice page's own print CSS has to hide an application; this hides two
          buttons, which is the practical half of why the view has its own URL. */}
      <div style={p.bar} data-print-chrome>
        <button
          type="button"
          style={p.printButton}
          onClick={() => window.print()}
        >
          {w("print_now_label")}
        </button>
        <a style={p.backLink} href={practicePath(slug, scenarioId)}>
          {w("print_back_label")}
        </a>
      </div>
      <style>{p.PRINT_CSS}</style>
      <PrintSheets
        plan={plan}
        code={deck.code}
        deckAsOf={asSheetDate(deck.deck_as_of)}
        printedAt={printedAt}
        wording={deck.wording}
      />
    </div>
  );
};

export default PracticePrintPage;
