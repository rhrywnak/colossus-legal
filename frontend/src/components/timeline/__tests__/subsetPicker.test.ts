// =============================================================================
// subsetPicker.test.ts — the picker's decisions, with specific expected values
// =============================================================================
//
// T2.3 names five behaviours these tests exist to hold: renumber after uncheck,
// date-default ordering with a manual move, gap counting from `removed` flags,
// the payload builder's positions/notes, and the size line's sentence rule.
// Every assertion below is a concrete expected value, never "it did not throw".

import { describe, expect, it } from "vitest";

import type { SubsetDetail } from "../../../services/caseTimelineSubsets";
import type { TimelineEvent } from "../../../services/caseTimeline";
import {
  COMFORTABLE_MAX,
  gapCount,
  gapLine,
  initialPicks,
  isPicked,
  movePick,
  type Pick,
  pickedInPhase,
  positionOf,
  removedIdsOf,
  setPickNote,
  sizeLine,
  toSubsetPayload,
  togglePick,
} from "../subsetPicker";

/** The timeline's own order: already sorted by (event_date, id) by the API. */
const ORDER = ["a", "b", "c", "d", "e"];

const picksOf = (...ids: string[]): Pick[] => ids.map((event_id) => ({ event_id, note: "" }));

/** A minimal event — only the fields these functions actually read. */
const event = (id: string): TimelineEvent =>
  ({ id, title: `Event ${id}` }) as unknown as TimelineEvent;

/** A subset detail carrying the given ids, with `removed` set for `gaps`. */
const detailOf = (ids: string[], gaps: string[] = [], notes: Record<string, string> = {}) =>
  ({
    events: ids.map((id) => ({
      event: event(id),
      subset_note: notes[id] ?? "",
      removed: gaps.includes(id),
    })),
  }) as unknown as SubsetDetail;

describe("initialPicks", () => {
  it("is empty for an add, and keeps the STORED order for an edit", () => {
    expect(initialPicks(null)).toEqual([]);
    // Stored out of date order on purpose: a manual move was saved, and
    // re-deriving from dates here would silently undo it.
    expect(initialPicks(detailOf(["c", "a", "b"]))).toEqual(picksOf("c", "a", "b"));
  });

  it("carries each stored note through", () => {
    expect(initialPicks(detailOf(["a"], [], { a: "never reaches Emil's account" }))).toEqual([
      { event_id: "a", note: "never reaches Emil's account" },
    ]);
  });
});

describe("togglePick — date order by default", () => {
  it("inserts a new pick where DATE order puts it, not at the end", () => {
    const picks = togglePick(picksOf("a", "c"), "b", ORDER);
    expect(picks.map((p) => p.event_id)).toEqual(["a", "b", "c"]);
  });

  it("appends when the new pick is latest, and prepends when it is earliest", () => {
    expect(togglePick(picksOf("a", "b"), "d", ORDER).map((p) => p.event_id)).toEqual([
      "a",
      "b",
      "d",
    ]);
    expect(togglePick(picksOf("c", "d"), "a", ORDER).map((p) => p.event_id)).toEqual([
      "a",
      "c",
      "d",
    ]);
  });

  it("unticking removes it and the REST RENUMBER — the T2.3 case", () => {
    const after = togglePick(picksOf("a", "b", "c"), "b", ORDER);
    expect(after.map((p) => p.event_id)).toEqual(["a", "c"]);
    // The renumber is the point: c was 3 and must now be 2, with no hole at 2.
    expect(positionOf(after, "a")).toBe(1);
    expect(positionOf(after, "c")).toBe(2);
    expect(toSubsetPayload(after).map((r) => r.position)).toEqual([1, 2]);
  });

  it("keeps a ticked note when a DIFFERENT event is unticked", () => {
    const with_note = setPickNote(picksOf("a", "b", "c"), "c", "the handoff");
    const after = togglePick(with_note, "b", ORDER);
    expect(after).toEqual([
      { event_id: "a", note: "" },
      { event_id: "c", note: "the handoff" },
    ]);
  });

  it("sorts an event the timeline does not list to the END rather than refusing", () => {
    // A gap: referenced by the subset, absent from the live list.
    expect(togglePick(picksOf("a"), "zz", ORDER).map((p) => p.event_id)).toEqual(["a", "zz"]);
  });
});

describe("movePick — the manual half of the ruling", () => {
  it("moves one step later and renumbers", () => {
    const after = movePick(picksOf("a", "b", "c"), "a", 1);
    expect(after.map((p) => p.event_id)).toEqual(["b", "a", "c"]);
    expect(positionOf(after, "a")).toBe(2);
    expect(positionOf(after, "b")).toBe(1);
  });

  it("moves one step earlier", () => {
    expect(movePick(picksOf("a", "b", "c"), "c", -1).map((p) => p.event_id)).toEqual([
      "a",
      "c",
      "b",
    ]);
  });

  it("a move off either end is a no-op, not an error", () => {
    const picks = picksOf("a", "b");
    expect(movePick(picks, "a", -1).map((p) => p.event_id)).toEqual(["a", "b"]);
    expect(movePick(picks, "b", 1).map((p) => p.event_id)).toEqual(["a", "b"]);
  });

  it("a move of something not picked changes nothing", () => {
    expect(movePick(picksOf("a"), "zz", 1).map((p) => p.event_id)).toEqual(["a"]);
  });

  it("a manual move SURVIVES a later tick — date order does not reassert itself", () => {
    // b moved before a; then c is ticked. c goes after a (date order among the
    // picks it passes), and b stays where the author put it.
    const moved = movePick(picksOf("a", "b"), "b", -1);
    expect(moved.map((p) => p.event_id)).toEqual(["b", "a"]);
    const after = togglePick(moved, "c", ORDER);
    expect(after.map((p) => p.event_id)).toEqual(["b", "a", "c"]);
  });
});

describe("gapCount — from the removed flags, never from the visible list", () => {
  it("counts only picks whose event is removed", () => {
    const removed = removedIdsOf(detailOf(["a", "b", "c"], ["b", "c"]));
    expect(removed).toEqual(new Set(["b", "c"]));
    expect(gapCount(picksOf("a", "b", "c"), removed)).toBe(2);
  });

  it("is zero when nothing was removed, and for an add", () => {
    expect(gapCount(picksOf("a", "b"), removedIdsOf(detailOf(["a", "b"])))).toBe(0);
    expect(removedIdsOf(null)).toEqual(new Set());
  });

  it("does not count a removed event that is NOT picked", () => {
    const removed = removedIdsOf(detailOf(["a", "b"], ["b"]));
    expect(gapCount(picksOf("a"), removed)).toBe(0);
  });
});

describe("toSubsetPayload — positions 1..N, notes trimmed, empties omitted", () => {
  it("numbers 1..N in array order", () => {
    expect(toSubsetPayload(picksOf("c", "a", "b"))).toEqual([
      { event_id: "c", position: 1 },
      { event_id: "a", position: 2 },
      { event_id: "b", position: 3 },
    ]);
  });

  it("trims a note and OMITS one that is empty or only spaces", () => {
    let picks = setPickNote(picksOf("a", "b", "c"), "a", "  Admissions p. 1  ");
    picks = setPickNote(picks, "b", "   ");
    expect(toSubsetPayload(picks)).toEqual([
      { event_id: "a", position: 1, note: "Admissions p. 1" },
      { event_id: "b", position: 2 },
      { event_id: "c", position: 3 },
    ]);
    // Explicit: the key is absent, not present-and-empty. A blank `note` on the
    // wire would rewrite every untouched note to "" on the next save.
    expect("note" in toSubsetPayload(picks)[1]).toBe(false);
  });

  it("an empty story is an empty set, not a refusal", () => {
    expect(toSubsetPayload([])).toEqual([]);
  });

  it("carries a GAP through a save — the reference is not dropped", () => {
    // The whole reason picks are ids: "zz" is not on the timeline any more, and
    // it must still be on the wire, in its place, after an edit and a Save.
    const payload = toSubsetPayload(picksOf("a", "zz", "b"));
    expect(payload.map((r) => r.event_id)).toEqual(["a", "zz", "b"]);
    expect(payload.map((r) => r.position)).toEqual([1, 2, 3]);
  });
});

describe("pickedInPhase / isPicked / positionOf", () => {
  it("counts only this phase's picked events", () => {
    const phase = [event("a"), event("b"), event("c")];
    expect(pickedInPhase(picksOf("a", "c", "d"), phase)).toBe(2);
    expect(pickedInPhase([], phase)).toBe(0);
  });

  it("positionOf is null for an unpicked event, so the ord column stays blank", () => {
    expect(positionOf(picksOf("a"), "b")).toBeNull();
    expect(isPicked(picksOf("a"), "b")).toBe(false);
  });
});

describe("the size line is a sentence, not a block", () => {
  const wording = {
    subsets_size_line_template: "A story a person can hold is 12–20 events — this one is {count}.",
    subsets_gap_count_template: "{count} gaps",
  };

  it("says nothing at or below the comfortable maximum", () => {
    expect(COMFORTABLE_MAX).toBe(20);
    expect(sizeLine(wording, 20)).toBeNull();
    expect(sizeLine(wording, 15)).toBeNull();
    expect(sizeLine(wording, 0)).toBeNull();
  });

  it("names the count once past it, filling the STORED template", () => {
    expect(sizeLine(wording, 21)).toBe(
      "A story a person can hold is 12–20 events — this one is 21.",
    );
  });

  it("gapLine fills the stored gap template", () => {
    expect(gapLine(wording, 3)).toBe("3 gaps");
  });

  it("throws by name when the wording store is missing a key", () => {
    // cw() refuses to render a blank control — the backend and this build
    // disagreeing about the store must be loud, not invisible.
    expect(() => gapLine({}, 3)).toThrow(/subsets_gap_count_template/);
  });
});
