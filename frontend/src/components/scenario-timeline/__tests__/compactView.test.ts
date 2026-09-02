// =============================================================================
// compactView.test.ts — what a stored value means, and what a broken store does
// =============================================================================
//
// C3 names the four cases: absent → false; `"1"` → true; any other value →
// false; `localStorage` throwing → false, never a crash. Plus the round trip,
// because a read and a write that disagree about the string are a button that
// appears to do nothing after a reload — and nothing else in this build would
// notice.
//
// ## ⚑ NOT A BROWSER, ON PURPOSE
//
// These tests run in node with no `localStorage` at all, and every function
// under test takes its store as an ARGUMENT. That is what lets the throwing
// case be written down: a real browser only throws in a private window or with
// site data blocked, which is not a state a test suite can ask for.

import { describe, expect, it, vi } from "vitest";

import {
  browserStore,
  COMPACT_ON,
  COMPACT_STORAGE_KEY,
  type CompactStore,
  decodeCompact,
  readCompact,
  writeCompact,
} from "../compactView";

/** A store backed by a plain object — the browser, minus the browser. */
function fakeStore(initial: Record<string, string> = {}): CompactStore & {
  held: Record<string, string>;
} {
  const held: Record<string, string> = { ...initial };
  return {
    held,
    getItem: (key) => (key in held ? held[key] : null),
    setItem: (key, value) => {
      held[key] = value;
    },
    removeItem: (key) => {
      delete held[key];
    },
  };
}

/** A store that refuses everything — a private window, or site data blocked. */
function refusingStore(): CompactStore {
  return {
    getItem: () => {
      throw new Error("SecurityError: the operation is insecure");
    },
    setItem: () => {
      throw new Error("QuotaExceededError");
    },
    removeItem: () => {
      throw new Error("QuotaExceededError");
    },
  };
}

// -----------------------------------------------------------------------------
// What a value MEANS
// -----------------------------------------------------------------------------

describe("decodeCompact — one string is compact and nothing else is", () => {
  it("an absent key is DETAILS — the default every reader starts with", () => {
    expect(decodeCompact(null)).toBe(false);
  });

  it('"1" is compact', () => {
    expect(decodeCompact(COMPACT_ON)).toBe(true);
    expect(COMPACT_ON).toBe("1");
  });

  it('"0" is details, and would have been compact under a truthiness check', () => {
    // The reason this is an equality and not `Boolean(raw)`: every non-empty
    // string is truthy, so "0" and "false" would both have flipped the reader
    // into a view they never chose.
    expect(decodeCompact("0")).toBe(false);
    expect(decodeCompact("false")).toBe(false);
  });

  it("anything else at all is details", () => {
    // A value left by some future version of this build, or by a person with
    // devtools open. Unrecognised means the design's default, never a guess.
    expect(decodeCompact("")).toBe(false);
    expect(decodeCompact("true")).toBe(false);
    expect(decodeCompact("compact")).toBe(false);
    expect(decodeCompact("11")).toBe(false);
  });
});

// -----------------------------------------------------------------------------
// Reading
// -----------------------------------------------------------------------------

describe("readCompact", () => {
  it("is false when the key was never written", () => {
    expect(readCompact(fakeStore())).toBe(false);
  });

  it("is true when the key holds the one value that means compact", () => {
    expect(readCompact(fakeStore({ [COMPACT_STORAGE_KEY]: "1" }))).toBe(true);
  });

  it("ignores a value under some OTHER key", () => {
    // The window's own record lives beside this one and must not be read as it.
    expect(readCompact(fakeStore({ "colossus.subsetWindow.s-1": "1" }))).toBe(false);
  });

  it("is false, not a crash, when there is no store at all", () => {
    expect(readCompact(undefined)).toBe(false);
  });

  it("is false, not a crash, when the store THROWS", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(readCompact(refusingStore())).toBe(false);
    // Standing Rule 1: degraded, but not silent. A reader gets the default view
    // and an operator gets a line saying why.
    expect(warn).toHaveBeenCalled();
    warn.mockRestore();
  });
});

// -----------------------------------------------------------------------------
// Writing, and the round trip
// -----------------------------------------------------------------------------

describe("writeCompact", () => {
  it("writes the one value that means compact", () => {
    const store = fakeStore();
    writeCompact(store, true);
    expect(store.held[COMPACT_STORAGE_KEY]).toBe("1");
  });

  it("REMOVES the key for details rather than writing a second falsy value", () => {
    // The design says `"1"` or absent. Writing "0" would give this build two
    // ways of saying details, one of which no test asserts.
    const store = fakeStore({ [COMPACT_STORAGE_KEY]: "1" });
    writeCompact(store, false);
    expect(COMPACT_STORAGE_KEY in store.held).toBe(false);
  });

  it("survives a store with no room, and says so", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(() => writeCompact(refusingStore(), true)).not.toThrow();
    expect(warn).toHaveBeenCalled();
    warn.mockRestore();
  });

  it("survives having no store at all", () => {
    expect(() => writeCompact(undefined, true)).not.toThrow();
  });

  it("round-trips: what is written is what is read back", () => {
    // The guard against the read and the write drifting apart on the string.
    const store = fakeStore();
    writeCompact(store, true);
    expect(readCompact(store)).toBe(true);
    writeCompact(store, false);
    expect(readCompact(store)).toBe(false);
  });

  it("pressing the button twice leaves the store as it started", () => {
    // Because details REMOVES rather than writes, a reader who toggles and
    // toggles back has exactly what a reader who never pressed it has.
    const store = fakeStore();
    const before = JSON.stringify(store.held);
    writeCompact(store, true);
    writeCompact(store, false);
    expect(JSON.stringify(store.held)).toBe(before);
  });
});

describe("browserStore", () => {
  it("is undefined where there is no localStorage — this test run, exactly", () => {
    // vitest runs in node here (CLAUDE.md rule 30: no jsdom). The assertion is
    // that naming an absent global is not a crash, which is the same code path
    // a sandboxed frame takes.
    expect(browserStore()).toBeUndefined();
  });
});
