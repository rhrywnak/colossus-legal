// =============================================================================
// subsetWindow.test.ts — where the window opens, and what it remembers
// =============================================================================
//
// T3.4 names four behaviours these hold: the open-position decision, the clamp,
// the localStorage codec (round-trips, tolerates garbage and absence) and the
// selector ordering. Every assertion is a concrete expected value.

import { describe, expect, it } from "vitest";

import {
  clampToViewport,
  decodeWindowState,
  DEFAULT_HEIGHT_RATIO,
  DEFAULT_WIDTH,
  encodeWindowState,
  initialSubsetId,
  MARGIN_OPEN_MIN_WIDTH,
  MIN_HEIGHT,
  MIN_WIDTH,
  openStateFor,
  selectorOrder,
  type WindowState,
  windowStorageKey,
} from "../subsetWindow";

const STATE: WindowState = {
  x: 100,
  y: 96,
  width: 420,
  height: 540,
  minimized: false,
  subsetId: "s-1",
};

describe("the open-position rule (design §10)", () => {
  it("opens in the right margin when there is room beside the content", () => {
    // 1440 wide, content ends at 900 → 540 free, comfortably over the threshold.
    const open = openStateFor(1440, 900, 900, "s-1");
    expect(open.minimized).toBe(false);
    expect(open.width).toBe(DEFAULT_WIDTH);
    expect(open.height).toBe(Math.round(900 * DEFAULT_HEIGHT_RATIO));
    expect(open.subsetId).toBe("s-1");
  });

  it("opens MINIMIZED when the margin is too narrow — it never covers the text", () => {
    // §5D: Marie is reading this beside a question. A narrow window over the
    // line she is answering is worse than a title bar out of the way.
    const open = openStateFor(1200, 900, 900, "s-1");
    expect(1200 - 900).toBeLessThan(MARGIN_OPEN_MIN_WIDTH);
    expect(open.minimized).toBe(true);
  });

  it("the threshold is exact: 460 free opens, 459 minimizes", () => {
    expect(openStateFor(1360, 900, 900, null).minimized).toBe(false);
    expect(openStateFor(1359, 900, 900, null).minimized).toBe(true);
  });

  it("never places the window at a negative x on a narrow viewport", () => {
    const open = openStateFor(320, 600, 320, null);
    expect(open.x).toBeGreaterThanOrEqual(0);
    expect(open.y).toBeGreaterThanOrEqual(0);
  });

  it("a very short viewport still yields a usable height", () => {
    expect(openStateFor(1440, 100, 900, null).height).toBe(MIN_HEIGHT);
  });
});

describe("the clamp — a remembered position must never strand the window", () => {
  it("pulls a window back when the viewport got smaller", () => {
    // Stored on a wide screen; reopened on a narrow one. Left verbatim, the
    // title bar — the only drag handle — would be off the right edge, and the
    // reader's only remedy would be clearing site data.
    const out = clampToViewport({ ...STATE, x: 1800, y: 1400 }, 1280, 800);
    expect(out.x).toBe(1280 - MIN_WIDTH);
    expect(out.y).toBe(800 - MIN_HEIGHT);
  });

  it("leaves a position that is already inside alone", () => {
    const out = clampToViewport(STATE, 1440, 900);
    expect(out.x).toBe(100);
    expect(out.y).toBe(96);
  });

  it("clamps a remembered SIZE to the viewport, and never below the minimum", () => {
    const big = clampToViewport({ ...STATE, width: 2000, height: 2000 }, 800, 600);
    expect(big.width).toBe(800);
    expect(big.height).toBe(600);
    const small = clampToViewport({ ...STATE, width: 10, height: 10 }, 1440, 900);
    expect(small.width).toBe(MIN_WIDTH);
    expect(small.height).toBe(MIN_HEIGHT);
  });

  it("survives a viewport smaller than the minimum without going negative", () => {
    const out = clampToViewport(STATE, 200, 100);
    expect(out.x).toBe(0);
    expect(out.y).toBe(0);
    expect(out.width).toBe(MIN_WIDTH);
    expect(out.height).toBe(MIN_HEIGHT);
  });

  it("carries minimized and subsetId through untouched", () => {
    const out = clampToViewport({ ...STATE, minimized: true }, 1440, 900);
    expect(out.minimized).toBe(true);
    expect(out.subsetId).toBe("s-1");
  });
});

describe("the codec tolerates absence and garbage", () => {
  it("round-trips a state exactly", () => {
    expect(decodeWindowState(encodeWindowState(STATE))).toEqual(STATE);
  });

  it("round-trips a null subsetId", () => {
    const none = { ...STATE, subsetId: null };
    expect(decodeWindowState(encodeWindowState(none))).toEqual(none);
  });

  it("returns null for absence and for an empty string", () => {
    expect(decodeWindowState(null)).toBeNull();
    expect(decodeWindowState("")).toBeNull();
    expect(decodeWindowState("   ")).toBeNull();
  });

  it("returns null for garbage rather than throwing", () => {
    expect(decodeWindowState("{not json")).toBeNull();
    expect(decodeWindowState("[]")).toBeNull();
    expect(decodeWindowState("null")).toBeNull();
    expect(decodeWindowState('"a string"')).toBeNull();
    expect(decodeWindowState("42")).toBeNull();
  });

  it("REFUSES a numeric field that arrived as a string", () => {
    // The failure this exists for: a blob from an older build carrying
    // width: "420" would reach the window as a string and lay it out at zero
    // width, which looks like the window failing to open at all.
    expect(decodeWindowState(JSON.stringify({ ...STATE, width: "420" }))).toBeNull();
  });

  it("refuses a missing field and a non-finite number", () => {
    expect(decodeWindowState(JSON.stringify({ x: 1, y: 2, width: 3 }))).toBeNull();
    expect(decodeWindowState('{"x":null,"y":2,"width":3,"height":4,"minimized":false}')).toBeNull();
  });

  it("refuses a minimized flag that is not a boolean", () => {
    expect(decodeWindowState(JSON.stringify({ ...STATE, minimized: "yes" }))).toBeNull();
  });

  it("degrades a wrong-typed subsetId to null rather than refusing the whole state", () => {
    // Position and size are the expensive things to lose; which subset was
    // showing is cheap to re-derive from what is attached.
    const out = decodeWindowState(JSON.stringify({ ...STATE, subsetId: 7 }));
    expect(out).not.toBeNull();
    expect(out?.subsetId).toBeNull();
    expect(out?.x).toBe(100);
  });

  it("keys storage by scenario, so two scenarios remember separately", () => {
    expect(windowStorageKey("abc")).toBe("colossus.subsetWindow.abc");
    expect(windowStorageKey("abc")).not.toBe(windowStorageKey("def"));
  });
});

describe("the selector's order", () => {
  const attached = [
    { id: "a", position: 2 },
    { id: "b", position: 0 },
    { id: "c", position: 1 },
  ];

  it("is attachment order when nothing is remembered", () => {
    expect(selectorOrder(attached, null).map((s) => s.id)).toEqual(["b", "c", "a"]);
  });

  it("puts the remembered subset first, keeping the rest in order", () => {
    expect(selectorOrder(attached, "a").map((s) => s.id)).toEqual(["a", "b", "c"]);
  });

  it("IGNORES a remembered subset that is no longer attached", () => {
    // Detached on another screen. It must not come back as the selection just
    // because this browser remembers picking it.
    expect(selectorOrder(attached, "gone").map((s) => s.id)).toEqual(["b", "c", "a"]);
  });

  it("initialSubsetId opens on the remembered one, else the first attached", () => {
    expect(initialSubsetId(attached, "a")).toBe("a");
    expect(initialSubsetId(attached, null)).toBe("b");
    expect(initialSubsetId(attached, "gone")).toBe("b");
  });

  it("initialSubsetId is null when nothing is attached — the button is hidden", () => {
    expect(initialSubsetId([], null)).toBeNull();
    expect(initialSubsetId([], "a")).toBeNull();
  });
});
