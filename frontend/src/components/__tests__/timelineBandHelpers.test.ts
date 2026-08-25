/**
 * Pure-helper tests for the Timeline band rollup.
 *
 * Locks `buildPhaseSummaries`: one summary per phase, in phase order, with
 * `eventCount` derived by matching each event's `phase` to the phase id — and,
 * since Phase B, the matched/unmatched split that stops an event being dropped
 * in silence. No DOM / RTL.
 */
import { describe, expect, it } from "vitest";
import {
  buildPhaseSummaries,
  type CaseTimeline,
  type TimelineEvent,
  type TimelinePhase,
} from "../../services/caseTimeline";

const phase = (id: string, order: number): TimelinePhase => ({
  id,
  label: id.toUpperCase(),
  date_range: "2010–2012",
  color: "#1570ef",
  sort_order: order,
});

/** An event carrying only what the rollup reads; the rest is realistic filler. */
const event = (id: string, phaseSlug: string): TimelineEvent => ({
  id,
  event_date: "2010-02-12",
  date_precision: "day",
  approximate: false,
  phase: phaseSlug,
  title: "An event",
  attributes: {},
  tags: [],
  links: [],
  note_count: 0,
  created_at: "2026-08-25T12:00:00Z",
  updated_at: "2026-08-25T12:00:00Z",
});

const timeline = (
  phases: TimelinePhase[],
  events: TimelineEvent[],
): CaseTimeline => ({
  phases,
  tags: [],
  events,
  wording: { band_mismatch_template: "{shown} of {total}" },
  phase_window_events: 4,
});

describe("buildPhaseSummaries", () => {
  const three = timeline(
    [phase("estate", 1), phase("probate", 2), phase("appeals", 3)],
    [
      event("a", "estate"),
      event("b", "estate"),
      event("c", "probate"),
      event("d", "unmapped"),
    ],
  );

  it("returns one summary per phase, in phase order", () => {
    expect(buildPhaseSummaries(three).phases.map((s) => s.id)).toEqual([
      "estate",
      "probate",
      "appeals",
    ]);
  });

  it("counts events per phase by matching phase id", () => {
    const byId = Object.fromEntries(
      buildPhaseSummaries(three).phases.map((s) => [s.id, s.eventCount]),
    );
    expect(byId).toEqual({ estate: 2, probate: 1, appeals: 0 });
  });

  it("carries label, date_range, and color through unchanged", () => {
    const [estate] = buildPhaseSummaries(three).phases;
    expect(estate).toMatchObject({
      label: "ESTATE",
      date_range: "2010–2012",
      color: "#1570ef",
    });
  });

  it("returns an empty array when there are no phases", () => {
    expect(buildPhaseSummaries(timeline([], [])).phases).toEqual([]);
  });

  // ── the silent path that Phase B closed ──────────────────────────────────

  it("reports an event whose phase has no pill instead of dropping it", () => {
    const rolled = buildPhaseSummaries(three);
    expect(rolled.matched).toBe(3);
    expect(rolled.unmatched).toBe(1);
    expect(rolled.phases.reduce((n, p) => n + p.eventCount, 0)).toBe(3);
  });

  it("reports nothing unmatched when every event has a phase row", () => {
    const rolled = buildPhaseSummaries(
      timeline([phase("estate", 1)], [event("a", "estate"), event("b", "estate")]),
    );
    expect(rolled.matched).toBe(2);
    expect(rolled.unmatched).toBe(0);
  });

  it("counts every event as unmatched when there are no phases at all", () => {
    const rolled = buildPhaseSummaries(timeline([], [event("a", "estate")]));
    expect(rolled.unmatched).toBe(1);
    expect(rolled.matched).toBe(0);
  });
});
