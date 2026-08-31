// =============================================================================
// editSubsets.ts — which subsets this scenario carries, and in what order
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 4, approved as drawn, and
// design §11 item 4. The pure half of `ScenarioSubsetsSection.tsx`.
//
// ## ⚑ TWO READS, ONE LIST, AND THAT IS THE WHOLE PROBLEM
//
// The section draws EVERY subset in the case with a state on each. The case's
// subsets come from `GET /api/timeline/subsets`; which ones this scenario
// carries comes from `GET /cases/:slug/scenarios/:id/subsets`. Neither read
// knows about the other, and merging them inside a `.map()` is how a row ends up
// showing "not attached" beside a Detach button.
//
// So the merge is here, where a test can reach it.

import type { AttachedSubset } from "../scenario-timeline/scenarioTimeline";
import type { SubsetSummary } from "../../services/caseTimelineSubsets";

/** One row of the Timeline subsets section (mockup `.srow`). */
export type SubsetRow = {
  id: string;
  name: string;
  description: string;
  /** Every reference the subset holds, gaps included. */
  eventCount: number;
  /** Does THIS scenario carry it? Decides the ground, the state word and which button. */
  attached: boolean;
};

/**
 * The case's subsets, merged with this scenario's attachments, in display order.
 *
 * ## The ordering, and why it is not just alphabetical
 *
 * Attached first, in ATTACHMENT order — `position` on `scenario_subsets`, which
 * is what the scenario's author chose and the same order the window's selector
 * uses. Then everything else by name.
 *
 * Two reasons. The rows a reader can act on destructively (Detach) are the ones
 * they already own, and burying those among twenty they do not carry makes the
 * section a search task. And the attached block matching the selector's order
 * means the reader learns one order for this scenario rather than two.
 *
 * Ties in `position` fall back to name so the list cannot reorder itself between
 * renders — `Array.prototype.sort` is stable in modern engines, but two rows
 * that compare equal on a number nobody guaranteed unique is not a thing to lean
 * on when the alternative is one more comparison.
 *
 * An attachment naming a subset the case list does not carry is DROPPED, and
 * that is deliberate: it can only mean the subset was deleted between the two
 * reads, and a row with a name nobody can render is worse than one fewer row.
 * The count is reported by [`attachedCount`], which counts the merged rows, so
 * the section cannot claim more than it draws.
 */
export function subsetRows(
  all: SubsetSummary[],
  attached: AttachedSubset[],
): SubsetRow[] {
  const positions = new Map(attached.map((a) => [a.id, a.position ?? 0]));
  const rows: SubsetRow[] = all.map((s) => ({
    id: s.id,
    name: s.name,
    description: s.description,
    eventCount: s.event_count,
    attached: positions.has(s.id),
  }));
  return rows.sort((a, b) => {
    if (a.attached !== b.attached) return a.attached ? -1 : 1;
    if (a.attached && b.attached) {
      const pa = positions.get(a.id) ?? 0;
      const pb = positions.get(b.id) ?? 0;
      if (pa !== pb) return pa - pb;
    }
    return a.name.localeCompare(b.name, "en-US");
  });
}

/**
 * How many of the drawn rows this scenario carries.
 *
 * Counted off the MERGED rows rather than off the attachment list, so a stale
 * attachment dropped by [`subsetRows`] cannot make the section report a number
 * larger than the rows underneath it.
 */
export function attachedCount(rows: SubsetRow[]): number {
  return rows.filter((row) => row.attached).length;
}
