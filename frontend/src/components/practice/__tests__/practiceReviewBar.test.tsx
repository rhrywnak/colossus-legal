// =============================================================================
// practiceReviewBar.test.tsx — the Done reviewing button is the reviewer's alone
// =============================================================================
//
// Markup both ways (CC_TASK_SIMPLE_COUNTS_v1). The server decides
// `can_mark_reviewed`; the bar renders the button on that boolean and nothing
// else. Mutation-proved in the report: dropping the guard reds the second test.

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import PracticeReviewBar from "../PracticeReviewBar";
import type { DeckReview } from "../../../services/practice";

const wording = {
  deck_review_awaiting_one: "{count} answer awaiting {reviewer}'s review",
  deck_review_awaiting_template: "{count} answers awaiting {reviewer}'s review",
  deck_review_done_label: "Done reviewing",
  deck_review_failed: "Could not mark this deck reviewed — nothing was changed.",
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
