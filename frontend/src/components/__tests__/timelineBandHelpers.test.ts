/**
 * Pure-helper tests for the Timeline band rollup.
 *
 * Locks `buildPhaseSummaries`: one summary per phase, in phase order, with
 * `eventCount` derived by matching each event's `phase` to the phase id —
 * including the zero-event and unmatched-event boundaries. No DOM / RTL.
 */
import { describe, expect, it } from "vitest";
import {
  buildPhaseSummaries,
  type CaseTimeline,
} from "../../services/caseTimeline";

const timeline: CaseTimeline = {
  phases: [
    { id: "estate", label: "Estate", date_range: "2010–2012", color: "#1570ef" },
    { id: "probate", label: "Probate", date_range: "2012–2013", color: "#027a48" },
    { id: "appeals", label: "Appeals", date_range: "2014", color: "#b54708" },
  ],
  events: [
    { phase: "estate" },
    { phase: "estate" },
    { phase: "probate" },
    { phase: "unmapped" }, // belongs to no listed phase — counted nowhere
  ],
};

describe("buildPhaseSummaries", () => {
  it("returns one summary per phase, in phase order", () => {
    const summaries = buildPhaseSummaries(timeline);
    expect(summaries.map((s) => s.id)).toEqual(["estate", "probate", "appeals"]);
  });

  it("counts events per phase by matching phase id", () => {
    const summaries = buildPhaseSummaries(timeline);
    const byId = Object.fromEntries(summaries.map((s) => [s.id, s.eventCount]));
    expect(byId).toEqual({ estate: 2, probate: 1, appeals: 0 });
  });

  it("carries label, date_range, and color through unchanged", () => {
    const [estate] = buildPhaseSummaries(timeline);
    expect(estate).toMatchObject({
      label: "Estate",
      date_range: "2010–2012",
      color: "#1570ef",
    });
  });

  it("returns an empty array when there are no phases", () => {
    expect(buildPhaseSummaries({ phases: [], events: [] })).toEqual([]);
  });
});
