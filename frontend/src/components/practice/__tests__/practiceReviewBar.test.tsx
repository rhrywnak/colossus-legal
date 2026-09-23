// =============================================================================
// practiceReviewBar.test.tsx — the Done reviewing button is the reviewer's alone
// =============================================================================
//
// Markup both ways (CC_TASK_SIMPLE_COUNTS_v1). The server decides
// `can_mark_reviewed`; the bar renders the button on that boolean and nothing
// else. Mutation-proved in the report: dropping the guard reds the second test.
//
// ## What this file can and cannot say about the confirmation
//
// `renderToStaticMarkup` renders the FIRST frame. It can prove what the bar
// draws before anybody touches it — which is now the thing worth proving, since
// the defect was that the write happened with no question drawn at all. It
// cannot fire a click, so which actions may write is proved over the closed
// union of actions in `deckReviewConfirmModel.test.ts` instead.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import PracticeReviewBar, { reviewBarLine } from "../PracticeReviewBar";
import { deckReviewConfirmSentence } from "../deckReviewConfirmModel";
import type { DeckReview } from "../../../services/practice";

const wording = {
  // The sentences as migration 20260923071048 leaves them: the reader's own
  // number, addressed to the reader (ruled 2026-09-23).
  deck_review_awaiting_one: "{count} question awaiting your review",
  deck_review_awaiting_template: "{count} questions awaiting your review",
  deck_review_done_label: "Done reviewing",
  deck_review_failed: "Could not mark this deck reviewed — nothing was changed.",
  // CC_TASK_REVIEW_PAGE_v1: the bar's fifth string.
  deck_review_oldest_template: "· oldest waiting since {date}",
  // REVIEW_COUNTS_HONEST: the question the button now asks first.
  deck_review_confirm_template: "Mark all {count} answers in {code} as reviewed?",
  deck_review_confirm_one: "Mark the {count} answer in {code} as reviewed?",
  deck_review_confirm_yes_label: "Yes",
  deck_review_confirm_cancel_label: "Cancel",
};

function bar(review: DeckReview): string {
  return renderToStaticMarkup(
    <PracticeReviewBar
      slug="c"
      scenarioId="s"
      code="S-13"
      review={review}
      wording={wording}
      shown={[]}
      servedAt="2026-09-22T21:31:00Z"
      onReviewed={() => {}}
    />,
  );
}

describe("PracticeReviewBar", () => {
  it("offers Done reviewing to the reviewer (cpenzien's payload)", () => {
    const html = bar({ awaiting: 3, can_mark_reviewed: true });
    expect(html).toContain("3 questions awaiting your review");
    expect(html).toContain(">Done reviewing</button>");
  });

  it("(M) draws NOTHING for somebody who may not review", () => {
    // Ruled 2026-09-23. It used to draw the same count with no button — which
    // showed the witness a number about answers she had written, under a
    // sentence naming Chuck. Dropping the guard reds this test and nothing
    // else, which is why the assertion is the whole markup being empty rather
    // than the button being absent.
    expect(bar({ awaiting: 3, can_mark_reviewed: false })).toBe("");
  });

  it("names nobody in the sentence it does draw", () => {
    // The bench left the templates with the ruling; a build that put a name
    // back would be telling one reader about another reader's queue.
    const html = bar({ awaiting: 3, can_mark_reviewed: true });
    expect(html).not.toContain("Chuck");
    expect(html).not.toContain("{reviewer}");
  });

  it("draws nothing when nothing waits", () => {
    expect(bar({ awaiting: 0, can_mark_reviewed: true })).toBe("");
  });
});

// ── The oldest-waiting clause (CC_TASK_REVIEW_PAGE_v1, ruling STOP-A) ────────

describe("the oldest-waiting clause", () => {
  it("is appended when the server sent a date", () => {
    const line = reviewBarLine(
      { awaiting: 42, can_mark_reviewed: false, oldest: "20 Aug" },
      wording,
    );
    expect(line).toContain("42");
    expect(line).toContain("oldest waiting since 20 Aug");
  });

  it("is WITHHELD when the server sent no date", () => {
    // Filling `{date}` with an empty string would print a sentence that trails
    // off — which reads as a rendering fault rather than as an absent fact.
    const line = reviewBarLine(
      { awaiting: 3, can_mark_reviewed: false },
      wording,
    );
    expect(line).not.toContain("oldest");
    expect(line).not.toContain("{date}");
  });

  it("prints the date the server composed and formats nothing", () => {
    // The browser holds no date format. Whatever the server sent is what shows.
    const line = reviewBarLine(
      { awaiting: 1, can_mark_reviewed: true, oldest: "2 Mar" },
      wording,
    );
    expect(line).toContain("2 Mar");
  });

  it("fills the count and nothing else", () => {
    // `{reviewer}` was the other placeholder until 2026-09-23, and the bench is
    // no longer on this sentence at all. What remains must still be filled: a
    // template arriving with a raw `{count}` is the failure this asserts.
    const line = reviewBarLine(
      { awaiting: 5, can_mark_reviewed: true, oldest: "20 Aug" },
      wording,
    );
    expect(line).toContain("5 questions awaiting your review");
    expect(line).not.toContain("{");
  });
});

// ── The confirmation (CC_TASK_REVIEW_COUNTS_HONEST_v1, ruled 2026-09-20) ─────

describe("Done reviewing asks before it writes", () => {
  it("draws the button and NO question until it is pressed", () => {
    // The first frame is the whole claim here: the defect was a bar that wrote
    // without ever drawing a question, so "no question yet" and "a button that
    // is not a write" are the two halves of what shipped wrong.
    const html = bar({ awaiting: 12, can_mark_reviewed: true });
    expect(html).toContain(">Done reviewing</button>");
    expect(html).not.toContain("data-review-confirm");
    expect(html).not.toContain("as reviewed?");
    expect(html).not.toContain(">Yes</button>");
    expect(html).not.toContain(">Cancel</button>");
  });

  it("never offers the question to somebody who may not mark the deck", () => {
    // Now vacuously true of the markup — they get no bar at all — and kept for
    // what it still says: the question is not reachable without permission.
    const html = bar({ awaiting: 12, can_mark_reviewed: false });
    expect(html).not.toContain("<button");
    expect(html).not.toContain("as reviewed?");
  });
});

describe("the question the bar would ask", () => {
  // The sentence is composed by `deckReviewConfirmSentence`, which this file
  // reaches through the same two stored rows the bar passes it — so a template
  // renamed in the store and not in the bar fails HERE rather than on screen.
  const ask = (count: number, code: string) =>
    deckReviewConfirmSentence({
      state: { phase: "confirming", count },
      one: wording.deck_review_confirm_one,
      many: wording.deck_review_confirm_template,
      code,
    });

  it("names the count and the deck, with no placeholder left behind", () => {
    const sentence = ask(12, "S-13");
    expect(sentence).toBe("Mark all 12 answers in S-13 as reviewed?");
    expect(sentence).not.toContain("{");
  });

  it("reads its singular at exactly one waiting answer", () => {
    expect(ask(1, "S-13")).toBe("Mark the 1 answer in S-13 as reviewed?");
  });
});

// ── The machine actually BINDS the component ────────────────────────────────
//
// `deckReviewConfirmModel.test.ts` proves that no action but Yes returns
// `send`. That is only a proof about the BAR if the bar has no second way to
// write — and a reducer beside a component that also calls the service directly
// would pass both files and ship the original defect.
//
// There is no jsdom here to catch that, so this reads the source. It is the same
// structural-scan pattern `onePageSurface.test.ts` uses, with the same mandatory
// first step: strip `//` comments before searching. This codebase documents its
// rules next to its rules — the component's own comment says the words
// "markDeckReviewed" and "wired ... directly" — and a scanner that skipped this
// would find the documentation and pass.

const withoutComments = (source: string): string =>
  source
    .split("\n")
    .map((line) => {
      const at = line.indexOf("//");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");

describe("the confirmation is the ONLY way to the write", () => {
  const source = () =>
    withoutComments(readFileSync(join(__dirname, "..", "PracticeReviewBar.tsx"), "utf8"));

  it("calls markDeckReviewed exactly once, and only behind step.send", () => {
    const code = source();
    const calls = code.match(/markDeckReviewed\(/g) ?? [];
    expect(calls, "a second call site is a second way to move the shared mark").toHaveLength(1);

    // The call must sit AFTER the early return that `step.send` guards. Slicing
    // rather than matching a shape: an `if (!step.send) return;` that stopped
    // preceding the call would be invisible to a regex over the whole file.
    const guard = code.indexOf("if (!step.send) return;");
    expect(guard, "the send guard is gone").toBeGreaterThan(-1);
    expect(code.indexOf("markDeckReviewed(")).toBeGreaterThan(guard);
  });

  it("routes every control through the machine — no handler writes on its own", () => {
    const code = source();
    // Every onClick in the bar dispatches an action; none calls the service or
    // sets state itself. `act(` is the one door.
    const clicks = code.match(/onClick=\{[^}]*\}/g) ?? [];
    expect(clicks.length, "anti-vacuity: the bar has controls to check").toBeGreaterThanOrEqual(3);
    for (const click of clicks) {
      expect(click, `a control that does not go through the machine: ${click}`).toContain("act({");
    }
  });

  it("imports the machine rather than holding a phase of its own", () => {
    const code = source();
    expect(code).toContain("deckReviewConfirmStep");
    // A `useState` holding the open/closed flag directly would be a second
    // machine, and the closed-union proof would stop covering the bar.
    expect(code).not.toMatch(/useState\(false\).*confirm/i);
  });
});
