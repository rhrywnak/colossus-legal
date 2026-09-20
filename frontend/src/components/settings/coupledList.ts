// =============================================================================
// coupledList — editing a set of index-aligned lists as one table, with no React
// =============================================================================
//
// A coupled group is several stored rows that are read in step: the third
// login's display name is the third name. The editor shows them as a table —
// one row per entry, one column per stored row — and every operation below
// keeps the columns the same length BY CONSTRUCTION rather than by checking.
//
// ## Why that phrasing matters
//
// The defect this replaces was a length check that could not be satisfied: two
// lists that had to match, saved one at a time, so growing them was impossible
// in either order. The answer is not a better check. It is a shape in which
// "one column is longer" cannot be expressed — add a row and every column gains
// a cell, remove one and every column loses a cell.
//
// Nothing here knows what a reviewer is. The columns, their labels and their
// placeholders all arrive from the server.

import type { CoupledGroupDto, SettingDto } from "../../services/settings";

/** One entry: exactly one cell per column, in the group's column order. */
export type Entry = string[];

/** A blank entry of the right width for this group. */
export function blankEntry(group: CoupledGroupDto): Entry {
  return group.columns.map(() => "");
}

/** Append a blank entry. Every column grows by one. */
export function addEntry(group: CoupledGroupDto, entries: Entry[]): Entry[] {
  return [...entries, blankEntry(group)];
}

/**
 * Drop one entry. Every column shrinks by one.
 *
 * An out-of-range index returns the list unchanged rather than throwing: the
 * only way to reach one is a stale click during a re-render, and dropping the
 * wrong row would be worse than dropping none.
 */
export function removeEntry(entries: Entry[], at: number): Entry[] {
  if (at < 0 || at >= entries.length) return entries;
  return entries.filter((_, index) => index !== at);
}

/** Set one cell. Returns a new table; the shape cannot change. */
export function editCell(
  entries: Entry[],
  at: number,
  column: number,
  value: string,
): Entry[] {
  if (at < 0 || at >= entries.length) return entries;
  return entries.map((entry, index) =>
    index === at ? entry.map((cell, c) => (c === column ? value : cell)) : entry,
  );
}

/**
 * What the editor can say about the table BEFORE it is submitted.
 *
 * ## Why the browser checks at all, when the backend checks properly
 *
 * It does not re-implement the rules — the backend refuses the same things with
 * better sentences, and its refusal is rendered verbatim when it comes. This
 * exists only to disable the Save button while the table is obviously not
 * submittable, so the operator is not invited to press a control that cannot
 * work. Every rule here is also enforced on the backend; none is enforced ONLY
 * here.
 */
export function whyNotSubmittable(
  group: CoupledGroupDto,
  entries: Entry[],
): string | null {
  if (entries.length === 0) {
    return `Add at least one ${group.entry_noun}.`;
  }
  for (let at = 0; at < entries.length; at += 1) {
    for (let c = 0; c < group.columns.length; c += 1) {
      if (entries[at][c].trim() === "") {
        return `${group.columns[c].label} is empty on ${group.entry_noun} ${at + 1}.`;
      }
    }
  }
  return null;
}

/** Has the table moved from what the server sent? */
export function isDirty(original: Entry[], entries: Entry[]): boolean {
  if (original.length !== entries.length) return true;
  return entries.some((entry, at) =>
    entry.some((cell, c) => cell.trim() !== (original[at]?.[c] ?? "").trim()),
  );
}

/**
 * The coupled groups whose rows live in this block.
 *
 * Worked out from the rows the server placed, never from a list of keys: a page
 * holding its own idea of which group belongs where would be the second copy of
 * the grouping this design exists to avoid.
 */
export function groupsInBlock(
  settings: readonly SettingDto[],
  groups: readonly CoupledGroupDto[],
  blockId: string,
): CoupledGroupDto[] {
  return groups.filter((group) =>
    settings.some(
      (setting) => setting.group_id === group.id && setting.block_id === blockId,
    ),
  );
}

/**
 * The rows of a block that are still edited on their own.
 *
 * A coupled row is rendered by its group's editor, so listing it here too would
 * put two controls for one value on one screen — and one of them would be the
 * single-row field whose save the backend now refuses.
 */
export function uncoupledIn(
  settings: readonly SettingDto[],
  blockId: string,
): SettingDto[] {
  return settings.filter(
    (setting) => setting.block_id === blockId && setting.group_id === null,
  );
}
