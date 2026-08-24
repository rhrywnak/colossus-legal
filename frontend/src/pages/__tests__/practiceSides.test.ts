// =============================================================================
// practiceSides.test.ts — one side at a time, in the authored order (v8)
// =============================================================================
//
// ## The fixture is S-5 AS IT STANDS ON DEV, measured 2026-08-23
//
// Not invented. `sort_order` 1–15, read from `practice_questions` on the DEV
// pipeline database, and it carries a fact no invented fixture would have
// produced: **the deck order is g3, g4, g2, g1, g5** — the authored order is
// deliberately NOT the key numbering. A test built on a tidy g1…g5 would pass
// against an implementation that sorted by key, which is wrong, and would never
// have noticed.
//
// ## ⚑ THE ARRAY ORDER IS THE CONTRACT
//
// `PracticeQuestion` carries no `sort_order` field — the wire has none, so the
// browser CANNOT re-derive the deck order. `list_deck`'s `ORDER BY sort_order`
// is the single authority and the array as received is the sequence. That is
// why these tests pin SEQUENCES and never counts: a count assertion would go on
// passing if the server's ORDER BY were dropped tomorrow, and the screen would
// quietly deal Marie her questions in whatever order Postgres felt like.
//
// Proved by mutation on 2026-08-23: shuffling the fixture turns every sequence
// assertion below RED. See the report for the transcript.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { antecedentOf } from "../../components/practice/printSheetPlan";

import { orderedDeck, sideSections } from "../practiceQueue";
import type { PracticeQuestion } from "../../services/practice";

/** One measured row, spelled out with only the fields the ordering reads. */
const q = (
  deck_key: string,
  side: "george" | "chuck",
  kind: string,
  follows_key: string | null,
): PracticeQuestion =>
  ({
    id: deck_key,
    deck_key,
    side,
    kind,
    follows_key,
    text: `${deck_key} — the question as authored`,
    hidden: false,
  }) as unknown as PracticeQuestion;

/** S-5 in `sort_order`, exactly as DEV holds it. */
const S5: PracticeQuestion[] = [
  q("g3", "george", "cross", null),
  q("g4", "george", "cross", null),
  q("g2", "george", "cross", null),
  q("g1", "george", "cross", null),
  q("g5", "george", "cross", null),
  q("c1", "chuck", "direct", null),
  q("c2", "chuck", "direct", null),
  q("c3", "chuck", "direct", null),
  q("c4", "chuck", "direct", null),
  q("c5", "chuck", "direct", null),
  q("r1", "chuck", "redirect", "g1"),
  q("r2", "chuck", "redirect", "g2"),
  q("r3", "chuck", "redirect", "g3"),
  q("r4", "chuck", "redirect", "g4"),
  q("r5", "chuck", "redirect", "g5"),
];

/** The rows a side shows, flattened in reading order. */
const shown = (deck: PracticeQuestion[], side: "george" | "chuck") =>
  sideSections(deck, side).flatMap((part) => part.questions.map((x) => x.id));

describe("sideSections — one side at a time", () => {
  it("shows the defense's cross in the authored deck order, and NOTHING else", () => {
    // The whole sequence, pinned. Note g3 first: the authored order is not the
    // key order, so this fails against a sort as well as against an interleave.
    expect(shown(S5, "george")).toEqual(["g3", "g4", "g2", "g1", "g5"]);
  });

  it("shows Chuck's directs then his redirects, each run in deck order", () => {
    expect(shown(S5, "chuck")).toEqual([
      "c1",
      "c2",
      "c3",
      "c4",
      "c5",
      "r1",
      "r2",
      "r3",
      "r4",
      "r5",
    ]);
  });

  it("renders no question from the other side, in either direction", () => {
    // Stated as the ABSENCE it is, rather than by counting: the defect this
    // guards is a Chuck question appearing in Marie's cross, and a count would
    // still pass if one were swapped for another.
    for (const id of shown(S5, "george")) expect(id.startsWith("g")).toBe(true);
    for (const id of shown(S5, "chuck")) expect(id.startsWith("g")).toBe(false);
    expect(shown(S5, "george")).not.toContain("c1");
    expect(shown(S5, "chuck")).not.toContain("g1");
  });

  it("labels Chuck's two runs and leaves the defense's single run unlabelled", () => {
    expect(sideSections(S5, "chuck").map((part) => part.labelKey)).toEqual([
      "directs_subheader",
      "redirects_subheader",
    ]);
    expect(sideSections(S5, "george").map((part) => part.labelKey)).toEqual([null]);
  });

  it("withholds the heading when a side holds only one kind", () => {
    // A heading above the only section on screen tells a reader that a second,
    // different section exists somewhere — which would be false.
    const directsOnly = S5.filter((x) => x.kind !== "redirect");
    expect(sideSections(directsOnly, "chuck").map((part) => part.labelKey)).toEqual([null]);
    const redirectsOnly = S5.filter((x) => x.kind === "redirect");
    expect(sideSections(redirectsOnly, "chuck").map((part) => part.labelKey)).toEqual([null]);
  });

  it("groups by kind even when the deck authors a redirect early", () => {
    // Chuck's openings must not be scattered through his repairs. No deck
    // authored so far does this, which is exactly why it is pinned: the
    // grouping is invisible on today's data and would rot unnoticed.
    const scrambled = [S5[10], S5[5], S5[11], S5[6]]; // r1, c1, r2, c2
    expect(shown(scrambled, "chuck")).toEqual(["c1", "c2", "r1", "r2"]);
  });

  it("drops a deleted question from the side it was on", () => {
    const withHidden = S5.map((x) =>
      x.deck_key === "g2" ? ({ ...x, hidden: true } as PracticeQuestion) : x,
    );
    expect(shown(withHidden, "george")).toEqual(["g3", "g4", "g1", "g5"]);
  });

  it("gives the picker both counts off the whole deck, not the side showing", () => {
    expect(orderedDeck(S5, "george")).toHaveLength(5);
    expect(orderedDeck(S5, "chuck")).toHaveLength(10);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Reachability — the ordering above is only worth anything if it is RENDERED
// ─────────────────────────────────────────────────────────────────────────────
//
// No jsdom and no `@testing-library/*` (CLAUDE.md rule 30), so these are source
// scans: they prove the list wires the right component and reads the right
// helper. Precedent and its stated limit: `onePageSurface.test.ts`.
//
// They exist because of `QuestionLine` — built, tested, and rendered by nothing
// for a fortnight. A pure ordering function with a green test and no call site
// is the same failure wearing a different name.

describe("the list renders what these functions decide", () => {
  const source = readFileSync(
    join(__dirname, "..", "..", "components", "practice", "PracticeDeckList.tsx"),
    "utf8",
  );
  // ⚑ JSX OPENING TAGS ONLY, never a raw substring search. This file's own
  // comments name every component it draws, and a substring scan would find
  // them there and call a deleted component "rendered". `//` stripping is not
  // enough — these are `{/* … */}` blocks, which is the same trap that made the
  // wording scanner read an example key out of a comment earlier today.
  const tags = source.match(/<[a-zA-Z][^>]*>/g) ?? [];
  // The tag NAME, ended. `startsWith("<PrintAntecedent")` was the first version
  // and it was vacuous: it matched `<PrintAntecedentXX` too, so the mutation
  // that renames the component away stayed green. A name must end at a space, a
  // newline, a `/` or a `>` to count as that component being drawn.
  const draws = (tag: string) =>
    tags.some((t) => new RegExp(`^<${tag}(\\s|/|>)`).test(t));

  it("draws the side picker and the shared antecedent", () => {
    expect(draws("PracticeSidePicker")).toBe(true);
    expect(draws("PrintAntecedent")).toBe(true);
  });

  it("gets its order from sideSections and not from the array it was handed", () => {
    expect(source).toContain("sideSections(questions, side)");
  });

  it("resolves the antecedent against the whole deck, not the side showing", () => {
    // A redirect's target is a DEFENSE question — always on the other side. If
    // the lookup pool were this side's rows, every antecedent on screen would
    // render as "no longer in the deck".
    expect(source).toContain("antecedentOf(question, visible)");
  });

  it("no longer names the interleave", () => {
    expect(source).not.toContain('"mixed"');
    expect(source).not.toContain("deck_count_template");
  });
});

describe("a redirect resolves to the defense question it repairs", () => {
  it("finds it BY KEY, across the side boundary", () => {
    // r3 follows g3, which sits FIRST in the deck while r3 sits third among the
    // redirects. Pairing by position would hand r3 the text of g2.
    const found = antecedentOf(S5[12], S5);
    expect(found).toEqual({ kind: "resolved", antecedent: S5[0] });
    expect(found?.kind === "resolved" && found.antecedent.deck_key).toBe("g3");
  });

  it("says so plainly when the question it repairs has been deleted", () => {
    // Silence would leave Chuck judging a redirect as though it stood alone,
    // which is the one judgement a redirect cannot survive.
    const withoutG3 = S5.filter((x) => x.deck_key !== "g3");
    expect(antecedentOf(S5[12], withoutG3)).toEqual({ kind: "missing" });
  });
});
