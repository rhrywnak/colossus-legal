// =============================================================================
// THE SHAPE KEEPS THE COLUMNS IN STEP — that is the whole fix
// =============================================================================
//
// The defect: two stored lists that had to be the same length, edited one at a
// time, so growing them was impossible in either order and the feature's own
// purpose could not be performed.
//
// The fix is not a better length check. It is a table in which "one column is
// longer than another" cannot be expressed. The last test in this file is that
// claim, asserted over a sequence of edits rather than over one.

import { describe, expect, it } from "vitest";

import type { CoupledGroupDto, SettingDto } from "../../../services/settings";
import {
  addEntry,
  blankEntry,
  editCell,
  groupsInBlock,
  isDirty,
  removeEntry,
  uncoupledIn,
  whyNotSubmittable,
  type Entry,
} from "../coupledList";

const bench: CoupledGroupDto = {
  id: "reviewer_bench",
  label: "Who may press “Done reviewing”",
  note: "Two lists read in step.",
  entry_noun: "reviewer",
  columns: [
    { key: "practice_reviewer_usernames", label: "Sign-in name", placeholder: "cpenzien" },
    { key: "practice_reviewer_display_names", label: "Name shown on screen", placeholder: "Chuck" },
  ],
  entries: [["cpenzien", "Chuck"]],
};

function setting(over: Partial<SettingDto> & { key: string }): SettingDto {
  return {
    value: "x",
    default_value: "x",
    meaning: "What this does.",
    input_hint: "free text",
    bounds_label: null,
    dormant_note: null,
    last_changed: "Never changed since it shipped",
    area_id: "practice",
    block_id: "practice_params",
    changed_from_default: null,
    group_id: null,
    ...over,
  };
}

/** Every column has the same number of cells. */
const columnLengths = (entries: Entry[], group: CoupledGroupDto) =>
  group.columns.map((_, c) => entries.filter((entry) => entry[c] !== undefined).length);

describe("the table's shape", () => {
  it("adds a cell to EVERY column at once (M)", () => {
    // MUTATION: push a one-cell entry and this reds — which is the deadlock in
    // miniature, one column growing without the other.
    const grown = addEntry(bench, bench.entries);
    expect(grown.length).toBe(2);
    expect(grown[1]).toEqual(["", ""]);
    expect(columnLengths(grown, bench)).toEqual([2, 2]);
  });

  it("removes a cell from every column at once (M)", () => {
    const two = addEntry(bench, bench.entries);
    const back = removeEntry(two, 0);
    expect(back.length).toBe(1);
    expect(columnLengths(back, bench)).toEqual([1, 1]);
  });

  it("ignores a removal that is out of range rather than dropping the wrong row", () => {
    expect(removeEntry(bench.entries, 7)).toEqual(bench.entries);
    expect(removeEntry(bench.entries, -1)).toEqual(bench.entries);
  });

  it("edits exactly one cell and leaves the rest alone (M)", () => {
    const two = [["cpenzien", "Chuck"], ["roman", "Roman"]];
    const edited = editCell(two, 1, 1, "R.");
    expect(edited).toEqual([["cpenzien", "Chuck"], ["roman", "R."]]);
    // The source table is untouched — the editor holds it in React state.
    expect(two[1][1]).toBe("Roman");
  });

  it("gives a blank entry exactly one cell per column", () => {
    expect(blankEntry(bench)).toEqual(["", ""]);
  });
});

describe("what the editor can say before submitting", () => {
  it("refuses an empty table, naming what to add (M)", () => {
    expect(whyNotSubmittable(bench, [])).toBe("Add at least one reviewer.");
  });

  it("names the column and the 1-based row of a blank cell (M)", () => {
    const table = [["cpenzien", "Chuck"], ["roman", "  "]];
    expect(whyNotSubmittable(bench, table)).toBe(
      "Name shown on screen is empty on reviewer 2.",
    );
  });

  it("says nothing about a table that is ready", () => {
    expect(whyNotSubmittable(bench, [["roman", "Roman"]])).toBeNull();
  });
});

describe("isDirty", () => {
  it("is false for the table as it arrived, and true once anything moves", () => {
    expect(isDirty(bench.entries, [["cpenzien", "Chuck"]])).toBe(false);
    // Whitespace alone is not a change — the backend trims what it is sent.
    expect(isDirty(bench.entries, [["  cpenzien  ", "Chuck"]])).toBe(false);
    expect(isDirty(bench.entries, [["cpenzien", "Charles"]])).toBe(true);
    expect(isDirty(bench.entries, addEntry(bench, bench.entries))).toBe(true);
    expect(isDirty(bench.entries, [])).toBe(true);
  });
});

describe("placing a group on the page", () => {
  it("finds a group from where the SERVER put its rows (M)", () => {
    // MUTATION: match on the group's own column keys instead of the rows'
    // `block_id` and the page grows a second copy of the grouping — the thing
    // the server-declared design exists to prevent.
    const rows = [
      setting({ key: "practice_reviewer_usernames", group_id: "reviewer_bench" }),
      setting({ key: "talking_points_cap", block_id: "core", area_id: "core" }),
    ];

    expect(groupsInBlock(rows, [bench], "practice_params").map((g) => g.id)).toEqual([
      "reviewer_bench",
    ]);
    expect(groupsInBlock(rows, [bench], "core")).toEqual([]);
  });

  it("withholds coupled rows from the block's own row list (M)", () => {
    // MUTATION: drop the `group_id === null` clause and the reviewer row renders
    // BOTH inside the editor and as a lone field beneath it — and that field's
    // save is exactly what the backend now refuses.
    const rows = [
      setting({ key: "practice_reviewer_usernames", group_id: "reviewer_bench" }),
      setting({ key: "practice_reviewer_display_names", group_id: "reviewer_bench" }),
      setting({ key: "practice_read_max_words" }),
    ];

    expect(uncoupledIn(rows, "practice_params").map((s) => s.key)).toEqual([
      "practice_read_max_words",
    ]);
  });

  it("keeps rows of other blocks out, in the order they arrived", () => {
    const rows = [
      setting({ key: "a", block_id: "practice_report" }),
      setting({ key: "b" }),
      setting({ key: "c", block_id: "practice_report" }),
    ];
    expect(uncoupledIn(rows, "practice_report").map((s) => s.key)).toEqual(["a", "c"]);
  });
});

/**
 * ⚑ No sequence of edits can put the columns out of step.
 *
 * Every other test here checks one operation. This one walks a realistic
 * session — add two, edit some cells, remove one, add another — and asserts the
 * invariant after every single step. It is the claim the whole feature rests on,
 * and it is the claim the old design could not make at all.
 */
describe("the invariant, over a whole session", () => {
  it("keeps every column the same length after every operation (M)", () => {
    let table: Entry[] = bench.entries;
    const steps: Array<() => Entry[]> = [
      () => addEntry(bench, table),
      () => editCell(table, 1, 0, "roman"),
      () => editCell(table, 1, 1, "Roman"),
      () => addEntry(bench, table),
      () => editCell(table, 2, 0, "docmarie"),
      () => removeEntry(table, 0),
      () => addEntry(bench, table),
      () => removeEntry(table, 1),
    ];

    for (const [at, step] of steps.entries()) {
      table = step();
      const lengths = columnLengths(table, bench);
      expect(
        new Set(lengths).size,
        `after step ${at + 1} the columns are ${lengths.join(" vs ")}`,
      ).toBe(1);
      // And every entry is exactly as wide as the group.
      for (const entry of table) {
        expect(entry.length).toBe(bench.columns.length);
      }
    }
  });
});
