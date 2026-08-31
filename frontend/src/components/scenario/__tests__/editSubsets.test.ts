// =============================================================================
// editSubsets.test.ts — merging two reads into one list of rows
// =============================================================================
//
// T5.4's second named suite. The merge is where a row can end up saying "not
// attached" beside a Detach button, so every case that decides which button a
// row draws is asserted here.

import { describe, expect, it } from "vitest";

import type { AttachedSubset } from "../../scenario-timeline/scenarioTimeline";
import type { SubsetSummary } from "../../../services/caseTimelineSubsets";
import { attachedCount, subsetRows } from "../editSubsets";

function subset(over: Partial<SubsetSummary> & { id: string; name: string }): SubsetSummary {
  return {
    description: "",
    event_count: 0,
    gap_count: 0,
    carried_by: [],
    created_by: "cc",
    created_at: "2026-08-31T00:00:00Z",
    updated_by: "cc",
    updated_at: "2026-08-31T00:00:00Z",
    ...over,
  };
}

const link = (id: string, position: number): AttachedSubset => ({
  id,
  name: "",
  event_count: 0,
  gap_count: 0,
  position,
});

describe("subsetRows — which button each row draws", () => {
  it("marks the ones this scenario carries, and only those", () => {
    const rows = subsetRows(
      [subset({ id: "a", name: "The $50,000" }), subset({ id: "b", name: "The fee engine" })],
      [link("a", 0)],
    );
    expect(rows.map((r) => [r.name, r.attached])).toEqual([
      ["The $50,000", true],
      ["The fee engine", false],
    ]);
  });

  it("carries the description and the count through for the row to draw", () => {
    const rows = subsetRows(
      [subset({ id: "a", name: "The $50,000", description: "Emil's own money…", event_count: 15 })],
      [],
    );
    expect(rows[0].description).toBe("Emil's own money…");
    expect(rows[0].eventCount).toBe(15);
  });
});

describe("subsetRows — the ordering", () => {
  it("puts attached first, in ATTACHMENT order — not alphabetical", () => {
    // Attachment order is what the scenario's author chose and what the window's
    // selector uses. A reader should learn ONE order for this scenario.
    const rows = subsetRows(
      [
        subset({ id: "a", name: "Alpha" }),
        subset({ id: "z", name: "Zulu" }),
        subset({ id: "m", name: "Mike" }),
      ],
      [link("z", 0), link("m", 1)],
    );
    expect(rows.map((r) => r.name)).toEqual(["Zulu", "Mike", "Alpha"]);
  });

  it("sorts the unattached remainder by name", () => {
    const rows = subsetRows(
      [
        subset({ id: "c", name: "Charlie" }),
        subset({ id: "a", name: "Alpha" }),
        subset({ id: "b", name: "Bravo" }),
      ],
      [],
    );
    expect(rows.map((r) => r.name)).toEqual(["Alpha", "Bravo", "Charlie"]);
  });

  it("falls back to name when two attachments share a position", () => {
    // `position` is not guaranteed unique by anything, and a list that reorders
    // itself between renders is a list a reader cannot click confidently.
    const rows = subsetRows(
      [subset({ id: "x", name: "Xray" }), subset({ id: "d", name: "Delta" })],
      [link("x", 0), link("d", 0)],
    );
    expect(rows.map((r) => r.name)).toEqual(["Delta", "Xray"]);
  });
});

describe("subsetRows — the edges", () => {
  it("DROPS an attachment naming a subset the case list does not carry", () => {
    // It can only mean the subset was deleted between the two reads. A row with
    // a name nobody can render is worse than one fewer row.
    const rows = subsetRows([subset({ id: "a", name: "Alpha" })], [link("ghost", 0), link("a", 1)]);
    expect(rows.map((r) => r.id)).toEqual(["a"]);
    expect(rows[0].attached).toBe(true);
  });

  it("shows every subset in the case when the scenario carries none", () => {
    const rows = subsetRows([subset({ id: "a", name: "Alpha" })], []);
    expect(rows).toHaveLength(1);
    expect(rows[0].attached).toBe(false);
  });

  it("renders nothing rather than throwing when the case has no subsets", () => {
    expect(subsetRows([], [])).toEqual([]);
    // Even with a stale attachment: the section draws an empty list and the
    // create link under it, which is the honest state of a case with no stories.
    expect(subsetRows([], [link("ghost", 0)])).toEqual([]);
  });
});

describe("attachedCount", () => {
  it("counts the DRAWN rows, so the section cannot claim more than it shows", () => {
    // Counted off the merged rows and not off the attachment list — a stale
    // attachment dropped above must not inflate this.
    const rows = subsetRows([subset({ id: "a", name: "Alpha" })], [link("ghost", 0), link("a", 1)]);
    expect(attachedCount(rows)).toBe(1);
  });

  it("is zero on an empty list", () => {
    expect(attachedCount([])).toBe(0);
  });
});
