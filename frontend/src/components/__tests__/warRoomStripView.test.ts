// =============================================================================
// warRoomStripView.test.ts — the four queue tiles from a fixture
// =============================================================================

import { describe, expect, it } from "vitest";

import { warRoomStripView } from "../warRoomStripView";
import { s1, s11, warRoomWording } from "./warRoomFixtures";

describe("warRoomStripView", () => {
  it("sums the cards into the four tiles, in the mockup's order", () => {
    const tiles = warRoomStripView([s1(), s11()], warRoomWording);
    expect(tiles.map((t) => [t.key, t.value, t.label])).toEqual([
      ["answered", "42 of 54", "Questions answered"],
      ["waiting", "12", "Waiting for Marie"],
      ["new_for_you", "42", "New answers for you"],
      ["candidates", "34", "Candidates for Roman"],
    ]);
  });

  it("reproduces the mockup's arithmetic: 185 − 87 = 98 waiting (GO v1 ruling 5)", () => {
    const answered = { chuck_answered: 0, chuck_total: 0, defense_answered: 0, defense_total: 0 };
    const tiles = warRoomStripView(
      [
        s1({ answered: { ...answered, total: 87, of: 185 } }),
      ],
      warRoomWording,
    );
    expect(tiles[0].value).toBe("87 of 185");
    expect(tiles[1].value).toBe("98");
  });

  it("counts BOTH sides as waiting — Chuck's unanswered and the defense's", () => {
    const tiles = warRoomStripView([s11()], warRoomWording);
    // S-11: Chuck 0/7 + defense 0/5.
    expect(tiles[1].value).toBe("12");
  });

  it("flags owed queues amber and leaves empty ones in ink", () => {
    const owed = warRoomStripView([s1(), s11()], warRoomWording);
    expect(owed.map((t) => t.warning)).toEqual([false, true, true, false]);
    const clear = warRoomStripView([s1({ new_answers_for_viewer: 0 })], warRoomWording);
    expect(clear.map((t) => t.warning)).toEqual([false, false, false, false]);
  });

  it("renders zeroes, not blanks, for a case with no scenarios", () => {
    expect(warRoomStripView([], warRoomWording).map((t) => t.value)).toEqual(["0 of 0", "0", "0", "0"]);
  });
});
