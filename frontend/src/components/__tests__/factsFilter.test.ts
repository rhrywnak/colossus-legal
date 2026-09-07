/**
 * The Scenario facts list's three filters and their counts (v2.1, change D).
 *
 * Pure helpers only (CLAUDE.md Rule 30). The counts are exactly the class of
 * value that has gone wrong on this page before: .374 put "No candidates
 * gathered yet" over a pool of 148 by collapsing "not read yet" into zero, and
 * the `null` state below is what keeps those two apart.
 */
import { describe, expect, it } from "vitest";

import {
  FACTS_FILTERS,
  factsCounts,
  showsCandidates,
  showsIncluded,
} from "../factsFilter";
import type { ScenarioCard } from "../../services/scenarioCards";

/**
 * One card, thin: only the fields the two counters read.
 *
 * `proposed` is the one that matters and it is a PRESENCE, not a flag: a card
 * carries a proposal object when the latest completed scan put it forward, and
 * absence is what "nothing proposed this" means. There is no "proposed" status
 * token to set — see `isProposed` — so a fixture that set one would be testing
 * a model the payload does not have.
 */
function card(
  status: ScenarioCard["status"],
  proposed: boolean = false,
): ScenarioCard {
  return {
    graph_node_id: `n-${Math.random()}`,
    status,
    confidence: { band: "high", label: "high" },
    quote: { text: "x" },
    pinpoint: { label: "p", viewer_href: "", document_id: "d", page: 1 },
    speaker: { name: null, attribution: "extracted" },
    bears_on: [],
    human_links: [],
    card: null,
    ...(proposed ? { proposed: { run_id: "r1", reason: "because" } } : {}),
  } as unknown as ScenarioCard;
}

describe("factsCounts", () => {
  it("matches the live S-7 arithmetic the mockup was drawn from", () => {
    // Measured on DEV 2026-09-07 through the real payload: 15 included cards,
    // 34 carrying a live proposal. The mockup's three pills read 15 / 34 / 49,
    // and 49 is what pins the Included column to the evidence rather than to
    // the list's row count. This is the fence on that reading.
    const cards = [
      ...Array.from({ length: 15 }, () => card("included")),
      ...Array.from({ length: 34 }, () => card("undecided", true)),
    ];
    expect(factsCounts(cards)).toEqual({ included: 15, candidates: 34, all: 49 });
  });

  it("reports NOTHING while the pool is unread — not three zeroes", () => {
    // The .374 defect, in one assertion. A pill showing `0` before its fetch
    // lands is a claim about a scenario nobody has read yet.
    expect(factsCounts(null)).toEqual({
      included: null,
      candidates: null,
      all: null,
    });
  });

  it("counts the ruled-in EVIDENCE only, not the hand-written facts", () => {
    // The pill reads 15 over a list of 22 rows on S-7 today, and that is the
    // mockup's own arithmetic (15 + 34 = 49). "Included" names a RULING; a
    // hand-written fact was never ruled in (§8 — authored, uncited, never
    // machine-touched), so counting it under that word would say something
    // false about how it got there. Every such row also says so on its face.
    // See the module header, and the DEV measurement recorded there.
    const cards = [card("included"), card("included")];
    expect(factsCounts(cards).included).toBe(2);
  });

  it("reports three zeroes for a genuinely empty scenario", () => {
    // The other side of the same distinction: read, and empty. This one IS a
    // number, and a new scenario should say so rather than showing bare words.
    expect(factsCounts([])).toEqual({ included: 0, candidates: 0, all: 0 });
  });

  it("counts an undecided card under neither Included nor the sum", () => {
    // The pool's raw evidence is not content and, unproposed, not work waiting
    // either. It belongs to no bucket.
    const cards = [card("included"), card("included"), card("undecided")];
    const counts = factsCounts(cards);
    expect(counts.included).toBe(2);
    expect(counts.candidates).toBe(0);
    expect(counts.all).toBe(2);
  });

  it("counts only PROPOSED cards as Candidates, not the whole raw pool", () => {
    // A candidate is what a completed scan is putting forward. An undecided card
    // nothing proposed is raw evidence sitting in the pool, and counting it
    // would put a freshly created scenario's untouched pool behind a
    // "Candidates" pill — the defect 2.15 piece 3a was written to prevent, in a
    // new place.
    const cards = [
      card("undecided", true),
      card("undecided", true),
      card("undecided", false),
    ];
    expect(factsCounts(cards).candidates).toBe(2);
  });

  it("never counts a dropped card anywhere", () => {
    // A set-aside fact is a judgment already made. It is not content and it is
    // not work waiting, so it belongs to neither bucket — and therefore not to
    // All either.
    const cards = [card("dropped"), card("dropped")];
    expect(factsCounts(cards)).toEqual({ included: 0, candidates: 0, all: 0 });
  });

  it("makes All the exact sum of the other two", () => {
    // The buckets are disjoint by construction, so the sum is exact. Counting
    // "everything the two lists would show" a third way would be a third place
    // for the arithmetic to be wrong — and the number nobody checks is the one
    // that drifts.
    const cards = [
      card("included"),
      card("undecided", true),
      card("undecided", true),
      card("dropped"),
    ];
    const counts = factsCounts(cards);
    expect(counts.included).toBe(1);
    expect(counts.candidates).toBe(2);
    expect(counts.all).toBe(3);
  });
});

describe("which lists each filter shows", () => {
  it("shows the scenario's own content under Included", () => {
    expect(showsIncluded("included")).toBe(true);
    expect(showsCandidates("included")).toBe(false);
  });

  it("shows the queue under Candidates", () => {
    expect(showsIncluded("candidates")).toBe(false);
    expect(showsCandidates("candidates")).toBe(true);
  });

  it("shows both under All — which is the point of having it", () => {
    // "Did I already rule on this?" is asked across the two, and answering it
    // used to mean scrolling between two sections.
    expect(showsIncluded("all")).toBe(true);
    expect(showsCandidates("all")).toBe(true);
  });

  it("leaves no filter showing nothing at all", () => {
    // A filter that renders neither list would be a blank card with a lit pill
    // above it — indistinguishable from a failed read, and there would be
    // nothing on screen to say which.
    for (const filter of FACTS_FILTERS) {
      expect(showsIncluded(filter) || showsCandidates(filter)).toBe(true);
    }
  });

  it("draws the three pills in the order the mockup does", () => {
    expect(FACTS_FILTERS).toEqual(["included", "candidates", "all"]);
  });
});
