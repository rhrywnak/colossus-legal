/**
 * The drag auto-scroll's arithmetic (task 2.13c amendment).
 *
 * Roman could drop a card on a seam but could not move it past the edge of the
 * visible window: the facts list scrolls in its own region, nothing scrolled that
 * region during a drag, and the scrollbar is unreachable with a card in hand.
 *
 * The frame loop and the DOM node need a browser. The SPEED does not, and the
 * speed is where the bugs live — a wrong sign scrolls away from the cursor, a
 * missing clamp kills the scroll exactly when the human pushes hardest, and
 * overlapping bands on a short region pick a direction at random. All of that is
 * `autoScrollSpeed`, and all of it is here.
 */
import { describe, expect, it } from "vitest";

import {
  autoScrollSpeed,
  EDGE_BAND_PX,
  MAX_SCROLL_PX_PER_FRAME,
} from "../dragAutoScroll";

/** A region 600px tall, well over two edge bands, at a realistic offset. */
const TOP = 100;
const BOTTOM = 700;

describe("autoScrollSpeed", () => {
  it("does nothing in the quiet middle", () => {
    // The middle of the region must be dead, or a careful drop drifts under the
    // cursor while the human is trying to aim at a seam.
    expect(autoScrollSpeed(400, TOP, BOTTOM)).toBe(0);
  });

  it("scrolls UP near the top edge and DOWN near the bottom", () => {
    // The sign is the one error that makes the feature actively hostile: the list
    // running away from the direction the human is reaching.
    expect(autoScrollSpeed(TOP + 5, TOP, BOTTOM)).toBeLessThan(0);
    expect(autoScrollSpeed(BOTTOM - 5, TOP, BOTTOM)).toBeGreaterThan(0);
  });

  it("goes faster the closer the cursor gets to an edge", () => {
    const nearEdge = Math.abs(autoScrollSpeed(TOP + 4, TOP, BOTTOM));
    const midBand = Math.abs(autoScrollSpeed(TOP + EDGE_BAND_PX / 2, TOP, BOTTOM));
    const justInside = Math.abs(autoScrollSpeed(TOP + EDGE_BAND_PX - 1, TOP, BOTTOM));

    expect(nearEdge).toBeGreaterThan(midBand);
    expect(midBand).toBeGreaterThan(justInside);
  });

  it("reaches full speed exactly at the edge and zero at the band's inner rim", () => {
    expect(autoScrollSpeed(TOP, TOP, BOTTOM)).toBe(-MAX_SCROLL_PX_PER_FRAME);
    expect(autoScrollSpeed(BOTTOM, TOP, BOTTOM)).toBe(MAX_SCROLL_PX_PER_FRAME);
    // The inner boundary is the first point that must NOT move: a band that
    // still scrolls at its far edge has no quiet middle.
    expect(autoScrollSpeed(TOP + EDGE_BAND_PX, TOP, BOTTOM)).toBe(0);
    expect(autoScrollSpeed(BOTTOM - EDGE_BAND_PX, TOP, BOTTOM)).toBe(0);
  });

  it("keeps scrolling at full speed when the cursor goes PAST an edge", () => {
    // The failure this prevents: a human dragging hard toward the top pulls the
    // cursor above the region, and an unclamped distance would compute a speed of
    // zero — the scroll dying at the exact moment they are asking for more of it.
    expect(autoScrollSpeed(TOP - 50, TOP, BOTTOM)).toBe(-MAX_SCROLL_PX_PER_FRAME);
    expect(autoScrollSpeed(BOTTOM + 50, TOP, BOTTOM)).toBe(MAX_SCROLL_PX_PER_FRAME);
  });

  it("never exceeds the maximum speed", () => {
    // Every position across the region and well beyond both edges.
    for (let y = TOP - 200; y <= BOTTOM + 200; y += 7) {
      expect(Math.abs(autoScrollSpeed(y, TOP, BOTTOM))).toBeLessThanOrEqual(
        MAX_SCROLL_PX_PER_FRAME,
      );
    }
  });

  it("picks the nearer edge when a short region's bands overlap", () => {
    // A region shorter than two bands makes every point "near" both edges. The
    // direction must follow the closer one rather than whichever branch happens
    // to be tested first.
    const shortTop = 0;
    const shortBottom = 40; // far shorter than one 64px band
    expect(autoScrollSpeed(5, shortTop, shortBottom)).toBeLessThan(0);
    expect(autoScrollSpeed(35, shortTop, shortBottom)).toBeGreaterThan(0);
    // Dead centre is a tie, and a tie must not pick a direction.
    expect(autoScrollSpeed(20, shortTop, shortBottom)).toBe(0);
  });

  it("refuses to scroll a region that cannot be scrolled", () => {
    // Degenerate inputs reach this from a real DOM: a collapsed region has a
    // zero-height rect, and a band of zero would divide by zero below.
    expect(autoScrollSpeed(50, 100, 100)).toBe(0);
    expect(autoScrollSpeed(50, 700, 100)).toBe(0);
    expect(autoScrollSpeed(TOP + 1, TOP, BOTTOM, 0)).toBe(0);
    expect(autoScrollSpeed(TOP + 1, TOP, BOTTOM, EDGE_BAND_PX, 0)).toBe(0);
  });

  it("honours a caller's own band and speed", () => {
    // The defaults are constants, not assumptions baked into the arithmetic.
    expect(autoScrollSpeed(TOP, TOP, BOTTOM, 10, 5)).toBe(-5);
    // 10px band: a cursor 20px in is outside it and must not move.
    expect(autoScrollSpeed(TOP + 20, TOP, BOTTOM, 10, 5)).toBe(0);
  });
});
