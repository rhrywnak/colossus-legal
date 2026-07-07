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
  filterByStatus,
  findOrphans,
  orphansVisibleUnder,
} from "../candidateWorkbench";
import type { CandidateDto, FactStatus } from "../../services/scenarioGather";
import type { ScenarioFactDto } from "../../services/scenarioFacts";

// ── Fixtures ─────────────────────────────────────────────────────────────────

const candidate = (id: string, status: FactStatus): CandidateDto => ({
  content: { evidence_id: id, title: "", pattern_tags: [], about: [] },
  status,
  role: null,
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
