// =============================================================================
// practiceReviewBar.test.tsx — the Done reviewing button is the reviewer's alone
// =============================================================================
//
// Markup both ways (CC_TASK_SIMPLE_COUNTS_v1). The server decides
// `can_mark_reviewed`; the bar renders the button on that boolean and nothing
// else. Mutation-proved in the report: dropping the guard reds the second test.

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import PracticeReviewBar, { reviewBarLine } from "../PracticeReviewBar";
import type { DeckReview } from "../../../services/practice";

const wording = {
  deck_review_awaiting_one: "{count} answer awaiting {reviewer}'s review",
  deck_review_awaiting_template: "{count} answers awaiting {reviewer}'s review",
  deck_review_done_label: "Done reviewing",
  deck_review_failed: "Could not mark this deck reviewed — nothing was changed.",
  // CC_TASK_REVIEW_PAGE_v1: the bar's fifth string.
  deck_review_oldest_template: "· oldest waiting since {date}",
};

function bar(review: DeckReview): string {
  return renderToStaticMarkup(
    <PracticeReviewBar slug="c" scenarioId="s" review={review} wording={wording} onReviewed={() => {}} />,
  );
}

describe("PracticeReviewBar", () => {
  it("offers Done reviewing to the reviewer (cpenzien's payload)", () => {
    const html = bar({ awaiting: 3, can_mark_reviewed: true, reviewer_display_name: "Chuck" });
    expect(html).toContain("3 answers awaiting Chuck&#x27;s review");
    expect(html).toContain(">Done reviewing</button>");
  });

  it("shows everyone else the same count and NO button (M)", () => {
    const html = bar({ awaiting: 3, can_mark_reviewed: false, reviewer_display_name: "Chuck" });
    expect(html).toContain("3 answers awaiting Chuck&#x27;s review");
    expect(html).not.toContain("Done reviewing");
    expect(html).not.toContain("<button");
  });

  it("draws nothing when nothing waits", () => {
    expect(bar({ awaiting: 0, can_mark_reviewed: true, reviewer_display_name: "Chuck" })).toBe("");
  });
});

// ── The oldest-waiting clause (CC_TASK_REVIEW_PAGE_v1, ruling STOP-A) ────────

describe("the oldest-waiting clause", () => {
  it("is appended when the server sent a date", () => {
    const line = reviewBarLine(
      { awaiting: 42, can_mark_reviewed: false, reviewer_display_name: "Chuck", oldest: "20 Aug" },
      wording,
    );
    expect(line).toContain("42");
    expect(line).toContain("Chuck");
    expect(line).toContain("oldest waiting since 20 Aug");
  });

  it("is WITHHELD when the server sent no date", () => {
    // Filling `{date}` with an empty string would print a sentence that trails
    // off — which reads as a rendering fault rather than as an absent fact.
    const line = reviewBarLine(
      { awaiting: 3, can_mark_reviewed: false, reviewer_display_name: "Chuck" },
      wording,
    );
    expect(line).not.toContain("oldest");
    expect(line).not.toContain("{date}");
  });

  it("prints the date the server composed and formats nothing", () => {
    // The browser holds no date format. Whatever the server sent is what shows.
    const line = reviewBarLine(
      { awaiting: 1, can_mark_reviewed: true, reviewer_display_name: "Chuck", oldest: "2 Mar" },
      wording,
    );
    expect(line).toContain("2 Mar");
  });

  it("names the whole bench when {reviewer} carries two names", () => {
    // The joining is done server-side; the bar fills one placeholder with
    // whatever it was handed, which is what keeps the deck bar and the War
    // Room pill saying the same thing.
    const line = reviewBarLine(
      {
        awaiting: 5,
        can_mark_reviewed: true,
        reviewer_display_name: "Chuck · Roman",
        oldest: "20 Aug",
      },
      wording,
    );
    expect(line).toContain("Chuck · Roman");
  });
});
