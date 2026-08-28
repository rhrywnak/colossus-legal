// =============================================================================
// sectionCollapse.test.ts — the remembered fold, and what an absent value means
// =============================================================================
//
// Ruled 2026-08-28: the scenario page's two long sections arrive COLLAPSED and
// remember the human's answer per scenario. The three things worth pinning are
// the key format (a wrong one does not fail, it silently forgets), the
// absent-means-collapsed default, and the per-scenario isolation — S-10 expanded
// must not expand S-11.
//
// There is no jsdom in this suite, so `localStorage` does not exist here. Each
// test installs its own in-memory stand-in, which is better than jsdom would be:
// the throwing cases below are exactly what a private window or a
// site-data-blocked browser does, and they are hard to provoke any other way.

import { afterEach, describe, expect, it, vi } from "vitest";

import {
  readSectionOpen,
  sectionCollapseKey,
  writeSectionOpen,
} from "../sectionCollapse";

/** A minimal in-memory `localStorage`, seeded with whatever a test needs. */
function installStore(seed: Record<string, string> = {}): Map<string, string> {
  const store = new Map(Object.entries(seed));
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => {
        store.set(k, v);
      },
    },
  });
  return store;
}

/** A store that refuses — a private window, or site data blocked. */
function installRefusingStore(): void {
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: () => {
        throw new Error("access denied");
      },
      setItem: () => {
        throw new Error("quota exceeded");
      },
    },
  });
}

afterEach(() => {
  Reflect.deleteProperty(globalThis, "localStorage");
  vi.restoreAllMocks();
});

describe("the key", () => {
  it("names the scenario AND the section", () => {
    // The shape Roman specified. A key missing either half is the bug this
    // format exists to prevent: without the scenario, one page's answer leaks
    // onto every other; without the section, the two folds share one answer.
    expect(sectionCollapseKey("S-10", "candidates")).toBe(
      "scenario:S-10:candidates:collapsed",
    );
    expect(sectionCollapseKey("S-10", "facts")).toBe("scenario:S-10:facts:collapsed");
  });

  it("gives two scenarios two different keys", () => {
    expect(sectionCollapseKey("S-10", "candidates")).not.toBe(
      sectionCollapseKey("S-11", "candidates"),
    );
  });
});

describe("what an absent value means", () => {
  it("is COLLAPSED — the new default", () => {
    // The whole point of the change. Nothing stored is not "we don't know", it
    // is the answer for a human who has never expressed a preference.
    installStore();
    expect(readSectionOpen("S-10", "candidates")).toBe(false);
    expect(readSectionOpen("S-10", "facts")).toBe(false);
  });

  it("is COLLAPSED for a value nobody wrote", () => {
    // A hand-edited store, or a future format change. Anything unrecognised
    // falls back to the default rather than being coerced into a boolean —
    // `Boolean("false")` is `true`, which is how this goes wrong quietly.
    installStore({
      "scenario:S-10:candidates:collapsed": "yes",
      "scenario:S-10:facts:collapsed": "",
    });
    expect(readSectionOpen("S-10", "candidates")).toBe(false);
    expect(readSectionOpen("S-10", "facts")).toBe(false);
  });
});

describe("remembering the human's answer", () => {
  it("survives a reload, per section", () => {
    // Expanding one section must not expand the other on the next visit.
    installStore();
    writeSectionOpen("S-10", "candidates", true);

    expect(readSectionOpen("S-10", "candidates")).toBe(true);
    expect(readSectionOpen("S-10", "facts")).toBe(false);
  });

  it("does not leak from one scenario to another", () => {
    // Roman's example, exactly: S-10 expanded must not expand S-11.
    installStore();
    writeSectionOpen("S-10", "candidates", true);

    expect(readSectionOpen("S-11", "candidates")).toBe(false);
  });

  it("collapsing again is stored, not just forgotten", () => {
    // A human who expands and then collapses has expressed a preference. It
    // happens to match the default today, but writing it means the section
    // behaves the same way whatever the default later becomes.
    const store = installStore();
    writeSectionOpen("S-10", "facts", true);
    writeSectionOpen("S-10", "facts", false);

    expect(store.get("scenario:S-10:facts:collapsed")).toBe("true");
    expect(readSectionOpen("S-10", "facts")).toBe(false);
  });
});

describe("a browser that refuses to store anything", () => {
  it("falls back to collapsed and says so in the console", () => {
    // The Standing Rule 1 carve-out for cosmetic browser-storage preferences:
    // degrade to the default WITHOUT a banner, but stay observable. A private
    // window must not put an error in front of a human mid-triage over a fold.
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    installRefusingStore();

    expect(readSectionOpen("S-10", "candidates")).toBe(false);
    expect(warn).toHaveBeenCalled();
  });

  it("a failed write does not throw into the click handler", () => {
    // The toggle calls this from an onClick. An exception here would take the
    // fold — and whatever React was rendering — down with it, over a preference.
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    installRefusingStore();

    expect(() => writeSectionOpen("S-10", "facts", true)).not.toThrow();
    expect(warn).toHaveBeenCalled();
  });

  it("survives having no localStorage at all", () => {
    // Not hypothetical: this very test file runs in a Node environment with no
    // DOM, and a bare `localStorage` reference there is a ReferenceError.
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});

    expect(readSectionOpen("S-10", "candidates")).toBe(false);
    expect(() => writeSectionOpen("S-10", "candidates", true)).not.toThrow();
    expect(warn).toHaveBeenCalled();
  });
});
