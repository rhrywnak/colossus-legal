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

describe("pickByCount", () => {
  it("takes the singular at exactly 1 and the plural at 0 and 2", () => {
    expect(pickByCount(1, "one", "many")).toBe("one");
    expect(pickByCount(2, "one", "many")).toBe("many");
    expect(pickByCount(0, "one", "many")).toBe("many");
  });
});

describe("the viewer badge", () => {
  it("reads '1 answer' at 1 and '2 answers' at 2", () => {
    const one = cardBadges(s1({ new_answers_for_viewer: 1 }), warRoomWording);
    expect(one[0].text).toBe("1 answer you haven't reviewed");
    const two = cardBadges(s1({ new_answers_for_viewer: 2 }), warRoomWording);
    expect(two[0].text).toBe("2 answers you haven't reviewed");
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
      cardBadges(s1({ new_answers_for_viewer: 0, marie_changed: n }), wording)[0].text;
    expect(at(1)).toBe("1 item new or changed for Marie");
    expect(at(2)).toBe("2 items new or changed for Marie");
  });
});

describe("the deck review bar", () => {
  const wording = {
    deck_review_new_one: "{count} answer new since you last reviewed",
    deck_review_new_template: "{count} answers new since you last reviewed",
  };

  it("picks the singular row at 1 and the plural row at 2", () => {
    expect(reviewBarLine(1, wording)).toBe("1 answer new since you last reviewed");
    expect(reviewBarLine(2, wording)).toBe("2 answers new since you last reviewed");
  });
});
