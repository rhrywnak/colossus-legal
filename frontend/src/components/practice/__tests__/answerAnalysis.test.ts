// =============================================================================
// answerAnalysis.test.ts — the switch that decides whether a model sees her words
// =============================================================================
//
// The pure half of CC_TASK_PRACTICE_POLISH_v1 item 3, and the reason that half
// is a module at all: this project renders no components in tests (CLAUDE.md
// rule 30), so a decision made inside `PracticeStart.tsx` is a decision nothing
// can check. The decision here is "does a model see her answer", and its default
// is OFF.
//
// The sibling of `scenario-timeline/__tests__/compactView.test.ts`, deliberately
// — same helper shape, same failure modes, same proofs.

import { afterEach, describe, expect, it, vi } from "vitest";

import {
  ANALYSIS_ON,
  ANALYSIS_STORAGE_KEY,
  analysisStateKey,
  browserStore,
  decodeAnalysis,
  readAnalysis,
  writeAnalysis,
  type AnalysisStore,
} from "../answerAnalysis";

afterEach(() => {
  vi.restoreAllMocks();
});

/** A store backed by a plain object, so a test can see what was written. */
function store(initial: Record<string, string> = {}): AnalysisStore & {
  held: Record<string, string>;
} {
  const held: Record<string, string> = { ...initial };
  return {
    held,
    getItem: (key) => held[key] ?? null,
    setItem: (key, value) => {
      held[key] = value;
    },
    removeItem: (key) => {
      delete held[key];
    },
  };
}

/** A store that throws on every access — a private window, or blocked data. */
const throwing: AnalysisStore = {
  getItem: () => {
    throw new Error("site data is blocked");
  },
  setItem: () => {
    throw new Error("site data is blocked");
  },
  removeItem: () => {
    throw new Error("site data is blocked");
  },
};

describe("what a stored value means", () => {
  it("turns the analysis on for exactly one string", () => {
    expect(decodeAnalysis(ANALYSIS_ON)).toBe(true);
    expect(decodeAnalysis("1")).toBe(true);
  });

  // ⚑ MUTATION: write the read as a truthiness check — `raw ? true : false` —
  // and BOTH of these flip to on. Either would send an answer to a model on a
  // page whose switch read "off".
  it("reads every other string as off, including the truthy ones", () => {
    expect(decodeAnalysis(null)).toBe(false);
    expect(decodeAnalysis("0")).toBe(false);
    expect(decodeAnalysis("false")).toBe(false);
    expect(decodeAnalysis("true")).toBe(false);
    expect(decodeAnalysis("")).toBe(false);
  });
});

describe("reading the switch", () => {
  it("is off for a browser that has never seen the page", () => {
    expect(readAnalysis(store())).toBe(false);
  });

  it("is on once the key holds the sentinel", () => {
    expect(readAnalysis(store({ [ANALYSIS_STORAGE_KEY]: ANALYSIS_ON }))).toBe(true);
  });

  // ⚑ THE FAILURE DIRECTION THAT MATTERS. A browser that refuses storage must
  // land on OFF — the state in which nothing she types leaves the building —
  // and must not throw on a page a witness is using the night before she
  // testifies.
  it("is off, loudly but without a banner, when the browser refuses", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(readAnalysis(throwing)).toBe(false);
    expect(warn).toHaveBeenCalledTimes(1);
  });

  it("is off when there is no storage at all", () => {
    expect(readAnalysis(undefined)).toBe(false);
  });
});

describe("remembering the choice", () => {
  it("writes the sentinel when it goes on", () => {
    const s = store();
    writeAnalysis(s, true);
    expect(s.held[ANALYSIS_STORAGE_KEY]).toBe(ANALYSIS_ON);
    expect(readAnalysis(s)).toBe(true);
  });

  // ⚑ REMOVES rather than writing "0": the design has one way to say off, and a
  // second one would be a value no test asserts and no reader can tell apart.
  it("removes the key when it goes off, leaving the store as it started", () => {
    const s = store();
    writeAnalysis(s, true);
    writeAnalysis(s, false);
    expect(ANALYSIS_STORAGE_KEY in s.held).toBe(false);
    expect(s.held).toEqual({});
    expect(readAnalysis(s)).toBe(false);
  });

  it("never throws when the browser refuses to remember", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(() => writeAnalysis(throwing, true)).not.toThrow();
    expect(warn).toHaveBeenCalledTimes(1);
  });

  it("does nothing, quietly, when there is no storage", () => {
    expect(() => writeAnalysis(undefined, true)).not.toThrow();
  });
});

describe("the state word", () => {
  // Two KEYS and not one template: the bar renders the state the switch is in,
  // and a template with a slot would have the page composing a word out of a
  // boolean with the halves living in different files.
  it("names one stored row per state, and they are different rows", () => {
    expect(analysisStateKey(true)).toBe("answer_analysis_on");
    expect(analysisStateKey(false)).toBe("answer_analysis_off");
    expect(analysisStateKey(true)).not.toBe(analysisStateKey(false));
  });
});

describe("the browser's own storage", () => {
  it("is undefined rather than a throw where there is none", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const held = globalThis.localStorage;
    // Merely NAMING `localStorage` throws in a sandboxed frame, which is why the
    // reference is inside the helper's `try`. Reproduced here with a getter.
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      get() {
        throw new Error("site data is blocked");
      },
    });
    try {
      expect(browserStore()).toBeUndefined();
      expect(warn).toHaveBeenCalledTimes(1);
    } finally {
      Object.defineProperty(globalThis, "localStorage", {
        configurable: true,
        value: held,
      });
    }
  });
});

// =============================================================================
// ⚑ THE CLOSED SET: which acts may ask a model to read an answer
// =============================================================================
//
// The ruling is "off ⇒ the read is NEVER requested", and a test that only
// checked the one path a person happens to think of would leave the others to
// drift. So: every act the answer flow can perform, each paired with what it
// asks for, and exactly ONE of them may ask for a read.
//
// The list is the vocabulary, not the wiring — that the pages actually call
// `submitPracticeAnswer` with this value is `practice.test.ts` (the request
// body) and `practicePolishSurface.test.ts` (the call sites). This is the rule
// those two are checked against.
describe("the closed set of answer-flow acts", () => {
  type Act = { name: string; submits: boolean; analysis: boolean };

  const ACTS: Act[] = [
    { name: "answer submitted while the analysis is ON", submits: true, analysis: true },
    { name: "answer submitted while the analysis is OFF", submits: true, analysis: false },
    { name: "opening the question page", submits: false, analysis: true },
    { name: "typing in the box", submits: false, analysis: true },
    { name: "showing an earlier version", submits: false, analysis: true },
    { name: "walking the deck in practice mode", submits: false, analysis: true },
    { name: "flicking the switch itself", submits: false, analysis: true },
  ];

  /** Does this act ask a model to read an answer? */
  const asksForARead = (act: Act): boolean => act.submits && act.analysis;

  it("asks for a read on exactly one act, and that act is the ON submit", () => {
    const asking = ACTS.filter(asksForARead).map((act) => act.name);
    expect(asking).toEqual(["answer submitted while the analysis is ON"]);
  });

  // MUTATION: `act.submits || act.analysis`, or dropping the `analysis` term —
  // the off submit starts asking and this goes red. Without this assertion the
  // one above could still pass on a rule that asked for reads on acts that make
  // no request at all.
  it("asks for nothing on every act that writes no answer", () => {
    for (const act of ACTS.filter((a) => !a.submits)) {
      expect(asksForARead(act)).toBe(false);
    }
  });
});
