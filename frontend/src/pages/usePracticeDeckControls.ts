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

import type { PracticeQuestion } from "../services/practice";
import { availableDeck, editorDeck, orderedDeck, V0_QUESTION_COUNT } from "./practiceQueue";

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
  /**
   * The same side INCLUDING hidden questions — what the deck EDITOR renders.
   *
   * Marie never sees these; the editor must, or a hidden question is one nobody
   * can put back. Carried on the same view object so the two lists cannot drift
   * apart in anything but that one step.
   */
  all: PracticeQuestion[];
}


/**
 * Hold the start screen's row state: what she has set aside today, and how many
 * questions she wants to be asked.
 *
 * ## What left on 2026-09-17
 *
 * This hook used to take `setDeck` and write a flag — the write returned the
 * stored note, and the one question was patched in place so the screen showed
 * what the database had rather than what she typed. The flag's writer retired
 * (CC_TASK_DEFECT_SWEEP_v1 defect 6): its control was never built, so `saveFlag`
 * and its two state values were returned by this hook and read by no component
 * for the whole of their life. With the write gone there is nothing for this
 * hook to patch, so `setDeck` went with it.
 */
export function usePracticeDeckControls(): PracticeDeckControls {
  const [skippedToday, setSkippedToday] = React.useState<ReadonlySet<string>>(
    () => new Set<string>(),
  );
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

  const view = (questions: PracticeQuestion[], who: DeckSide): DeckView => {
    const available = availableDeck(questions, who, skippedToday);
    return {
      ordered: orderedDeck(questions, who),
      available,
      count: Math.min(count, available.length),
      all: editorDeck(questions, who),
    };
  };

  return {
    skippedToday,
    toggleSkip,
    count,
    setCount,
    view,
  };
}
