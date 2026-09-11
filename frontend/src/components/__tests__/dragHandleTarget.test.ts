/**
 * The drag grip is a TARGET you can hit (fix/running-view-visible, item 2).
 *
 * It rendered at `fontSize: 13` inside a `minWidth: 24` box — roughly 13×16 px of
 * hittable area — against the 44pt minimum Apple's HIG names and WCAG 2.5.5
 * repeats. On the iPad this drag was built for it was not reliably hittable at
 * all.
 *
 * Two numbers and one rule, and none of them can be checked by rendering here
 * (RTL/jsdom are not set up — CLAUDE.md Rule 30). So the numbers are exported and
 * asserted by value, and the rule — the grip is the ONLY handle — is read off the
 * source, which is the same fence `practiceDeckDragStructure.test.ts` uses.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { GRIP_GLYPH_PX, GRIP_TARGET_PX } from "../dragReorder";

/** Source with its comments removed — this repo documents its rules next to its
 *  rules, so a scanner looking for a name finds the DOCUMENTATION first. */
const withoutComments = (source: string): string =>
  source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => {
      const at = line.indexOf("//");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");

const read = (...parts: string[]) =>
  withoutComments(readFileSync(join(__dirname, "..", ...parts), "utf8"));

const grip = read("dragReorder.tsx");
const row = read("practice", "PracticeDeckRow.tsx");

describe("the grip is big enough to hit", () => {
  /**
   * 44 is the floor, not a preference.
   *
   * Apple's HIG minimum touch target is 44pt; WCAG 2.5.5 (Target Size) names the
   * same figure. Asserted as `>=` rather than `===` so making it larger is not a
   * test failure, and by value so making it smaller is.
   */
  it("gives the grip a 44 px minimum target", () => {
    expect(GRIP_TARGET_PX).toBeGreaterThanOrEqual(44);
  });

  /** The glyph has to be visible, not merely present — NN/g's "grab handle". */
  it("draws a glyph you can see you are aiming at", () => {
    expect(GRIP_GLYPH_PX).toBeGreaterThanOrEqual(20);
    // And it must fit inside its own box with room to breathe.
    expect(GRIP_GLYPH_PX).toBeLessThan(GRIP_TARGET_PX);
  });

  it("applies both to the element, in both dimensions", () => {
    expect(grip).toContain("minWidth: GRIP_TARGET_PX");
    expect(grip).toContain("minHeight: GRIP_TARGET_PX");
    expect(grip).toContain("fontSize: GRIP_GLYPH_PX");
  });

  /**
   * The row must not shrink it back.
   *
   * `style={{ fontSize: 13 }}` at the call site is the exact thing that made the
   * grip unhittable, and `style` is spread LAST in `DragHandle`, so any caller
   * can still undo the fix in one line. This is what would catch that.
   */
  it("is not overridden by the practice row", () => {
    expect(row).toContain("<DragHandle");
    expect(row).not.toContain("fontSize: 13");
  });
});

describe("it looks grabbable, and only while it is", () => {
  it("offers grab and grabbing cursors", () => {
    expect(grip).toContain('cursor: pressed ? "grabbing" : "grab"');
    // …and nothing to grab when there is nothing to drag.
    expect(grip).toContain('{ cursor: "default" }');
  });

  /**
   * A tint that answers a pointer, from tokens — never a literal colour.
   *
   * `--bg-page` is the token tokens.css documents for exactly this ("nav hover
   * states"); `--border-default` is one step stronger for the pressed state.
   */
  it("tints on hover and press, from tokens", () => {
    expect(grip).toContain('"var(--bg-page)"');
    expect(grip).toContain('"var(--border-default)"');
    expect(grip).not.toMatch(/background:\s*"#[0-9a-fA-F]{3,8}"/);
  });

  /** Nothing at rest — eleven permanent grey patches would read as buttons. */
  it("shows nothing at rest", () => {
    expect(grip).toContain('"transparent"');
  });

  /**
   * `touchAction: none` is what lets the touch sensor see the gesture at all.
   *
   * Without it the browser claims the finger for scrolling and the drag never
   * begins — the iPad failure this whole feature exists to fix.
   */
  it("keeps touchAction none while grabbable", () => {
    expect(grip).toContain('touchAction: "none"');
  });
});

describe("it is still the ONLY handle", () => {
  /**
   * The sensor's listeners go to the grip and nowhere else.
   *
   * If they ever reach the row, a drag can start on the question text again —
   * and on a touch screen that makes the row unreadable, because every attempt
   * to select or scroll picks it up.
   */
  it("takes the listeners, and the row does not", () => {
    expect(row).toContain("handleProps={canDrag");
    expect(row).not.toContain("{...listeners}");
    expect(row.includes("draggable")).toBe(false);
  });

  /**
   * dnd-kit's own handlers are COMPOSED, not replaced.
   *
   * `handleProps` carries `onPointerDown`; an `onPointerDown` of our own written
   * after the spread silently shadows it, and the failure is the nastiest shape
   * there is — the hover tint works, so the control looks alive, and the drag is
   * simply dead.
   */
  it("calls dnd-kit's handlers before its own", () => {
    for (const handler of ["onPointerDown", "onPointerUp", "onMouseEnter", "onMouseLeave"]) {
      expect(grip, `${handler} must chain to handleProps`).toContain(
        `handleProps?.${handler}?.(event)`,
      );
    }
  });
});
