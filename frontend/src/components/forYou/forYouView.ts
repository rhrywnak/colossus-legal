// forYouView.ts — the page's pure decisions (CC_TASK_FOR_YOU_v1 L1).
//
// Everything here is a function from data to data: no React, no fetch, no
// styles. It is where the page's two judgements live, so both can be tested
// without a browser (CLAUDE.md rule 30 — there is no component-test rig here).
//
// ## What it does NOT do
//
// It composes no sentence and fills no template. Every string a person reads
// arrives already written from the server, and the whole point of that law is
// that a component cannot quietly start writing prose of its own.

import type { ForYouDay, ForYouRow } from "../../services/forYou";

/** The three day headings, in the order the page shows them. */
// STRUCTURAL: the three members of the `ForYouDay` wire vocabulary, which the
// SERVER assigns (in the case's timezone), in the only order a newest-first
// list can read. Not a deployment value: a fourth heading would be a schema
// change and a code change made together, and a different order would be a
// list that ran backwards.
export const DAY_ORDER: ForYouDay[] = ["today", "yesterday", "earlier"];

/** One day's worth of rows, under one heading. */
export type ForYouGroup = {
  day: ForYouDay;
  rows: ForYouRow[];
};

/**
 * The rows, in the order they are shown, split under their day headings.
 *
 * The server has already ordered them newest first and told each one which day
 * it belongs to; this only gathers them. A day with no rows is DROPPED rather
 * than rendered as an empty heading — a "YESTERDAY" with nothing under it reads
 * as a list that failed to load.
 */
export function groupByDay(rows: ForYouRow[]): ForYouGroup[] {
  return DAY_ORDER.map((day) => ({
    day,
    rows: rows.filter((row) => row.day === day),
  })).filter((group) => group.rows.length > 0);
}

/**
 * What the menu badge shows, or `null` for no badge at all.
 *
 * ## Domain note: ABSENT at zero, never "0"
 *
 * A "0" beside a menu item is a thing to read and dismiss every time the page
 * loads; an absent badge is nothing to read at all. The count is also refused
 * when it is not a finite non-negative number — a badge is a claim about work
 * waiting, and a nonsense claim is worse than none.
 */
export function badgeLabel(count: number | null | undefined): string | null {
  if (typeof count !== "number" || !Number.isFinite(count) || count <= 0) {
    return null;
  }
  return String(Math.floor(count));
}
