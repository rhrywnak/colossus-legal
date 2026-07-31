/**
 * Unit tests for the pure workbench logic (Phase 1a.6).
 *
 * The panel that consumes these has no component-test infra (Rule 30), so the
 * risk-bearing behavior — status filtering, the status→actions mapping, and the
 * orphan detection that upholds the confirmed-fact guarantee — is pinned here.
 */
import { describe, expect, it } from "vitest";

import {
  actionsForStatus,
  countByStatus,
  filterByStatus,
  findOrphans,
  formatConfidencePct,
  orphansVisibleUnder,
  roleConfidenceLabel,
  candidateChip,
} from "../candidateWorkbench";
import type { CandidateDto, FactStatus } from "../../services/scenarioGather";
import type { ScenarioFactDto } from "../../services/scenarioFacts";

// ── Fixtures ─────────────────────────────────────────────────────────────────

const candidate = (id: string, status: FactStatus, ordinal: number | null = null): CandidateDto => ({
  content: { evidence_id: id, title: "", pattern_tags: [], about: [] },
  status,
  role: null,
  confidence: null,
  note: null,
  ordinal,
});

/** A scored candidate carrying a model role + confidence (what the merge writes). */
const scored = (id: string, role: string, confidence: number): CandidateDto => ({
  content: { evidence_id: id, title: "", pattern_tags: [], about: [] },
  status: "undecided",
  role,
  confidence,
  note: null,
  ordinal: null,
});

const savedRef = (id: string): ScenarioFactDto => ({
  graph_node_id: id,
  role: null,
  note: null,
  content: null,
});

const POOL: CandidateDto[] = [
  candidate("ev-u1", "undecided"),
  candidate("ev-u2", "undecided"),
  candidate("ev-i1", "included"),
  candidate("ev-d1", "dropped"),
];

// ── filterByStatus ───────────────────────────────────────────────────────────

describe("filterByStatus", () => {
  it("selects only the matching status for a concrete filter", () => {
    expect(filterByStatus(POOL, "undecided").map((c) => c.content.evidence_id)).toEqual([
      "ev-u1",
      "ev-u2",
    ]);
    expect(filterByStatus(POOL, "included").map((c) => c.content.evidence_id)).toEqual([
      "ev-i1",
    ]);
    expect(filterByStatus(POOL, "dropped").map((c) => c.content.evidence_id)).toEqual([
      "ev-d1",
    ]);
  });

  it("returns everything for the 'all' filter", () => {
    expect(filterByStatus(POOL, "all")).toHaveLength(4);
  });

  it("returns an empty array when nothing matches", () => {
    expect(filterByStatus([], "undecided")).toEqual([]);
  });
});

// ── actionsForStatus ─────────────────────────────────────────────────────────

describe("actionsForStatus", () => {
  it("offers include + drop on an undecided candidate", () => {
    expect(actionsForStatus("undecided")).toEqual(["include", "drop"]);
  });

  it("offers only drop on an included candidate", () => {
    expect(actionsForStatus("included")).toEqual(["drop"]);
  });

  it("offers only un-drop on a dropped candidate", () => {
    expect(actionsForStatus("dropped")).toEqual(["undrop"]);
  });
});

// ── findOrphans + orphansVisibleUnder ──────────────────────────────────────────

describe("findOrphans", () => {
  it("returns saved refs whose node is absent from the gather set", () => {
    const known = new Set(["ev-i1", "ev-u1"]);
    const saved = [savedRef("ev-i1"), savedRef("ev-gone"), savedRef("ev-u1")];

    const orphans = findOrphans(saved, known).map((o) => o.graph_node_id);
    expect(orphans).toEqual(["ev-gone"]);
  });

  it("returns nothing when every saved ref is known to gather", () => {
    const known = new Set(["ev-i1", "ev-d1"]);
    expect(findOrphans([savedRef("ev-i1"), savedRef("ev-d1")], known)).toEqual([]);
  });
});

// ── countByStatus ──────────────────────────────────────────────────────────────

describe("countByStatus", () => {
  it("counts each status and the total from a mixed list", () => {
    // POOL fixture has 2 undecided, 1 included, 1 dropped.
    expect(countByStatus(POOL)).toEqual({
      undecided: 2,
      included: 1,
      dropped: 1,
      total: 4,
    });
  });

  it("returns all zeros for an empty list", () => {
    expect(countByStatus([])).toEqual({
      undecided: 0,
      included: 0,
      dropped: 0,
      total: 0,
    });
  });

  it("puts every item in one bucket for a single-status list", () => {
    const allIncluded = [
      candidate("ev-a", "included"),
      candidate("ev-b", "included"),
      candidate("ev-c", "included"),
    ];
    expect(countByStatus(allIncluded)).toEqual({
      undecided: 0,
      included: 3,
      dropped: 0,
      total: 3,
    });
  });

  it("keeps total equal to the input length (a derivation, never a stored number)", () => {
    const counts = countByStatus(POOL);
    expect(counts.total).toBe(POOL.length);
    expect(counts.undecided + counts.included + counts.dropped).toBe(POOL.length);
  });
});

describe("orphansVisibleUnder", () => {
  it("surfaces orphans only under the included and all filters", () => {
    // A confirmed fact is expected under `included`/`all`; an orphan's true
    // status is unknown (the old endpoint is statusless), so it is not shown
    // under the undecided/dropped views.
    expect(orphansVisibleUnder("included")).toBe(true);
    expect(orphansVisibleUnder("all")).toBe(true);
    expect(orphansVisibleUnder("undecided")).toBe(false);
    expect(orphansVisibleUnder("dropped")).toBe(false);
  });
});

// ── display order (sortByConfidence removed) ────────────────────────────────────

describe("filterByStatus order preservation", () => {
  // `sortByConfidence` is gone: order is backend-supplied ascending candidate-id
  // order, and the workbench must not re-sort. What still needs pinning is that
  // FILTERING does not disturb that order — a filter that reordered would
  // reintroduce the reshuffling the removal was meant to stop.
  it("preserves the backend's order when filtering", () => {
    const list = [
      candidate("ev-1", "undecided", 1),
      candidate("ev-2", "included", 2),
      candidate("ev-3", "undecided", 3),
      candidate("ev-4", "included", 4),
    ];

    expect(filterByStatus(list, "all").map((c) => c.ordinal)).toEqual([1, 2, 3, 4]);
    expect(filterByStatus(list, "included").map((c) => c.ordinal)).toEqual([2, 4]);
    expect(filterByStatus(list, "undecided").map((c) => c.ordinal)).toEqual([1, 3]);
  });

  it("does not reorder when a card is scored", () => {
    // The side-effect-free guarantee at the helper level: a high-confidence score
    // must not promote a card. (Under the old sort, ev-3 would have jumped first.)
    const list = [
      candidate("ev-1", "undecided", 1),
      candidate("ev-2", "undecided", 2),
      { ...candidate("ev-3", "undecided", 3), role: "supports", confidence: 0.99 },
    ];

    expect(filterByStatus(list, "all").map((c) => c.ordinal)).toEqual([1, 2, 3]);
  });
});

describe("formatConfidencePct", () => {
  it("renders a fraction as a whole percent (matching the scan-run panel)", () => {
    expect(formatConfidencePct(0.85)).toBe("85%");
    expect(formatConfidencePct(0.554)).toBe("55%");
  });

  it("renders a real 0 score as '0%' (0 is a score, never conflated with unscored)", () => {
    expect(formatConfidencePct(0)).toBe("0%");
  });
});

// ── roleConfidenceLabel ─────────────────────────────────────────────────────────

describe("roleConfidenceLabel", () => {
  it("composes 'role · NN%' for a scored pick (echoing the scan-run panel)", () => {
    expect(roleConfidenceLabel("corroborates", 0.85)).toBe("corroborates · 85%");
    expect(roleConfidenceLabel("rebuts", 0.55)).toBe("rebuts · 55%");
  });

  it("shows just the percent when a score has no role", () => {
    expect(roleConfidenceLabel(null, 0.4)).toBe("40%");
  });

  it("keeps a real 0 score as a percent", () => {
    expect(roleConfidenceLabel("contradicts", 0)).toBe("contradicts · 0%");
  });
});

// ── candidateChip ───────────────────────────────────────────────────────────────

describe("candidateChip", () => {
  it("renders the ordinal as a speakable handle", () => {
    // The whole point of replacing the hash chip: a human can say this one out
    // loud, write it in a margin, and compare two of them.
    expect(candidateChip(1)).toBe("C-1");
    expect(candidateChip(14)).toBe("C-14");
    expect(candidateChip(147)).toBe("C-147");
  });

  it("renders nothing when the candidate has no ordinal yet", () => {
    // null/undefined must NOT become "C-0" or "C-?" — both would read as real
    // ids for cards that do not exist (Standing Rule 1). The caller renders no
    // chip at all.
    expect(candidateChip(null)).toBeNull();
    expect(candidateChip(undefined)).toBeNull();
  });

  it("renders 0 as a real id rather than treating it as absent", () => {
    // Guards the `== null` check against a truthiness bug: 0 is falsy in JS, so a
    // `!ordinal` test would swallow it. The backend starts sequences at 1, so this
    // should never occur — but if it ever does, it must not silently vanish.
    expect(candidateChip(0)).toBe("C-0");
  });
});
