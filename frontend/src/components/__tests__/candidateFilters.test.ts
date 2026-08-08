// =============================================================================
// candidateFilters.test.ts — the filter model and its §9 counts (task 1.7E-a)
// =============================================================================
//
// The headline tests are the RECONCILIATION ones: the four state counts sum to
// All, and "Rulable now" and "Proposed" are each a SUBSET of "Not ruled" rather
// than a state of their own. §9's promise is not that the numbers look plausible —
// it is that they add up, and a filter bar that quietly double-counts is the
// defect this task exists to remove.

import { describe, expect, it } from "vitest";

import {
  candidateCounterLine,
  candidateCounts,
  candidateState,
  defaultFilters,
  filterCandidates,
  hasAnyFilter,
  isProposed,
  isRulableNow,
  stateChip,
  stateOptions,
  UNFILTERED,
  type CandidateFilters,
} from "../candidateFilters";
import type { ScenarioCard } from "../../services/scenarioCards";

// ─── Fixtures ───────────────────────────────────────────────────────────────

let nextId = 0;

/** A plain unruled, scan-scored, rulable candidate. */
function card(overrides: Partial<ScenarioCard> = {}): ScenarioCard {
  nextId += 1;
  return {
    code: `C-${nextId}`,
    graph_node_id: `ev-${nextId}`,
    quote: {
      text: "I do not recall that meeting.",
      context_before: "",
      context_after: "",
      context_before_complete: true,
      context_after_complete: true,
      context_before_notice: null,
      context_after_notice: null,
      question: null,
    },
    pinpoint: {
      document_id: "doc-7",
      document_title: "CFS interrogatory responses",
      label: "CFS interrogatory responses at 14",
      page: 14,
      viewer_href: "/documents/doc-7?page=14&tab=document",
    },
    speaker: { name: "R. Phillips", attribution: "extracted" },
    statement_kind: "denial",
    stance: {
      verb: "disputes",
      object: "¶54 — CFS knew of the meeting",
      summary: "This disputes ¶54 — CFS knew of the meeting",
    },
    bears_on: [],
    grounding: { state: "exact", label: "Grounded — found on the page" },
    confidence: { band: "medium", label: "Scan was fairly confident" },
    status: "undecided",
    status_label: "Not yet decided",
    defer_required: false,
    defer_required_reason: null,
    defer_reason: null,
    // Task 2.10: no human has linked this one — the ordinary state.
    human_links: [],
    human_link_summary: null,
    ...overrides,
  };
}

/** The C-222 class: scored, but linked to no accusation — Defer only. */
function deferOnly(): ScenarioCard {
  return card({
    stance: null,
    defer_required: true,
    defer_required_reason:
      "A scan scored this item, but it is not linked to any accusation yet, so " +
      "there is nothing for it to support or dispute. It can only be deferred for " +
      "now — it stays in the queue and returns when linking arrives.",
  });
}

const included = () => card({ status: "included", status_label: "Included" });
const excluded = () => card({ status: "dropped", status_label: "Set aside" });
const parked = () => card({ defer_reason: "Need to read the full page first" });
const neverScanned = () => card({ confidence: { band: "unscored", label: "Not scanned" } });

/** A card the latest completed scan is PROPOSING (2026-08-08). */
const proposed = () =>
  card({
    proposed: {
      role_label: "Scan: supports",
      reason: "direct admission by the accusing party",
      duplicate_count: 1,
      duplicate_label: null,
    },
  });

// ─── The four states ────────────────────────────────────────────────────────

describe("candidateState", () => {
  it("reads the machine token, never the prose", () => {
    // Filtering on `status_label` would break the day the wording changes, which
    // is exactly why the payload ships both.
    expect(candidateState(card({ status: "included", status_label: "Anything at all" }))).toBe(
      "included",
    );
  });

  it("tells a PARKED card from one nobody has looked at", () => {
    // Both are `undecided` on the wire — the defer reason is the whole difference,
    // and a human clearing a pool needs to see it.
    expect(candidateState(parked())).toBe("deferred");
    expect(candidateState(card())).toBe("not_ruled");
  });

  it("calls a dropped card excluded", () => {
    expect(candidateState(excluded())).toBe("excluded");
  });

  it("keeps a ruling ahead of a stale defer reason", () => {
    // An included card that was once parked is INCLUDED. Reading the defer reason
    // first would leave a decided card sitting in the deferred pile.
    expect(candidateState(card({ status: "included", defer_reason: "was parked once" }))).toBe(
      "included",
    );
  });
});

describe("the rulable predicate", () => {
  it("counts an unruled, linked card", () => {
    expect(isRulableNow(card())).toBe(true);
  });

  it("refuses a defer-only card — Include and Exclude are shut on it", () => {
    expect(isRulableNow(deferOnly())).toBe(false);
  });

  it("refuses a card that has already been ruled", () => {
    expect(isRulableNow(included())).toBe(false);
    expect(isRulableNow(excluded())).toBe(false);
    expect(isRulableNow(parked())).toBe(false);
  });
});

describe("the proposed facet", () => {
  it("reads the projection, not the confidence band", () => {
    // The retired "scan-scored" facet read the band, and a band only existed once
    // a verdict had been MERGED — so it measured merges, and a card that was ruled
    // IN silently left the facet when its confidence was nulled. Presence of the
    // projection is the honest test.
    expect(isProposed(proposed())).toBe(true);
    expect(isProposed(card({ confidence: { band: "high", label: "Scan was confident" } }))).toBe(
      false,
    );
  });

  it("is empty on a scenario nothing has scanned", () => {
    expect(isProposed(neverScanned())).toBe(false);
  });
});

// ─── The counts (§9) ────────────────────────────────────────────────────────

describe("candidateCounts", () => {
  const pool = [
    card(),
    card(),
    deferOnly(),
    included(),
    included(),
    excluded(),
    parked(),
    neverScanned(),
  ];
  const counts = candidateCounts(pool);

  it("counts every facet from the one pool", () => {
    expect(counts.all).toBe(8);
    expect(counts.not_ruled).toBe(4); // two plain, the defer-only one, the unscanned one
    expect(counts.rulable).toBe(3); // …minus the defer-only one
    expect(counts.included).toBe(2);
    expect(counts.excluded).toBe(1);
    expect(counts.deferred).toBe(1);
    expect(counts.proposed).toBe(0);
  });

  it("RECONCILES: the four states partition the pool exactly", () => {
    // THE §9 TEST. If a card could land in two states or in none, this sum drifts
    // and the filter bar starts lying about how much work is left.
    expect(counts.not_ruled + counts.included + counts.excluded + counts.deferred).toBe(
      counts.all,
    );
  });

  it("RECONCILES: rulable is a SUBSET of not-ruled, never a sixth state", () => {
    expect(counts.rulable).toBeLessThanOrEqual(counts.not_ruled);
  });

  it("RECONCILES: proposed is a SUBSET of not-ruled too", () => {
    // Precedence R-a is what guarantees it: a human-touched card is never
    // proposed, so a proposal can only ever be one of the not-ruled.
    const withProposals = candidateCounts([proposed(), proposed(), included(), parked()]);
    expect(withProposals.proposed).toBe(2);
    expect(withProposals.proposed).toBeLessThanOrEqual(withProposals.not_ruled);
  });

  it("counts an empty pool as zeroes rather than refusing", () => {
    // An empty scenario is a real state, not an error: it renders a bar of (0)s.
    expect(candidateCounts([])).toMatchObject({ all: 0, not_ruled: 0, rulable: 0 });
  });

  it("agrees with the dropdown options it feeds", () => {
    // The options read the same derivation — this asserts nobody has slipped a
    // second count in between (ruling R1, the half that survives 1.7G).
    const options = stateOptions(counts);
    expect(options.find((o) => o.facet === "all")?.count).toBe(counts.all);
    expect(options.find((o) => o.facet === "rulable")?.count).toBe(counts.rulable);
    expect(options.find((o) => o.facet === "proposed")?.count).toBe(counts.proposed);
  });

  it("every option carries its count into the dropdown (ruling R3)", () => {
    // 1.7E's R1 declined selects because "a count inside a closed dropdown is a
    // count nobody can see". R3 overrules it on the condition that the counts come
    // WITH the options, Bias-Analysis style — so an option with no count is the
    // regression that ruling was worried about.
    for (const option of stateOptions(counts)) {
      expect(option.count, `${option.label} has no count`).toBeTypeOf("number");
    }
  });

  it("says on the Rulable option that it is part of Not ruled", () => {
    // The one overlapping facet has to declare itself, or a reader adds it in.
    const rulable = stateOptions(counts).find((o) => o.facet === "rulable");
    expect(rulable?.hint).toContain("Part of Not ruled");
  });

  it("leads with Proposed and drops the Scan facets entirely", () => {
    // Proposed comes FIRST because it is what the queue opens on when a scan has
    // run, and it is the work the page exists to get through. The Scan dropdown is
    // gone with the facet that measured merges rather than scans.
    expect(stateOptions(counts).map((o) => o.facet)).toEqual([
      "proposed",
      "all",
      "not_ruled",
      "rulable",
      "deferred",
      "included",
      "excluded",
    ]);
  });
});

// ─── Filtering ──────────────────────────────────────────────────────────────

describe("filterCandidates", () => {
  const pool = [card(), deferOnly(), included(), excluded(), parked(), neverScanned()];

  function shown(filters: CandidateFilters): number {
    return filterCandidates(pool, filters).length;
  }

  it("shows everything when nothing is filtered", () => {
    expect(shown(UNFILTERED)).toBe(pool.length);
  });

  it("each state facet shows exactly what its option counted", () => {
    const counts = candidateCounts(pool);
    expect(shown({ state: "not_ruled" })).toBe(counts.not_ruled);
    expect(shown({ state: "rulable" })).toBe(counts.rulable);
    expect(shown({ state: "included" })).toBe(counts.included);
    expect(shown({ state: "excluded" })).toBe(counts.excluded);
    expect(shown({ state: "deferred" })).toBe(counts.deferred);
  });

  it("shows exactly the proposed cards under Proposed", () => {
    const withProposals = [...pool, proposed(), proposed()];
    const only = filterCandidates(withProposals, { state: "proposed" });
    expect(only).toHaveLength(2);
    expect(only.every((c) => c.proposed != null)).toBe(true);
  });

  it("preserves the payload's order — the browser re-sorts nothing", () => {
    // The backend sorts by C-ordinal and says why (`sort_by_code`). Re-deriving
    // that order here would be the client taking a display string apart.
    const order = filterCandidates(pool, UNFILTERED).map((c) => c.graph_node_id);
    expect(order).toEqual(pool.map((c) => c.graph_node_id));
  });

  it("returns an empty list rather than everything when a facet matches nothing", () => {
    // The failure mode worth guarding: a filter that silently degrades to "show
    // all" would look like a working filter over a pool with no matches.
    expect(filterCandidates([included()], { state: "excluded" })).toEqual([]);
  });
});

describe("the default view", () => {
  it("queue_defaults_to_proposed_when_proposals_exist_and_keeps_the_computed_default_otherwise", () => {
    // Architect ruling R8, both branches in one test because the rule IS the pair.
    //
    // With proposals: they lead. They are what the human came to rule, and before
    // this they arrived buried in a pool of 148 with nothing marking them.
    expect(defaultFilters(candidateCounts([proposed(), card(), included()]))).toEqual({
      state: "proposed",
    });

    // Without: the 1.7E computed default STANDS. A scanned-and-fully-ruled
    // scenario must not open on all 148 — that is the wall this page spent three
    // tasks removing.
    expect(defaultFilters(candidateCounts([card(), included()]))).toEqual({ state: "rulable" });
    expect(defaultFilters(candidateCounts([deferOnly(), included()]))).toEqual({
      state: "not_ruled",
    });
  });
});

describe("hasAnyFilter", () => {
  it("is false only for the untouched bar", () => {
    expect(hasAnyFilter(UNFILTERED)).toBe(false);
    expect(hasAnyFilter({ state: "rulable" })).toBe(true);
    expect(hasAnyFilter({ state: "proposed" })).toBe(true);
  });
});

describe("the counter line", () => {
  it("names the total pool, not the filtered view", () => {
    expect(candidateCounterLine(24, 148, { state: "rulable" })).toBe(
      "Filtered: 24 of 148 candidates",
    );
  });

  it("reads 'Showing all' only with the bar untouched", () => {
    expect(candidateCounterLine(148, 148, UNFILTERED)).toBe("Showing all 148 candidates");
  });
});

// ─── The state chip ─────────────────────────────────────────────────────────

describe("stateChip", () => {
  it("gives every state an icon AND a word, never colour alone", () => {
    // A greyscale print, a colour-vision difference, and a screenshot at a
    // distance all have to survive the loss of hue (§2c).
    for (const state of ["not_ruled", "included", "excluded", "deferred"] as const) {
      const chip = stateChip(state);
      expect(chip.icon.length, `${state} needs an icon`).toBeGreaterThan(0);
      expect(chip.label.length, `${state} needs a word`).toBeGreaterThan(0);
    }
  });

  it("gives the four states four distinct words and four distinct tones", () => {
    const chips = (["not_ruled", "included", "excluded", "deferred"] as const).map(stateChip);
    expect(new Set(chips.map((c) => c.label)).size).toBe(4);
    expect(new Set(chips.map((c) => c.tone)).size).toBe(4);
  });

  it("says Deferred rather than the payload's 'Not yet decided'", () => {
    // A deferred row's status IS `undecided`, so `status_label` reads "Not yet
    // decided" — the one thing the chip must not say about a card a human
    // deliberately parked.
    expect(stateChip(candidateState(parked())).label).toBe("Deferred");
  });
});
