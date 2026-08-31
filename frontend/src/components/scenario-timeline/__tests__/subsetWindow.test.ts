// =============================================================================
// subsetWindow.test.ts — where the window opens, and what it remembers
// =============================================================================
//
// T3.4 named four behaviours these hold: the open-position decision, the clamp,
// the localStorage codec (round-trips, tolerates garbage and absence) and the
// selector ordering. Every assertion is a concrete expected value.
//
// ## ⚑ THE OPEN-POSITION BLOCK WAS REWRITTEN FOR T4, AND THAT IS THE POINT
//
// The first four tests in this file asserted the §10 rule: open in the right
// margin when 460px of free width exists beside the content, otherwise open
// MINIMIZED, and one of them pinned the threshold to the pixel. Every one of
// them PASSED on the build that shipped defect D8 — a reader who clicked View
// Timeline on a MacBook got a grey bar in the corner, and the suite agreed that
// was correct, because the suite was asserting the rule rather than the
// outcome. Design §11 withdraws the rule. The tests below assert the new one:
// full size, at the right edge, EVERY time, and no free-width test anywhere.

import { describe, expect, it } from "vitest";

import {
  clampToViewport,
  decodeWindowState,
  DEFAULT_HEIGHT,
  DEFAULT_WIDTH,
  encodeWindowState,
  initialSubsetId,
  MIN_HEIGHT,
  MIN_WIDTH,
  minimizedPosition,
  namedSubset,
  openStateFor,
  previewWindowState,
  RIGHT_MARGIN,
  selectorOrder,
  TOP_BELOW_HEADER,
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

/** A MacBook-ish viewport with the app's header strip about 64px tall. */
const HEADER_BOTTOM = 64;

describe("the first-open rule (design §11 — §10 WITHDRAWN)", () => {
  it("opens FULL SIZE at the right edge, 20px below the header", () => {
    const open = openStateFor(1440, 900, HEADER_BOTTOM, "s-1");
    expect(open.minimized).toBe(false);
    expect(open.width).toBe(DEFAULT_WIDTH);
    expect(open.height).toBe(DEFAULT_HEIGHT);
    expect(open.x).toBe(1440 - DEFAULT_WIDTH - RIGHT_MARGIN);
    expect(open.y).toBe(HEADER_BOTTOM + TOP_BELOW_HEADER);
    expect(open.subsetId).toBe("s-1");
  });

  it("opens full size REGARDLESS OF FREE WIDTH — defect D8, by name", () => {
    // The §10 rule minimized the window when less than 460px of free width sat
    // beside the content column. That is precisely the MacBook case, and it is
    // what Roman met on 08-31. There is no free-width argument left to pass, so
    // this asserts the outcome: a 1200px viewport — narrower than the whole
    // old threshold plus a 1080px content column — still opens the window.
    const narrow = openStateFor(1200, 900, HEADER_BOTTOM, "s-1");
    expect(narrow.minimized).toBe(false);
    expect(narrow.width).toBe(DEFAULT_WIDTH);
    expect(narrow.height).toBe(DEFAULT_HEIGHT);
  });

  it("opens full size on the exact widths the old rule split on", () => {
    // 1360 opened and 1359 minimized under §10. Both open now, identically.
    const wide = openStateFor(1360, 900, HEADER_BOTTOM, null);
    const narrow = openStateFor(1359, 900, HEADER_BOTTOM, null);
    expect(wide.minimized).toBe(false);
    expect(narrow.minimized).toBe(false);
    expect(wide.height).toBe(narrow.height);
  });

  it("never places the window at a negative x on a narrow viewport", () => {
    const open = openStateFor(320, 600, HEADER_BOTTOM, null);
    expect(open.x).toBeGreaterThanOrEqual(0);
    expect(open.y).toBeGreaterThanOrEqual(0);
    // Narrower than the drawn 460: the window shrinks to the viewport rather
    // than hanging off the right edge with its close button unreachable.
    expect(open.width).toBe(MIN_WIDTH);
  });

  it("a very short viewport still yields a usable height", () => {
    expect(openStateFor(1440, 100, HEADER_BOTTOM, null).height).toBe(MIN_HEIGHT);
  });

  it("clamps the drawn height to the room below the header, not past the fold", () => {
    // 700px tall, header at 64 → the window starts at 84 and 590 would end at
    // 674 … which fits. At 600 tall it does not, and the footer's two links
    // would be off-screen with no way to drag them back into view.
    expect(openStateFor(1440, 700, HEADER_BOTTOM, null).height).toBe(DEFAULT_HEIGHT);
    expect(openStateFor(1440, 600, HEADER_BOTTOM, null).height).toBe(
      600 - HEADER_BOTTOM - TOP_BELOW_HEADER,
    );
  });

  it("survives a header taller than the viewport without going negative", () => {
    const open = openStateFor(1440, 200, 400, null);
    expect(open.y).toBe(420);
    expect(open.height).toBe(MIN_HEIGHT);
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

describe("the minimized bar is pinned bottom-right (§5C)", () => {
  it("sits in the bottom-right corner of the viewport", () => {
    const at = minimizedPosition(1600, 900);
    expect(at.x).toBe(1600 - DEFAULT_WIDTH - RIGHT_MARGIN);
    expect(at.y).toBe(900 - MIN_HEIGHT - 16);
  });

  it("never goes negative on a viewport smaller than the bar", () => {
    const at = minimizedPosition(200, 100);
    expect(at.x).toBe(0);
    expect(at.y).toBe(0);
  });

  it("does NOT come from the stored position — restoring must not lose it", () => {
    // The whole reason this is computed: collapsing the window to read the page
    // underneath must not overwrite the corner the reader chose for it.
    const at = minimizedPosition(1600, 900);
    expect(at).not.toEqual({ x: STATE.x, y: STATE.y });
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

// -----------------------------------------------------------------------------
// PREVIEW (T5) — the two decisions the preview path broke once already
// -----------------------------------------------------------------------------

describe("namedSubset — which subset the title bar names", () => {
  const attached = [
    { id: "a", name: "The $50,000", event_count: 15 },
    { id: "b", name: "The redirects", event_count: 4 },
  ];

  it("names the SELECTED subset on an ordinary open", () => {
    expect(namedSubset(null, null, attached, "b")?.name).toBe("The redirects");
  });

  it("falls back to the first attached when nothing is selected", () => {
    expect(namedSubset(null, null, attached, null)?.name).toBe("The $50,000");
  });

  it("names the PREVIEWED subset even though it is not attached — the defect", () => {
    // Clicking Preview on "The fee engine" opened a window titled
    // "The $50,000 · 15 events" over the fee engine's events, because
    // `attached.find` missed and the fallback took `attached[0]`. The bar lied
    // about which story was on screen.
    const fee = { id: "fee", name: "The fee engine", event_count: 0 };
    expect(namedSubset("fee", fee, attached, null)).toEqual(fee);
  });

  it("names NOTHING while the previewed detail is still loading", () => {
    // Rather than falling back to an attached subset, which is how the bar
    // came to name the wrong one. Undefined withdraws the bar's name until the
    // right one arrives.
    expect(namedSubset("fee", null, attached, null)).toBeUndefined();
  });

  it("refuses a STALE detail from the previously previewed subset", () => {
    const old = { id: "a", name: "The $50,000", event_count: 15 };
    expect(namedSubset("fee", old, attached, null)).toBeUndefined();
  });

  it("is undefined when nothing is attached and nothing is previewed", () => {
    expect(namedSubset(null, null, [], null)).toBeUndefined();
  });
});

describe("previewWindowState", () => {
  it("opens at the ordinary first-open place, on the previewed subset", () => {
    const w = previewWindowState("fee", 1440, 900, 64);
    expect(w.subsetId).toBe("fee");
    expect(w.minimized).toBe(false);
    expect(w).toEqual(openStateFor(1440, 900, 64, "fee"));
  });

  it("does NOT consult the remembered position", () => {
    // A preview that opened wherever the reader last dragged the real window
    // would look like the real window — the one thing it must not be mistaken
    // for. Same inputs, same place, every time.
    expect(previewWindowState("fee", 1440, 900, 64)).toEqual(
      previewWindowState("fee", 1440, 900, 64),
    );
  });
});
