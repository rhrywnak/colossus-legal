// =============================================================================
// candidateFilters.test.ts — the filter model and its §9 counts (task 1.7E-a)
// =============================================================================
//
// The headline tests are the RECONCILIATION ones: the four state counts sum to
// All, and "Rulable now" and "Proposed" are each a SUBSET of "Not ruled" rather
// than a state of their own. §9's promise is not that the numbers look plausible —
// it is that they add up, and a filter bar that quietly double-counts is the
// defect this task exists to remove.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  candidateCounts,
  candidateState,
  defaultFilters,
  facetLabel,
  filterCandidates,
  hasAnyFilter,
  matchesFilter,
  isProposed,
  isRulableNow,
  stateChip,
  filterChips,
  filterProgress,
  UNFILTERED,
  type CandidateFilters,
} from "../candidateFilters";
import type { ScenarioCard } from "../../services/scenarioCards";
import { optionsFixture } from "./cardFixtures";

/** The stored words the chips are named from (ruling R6). */
const grammar = optionsFixture().card_grammar;

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
    // Ruling R3: the judge's SENTENCE is no longer part of being proposed — it
    // rides on the card, so that including the card does not delete it.
    scan_reason: "direct admission by the accusing party",
    proposed: {
      role_label: "Scan: supports",
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

  it("agrees with the chips it feeds", () => {
    // The chips read the same derivation — this asserts nobody has slipped a
    // second count in between (ruling R1, the half that survives every redesign
    // of the control).
    const chips = filterChips(counts, grammar);
    expect(chips.find((c) => c.facet === "full_pool")?.count).toBe(counts.all);
    expect(chips.find((c) => c.facet === "proposed")?.count).toBe(counts.proposed);
    expect(chips.find((c) => c.facet === "included")?.count).toBe(counts.included);
  });

  it("every chip carries its count on its FACE (ruling R3, kept)", () => {
    // 1.7E's R1 declined selects because "a count inside a closed dropdown is a
    // count nobody can see". R3 overruled it on the condition that the counts
    // travel with the options; the chips satisfy that condition more directly
    // than the select did, and a chip with no count is the regression both
    // rulings were worried about.
    for (const chip of filterChips(counts, grammar)) {
      expect(chip.count, `${chip.label} has no count`).toBeTypeOf("number");
    }
  });

  it("names every chip from the STORE, never from a literal here", () => {
    // A filter name is case-facing vocabulary — Marie and Chuck read it before
    // anything else on the page — so it is a row, and this is what proves the
    // control reads the row rather than a compiled-in word (ruling R6).
    const chips = filterChips(counts, grammar);
    expect(chips.map((c) => c.label)).toEqual([
      grammar.filter_proposed_label,
      grammar.filter_deferred_label,
      grammar.filter_included_label,
      grammar.filter_excluded_label,
      grammar.filter_full_pool_label,
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

  it("each chip shows exactly what it counted", () => {
    const counts = candidateCounts(pool);
    expect(shown({ state: "full_pool" })).toBe(counts.all);
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

// The R8 "default view" suite that stood here is SUPERSEDED by ruling R5, not
// deleted quietly: it pinned "Rulable now while any exist, else Not ruled", and
// both of those facets are gone. Its replacement is
// `rulable_now_is_gone_and_default_follows_proposals` further down, which keeps
// the half R5 preserved verbatim — proposals lead — and states the new fallback.
// One test rather than two, because two tests asserting one rule is how they end
// up disagreeing about it.

describe("hasAnyFilter", () => {
  it("is false only for the full pool", () => {
    // The full pool IS the unfiltered view — it is the denominator every other
    // chip is a slice of, which is exactly what its ⓘ says.
    expect(hasAnyFilter(UNFILTERED)).toBe(false);
    expect(hasAnyFilter({ state: "full_pool" })).toBe(false);
    expect(hasAnyFilter({ state: "proposed" })).toBe(true);
    expect(hasAnyFilter({ state: "included" })).toBe(true);
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


// ─── What the QUEUE asks about one card (2026-08-08) ────────────────────────
//
// Both of these feed the ruling acknowledgment: after a ruling the queue asks
// whether the card is still in the list, and if it is not, names the list it
// left. The measured defect behind them is the vanish — on beta.385 a ruled card
// silently left the Proposed filter and the click read as a dead button.

describe("matchesFilter", () => {
  it("gives the same answer the list itself computes", () => {
    // The property that matters: the queue's per-card question and the list's
    // own filtering must never disagree, or the acknowledgment would announce a
    // card leaving a list it is still in (or say nothing while it vanishes).
    const pool = [card(), deferOnly(), included(), excluded(), parked(), proposed()];

    for (const state of [
      "full_pool",
      "proposed",
      "deferred",
      "included",
      "excluded",
    ] as const) {
      const filters = { state };
      const byList = filterCandidates(pool, filters).map((c) => c.graph_node_id);
      const byPredicate = pool.filter((c) => matchesFilter(c, filters)).map((c) => c.graph_node_id);
      expect(byPredicate, `the two disagree under "${state}"`).toEqual(byList);
    }
  });

  it("is what tells the queue a ruled card has LEFT the proposed list", () => {
    // The vanish, as the queue detects it: a proposed card matches, and the same
    // card once ruled does not — which is the moment the acknowledgment has to
    // say where it went.
    const before = proposed();
    const after = { ...before, status: "included" as const, proposed: undefined };

    expect(matchesFilter(before, { state: "proposed" })).toBe(true);
    expect(matchesFilter(after, { state: "proposed" })).toBe(false);
  });
});

describe("facetLabel", () => {
  it("names each facet exactly as its own chip does", () => {
    // The sentence says "C-14 has left the Proposed list", and the chip above it
    // says "Proposed". Two names for one list is the §9 defect in words rather
    // than in numbers — so the label is READ from the chip table the bar
    // renders, never written out a second time.
    const counts = candidateCounts([card()]);
    for (const chip of filterChips(counts, grammar)) {
      expect(facetLabel(chip.facet, grammar)).toBe(chip.label);
    }
  });

  it("falls back to the facet key rather than an empty string", () => {
    // An unknown facet cannot happen through the typed API, and if it ever did,
    // "has left the  list" is a sentence with a hole in it. The key is ugly and
    // it is legible, which is the right trade for an impossible branch.
    expect(facetLabel("something_new" as never, grammar)).toBe("something_new");
  });
});

// ── Ruling R5: the five chips, and what the queue opens on ──────────────────

describe("the filter surface is five chips (ruling R5)", () => {
  it("offers exactly the five facets, in the ruled order", () => {
    // Proposed LEADS because it is the work the page exists to get through.
    // Full pool is LAST because it is the denominator rather than a slice.
    const chips = filterChips(candidateCounts([card()]), grammar);
    expect(chips.map((c) => c.facet)).toEqual([
      "proposed",
      "deferred",
      "included",
      "excluded",
      "full_pool",
    ]);
  });

  it("carries the explainer on the full pool and nowhere else", () => {
    // Roman's addition. Marie and Chuck will not know the term, and a filter
    // whose meaning a reader has to infer from its count is one they will not
    // press. The other four name themselves.
    const chips = filterChips(candidateCounts([card()]), grammar);
    const explained = chips.filter((c) => c.explainer !== undefined);
    expect(explained.map((c) => c.facet)).toEqual(["full_pool"]);
  });

  it("rulable_now is gone and the default follows proposals", () => {
    // Two of six options used to be subsets of a third, and the bar made a human
    // read a taxonomy before a count. A locked card now rides INSIDE Proposed,
    // stating its own condition with the type-ahead ready (Piece 4b).
    const withProposals = candidateCounts([proposed(), card()]);
    expect(defaultFilters(withProposals)).toEqual({ state: "proposed" });

    // And with nothing proposed, the honest opening view is the DENOMINATOR.
    // The old fallback led with a slice the human had not chosen and could not
    // see the edges of.
    const none = candidateCounts([card()]);
    expect(defaultFilters(none)).toEqual({ state: "full_pool" });
  });

  it("a locked card is inside Proposed rather than sorted out of it", () => {
    // The whole reason "Rulable now" could retire: `defer_required` no longer
    // decides which LIST a card appears in, only what its own face says.
    const locked = { ...proposed(), defer_required: true };
    expect(matchesFilter(locked, { state: "proposed" })).toBe(true);
  });
});

// ── The progress line: the proposed bucket, and Defer counts ───────────────

describe("which filter the queue opens on", () => {
  it("lands on INCLUDED once the proposals are cleared but inclusions remain", () => {
    // The middle branch, added in .391 and superseding ruling R5's `full_pool`
    // fallback. A scenario whose proposals are all ruled is not waiting to be
    // triaged — it has been built — and what its author wants to see is what they
    // built. Untested, a one-character slip in this branch would silently restore
    // the old behaviour and drop them into a pool of 148 they stopped caring
    // about, with every other test still green.
    expect(defaultFilters(candidateCounts([included()]))).toEqual({ state: "included" });
  });

  it("still leads with PROPOSED when a scan is proposing anything", () => {
    // Unchanged, and it must stay first: proposals are what the human came for.
    expect(defaultFilters(candidateCounts([proposed(), included()]))).toEqual({
      state: "proposed",
    });
  });

  it("falls back to the FULL POOL only when there is nothing else to lead with", () => {
    // Nothing proposed and nothing included. An empty Included list would be a
    // filtered view of nothing — the exact failure R5 named, reached from the
    // other side — so the denominator is the honest answer here.
    expect(defaultFilters(candidateCounts([card()]))).toEqual({ state: "full_pool" });
  });
});

describe("progress measures the proposed bucket, not the view", () => {
  it("counts the proposed bucket: what a scan put forward, plus what was acted on", () => {
    // Roman's ruling, 2026-08-10. Piece 1c measured the ACTIVE FILTER, which
    // fixed the original defect ("23 of 148" over a bar nobody owed) and
    // introduced a subtler one: the number moved when a human clicked a chip.
    //
    // The bucket is fixed instead. Two proposals still waiting, three cards
    // already ruled — five in the denominator, three addressed.
    const pool = [
      included(),
      excluded(),
      proposed(),
      proposed(),
      card({ status: "included" }),
    ];
    expect(filterProgress(pool)).toEqual({ ruled: 3, total: 5 });
  });

  it("no longer labels the sentence with a filter name — pinned at the render site", () => {
    // `{filter}` left the stored template with the rule it named. `fillSlots`
    // leaves an unrecognised slot VERBATIM, so a call site still passing `filter`
    // would render "3 of 5 addressed" correctly while a template that still had
    // the slot would render "3 of 5 {filter} ruled" — the failure is silent in
    // both directions, which is why it is pinned rather than trusted.
    const bar = readFileSync(join(__dirname, "..", "CandidateFilterBar.tsx"), "utf8");
    expect(bar).toContain("filter_progress_template");
    expect(bar).not.toMatch(/filter:\s*active\.label/);
  });

  it("is the QUEUE that must hand it every card — pinned at the call site", () => {
    // The filter-independence Roman ruled for is a property of the CALL, not of
    // this function: hand it a filtered list and it will faithfully measure the
    // filtered list. `CardQueue` therefore has to pass the whole pool, and the
    // one-word edit back to `visible` would silently restore the moving number
    // with every test still green. So the call site is pinned.
    const queue = readFileSync(join(__dirname, "..", "CardQueue.tsx"), "utf8");
    expect(queue).toContain("progress={filterProgress(state.cards)}");
    expect(queue).not.toContain("filterProgress(visible)");
  });

  it("leaves the raw pool OUT of the denominator entirely", () => {
    // The full pool never appears in this line. An untouched card that no scan
    // proposed is raw evidence — it belongs to the Full pool chip's count, not to
    // a sentence about work done.
    const raw = card({ status: "undecided" });
    expect(filterProgress([raw])).toEqual({ ruled: 0, total: 0 });
    expect(filterProgress([included(), raw])).toEqual({ ruled: 1, total: 1 });
  });

  it("COUNTS a deferred card — Include, Exclude and Defer are all rulings", () => {
    // This reverses the rule the previous test asserted, on Roman's ruling of
    // 2026-08-10 (§7: I/E/D are all rulings). The old note argued a deferred card
    // is "parked, and the work of deciding it is still outstanding" — true of the
    // card, and wrong about the human, who pressed a button and had it recorded.
    // A line that ignored one of the three told a curator who worked fourteen
    // cards that they had worked four.
    expect(filterProgress([parked()])).toEqual({ ruled: 1, total: 1 });
    expect(filterProgress([included(), parked()])).toEqual({ ruled: 2, total: 2 });
  });

  it("reads N of N once the bucket is cleared", () => {
    // The sentence a human wants at the end of a session. On DEV's S-5 — 21
    // included, 1 deferred, nothing else proposed — this is "22 of 22".
    const cleared = [included(), included(), parked()];
    expect(filterProgress(cleared)).toEqual({ ruled: 3, total: 3 });
  });
});
