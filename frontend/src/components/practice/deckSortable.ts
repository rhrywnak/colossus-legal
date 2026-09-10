// =============================================================================
// deckSortable.ts — the deck drag's two decisions, without a DOM
// =============================================================================
//
// The dnd-kit wiring itself lives in `PracticeDeckList` and `PracticeDeckRow`,
// where the elements are. What is here is the part that can be WRONG in a way a
// person notices, extracted so it can be tested without rendering anything —
// RTL and jsdom are not set up in this repo (CLAUDE.md Rule 30), so a helper
// that needs a browser is a helper that ships unproved.
//
// Two decisions:
//
//   1. WHEN a drag begins. Three sensors, three different sets of numbers, and
//      every one of them is the difference between a control that works and one
//      that fights you. See `SENSOR_CONSTRAINTS`.
//   2. WHERE a drop lands. dnd-kit reports `{active, over}` — two ids; the
//      server wants `{before}` — the row the dragged one lands on top of. The
//      conversion is `dropPosition`, which already exists and is already tested,
//      so this only adapts the shapes.
//
// ## Why dnd-kit at all (Law 4 — the pattern is the standard one)
//
// Native HTML5 drag-and-drop, which this replaces, has no press-and-hold, no
// sliding rows, and touch support that browsers implement inconsistently or not
// at all. Chuck edits Marie's decks on an iPad. The four sources the architect
// cited all describe the same replacement: NN/g's drag-and-drop guidance (a drag
// needs a visible target and a way to see where it will land), Apple's HIG on
// list reordering (press-and-hold, then the rows part), GitHub Primer's
// drag-and-drop pattern (a dedicated grip, and a keyboard path that is not the
// drag), and dnd-kit's own Sortable preset, which is the implementation of all
// three.

import React from "react";

import {
  KeyboardSensor,
  PointerSensor,
  TouchSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import { sortableKeyboardCoordinates } from "@dnd-kit/sortable";

import type { PracticeQuestion } from "../../services/practice";
import type { DeckSection } from "../../pages/practiceQueue";
import { dropPosition } from "../dragReorder";

// The sentence a screen reader hears while a row is in hand. A LITERAL, and OWED
// as a settings row — a migration is Roman's to author (CLAUDE.md rule 25) and
// this task did not authorise one. Deliberately not routed through `w()`:
// `dto::practice_wording_reach_tests` scans the practice sources for a literal
// `w("key")` and requires each to be a field on the backend's mirror, so a key
// with no row behind it would red the suite rather than degrade.
//
// dnd-kit ships its own announcements and they are generic — "Draggable item 3
// was moved". On this screen the rows are SENTENCES, so an ordinal identifies
// nothing; this names the question.
//
// OWED: settings row — practice_editor_lifted_notice
//   = "{question} lifted. Use the arrow keys to move it, then press space to drop it."
const LIFTED_NOTICE = (text: string) =>
  `${text} lifted. Use the arrow keys to move it, then press space to drop it.`;

/**
 * The numbers that decide when a press becomes a drag.
 *
 * Exported as data rather than written inline at the sensor, because they are
 * the feature — a test can assert them, and a person changing one can see all
 * three at once.
 */
// STRUCTURAL: INTERACTION PHYSICS, not deployment tuning. These three describe
// how a hand behaves, not how this installation is configured: the distance a
// finger or a mouse moves before a person means to have moved it, and how long a
// deliberate press lasts. They are the same on Chuck's iPad, on Roman's Mac and
// on a machine nobody has built yet, and all four cited patterns converge on
// them — Apple's HIG, NN/g's drag guidance, GitHub Primer, and dnd-kit's own
// Sortable defaults.
//
// Rule 2 lists thresholds as config, and the judgment that these are the
// exception is recorded here rather than assumed. What would make them
// configurable is an ACCESSIBILITY need — a longer hold for an unsteady hand —
// and that is a per-PERSON setting, not a per-deployment one: it belongs with the
// other accessibility preferences whenever those exist, not in the settings
// store, and it is named here so that day starts from a decision rather than a
// discovery.
export const SENSOR_CONSTRAINTS = {
  /**
   * Mouse: 5 px of travel before a press becomes a drag.
   *
   * Without a distance constraint, `PointerSensor` claims the pointer on
   * mousedown and the row's own controls stop working — every click on the grip
   * would begin a drag that goes nowhere. Five pixels is below the threshold at
   * which a person perceives movement and above the jitter of a click, which is
   * the range every one of the cited patterns lands in.
   */
  pointerDistancePx: 5,
  /**
   * Touch: hold for 250 ms, and tolerate 5 px of wander while holding.
   *
   * The delay is what makes a SCROLL a scroll. A finger that starts moving
   * immediately is scrolling the deck; a finger that stays put for a quarter of a
   * second and then moves is picking a row up. Without the delay, every attempt
   * to scroll the list would lift whichever row was under the thumb — which is
   * the iPad failure this task exists to fix, not a refinement of it.
   *
   * The tolerance is the other half: a finger is never perfectly still, and a
   * delay with zero tolerance cancels on the tremor of holding a tablet.
   */
  touchDelayMs: 250,
  touchTolerancePx: 5,
} as const;

/** What one `onDragEnd` means for the server, or `null` for "nothing to do". */
export type DeckDrop = { dragged: string; before: string | null };

/**
 * Turn dnd-kit's `{active, over}` into the server's `{dragged, before}`.
 *
 * `rows` is the list the drag ran over, in the order it is DRAWN — which for
 * Chuck's side is one run, not the whole side (see `PracticeDeckList` for why
 * each run is its own sortable context).
 *
 * `null` means the gesture named no position and nothing should be sent: a drop
 * outside any row (`over` is null), a drop onto itself, or a drop onto a row this
 * list does not hold. The server answers all three with a 200 that writes
 * nothing, so sending them would be a round trip for a shrug.
 *
 * ## Why the arithmetic is `dropPosition` and not dnd-kit's `arrayMove`
 *
 * dnd-kit's own helper returns the re-ordered ARRAY, which would make the
 * browser the authority on the deck's order. It is not: the order lives in
 * `sort_order`, the server owns it, and the browser names a NEIGHBOUR — "put it
 * above that one" — for the server to resolve against what is actually stored.
 * A tab left open across somebody else's edit would otherwise write back an
 * order computed from a deck that has moved on. `dropPosition` is the same
 * function the HTML5 drag used, with the same six tests, so the meaning of a drop
 * did not change when the mechanism did.
 */
export function deckDrop<T>(
  rows: T[],
  getId: (row: T) => string,
  activeId: string,
  overId: string | null,
): DeckDrop | null {
  if (overId === null) return null;
  const landing = dropPosition(rows, getId, activeId, overId);
  if (landing === null) return null;
  return { dragged: activeId, before: landing.before };
}

/**
 * Where the "+ Add question here" gap above `index` puts a new question.
 *
 * The gap control names the row ABOVE it, because that is the anchor that
 * survives a re-render — the row below may be the one just added. The gap above
 * the FIRST row has no row above it, which is `null`: the server reads that as
 * "the start of this side".
 *
 * Pure and separate from the component for one reason: the off-by-one here is
 * invisible on screen until somebody adds a question and it appears one row from
 * where they pressed.
 */
export function gapAnchor<T>(
  rows: T[],
  getId: (row: T) => string,
  index: number,
): string | null {
  if (index <= 0) return null;
  const above = rows[index - 1];
  return above === undefined ? null : getId(above);
}

/** What a deck list needs to hand `DndContext` and its live region. */
export interface DeckDragApi {
  sensors: ReturnType<typeof useSensors>;
  /** The announcement for the row in hand, or `""` when nothing is lifted. */
  lifted: string;
  onDragStart: (event: DragStartEvent) => void;
  onDragEnd: (event: DragEndEvent) => void;
  onDragCancel: () => void;
}

/**
 * The deck drag: three sensors, one announcement, and what a drop means.
 *
 * Lifted out of `PracticeDeckList` so that component reads as the list it is —
 * and because everything here is about the GESTURE rather than about the rows,
 * which is the same seam the two pure helpers above sit on.
 *
 * `sections` is the runs as drawn; `reorder` is the editor's write. The hook
 * looks the run up from the DRAGGED id, so one handler serves both of Chuck's
 * runs without either of them knowing about the other.
 *
 * ## React Learning: why `useSensors` and not a plain array
 *
 * `useSensors` memoises the list. A fresh sensor array on every render would
 * hand `DndContext` new sensors mid-gesture, and the drag in progress would be
 * abandoned — the row would drop back where it started, with nothing on screen
 * saying why. The same is true of `useSensor` for each one, which is why they
 * are called at the top level rather than built inside the array literal.
 */
export function useDeckDrag(
  sections: DeckSection[],
  reorder: (questionId: string, before: string | null) => void,
): DeckDragApi {
  const [lifted, setLifted] = React.useState<string | null>(null);

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: SENSOR_CONSTRAINTS.pointerDistancePx },
    }),
    useSensor(TouchSensor, {
      activationConstraint: {
        delay: SENSOR_CONSTRAINTS.touchDelayMs,
        tolerance: SENSOR_CONSTRAINTS.touchTolerancePx,
      },
    }),
    // Not a replacement for the ▲▼ buttons: those move a row one step, this
    // carries it. `sortableKeyboardCoordinates` is what makes the arrow keys mean
    // "up one row" rather than "up eight pixels".
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  );

  const rowsOf = (id: string): PracticeQuestion[] | null =>
    sections.find((part) => part.questions.some((q) => q.id === id))?.questions ?? null;

  return {
    sensors,
    lifted: lifted === null ? "" : LIFTED_NOTICE(lifted),
    onDragStart: (event: DragStartEvent) => {
      const id = String(event.active.id);
      setLifted(rowsOf(id)?.find((q) => q.id === id)?.text ?? null);
    },
    onDragEnd: (event: DragEndEvent) => {
      setLifted(null);
      const dragged = String(event.active.id);
      const rows = rowsOf(dragged);
      if (rows === null) return;
      // `rows` is the RUN the drag ran over, not the whole side — each run is its
      // own sortable context (Roman's ruling, 2026-09-10; see `PracticeDeckList`
      // for the snap-back that forced it).
      const drop = deckDrop(
        rows,
        (q) => q.id,
        dragged,
        event.over ? String(event.over.id) : null,
      );
      if (drop !== null) reorder(drop.dragged, drop.before);
    },
    onDragCancel: () => setLifted(null),
  };
}
