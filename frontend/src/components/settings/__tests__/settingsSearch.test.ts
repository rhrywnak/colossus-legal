// =============================================================================
// The four states' arithmetic
// =============================================================================
//
// One mutation-proved assertion per state of ADMIN_SETTINGS_MOCKUP_v3:
//
//   landing    the list is "differs from its default", and nothing else
//   area       the rail's counts come off the wire and are never recomputed
//   group open the pinned filter appears past forty rows and not at forty
//   search     the cap is fifty, and the TRUE total is reported alongside it
//
// Every one of these is a number on screen that would be wrong quietly. A cap
// that silently dropped the fifty-first row, or a threshold off by one, looks
// exactly like a page working correctly.

import { describe, expect, it } from "vitest";

import type { AreaDto, SettingDto } from "../../../services/settings";
import {
  changedFromDefault,
  filterWithin,
  GROUP_FILTER_THRESHOLD,
  highlight,
  matches,
  needsGroupFilter,
  normaliseQuery,
  searchSettings,
  SEARCH_RESULT_CAP,
  splitBySpend,
} from "../settingsSearch";

function setting(over: Partial<SettingDto> & { key: string }): SettingDto {
  return {
    value: "x",
    default_value: "x",
    meaning: "What this parameter does.",
    input_hint: "free text",
    bounds_label: null,
    dormant_note: null,
    last_changed: "Never changed since it shipped",
    area_id: "practice",
    block_id: "practice_report",
    changed_from_default: null,
    group_id: null,
    ...over,
  };
}

function area(id: string, label: string, count: number): AreaDto {
  return { id, label, count, note: null, blocks: [] };
}

const AREAS: AreaDto[] = [
  area("practice", "Practice", 313),
  area("war_room", "War Room", 46),
  area("undeclared", "Undeclared — read by nothing", 13),
];

// ── STATE 1 · LANDING ────────────────────────────────────────────────────────

describe("the landing list", () => {
  it("is exactly the rows the server marked as changed (M)", () => {
    // MUTATION: filter on `value !== default_value` instead of on the server's
    // field and this still passes — which is why the third row exists.
    const rows = [
      setting({ key: "a", value: "5", default_value: "3", changed_from_default: "Changed — default: 3" }),
      setting({ key: "b", value: "3", default_value: "3" }),
      // The trap: the strings differ, but the SERVER did not call it changed.
      // Only the server knows how it normalises a stored value, so only the
      // server's answer counts.
      setting({ key: "c", value: " 3", default_value: "3" }),
    ];

    expect(changedFromDefault(rows).map((row) => row.key)).toEqual(["a"]);
  });

  it("is empty when every row sits on its default", () => {
    expect(changedFromDefault([setting({ key: "a" })])).toEqual([]);
  });
});

// ── STATE 3 · ONE GROUP OPEN ─────────────────────────────────────────────────

describe("the pinned in-group filter", () => {
  it("appears past forty rows and not at forty (M)", () => {
    // MUTATION: `>=` instead of `>` and the forty-row case flips. The mockup is
    // explicit — "a pinned filter appears past 40" — and a group of exactly
    // forty is the one case a `>=` gets wrong.
    expect(needsGroupFilter(GROUP_FILTER_THRESHOLD)).toBe(false);
    expect(needsGroupFilter(GROUP_FILTER_THRESHOLD + 1)).toBe(true);
    expect(needsGroupFilter(0)).toBe(false);
  });

  it("narrows the group without capping it", () => {
    const rows = Array.from({ length: 60 }, (_, i) =>
      setting({ key: `practice_report_${i}`, meaning: i % 2 === 0 ? "even row" : "odd row" }),
    );

    // Uncapped on purpose: the group IS the cap, and a filter that hid its own
    // results would leave a human typing more to see less.
    expect(filterWithin(rows, "even").length).toBe(30);
    expect(filterWithin(rows, "").length).toBe(60);
  });
});

// ── STATE 4 · SEARCH ─────────────────────────────────────────────────────────

describe("search", () => {
  it("caps the rows it renders and still reports the true total (M)", () => {
    // MUTATION: report `shown` as the total and this goes red. A capped list
    // that says "50 matches" when there are 60 answers a different question
    // than the one that was asked.
    const rows = Array.from({ length: 60 }, (_, i) =>
      setting({ key: `practice_report_heading_${i}`, meaning: "heading over the read" }),
    );

    const results = searchSettings(rows, AREAS, "heading");

    expect(results.total).toBe(60);
    expect(results.shown).toBe(SEARCH_RESULT_CAP);
    expect(results.groups[0].matched).toBe(60);
    expect(results.groups[0].settings.length).toBe(SEARCH_RESULT_CAP);
  });

  it("groups by area in the rail's order, skipping areas with no match", () => {
    const rows = [
      setting({ key: "war_room_review_queue_owner_chip", area_id: "war_room", meaning: "the reviewer's name" }),
      setting({ key: "practice_reviewer_usernames", area_id: "practice", meaning: "who may press Done reviewing" }),
    ];

    const results = searchSettings(rows, AREAS, "reviewer");

    expect(results.total).toBe(2);
    expect(results.groups.map((g) => g.areaId)).toEqual(["practice", "war_room"]);
  });

  it("matches on the meaning, the key and the value alike", () => {
    const row = setting({ key: "practice_pill_george", value: "the defense", meaning: "The pill over the deck." });

    expect(matches(row, "pill")).toBe(true); // key
    expect(matches(row, "deck")).toBe(true); // meaning
    expect(matches(row, "defense")).toBe(true); // value
    expect(matches(row, "rehearsal")).toBe(false);
  });

  it("spends the cap across groups in order rather than per group", () => {
    const rows = [
      ...Array.from({ length: 49 }, (_, i) => setting({ key: `practice_${i}`, meaning: "shared word" })),
      ...Array.from({ length: 5 }, (_, i) =>
        setting({ key: `war_room_${i}`, area_id: "war_room", meaning: "shared word" }),
      ),
    ];

    const results = searchSettings(rows, AREAS, "shared");

    expect(results.total).toBe(54);
    expect(results.shown).toBe(SEARCH_RESULT_CAP);
    expect(results.groups[0].settings.length).toBe(49);
    expect(results.groups[1].settings.length).toBe(1);
    // The truncated group still tells the truth about itself.
    expect(results.groups[1].matched).toBe(5);
  });

  it("names the areas the cap never reached rather than heading them empty (M)", () => {
    // MUTATION: drop the split and render `results.groups` whole — the page then
    // shows ten headings with no rows under them, which is what the live store
    // actually did when "the" matched 837 rows and Practice took all fifty.
    const rows = [
      ...Array.from({ length: 60 }, (_, i) => setting({ key: `practice_${i}`, meaning: "the word" })),
      ...Array.from({ length: 9 }, (_, i) =>
        setting({ key: `war_room_${i}`, area_id: "war_room", meaning: "the word" }),
      ),
    ];

    const spend = splitBySpend(searchSettings(rows, AREAS, "the"));

    expect(spend.rendered.map((g) => g.areaId)).toEqual(["practice"]);
    // Named, not hidden: the count survives even though no row of it is shown.
    expect(spend.alsoMatched.map((g) => [g.areaId, g.matched])).toEqual([
      ["war_room", 9],
    ]);
  });

  it("leaves nothing in the also-matched line when the cap covered everything", () => {
    const rows = [setting({ key: "a", meaning: "only one" })];
    expect(splitBySpend(searchSettings(rows, AREAS, "only")).alsoMatched).toEqual([]);
  });

  it("returns every row when the query is blank", () => {
    const rows = [setting({ key: "a" }), setting({ key: "b" })];
    expect(searchSettings(rows, AREAS, "   ").total).toBe(2);
  });
});

// ── Marking the matched run ──────────────────────────────────────────────────

describe("highlight", () => {
  it("splits a match out of the surrounding text, case-insensitively", () => {
    expect(highlight("The Reviewer's name", "reviewer")).toEqual([
      { text: "The ", hit: false },
      { text: "Reviewer", hit: true },
      { text: "'s name", hit: false },
    ]);
  });

  it("marks every occurrence, not only the first", () => {
    expect(highlight("ab ab", "ab").filter((s) => s.hit).length).toBe(2);
  });

  it("returns the text untouched when nothing matches or nothing is typed", () => {
    expect(highlight("plain", "zzz")).toEqual([{ text: "plain", hit: false }]);
    expect(highlight("plain", "")).toEqual([{ text: "plain", hit: false }]);
  });

  it("preserves the original text exactly when rejoined", () => {
    // The page renders these segments in order; if they did not rejoin to the
    // source, the search would silently rewrite what a setting says.
    const source = "Who may press “Done reviewing” — the reviewers";
    expect(highlight(source, "review").map((s) => s.text).join("")).toBe(source);
  });
});

// ── The query itself ─────────────────────────────────────────────────────────

describe("normaliseQuery", () => {
  it("lowers and trims what was typed", () => {
    // Raised by the test-auditor gate: every other test in this file types a
    // lowercase query, so a `normaliseQuery` that had stopped lowering would
    // pass all of them. A human typing "Reviewer" is the untested direction.
    expect(normaliseQuery("  Reviewer  ")).toBe("reviewer");
    expect(normaliseQuery("   ")).toBe("");
  });

  it("matches a mixed-case query against a lowercase field", () => {
    const row = setting({ key: "practice_reviewer_usernames", meaning: "who may sign off" });
    expect(matches(row, normaliseQuery("REVIEWER"))).toBe(true);
  });
});
