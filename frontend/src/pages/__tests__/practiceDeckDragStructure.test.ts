/**
 * What the practice deck's drag IS, read off the source.
 *
 * Every assertion in `deckSortable.test.ts` is satisfiable by helpers nothing
 * calls, and every assertion about dnd-kit needs a DOM this repo has no harness
 * for (CLAUDE.md Rule 30). This is the available fence, and it is the one that
 * would catch the mistake actually worth catching: a later edit that re-adds a
 * `draggable` row, or that widens the two sortable contexts back into one.
 *
 * It reads the SOURCE and asserts what the files DECLARE. It cannot prove a drag
 * works on an iPad — nothing here can, and the report says so — it proves the
 * component is wired the way the ruling says.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const SRC = join(__dirname, "..", "..");
const read = (...parts: string[]) => readFileSync(join(SRC, ...parts), "utf8");

/**
 * Source with its comments removed.
 *
 * ⚑ Required before any scan of this repository, and this test cost the lesson
 * again: this codebase documents its rules NEXT TO its rules, so
 * `PracticeDeckRow` carries a doc comment saying "before dnd-kit the whole row
 * carried `draggable`" — and the first version of the assertion below read that
 * sentence and reported the row as still draggable. The Rust side states the
 * same rule once, above `domain::wording_tests::seeded_value_in`, and
 * `dto::practice_wording_reach_tests::without_comments` is its implementation.
 *
 * Line comments, block comments and JSX block comments all go, because all three
 * are used in these files.
 */
const withoutComments = (source: string): string =>
  source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => {
      const at = line.indexOf("//");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");

const list = withoutComments(read("components", "practice", "PracticeDeckList.tsx"));
const row = withoutComments(read("components", "practice", "PracticeDeckRow.tsx"));
const grip = withoutComments(read("components", "dragReorder.tsx"));
const sortable = withoutComments(read("components", "practice", "deckSortable.ts"));
const gap = withoutComments(read("components", "practice", "PracticeDeckGapAdd.tsx"));

describe("the deck list drives dnd-kit", () => {
  it("wraps the runs in one DndContext, driven by the deck-drag hook", () => {
    expect(list).toContain("<DndContext");
    expect(list).toContain("useDeckDrag(");
    // Every prop the context needs comes from that one hook, so there is one
    // place a sensor or a handler can go missing rather than five.
    for (const prop of ["sensors={sensors}", "onDragStart={onDragStart}", "onDragEnd={onDragEnd}"]) {
      expect(list, `${prop} is not wired`).toContain(prop);
    }
  });

  /**
   * All three drag paths exist: mouse, touch, keyboard.
   *
   * In `deckSortable`, where the hook is. Touch is the one this task exists for —
   * Chuck edits on an iPad and native HTML5 drag gave him nothing there — so a
   * missing `TouchSensor` would be the feature silently absent on the device it
   * was built for.
   */
  it("offers all three sensors", () => {
    for (const sensor of ["PointerSensor", "TouchSensor", "KeyboardSensor"]) {
      expect(sortable, `${sensor} is missing — a drag path is gone`).toContain(sensor);
    }
    // The keyboard sensor without this moves the row eight pixels, not one row.
    expect(sortable).toContain("sortableKeyboardCoordinates");
    // And the sensors are memoised: a fresh array mid-gesture abandons the drag.
    expect(sortable).toContain("useSensors(");
  });

  /**
   * ONE SortableContext PER RUN — Roman's ruling of 2026-09-10.
   *
   * Chuck's side is drawn in two runs partitioned by `kind`, so the displayed
   * order is not the stored order across the boundary between them. A single
   * context spanning the side would let a direct be dragged among the redirects,
   * slide, drop — and jump straight back when the list re-grouped it, because the
   * drag changed the row's position and not its `kind`. The context must
   * therefore be inside the `sections.map`, not around it.
   */
  it("opens a SortableContext INSIDE each run, not around the whole side", () => {
    const runs = list.indexOf("{sections.map((part) => (");
    const context = list.indexOf("<SortableContext");
    expect(runs).toBeGreaterThan(-1);
    expect(context).toBeGreaterThan(-1);
    expect(
      context,
      "one SortableContext around both runs would let a row be dragged between " +
        "them and then snap back — the ruling is one context per run",
    ).toBeGreaterThan(runs);
    // And its items are the RUN's questions, not the whole side's.
    expect(list).toContain("items={part.questions.map((q) => q.id)}");
  });

  it("sends the server a NEIGHBOUR, through the same reorder route", () => {
    expect(sortable).toContain("deckDrop(");
    expect(list).toContain("editor.reorder");
  });

  it("announces the lifted row by its own words", () => {
    expect(list).toContain('aria-live="polite"');
    expect(sortable).toContain("LIFTED_NOTICE");
  });
});

describe("the row is sortable and the GRIP is the handle", () => {
  it("uses useSortable and the sliding transform", () => {
    expect(row).toContain("useSortable(");
    // Without the transform the other rows do not part, which is the whole
    // difference between this and the HTML5 drag it replaced.
    expect(row).toContain("CSS.Transform.toString");
  });

  /**
   * The listeners are on the GRIP, and the row is not draggable.
   *
   * The specific regression: before tonight the whole row carried `draggable`, so
   * a drag could start on the question text — which on a touch screen made the
   * row nearly unusable, because every attempt to select or scroll picked it up.
   * If `draggable` ever comes back, or the listeners are spread on the row
   * instead of the handle, this is what says so.
   */
  it("passes the drag listeners to the handle and leaves the row alone", () => {
    expect(row).toContain("handleProps=");
    expect(
      row.includes("draggable"),
      "the row must not be draggable — the grip is the only handle",
    ).toBe(false);
    // And the listeners are not ALSO spread on the row, which would make the
    // whole row a handle again by a different route.
    expect(row).not.toContain("{...listeners}");
  });

  it("still offers the arrows, which are the simple keyboard path", () => {
    expect(row).toContain('editor.move(question.id, "up")');
    expect(row).toContain('editor.move(question.id, "down")');
  });
});

describe("the retired HTML5 mechanics are gone", () => {
  /**
   * `reorderProps` and `useDropTarget` existed only to wrap native drag events.
   *
   * The file itself survives, because `dropPosition` and `DragHandle` are both
   * still used and both are about what a drag MEANS rather than how a browser
   * reports one (Roman's ruling, 2026-09-10). What must not survive is a second
   * drag mechanism sitting beside the first.
   */
  it("no longer exports the native-drag wrappers", () => {
    expect(grip).not.toContain("export function reorderProps");
    expect(grip).not.toContain("export function useDropTarget");
  });

  it("keeps the two pieces that outlived the mechanism", () => {
    expect(grip).toContain("export function dropPosition");
    expect(grip).toContain("export const DragHandle");
  });

  it("and nothing imports the retired wrappers", () => {
    for (const source of [list, row, sortable, gap]) {
      expect(source).not.toContain("reorderProps");
      expect(source).not.toContain("useDropTarget");
    }
  });
});

describe("adding a question where you are", () => {
  it("offers a gap control that opens the SAME add form", () => {
    expect(list).toContain("<PracticeDeckGapAdd");
    expect(gap).toContain("ADD_HERE_LABEL");
    // The same component the bottom box opens — a second form with its own
    // fields would be a second place the tactic and redirect rules could drift.
    expect(gap).toContain("<PracticeAddQuestion");
    expect(list).toContain("<PracticeAddQuestion");
  });

  /**
   * The top gap sends `at_start`, never `after: null`.
   *
   * `after: null` is what the bottom box sends and the server reads it as the END
   * of the side. If the top gap sent it too, pressing the topmost line would add
   * the question at the bottom of the deck — the one place the person pressing it
   * did not point at.
   */
  it("distinguishes the top of the side from the end of it", () => {
    expect(gap).toContain("at_start: anchor === null");
    expect(list).toContain("gapAnchor(");
  });

  it("keeps the bottom box", () => {
    expect(list).toContain('w("editor_add_label")');
  });
});
