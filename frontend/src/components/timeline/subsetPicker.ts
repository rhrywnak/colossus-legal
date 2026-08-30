// =============================================================================
// subsetPicker.ts — every decision the subset picker makes, as pure functions
// =============================================================================
//
// The sibling of `timelineFilters.ts`, written for the same reason and in the
// same shape: this project has no component-testing tier, so anything decided
// inside a component is decided where no test can reach it. What is picked, what
// order the story runs in, how many of its events are gaps, and exactly what
// goes on the wire are all decided here, over plain data, where `vitest` can see
// them. The components above become arrangement.
//
// ## ⚑ A PICK IS AN EVENT ID, NOT AN EVENT
//
// Every function below keys on `event_id` and never holds an event. That is not
// tidiness — it is what lets a GAP survive an edit. A subset may reference an
// event that has since been soft-deleted on the chronology; the picker cannot
// list it, because the picker lists what exists. If picks were derived from the
// visible list, opening Edit and pressing Save would silently drop every gap the
// story had — shortening a story somebody counted, with nothing to see and
// nothing in the log. Picks are their own state, seeded from the subset, and a
// gap rides through a save untouched.
//
// ## Order: date by default, manual allowed
//
// Ruling 2026-08-30 (1). There is no "mode" flag, because a flag would be a
// second source of truth about the same list. `picks` is simply an ordered
// array: a newly ticked event is INSERTED where date order puts it, and a manual
// move splices it elsewhere. Both renumber 1..N afterwards, so the numbers on
// screen and the positions on the wire are the same numbers.

import type { ChronologyWording, TimelineEvent } from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import type { SubsetDetail, SubsetEventRef } from "../../services/caseTimelineSubsets";

/**
 * One picked event, as the modal holds it while somebody is working.
 *
 * No `position` field: position IS the array index, +1, and storing it twice is
 * how the number on screen and the number on the wire drift apart. `positionsOf`
 * is the one place the index becomes a number.
 */
export type Pick = {
  event_id: string;
  /** The author's one line. Held untrimmed while typing; trimmed on the wire. */
  note: string;
};

/**
 * How many events make a story a person can hold (design §5D).
 *
 * CONST: a design ruling, not a setting. There is no frontend config surface for
 * it and it is not per-deployment or per-case — §5D states twelve to twenty as
 * the size of a story a reader can hold, and the page OBSERVES that rather than
 * enforcing it (`sizeLine` returns a sentence, never a refusal).
 *
 * ⚑ It is coupled to `subsets_size_line_template`, whose stored value spells the
 * same range in words ("12–20 events"). Editing that row to say a different
 * range without changing this number would leave the sentence and the threshold
 * disagreeing — the sentence would appear at the wrong count. Deriving the
 * number from the store would need a numeric wording field, which is backend
 * work; recorded in the T2 report rather than half-done here.
 */
export const COMFORTABLE_MAX = 20;

/**
 * The picks a subset starts an edit with, in the order it stored them.
 *
 * `null` is the ADD case and yields an empty story. The detail's `events` are
 * already ordered by position, so this preserves the author's story order
 * rather than re-deriving it from dates — re-deriving would silently undo every
 * manual move the subset was saved with.
 */
export function initialPicks(detail: SubsetDetail | null): Pick[] {
  if (detail === null) return [];
  return detail.events.map((e) => ({
    event_id: e.event.id,
    note: e.subset_note,
  }));
}

/** True when this event is in the story. */
export function isPicked(picks: Pick[], eventId: string): boolean {
  return picks.some((p) => p.event_id === eventId);
}

/**
 * The 1-based story number of one event, or null when it is not picked.
 *
 * The picker draws this in the `ord` column, and draws nothing for an unpicked
 * row — which is why the miss is `null` and not `0`.
 */
export function positionOf(picks: Pick[], eventId: string): number | null {
  const index = picks.findIndex((p) => p.event_id === eventId);
  return index === -1 ? null : index + 1;
}

/**
 * Tick or untick one event.
 *
 * Ticking INSERTS at the place date order puts it, using the timeline's own
 * ordering (`orderedIds`, which arrives already sorted by `(event_date, id)` —
 * this module does not hold a second opinion about chronology). Unticking drops
 * it. Either way the caller gets a dense list; `positionsOf` renumbers 1..N
 * from the array, so the renumber cannot be forgotten.
 *
 * An event the timeline does not list — a gap — sorts to the END rather than
 * being refused, because a caller CAN legitimately re-tick one: the same subset
 * may hold it already. It is never offered a checkbox by the UI, so this is a
 * guard, not a path.
 */
export function togglePick(picks: Pick[], eventId: string, orderedIds: string[]): Pick[] {
  if (isPicked(picks, eventId)) {
    return picks.filter((p) => p.event_id !== eventId);
  }
  const rank = (id: string): number => {
    const at = orderedIds.indexOf(id);
    return at === -1 ? Number.MAX_SAFE_INTEGER : at;
  };
  const mine = rank(eventId);
  // The first pick that sorts AFTER this one is where this one belongs. None
  // means the end of the story.
  const at = picks.findIndex((p) => rank(p.event_id) > mine);
  const next = [...picks];
  next.splice(at === -1 ? next.length : at, 0, { event_id: eventId, note: "" });
  return next;
}

/**
 * Move one picked event one step earlier or later in the story.
 *
 * The manual half of ruling 2026-08-30 (1). A move at either end is a no-op
 * returning the SAME array contents rather than an error: the caller is a button
 * the reader can press twice, and refusing loudly would be a dialog about
 * nothing.
 */
export function movePick(picks: Pick[], eventId: string, delta: -1 | 1): Pick[] {
  const from = picks.findIndex((p) => p.event_id === eventId);
  if (from === -1) return picks;
  const to = from + delta;
  if (to < 0 || to >= picks.length) return picks;
  const next = [...picks];
  const [moved] = next.splice(from, 1);
  next.splice(to, 0, moved);
  return next;
}

/** Set one pick's note, leaving every other pick and the order alone. */
export function setPickNote(picks: Pick[], eventId: string, note: string): Pick[] {
  return picks.map((p) => (p.event_id === eventId ? { ...p, note } : p));
}

/**
 * How many picked events are gaps — references whose event is gone from the
 * chronology (design R1).
 *
 * Counted from the subset's own `removed` flags rather than from "is it in the
 * visible list", because the visible list is also narrowed by the page's
 * filters, and a filtered-out event is not a gap. Conflating the two would
 * report a story as broken because somebody typed in the search box.
 */
export function gapCount(picks: Pick[], removedIds: ReadonlySet<string>): number {
  return picks.filter((p) => removedIds.has(p.event_id)).length;
}

/** The ids of a subset's removed events, for `gapCount` and the marked rows. */
export function removedIdsOf(detail: SubsetDetail | null): Set<string> {
  if (detail === null) return new Set();
  return new Set(detail.events.filter((e) => e.removed).map((e) => e.event.id));
}

/**
 * How many of one phase's events are in the story.
 *
 * The picker's phase headers say "N events · M picked", and M is this. Takes the
 * phase's events rather than a phase id so the caller passes the group it is
 * already rendering — one traversal, and no second grouping opinion.
 */
export function pickedInPhase(picks: Pick[], phaseEvents: TimelineEvent[]): number {
  return phaseEvents.filter((e) => isPicked(picks, e.id)).length;
}

/**
 * What goes on the wire: the COMPLETE ordered set, positions 1..N.
 *
 * ## ⚑ Three rules the server also enforces, met here so it never has to refuse
 *
 * 1. Positions are 1-based and DENSE. They come from the array index, so an
 *    untick cannot leave a hole and a move cannot leave a duplicate.
 * 2. Notes are TRIMMED. A note of spaces is a note somebody cleared.
 * 3. An empty note is OMITTED, not sent as `""`. Absent and blank are the same
 *    fact and the wire says it once — which is also what stops a save from
 *    rewriting every untouched note to the empty string.
 */
export function toSubsetPayload(picks: Pick[]): SubsetEventRef[] {
  return picks.map((p, index) => {
    const note = p.note.trim();
    const ref: SubsetEventRef = { event_id: p.event_id, position: index + 1 };
    if (note !== "") ref.note = note;
    return ref;
  });
}

/**
 * The size line, when there is one to say.
 *
 * `null` below the comfortable maximum, which is what makes this a SENTENCE and
 * not a block (design §5D): the page observes that a long story is long, once,
 * and does not enforce anything. The caller renders nothing for `null`.
 */
export function sizeLine(wording: ChronologyWording, picked: number): string | null {
  if (picked <= COMFORTABLE_MAX) return null;
  return fill(cw(wording, "subsets_size_line_template"), { count: picked });
}

/** The amber gap line under a subset's count — "3 gaps". */
export function gapLine(wording: ChronologyWording, gaps: number): string {
  return fill(cw(wording, "subsets_gap_count_template"), { count: gaps });
}

/**
 * The picker pill's gap clause — "3 are gaps" — or nothing at all.
 *
 * `null` at zero, which is the whole reason this is its own stored row rather
 * than half of the picked template: a pill reading "15 picked · 0 are gaps"
 * reports an absence as if it were news. The caller drops the separator with it.
 *
 * Extracted for the same reason [`sizeLine`] is: it is a design ruling with a
 * boundary, and a ruling decided inside a component is decided where no test can
 * reach it.
 */
export function pillGapsLine(wording: ChronologyWording, gaps: number): string | null {
  if (gaps === 0) return null;
  return fill(cw(wording, "subsets_pill_gaps_template"), { count: gaps });
}
