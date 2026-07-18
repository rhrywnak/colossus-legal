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
  candidateBadgeLabel,
  countByStatus,
  filterByStatus,
  findOrphans,
  formatConfidencePct,
  orphansVisibleUnder,
  sortByConfidence,
  UNSCORED_LABEL,
} from "../candidateWorkbench";
import type { CandidateDto, FactStatus } from "../../services/scenarioGather";
import type { ScenarioFactDto } from "../../services/scenarioFacts";

// ── Fixtures ─────────────────────────────────────────────────────────────────

const candidate = (id: string, status: FactStatus): CandidateDto => ({
  content: { evidence_id: id, title: "", pattern_tags: [], about: [] },
  status,
  role: null,
  confidence: null,
  note: null,
});

/** A scored candidate carrying a model role + confidence (what the merge writes). */
const scored = (id: string, role: string, confidence: number): CandidateDto => ({
  content: { evidence_id: id, title: "", pattern_tags: [], about: [] },
  status: "undecided",
  role,
  confidence,
  note: null,
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

// ── sortByConfidence ────────────────────────────────────────────────────────────

describe("sortByConfidence", () => {
  it("orders scored candidates highest-confidence first", () => {
    const out = sortByConfidence([
      scored("ev-lo", "contradicts", 0.4),
      scored("ev-hi", "corroborates", 0.85),
      scored("ev-mid", "rebuts", 0.55),
    ]);
    expect(out.map((c) => c.content.evidence_id)).toEqual(["ev-hi", "ev-mid", "ev-lo"]);
  });

  it("pins unscored (null confidence) LAST as a distinct group, not sorted as 0", () => {
    // The unscored rows must fall BELOW even the lowest scored pick (0.4) — proof
    // they are partitioned last, not coalesced to 0 (which would interleave them
    // with a genuine 0.0-ish score).
    const out = sortByConfidence([
      candidate("ev-uns-b", "included"),
      scored("ev-lo", "contradicts", 0.4),
      candidate("ev-uns-a", "undecided"),
      scored("ev-hi", "corroborates", 0.85),
    ]);
    expect(out.map((c) => c.content.evidence_id)).toEqual([
      "ev-hi",
      "ev-lo",
      // unscored group, ordered by the stable evidence_id secondary key
      "ev-uns-a",
      "ev-uns-b",
    ]);
  });

  it("keeps a real 0 score ABOVE unscored rows (0 is a score, null is not)", () => {
    const out = sortByConfidence([
      candidate("ev-unscored", "included"),
      scored("ev-zero", "contradicts", 0),
    ]);
    expect(out.map((c) => c.content.evidence_id)).toEqual(["ev-zero", "ev-unscored"]);
  });

  it("breaks scored ties by the stable evidence_id secondary key", () => {
    const out = sortByConfidence([
      scored("ev-b", "rebuts", 0.7),
      scored("ev-a", "corroborates", 0.7),
    ]);
    expect(out.map((c) => c.content.evidence_id)).toEqual(["ev-a", "ev-b"]);
  });

  it("does not mutate the input array (returns a fresh sorted copy)", () => {
    const input = [scored("ev-lo", "contradicts", 0.4), scored("ev-hi", "corroborates", 0.85)];
    const before = input.map((c) => c.content.evidence_id);
    sortByConfidence(input);
    expect(input.map((c) => c.content.evidence_id)).toEqual(before);
  });
});

// ── formatConfidencePct ─────────────────────────────────────────────────────────

describe("formatConfidencePct", () => {
  it("renders a fraction as a whole percent (matching the scan-run panel)", () => {
    expect(formatConfidencePct(0.85)).toBe("85%");
    expect(formatConfidencePct(0.554)).toBe("55%");
  });

  it("renders a real 0 score as '0%', distinct from unscored", () => {
    expect(formatConfidencePct(0)).toBe("0%");
  });

  it("renders null as the 'unscored' marker, never '0%' or blank", () => {
    expect(formatConfidencePct(null)).toBe(UNSCORED_LABEL);
    expect(formatConfidencePct(null)).not.toBe("0%");
    expect(formatConfidencePct(null)).not.toBe("");
  });
});

// ── candidateBadgeLabel ─────────────────────────────────────────────────────────

describe("candidateBadgeLabel", () => {
  it("composes 'role · NN%' for a scored pick (echoing the scan-run panel)", () => {
    expect(candidateBadgeLabel("corroborates", 0.85)).toBe("corroborates · 85%");
    expect(candidateBadgeLabel("rebuts", 0.55)).toBe("rebuts · 55%");
  });

  it("shows just the percent when a score has no role", () => {
    expect(candidateBadgeLabel(null, 0.4)).toBe("40%");
  });

  it("shows the 'unscored' marker when there is no confidence, regardless of role", () => {
    expect(candidateBadgeLabel(null, null)).toBe(UNSCORED_LABEL);
    // Even if a role were somehow present, no model score ⇒ unscored (the slot
    // shows one thing; confidence is what gates it).
    expect(candidateBadgeLabel("rebuts", null)).toBe(UNSCORED_LABEL);
  });

  it("keeps a real 0 score as a percent, never 'unscored'", () => {
    expect(candidateBadgeLabel("contradicts", 0)).toBe("contradicts · 0%");
  });
});
