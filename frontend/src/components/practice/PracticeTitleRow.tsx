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

import {
  wordingOf,
  type PracticeQuestion,
  type PracticeWording,
} from "../../services/practice";
import * as e from "./practiceEditorStyles";
import * as s from "./practiceStyles";
import { printLockReason } from "./printSheetPlan";

type Props = {
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
const PracticeTitleRow: React.FC<Props> = ({
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
      <h1 style={s.h1}>
        {code} · {title}
      </h1>
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
