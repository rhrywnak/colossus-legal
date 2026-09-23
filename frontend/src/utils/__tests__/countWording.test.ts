// =============================================================================
// countWording.test.ts — singular at exactly 1, plural otherwise (GO v3)
// =============================================================================
//
// One pure test per count-bearing pair. Mutation-proved (report): swapping the
// pick in `pickByCount` turns every pair's test red.

import { describe, expect, it } from "vitest";

import { pickByCount } from "../countWording";
import { cardBadges } from "../../components/warRoomCardView";
import { reviewBarLine } from "../../components/practice/PracticeReviewBar";
import { s1, warRoomWording } from "../../components/__tests__/warRoomFixtures";

const SLUG = "awad_v_catholic_family_service";

describe("pickByCount", () => {
  it("takes the singular at exactly 1 and the plural at 0 and 2", () => {
    expect(pickByCount(1, "one", "many")).toBe("one");
    expect(pickByCount(2, "one", "many")).toBe("many");
    expect(pickByCount(0, "one", "many")).toBe("many");
  });
});

describe("the review pill", () => {
  it("reads '1 answer' at 1 and '2 answers' at 2", () => {
    const one = cardBadges(s1({ awaiting_review: 1 }), warRoomWording, SLUG, true);
    expect(one[0].text).toBe("1 answer awaiting your review");
    const two = cardBadges(s1({ awaiting_review: 2 }), warRoomWording, SLUG, true);
    expect(two[0].text).toBe("2 answers awaiting your review");
  });
});

describe("Marie's new-or-changed pill", () => {
  // The stored singular and plural read alike today, so distinct fixture words
  // prove WHICH row was picked rather than that the sentence happens to match.
  const wording = {
    ...warRoomWording,
    card_changed_one: "{count} item new or changed for Marie",
    card_changed_template: "{count} items new or changed for Marie",
  };

  it("picks the singular row at 1 and the plural row at 2", () => {
    const at = (n: number) =>
      cardBadges(s1({ awaiting_review: 0, marie_changed: n }), wording, SLUG, true)[0].text;
    expect(at(1)).toBe("1 item new or changed for Marie");
    expect(at(2)).toBe("2 items new or changed for Marie");
  });
});

describe("the deck review bar", () => {
  // `{reviewer}` left these templates on 2026-09-23: the count is the reader's
  // own, so the sentence addresses the reader. `{count}` is all that is filled.
  const wording = {
    deck_review_awaiting_one: "{count} item awaiting your review",
    deck_review_awaiting_template: "{count} items awaiting your review",
  };
  const review = (awaiting: number) => ({ awaiting, can_mark_reviewed: true });

  it("picks the singular row at 1 and the plural row at 2", () => {
    expect(reviewBarLine(review(1), wording)).toBe("1 item awaiting your review");
    expect(reviewBarLine(review(2), wording)).toBe("2 items awaiting your review");
  });
});
