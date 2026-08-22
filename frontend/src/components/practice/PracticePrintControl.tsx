// PracticePrintControl.tsx — the scenario title, and the way to Chuck's sheets.
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
  /** Where the print view lives, composed by the page. */
  printHref: string;
  /** The WHOLE deck, hidden rows included — the lock counts what would print. */
  questions: PracticeQuestion[];
  /** True while the deck editor is on. */
  editing: boolean;
  wording: PracticeWording;
};

/**
 * The title and the print control, on one line — mockup v1, view 1.
 *
 * ## Domain note: it ignores the *Who's asking?* selector
 *
 * Always the whole deck. That selector includes Mixed, and Mixed is a dealing
 * order rather than a thing anyone reviews — so following it would print a
 * shuffle of two sides under headings that promise one.
 */
const PracticePrintControl: React.FC<Props> = ({
  code,
  title,
  printHref,
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
    </div>
  );
};

export default PracticePrintControl;
