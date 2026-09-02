// =============================================================================
// dockPopoutCallSite.test.ts — the pop-out request is made from the CLICK
// =============================================================================
//
// T7.4 asks for it by name: "an assertion that the request is issued from the
// click handler and not an effect."
//
// ## ⚑ WHY THIS COSTS A WHOLE FILE
//
// `requestWindow` needs USER ACTIVATION. The browser grants it to a click's own
// call stack and to nothing afterwards, so a call made from a `useEffect` that
// runs after React commits the click's state change is refused:
//
//   NotAllowedError: Document PiP requires user activation
//
// T4's first build was exactly that, and StrictMode's double mount spent the
// activation twice over for good measure. It cost a debugging round, it was
// found by clicking rather than by reading, and design §12.2 records it as one
// of "the two constraints, stated so nobody rediscovers them". A regression
// here is invisible to every other test in this suite and to `tsc`: the code
// compiles, the types are right, and the window simply never opens.
//
// ## What this CANNOT prove
//
// That the browser grants the activation — that is the browser's ruling, made
// at runtime, and Roman clicking `View Timeline` on DEV is what knows it. This
// proves only the STRUCTURE that makes the grant possible: the call sits inside
// a `useCallback` that a click handler reaches, and no effect can reach it.
// Precedent for reading source in a test rather than rendering: this
// directory's own `badgeRetirement.test.ts`, and `onePageSurface.test.ts`
// before it.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

/**
 * Source with its comments removed.
 *
 * The standing hazard, the same one `badgeRetirement.test.ts` names: this
 * codebase documents its rules beside its rules, and the dock now carries three
 * paragraphs about `requestWindow` and where it must not be called. A scanner
 * that did not strip comments would find the word in the WARNING and report the
 * call site from the prose about it.
 */
function code(source: string): string {
  return source
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
}

const DOCK = code(readFileSync(join(__dirname, "..", "ScenarioTimelineDock.tsx"), "utf8"));

/**
 * Which React hook encloses the character at `index`.
 *
 * A crude but sufficient reading: whichever of the two opened LAST before this
 * point is the one we are inside. The dock declares its hooks at one level, one
 * after another, so "the nearest hook opened above me" is the enclosing one.
 * The alternative — parsing TypeScript — would be a great deal of machinery to
 * answer a question about a file whose shape a reader can see.
 */
function enclosingHook(source: string, index: number): "useCallback" | "useEffect" | "neither" {
  const before = source.slice(0, index);
  const callback = before.lastIndexOf("useCallback(");
  const effect = before.lastIndexOf("useEffect(");
  if (callback === -1 && effect === -1) return "neither";
  return callback > effect ? "useCallback" : "useEffect";
}

/** Every index at which `needle` occurs. */
function occurrences(source: string, needle: string): number[] {
  const out: number[] = [];
  let at = source.indexOf(needle);
  while (at !== -1) {
    out.push(at);
    at = source.indexOf(needle, at + 1);
  }
  return out;
}

describe("the picture-in-picture request lives in a click's own call stack", () => {
  it("is asked for exactly once in the dock", () => {
    // One call site is what makes the rest of this file's reasoning sound: two
    // would mean the second could sit anywhere and this test would still pass.
    expect(occurrences(DOCK, "requestWindow(")).toHaveLength(1);
  });

  it("sits inside a useCallback and not a useEffect", () => {
    const [at] = occurrences(DOCK, "requestWindow(");
    expect(enclosingHook(DOCK, at)).toBe("useCallback");
  });

  it("sits inside `floatOut` specifically", () => {
    const [at] = occurrences(DOCK, "requestWindow(");
    const before = DOCK.slice(0, at);
    // Compared at the END of each match: the two strings start in different
    // places and finish in the same one exactly when the nearest `useCallback(`
    // above the request is the one `floatOut` opened.
    const declaration = "const floatOut = useCallback(";
    expect(before.lastIndexOf(declaration) + declaration.length).toBe(
      before.lastIndexOf("useCallback(") + "useCallback(".length,
    );
  });

  it("is not awaited before it is issued", () => {
    // Design §12.2: "never after an await that yields". An `await` anywhere
    // above the call in the same callback hands the rest of the function to a
    // later microtask, by which time the activation is spent.
    const [at] = occurrences(DOCK, "requestWindow(");
    const body = DOCK.slice(DOCK.lastIndexOf("const floatOut = useCallback(", at), at);
    expect(body).not.toContain("await ");
  });
});

describe("every caller of `floatOut` is a handler", () => {
  it("is called only from useCallbacks — never from an effect", () => {
    // The call sites, not the declaration: `const floatOut = useCallback(` is
    // the declaration and dependency arrays name it without parentheses.
    const calls = occurrences(DOCK, "floatOut(").filter(
      (at) => !DOCK.slice(0, at).endsWith("const floatOut = use"),
    );
    expect(calls.length).toBeGreaterThan(0);
    for (const at of calls) expect(enclosingHook(DOCK, at)).toBe("useCallback");
  });

  it("is reached by BOTH doors — View Timeline and ⧉ (T7.2)", () => {
    // Neither direction is removed. `viewTimeline` is the front door T7 built;
    // `popOut` is the ⧉ in the docked window's bar, which still pops it out.
    expect(DOCK).toContain("floatOut(next)");
    expect(DOCK).toContain("floatOut(win)");
  });
});

describe("View Timeline is the button that opens the floating window (T7.1)", () => {
  it("is what the button's onClick names", () => {
    expect(DOCK).toContain("onClick={viewTimeline}");
  });

  it("no longer opens the in-page window directly — `openWindow` is gone", () => {
    // The rename is the fence. A future edit that restores the old handler
    // restores the old behaviour, and this says so before it ships.
    expect(DOCK).not.toContain("openWindow");
  });

  it("stores no `open` flag — design §12.3, ruled", () => {
    // T7.5: nothing reopens after a reload, and the stored record gains no
    // field. `subsetWindow.ts` owns the record; the dock is where an auto-open
    // would have been written, so the dock is where its absence is asserted.
    expect(DOCK).not.toContain("openStored");
    expect(DOCK).not.toContain("reopen");
  });
});
