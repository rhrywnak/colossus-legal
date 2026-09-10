/**
 * The deck drag's two decisions (task DECK_DRAG_AND_ADD part 2).
 *
 * The dnd-kit wiring needs a DOM and RTL/jsdom are not set up here (CLAUDE.md
 * Rule 30), so what is proved is the part that decides WHEN a drag begins and
 * WHERE a drop lands — which is where a person notices a mistake. That the
 * component actually uses these is proved separately, by the source-reading fence
 * in `pages/__tests__/practiceDeckDragStructure.test.ts`.
 */
import { describe, expect, it } from "vitest";

import { SENSOR_CONSTRAINTS, deckDrop, gapAnchor } from "../deckSortable";

/** Rows keyed the way the practice deck keys them. */
const rows = [{ id: "a" }, { id: "b" }, { id: "c" }, { id: "d" }];
const id = (row: { id: string }) => row.id;

// ─── When a drag begins ──────────────────────────────────────────────────────

describe("SENSOR_CONSTRAINTS", () => {
  /**
   * The touch delay is the whole iPad fix, so it is pinned by value.
   *
   * Without a delay, `TouchSensor` claims the finger the moment it lands and
   * every attempt to SCROLL the deck lifts whichever row was under the thumb.
   * A quarter of a second is the threshold the cited patterns use: long enough
   * that a scroll never trips it, short enough that a deliberate press does not
   * feel like waiting.
   */
  it("holds a touch for 250 ms before lifting a row", () => {
    expect(SENSOR_CONSTRAINTS.touchDelayMs).toBe(250);
  });

  /**
   * And tolerates a wandering finger, or the delay cancels on a tremor.
   *
   * A delay with zero tolerance is not a delay — a finger holding a tablet is
   * never perfectly still, so the gesture would abort before it began.
   */
  it("tolerates 5 px of wander during that hold", () => {
    expect(SENSOR_CONSTRAINTS.touchTolerancePx).toBe(5);
    expect(SENSOR_CONSTRAINTS.touchTolerancePx).toBeGreaterThan(0);
  });

  /**
   * The mouse needs travel, not time — otherwise a click is a drag.
   *
   * `PointerSensor` claims the pointer on mousedown unless a distance is set, and
   * a claimed pointer means the grip's own click never happens. Five pixels is
   * under the threshold at which movement is perceived and over the jitter of a
   * click.
   */
  it("waits 5 px of travel before a mouse press becomes a drag", () => {
    expect(SENSOR_CONSTRAINTS.pointerDistancePx).toBe(5);
    expect(SENSOR_CONSTRAINTS.pointerDistancePx).toBeGreaterThan(0);
  });
});

// ─── Where a drop lands ──────────────────────────────────────────────────────

describe("deckDrop", () => {
  it("names the row the dragged one lands above", () => {
    // Drag d onto a: d goes above a, so `before` is a. The same answer the HTML5
    // drag gave, because it is the same function underneath — the mechanism
    // changed and the meaning did not.
    expect(deckDrop(rows, id, "d", "a")).toEqual({ dragged: "d", before: "a" });
  });

  it("names the target when dragging DOWN the list", () => {
    // The dragged row is lifted out first, so the target's index is read in what
    // remains — otherwise every downward drop lands one place too high.
    expect(deckDrop(rows, id, "a", "c")).toEqual({ dragged: "a", before: "c" });
  });

  /**
   * A drop outside any row is nothing to do, not an error.
   *
   * dnd-kit reports `over: null` when the pointer is released off the list — the
   * commonest way a person changes their mind mid-drag. Sending it would be a
   * round trip for a shrug: the server answers a position it cannot name with a
   * 200 that writes nothing.
   */
  it("sends nothing when the drop was outside every row", () => {
    expect(deckDrop(rows, id, "a", null)).toBeNull();
  });

  it("sends nothing for a drop onto itself", () => {
    expect(deckDrop(rows, id, "b", "b")).toBeNull();
  });

  /**
   * A target this run does not hold names no position.
   *
   * With one SortableContext per run (Roman's ruling), dnd-kit will not offer a
   * cross-run target — but the handler looks the run up from the DRAGGED id, so
   * this is the guard for the case where the two disagree.
   */
  it("sends nothing for a target outside this run", () => {
    expect(deckDrop(rows, id, "a", "zz")).toBeNull();
  });
});

// ─── Where a new question goes ───────────────────────────────────────────────

describe("gapAnchor", () => {
  /**
   * A gap names the row ABOVE it, because that is what survives the re-render.
   *
   * The row below the gap may be the one just added. The row above is where the
   * person pointed and is still there afterwards.
   */
  it("names the row above the gap", () => {
    expect(gapAnchor(rows, id, 1)).toBe("a");
    expect(gapAnchor(rows, id, 3)).toBe("c");
  });

  /**
   * The gap above the FIRST row has no row above it.
   *
   * `null`, which the caller turns into `at_start: true` — NOT into `after: null`,
   * which the server reads as the end of the side. That distinction is the whole
   * reason `at_start` exists: without it, pressing the topmost gap would put the
   * question at the bottom of the deck, the one place the person did not point at.
   */
  it("names nothing above the first row", () => {
    expect(gapAnchor(rows, id, 0)).toBeNull();
  });

  it("names nothing for an index off the end", () => {
    // Defensive: a run that shrank under a stale render must not read undefined.
    expect(gapAnchor(rows, id, 99)).toBeNull();
    expect(gapAnchor([], id, 0)).toBeNull();
  });
});
