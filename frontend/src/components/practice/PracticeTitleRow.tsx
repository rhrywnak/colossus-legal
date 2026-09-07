// PracticeTitleRow.tsx — the scenario title, and the three things you do here.
//
// Split from `PracticeStart` when adding it took that file past 300 lines
// (Rule 17). It earns the split on its own terms too: this is the one control on
// the card that LEAVES the page, and it is the only one with two independent
// reasons to refuse.
//
// ## Why an anchor and not a button
//
// It OPENS a view. A person who wants the sheets in a window of their own — a
// second monitor, beside the deck they are editing — gets the browser's own
// middle-click and "open in new tab" for free from an `<a>`, and nothing from a
// `<button>` with an onClick. It carries `role="button"` because it reads as one.

import React from "react";

import ScenarioIdentityControls from "../scenario/ScenarioIdentityControls";
import ScenarioTimelineDock from "../scenario-timeline/ScenarioTimelineDock";
import * as ss from "../scenario/stripStyles";
import {
  wordingOf,
  type PracticeQuestion,
  type PracticeWording,
} from "../../services/practice";
import * as e from "./practiceEditorStyles";
import { printLockReason } from "./printSheetPlan";

type Props = {
  /** The scenario this card is about — what the three moved controls read.
   *
   *  Two scalars rather than the identity payload itself: the chip and the
   *  switch fetch it (see `ScenarioIdentityControls`), and the dock fetches its
   *  own subsets. Passing the payload down instead would teach this component
   *  and its parent about a DTO neither otherwise reads. */
  slug: string;
  scenarioId: string;
  /** `S-5` — the handle a human reads aloud. */
  code: string;
  /** The accusation, as the page titles itself. */
  title: string;
  /** Where the printed QUESTIONS live, composed by the page. */
  printHref: string;
  /** Where the printed ANSWERS live, composed by the page. */
  answersHref: string;
  /** Turn the deck editor on or off. */
  onToggleEditing: () => void;
  /** The WHOLE deck, hidden rows included — the lock counts what would print. */
  questions: PracticeQuestion[];
  /** True while the deck editor is on. */
  editing: boolean;
  wording: PracticeWording;
};

/**
 * The title on its own full-width line, and the three controls in a row below.
 *
 * ## Domain note: the title stopped sharing a row (Roman, 2026-08-25)
 *
 * The three controls used to sit beside the `h1`, which left the title about
 * 360px of a MacBook-width card and wrapped "The auction was unnecessary and
 * costly" onto three lines. The buttons are unchanged — same three, same order,
 * same styles — they simply moved to their own line underneath, and the title
 * takes the full width it needs to read horizontally.
 *
 * ## Three buttons for two people
 *
 * Chuck prints QUESTIONS to mark up, prints ANSWERS to read, and edits the
 * deck. Marie does none of the three. Roman named the four jobs this page
 * serves, and these are three of them sitting where a person looks first.
 *
 * ## Domain note: Edit the deck stopped being a text link
 *
 * It was one, in the list header below, and Chuck could not find it — which put
 * Edit, the reorder arrows and the old Hide behind the least discoverable thing
 * on the page. A button, beside the other two, in the same style.
 *
 * ## Domain note: the sheets ignore any selector
 *
 * Always the whole deck. The retired *Who's asking?* selector included Mixed,
 * and Mixed is a dealing order rather than a thing anyone reviews — printing it
 * would give a shuffle of two sides under headings that promise one.
 */
/**
 * The title and the three controls that came off the strip, on one line.
 *
 * ## ⚑ Why the title is `flex: 1` and why this wraps
 *
 * `ss.title` carries `overflow: hidden` + `text-overflow: ellipsis`, which in a
 * flex row makes the `h1` the ONLY item that can shrink — the chip, the switch
 * and the dock are all `nowrap` and refuse. Left at `flex: 0 1 auto` the title
 * therefore absorbs every pixel of shortfall and collapses to ZERO WIDTH before
 * any of them gives: measured in a narrow viewport, "S-11 · The $50,000"
 * disappeared entirely while the three controls sat there intact.
 *
 * `flex: 1` with `minWidth: 0` makes it take the slack and ellipsise instead,
 * which is what the strip's own title does. `flexWrap` is the backstop for a
 * viewport too narrow even for that: the controls drop to a second line rather
 * than the title being destroyed to keep them on one. At any normal width
 * nothing wraps and the line reads exactly as drawn.
 */
const titleLine: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  flexWrap: "wrap",
  minWidth: 0,
};

/** The title's share of that line: all the slack, and the first to give it up. */
const titleFlex: React.CSSProperties = { ...ss.title, flex: "1 1 auto", minWidth: 0 };

const PracticeTitleRow: React.FC<Props> = ({
  slug,
  scenarioId,
  code,
  title,
  printHref,
  answersHref,
  onToggleEditing,
  questions,
  editing,
  wording,
}) => {
  const w = (key: string) => wordingOf(wording, key);
  // A SENTENCE and not a boolean: there is nothing to put in `title` unless this
  // also has the reason, so the control cannot be disabled without saying why.
  // The standing rule of 2026-08-19, enforced by the return type.
  const lock = printLockReason(
    questions,
    editing,
    w("print_questions_empty_hint"),
    w("editor_busy_hint"),
  );
  return (
    <div style={e.titleRow}>
      {/* ⚑ THE TITLE LINE NOW CARRIES THE STRIP'S THREE LIVE PIECES.
          Roman, 2026-09-07, from a screenshot: `ScenarioHeaderStrip` sat above
          this card saying the same code and the same title an inch higher. The
          strip came off the page; what was LIVE on it moved here.

          `titleRow` is still a column — the Print / Edit row below is untouched
          — so this inner row is the title's own line, and the dock's `auto`
          margin pushes View Timeline to the far end of it. */}
      <div style={titleLine}>
        {/* The strip's own `title` style: 1.5rem / 800. The 28px `s.h1` this
            replaces was the practice card speaking in a different voice from
            every other scenario surface, which is what made two headers on one
            page read as two different things rather than one thing twice. */}
        <h1 style={titleFlex}>
          {code} · {title}
        </h1>

        {/* The chip and the Draft/Ready switch — the same components the strip
            mounted, with the tooltip and the write path untouched. They fetch
            the identity themselves because the practice deck payload carries
            neither `direction` nor `status`; see that component's header. */}
        <ScenarioIdentityControls slug={slug} scenarioId={scenarioId} />

        {/* View Timeline, at the right end of the same line, exactly as it sat
            in the strip's action slot. The dock hides its own button when the
            scenario carries no subset — "simply absent, nothing else shifts" —
            so this does not second-guess it with a count of its own. */}
        <span style={ss.actions}>
          <ScenarioTimelineDock slug={slug} scenarioId={scenarioId} />
        </span>
      </div>
      <div style={e.titleActions}>
        <a
          // No `href` at all when locked: an anchor without one is not focusable and
          // not activatable, which is the honest disabled state for a link. The
          // `onClick` guard covers a middle-click that ignores `aria-disabled`.
          href={lock === null ? printHref : undefined}
          target="_blank"
          rel="noopener noreferrer"
          role="button"
          aria-disabled={lock !== null}
          title={lock ?? undefined}
          style={{ ...e.printControl, ...(lock !== null ? e.lockedControl : {}) }}
          onClick={(event) => {
            if (lock !== null) event.preventDefault();
          }}
        >
          {w("print_questions_label")}
        </a>
        <a
          // Same lock and the same honest disabled state as its sibling: an empty
          // deck has no answers to print either, and a deck being edited is one
          // whose paper would be out of date before it left the printer.
          href={lock === null ? answersHref : undefined}
          target="_blank"
          rel="noopener noreferrer"
          role="button"
          aria-disabled={lock !== null}
          title={lock ?? undefined}
          style={{ ...e.printControl, ...(lock !== null ? e.lockedControl : {}) }}
          onClick={(event) => {
            if (lock !== null) event.preventDefault();
          }}
        >
          {w("print_answers_label")}
        </a>
        <button
          type="button"
          style={e.printControl}
          aria-pressed={editing}
          onClick={onToggleEditing}
        >
          {editing ? w("editor_done_label") : w("editor_switch_label")}
        </button>
      </div>
    </div>
  );
};

export default PracticeTitleRow;
