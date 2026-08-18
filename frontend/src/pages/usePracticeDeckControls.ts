// =============================================================================
// usePracticeDeckControls.ts — the start screen's two per-row controls
// =============================================================================
//
// Mockup v3's Skip today and Flag, as one hook: the state they hold, and the one
// write they make. Extracted from `PracticePage` because that component passed
// Rule 17's 300-line limit when they were added to it, and because this is a
// coherent thing on its own — everything the START screen can do to a question
// before a sitting begins.
//
// ## Why the two live together
//
// They are the same act from the page's point of view (something happened to a
// row) and they are read by the same component. Splitting them would put two
// hooks in the page where the mockup draws one control strip.

import React from "react";

import type { PracticeDeck, PracticeQuestion } from "../services/practice";
import { savePracticeFlag } from "../services/practiceFlow";
import { availableDeck, orderedDeck, V0_QUESTION_COUNT } from "./practiceQueue";

/** What the start screen needs to render and drive the two row controls. */
export interface PracticeDeckControls {
  /**
   * Ids kept out of THIS sitting.
   *
   * Session-scoped and deliberately not persisted: she is not saying the
   * question is wrong — that is what Flag says — only that she does not want it
   * this evening. A reload clears it, and the proof step checks exactly that.
   */
  skippedToday: ReadonlySet<string>;
  toggleSkip: (id: string) => void;
  /** Which row's flag write is in flight, so its control can say so. */
  savingFlagFor: string | null;
  /** The last flag write's failure. Surfaced, never swallowed. */
  flagError: string | null;
  saveFlag: (id: string, note: string) => void;
  /** How many she has chosen to be asked, and the pill that changes it. */
  count: number;
  setCount: (count: number) => void;
  /** What the start screen renders, for one side, right now. */
  view: (questions: PracticeQuestion[], who: DeckSide) => DeckView;
}

/** The three decks the start screen can be filtered to. */
export type DeckSide = "george" | "chuck" | "mixed";

/**
 * One side's questions as the start screen needs them.
 *
 * `ordered` is every question on this side — the LIST, which shows a skipped row
 * struck through rather than hiding it, so she can put it back. `available` is
 * what a sitting could actually deal. `count` is clamped to `available` because
 * a pill offering five out of four is a control that cannot do what it says.
 */
export interface DeckView {
  ordered: PracticeQuestion[];
  available: PracticeQuestion[];
  count: number;
}

/**
 * Hold the start screen's row state, and write a flag when she saves one.
 *
 * `setDeck` is taken rather than a reload callback because the write returns the
 * stored value: the backend TRIMS the note and treats a blank one as "clear", so
 * echoing what she typed would leave a flag on screen that the database does not
 * have. Patching the one question in place is both cheaper than refetching the
 * deck and the only version that shows the truth.
 */
export function usePracticeDeckControls(
  setDeck: React.Dispatch<React.SetStateAction<PracticeDeck | null>>,
): PracticeDeckControls {
  const [skippedToday, setSkippedToday] = React.useState<ReadonlySet<string>>(
    () => new Set<string>(),
  );
  const [savingFlagFor, setSavingFlagFor] = React.useState<string | null>(null);
  const [flagError, setFlagError] = React.useState<string | null>(null);
  const [count, setCount] = React.useState(V0_QUESTION_COUNT);

  const toggleSkip = (id: string) => {
    // A new Set every time: React compares by reference, and mutating the held
    // one would leave the row rendering its old state.
    setSkippedToday((was) => {
      const next = new Set(was);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const saveFlag = (id: string, note: string) => {
    setSavingFlagFor(id);
    setFlagError(null);
    savePracticeFlag(id, note)
      .then((stored) => {
        setDeck((was) =>
          was === null
            ? was
            : {
                ...was,
                questions: was.questions.map((q) =>
                  q.id === id ? { ...q, flag_note: stored } : q,
                ),
              },
        );
      })
      .catch((error: unknown) => {
        // eslint-disable-next-line no-console
        console.error("practice: the flag could not be saved", error);
        setFlagError(error instanceof Error ? error.message : String(error));
      })
      .finally(() => setSavingFlagFor(null));
  };

  const view = (questions: PracticeQuestion[], who: DeckSide): DeckView => {
    const available = availableDeck(questions, who, skippedToday);
    return {
      ordered: orderedDeck(questions, who),
      available,
      count: Math.min(count, available.length),
    };
  };

  return {
    skippedToday,
    toggleSkip,
    savingFlagFor,
    flagError,
    saveFlag,
    count,
    setCount,
    view,
  };
}
