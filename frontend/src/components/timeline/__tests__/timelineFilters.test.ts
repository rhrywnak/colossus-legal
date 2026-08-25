/**
 * Every decision the timeline page makes, tested where a test can reach it.
 *
 * There is no component-testing tier in this project, so these pure functions
 * ARE the page's behaviour: what the filters keep, how events group, which ones
 * would otherwise vanish, and how a date, a badge and a link read.
 */
import { describe, expect, it } from "vitest";
import type {
  CaseTimeline,
  TimelineEvent,
  TimelineLink,
  TimelinePhase,
  TimelineTag,
} from "../../../services/caseTimeline";
import {
  applyFilters,
  dotColor,
  formatEventDate,
  groupByPhase,
  isFiltered,
  linkRendering,
  NO_FILTERS,
  noteBadge,
  subtitleOf,
  tagOf,
  unknownPhaseEvents,
} from "../timelineFilters";

const WORDING = {
  count_template: "{events} events across {phases} phases",
  filtered_count_template: "Showing {phase} · {shown} of {total} events",
  all_tags_label: "All",
  note_count_one: "💬 1 note",
  note_count_template: "💬 {count} notes",
  no_document_label: "⚠ no document yet",
  link_unchecked_label: "◌ not checked",
};

const phase = (id: string, label: string, order: number): TimelinePhase => ({
  id,
  label,
  date_range: "2008–2009",
  color: "#b45309",
  sort_order: order,
});

const event = (over: Partial<TimelineEvent> & { id: string }): TimelineEvent => ({
  event_date: "2009-03-25",
  date_precision: "day",
  approximate: false,
  phase: "estate",
  title: "An event",
  fact: "Something happened.",
  attributes: {},
  tags: ["filing"],
  links: [],
  note_count: 0,
  created_at: "2026-08-25T12:00:00Z",
  updated_at: "2026-08-25T12:00:00Z",
  ...over,
});

const link = (over: Partial<TimelineLink>): TimelineLink => ({
  target_type: "document",
  target_id: "doc-x",
  resolution: "resolves",
  ...over,
});

// ─── applyFilters ────────────────────────────────────────────────────────────

describe("applyFilters", () => {
  const events = [
    event({ id: "a", title: "Auction Held", event_date: "2010-05-01", tags: ["financial"] }),
    event({ id: "b", title: "Brief Filed", event_date: "2011-03-14", tags: ["filing"] }),
    event({ id: "c", title: "Order Issued", event_date: "2012-04-12", tags: ["court_action"], phase: "appeals" }),
  ];

  it("keeps everything when nothing is set", () => {
    expect(applyFilters(events, NO_FILTERS).map((e) => e.id)).toEqual(["a", "b", "c"]);
  });

  it("narrows by tag", () => {
    expect(applyFilters(events, { ...NO_FILTERS, tag: "filing" }).map((e) => e.id)).toEqual(["b"]);
  });

  it("narrows by phase", () => {
    expect(applyFilters(events, { ...NO_FILTERS, phase: "appeals" }).map((e) => e.id)).toEqual(["c"]);
  });

  it("searches title and fact, case-insensitively", () => {
    expect(applyFilters(events, { ...NO_FILTERS, search: "auction" }).map((e) => e.id)).toEqual(["a"]);
    const byFact = applyFilters(
      [event({ id: "d", title: "Nothing", fact: "A UNIQUE phrase." })],
      { ...NO_FILTERS, search: "unique phrase" },
    );
    expect(byFact.map((e) => e.id)).toEqual(["d"]);
  });

  it("treats the date range as inclusive at both ends", () => {
    const kept = applyFilters(events, { ...NO_FILTERS, from: "2010-05-01", to: "2011-03-14" });
    expect(kept.map((e) => e.id)).toEqual(["a", "b"]);
  });

  it("composes: a tag AND a date range means both, never either", () => {
    const kept = applyFilters(events, {
      ...NO_FILTERS,
      tag: "filing",
      from: "2012-01-01",
    });
    expect(kept).toEqual([]);
  });

  it("preserves the order it was given", () => {
    const kept = applyFilters(events, { ...NO_FILTERS, from: "2010-01-01" });
    expect(kept.map((e) => e.id)).toEqual(["a", "b", "c"]);
  });

  it("ignores whitespace-only search text", () => {
    expect(applyFilters(events, { ...NO_FILTERS, search: "   " })).toHaveLength(3);
  });
});

describe("isFiltered", () => {
  it("is false for the empty filter set and true for each single filter", () => {
    expect(isFiltered(NO_FILTERS)).toBe(false);
    expect(isFiltered({ ...NO_FILTERS, tag: "filing" })).toBe(true);
    expect(isFiltered({ ...NO_FILTERS, phase: "estate" })).toBe(true);
    expect(isFiltered({ ...NO_FILTERS, search: "x" })).toBe(true);
    expect(isFiltered({ ...NO_FILTERS, from: "2010-01-01" })).toBe(true);
    expect(isFiltered({ ...NO_FILTERS, to: "2010-01-01" })).toBe(true);
  });
});

// ─── grouping, and the events that used to vanish ────────────────────────────

describe("groupByPhase", () => {
  const phases = [phase("estate", "PRE-PROBATE", 1), phase("appeals", "COA", 3)];

  it("groups events under their phase, in the phases' order", () => {
    const groups = groupByPhase(phases, [
      event({ id: "a", phase: "appeals" }),
      event({ id: "b", phase: "estate" }),
    ]);
    expect(groups.map((g) => g.phase.id)).toEqual(["estate", "appeals"]);
    expect(groups[0].events.map((e) => e.id)).toEqual(["b"]);
  });

  it("keeps a phase the filters emptied, rather than hiding it", () => {
    const groups = groupByPhase(phases, [event({ id: "a", phase: "estate" })]);
    expect(groups).toHaveLength(2);
    expect(groups[1].events).toEqual([]);
  });
});

describe("unknownPhaseEvents", () => {
  it("finds the event a phase list cannot place", () => {
    const orphan = event({ id: "x", phase: "mediation" });
    const found = unknownPhaseEvents([phase("estate", "PRE-PROBATE", 1)], [
      event({ id: "a", phase: "estate" }),
      orphan,
    ]);
    expect(found.map((e) => e.id)).toEqual(["x"]);
  });

  it("finds nothing when every event has a phase row", () => {
    expect(
      unknownPhaseEvents([phase("estate", "PRE-PROBATE", 1)], [event({ id: "a" })]),
    ).toEqual([]);
  });

  it("and what it finds is exactly what groupByPhase drops", () => {
    // The two halves must account for every event between them — that is the
    // whole point: nothing may fall out of the render.
    const phases = [phase("estate", "PRE-PROBATE", 1)];
    const events = [event({ id: "a" }), event({ id: "b", phase: "mediation" })];
    const grouped = groupByPhase(phases, events).reduce((n, g) => n + g.events.length, 0);
    expect(grouped + unknownPhaseEvents(phases, events).length).toBe(events.length);
  });
});

// ─── one date, one dot, one badge, one link ──────────────────────────────────

describe("formatEventDate", () => {
  it("prints a day-precision date in full", () => {
    expect(formatEventDate("2009-03-25", false, "day")).toBe("Mar 25, 2009");
  });

  it("prefixes an approximate date with ~", () => {
    expect(formatEventDate("2010-05-01", true, "day")).toBe("~ May 1, 2010");
  });

  it("never invents a day the source did not state", () => {
    expect(formatEventDate("2010-05-01", false, "month")).toBe("May 2010");
    expect(formatEventDate("2010-01-01", false, "year")).toBe("2010");
  });

  it("shows the stored value rather than a crash when the date is unreadable", () => {
    expect(formatEventDate("not-a-date", false, "day")).toBe("not-a-date");
  });
});

describe("dotColor", () => {
  const tags: TimelineTag[] = [
    { id: "filing", label: "Filing", color: "#7c3aed", sort_order: 3 },
  ];

  it("takes the first known tag's colour", () => {
    expect(dotColor(tags, event({ id: "a", tags: ["filing"] }), "#999")).toBe("#7c3aed");
  });

  it("falls back rather than refusing to render an unknown tag", () => {
    expect(dotColor(tags, event({ id: "a", tags: ["hearsay"] }), "#999")).toBe("#999");
    expect(dotColor(tags, event({ id: "a", tags: [] }), "#999")).toBe("#999");
  });

  it("skips an unknown tag to reach a known one", () => {
    expect(dotColor(tags, event({ id: "a", tags: ["hearsay", "filing"] }), "#999")).toBe("#7c3aed");
  });
});

describe("tagOf", () => {
  it("resolves a known id and returns undefined for a stranger", () => {
    const tags: TimelineTag[] = [{ id: "filing", label: "Filing", color: "#7c3aed", sort_order: 3 }];
    expect(tagOf(tags, "filing")?.label).toBe("Filing");
    expect(tagOf(tags, "hearsay")).toBeUndefined();
  });
});

describe("noteBadge", () => {
  it("says nothing when there are no notes", () => {
    expect(noteBadge(0, WORDING)).toBeNull();
  });

  it("uses the singular row for one and the template for more", () => {
    expect(noteBadge(1, WORDING)).toBe("💬 1 note");
    expect(noteBadge(3, WORDING)).toBe("💬 3 notes");
  });
});

describe("linkRendering", () => {
  it("renders a resolving link as a link, with its pinpoint", () => {
    const r = linkRendering(
      link({ label: "Morris Affidavit", pinpoint: "p. 2 ¶ 4" }),
      WORDING,
    );
    expect(r).toEqual({ kind: "link", label: "Morris Affidavit", pinpoint: "p. 2 ¶ 4" });
  });

  it("falls back to the target id when a resolving link has no label", () => {
    expect(linkRendering(link({ label: undefined }), WORDING)).toMatchObject({
      kind: "link",
      label: "doc-x",
    });
  });

  it("marks a missing target — never a link, never dropped", () => {
    const r = linkRendering(link({ resolution: "missing" }), WORDING);
    expect(r.kind).toBe("missing");
    expect(r.label).toBe("⚠ no document yet");
  });

  it("gives an unchecked target its OWN state, not the missing one", () => {
    const r = linkRendering(link({ resolution: "unchecked", target_type: "paperless_document" }), WORDING);
    expect(r.kind).toBe("unchecked");
    expect(r.label).toBe("◌ not checked");
    expect(r.label).not.toBe(WORDING.no_document_label);
  });
});

// ─── the subtitle, which is how a filtered page stays honest ─────────────────

describe("subtitleOf", () => {
  const data: CaseTimeline = {
    phases: [phase("estate", "PRE-PROBATE", 1), phase("appeals", "COA", 3)],
    tags: [],
    events: [event({ id: "a" }), event({ id: "b" }), event({ id: "c" })],
    wording: WORDING,
    phase_window_events: 4,
  };

  it("counts events and phases when nothing is filtered", () => {
    expect(subtitleOf(data, NO_FILTERS, 3)).toBe("3 events across 2 phases");
  });

  it("names the phase and both counts when one phase owns the page", () => {
    expect(subtitleOf(data, { ...NO_FILTERS, phase: "appeals" }, 1)).toBe(
      "Showing COA · 1 of 3 events",
    );
  });

  it("still says a filter is on when it is not a phase filter", () => {
    expect(subtitleOf(data, { ...NO_FILTERS, search: "auction" }, 1)).toBe(
      "Showing All · 1 of 3 events",
    );
  });

  it("shows the slug when a phase filter names a phase with no row", () => {
    expect(subtitleOf(data, { ...NO_FILTERS, phase: "mediation" }, 0)).toBe(
      "Showing mediation · 0 of 3 events",
    );
  });
});
