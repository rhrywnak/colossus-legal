// =============================================================================
// answerAnalysis.ts — "Answer analysis", remembered for this browser
// =============================================================================
//
// The switch on the practice bar decides whether a model is asked to read what
// Marie types. This module is what a stored value MEANS, how to read it, how to
// write it, and what to do when the browser refuses.
//
// The sibling of `scenario-timeline/compactView.ts`, deliberately down to the
// shape of its functions: that is the pattern this project has already ruled on
// for a remembered view preference, and a second pattern for the same job would
// be a second set of failure modes to reason about.
//
// ## ⚑ WHY A DECISION THIS SMALL IS A MODULE
//
// CLAUDE.md rule 30: this project has no component-testing tier, so a decision
// made inside `PracticeStart.tsx` is a decision no test can reach. The decision
// here is "does a model see her answer", which is not one to leave unwatched.
//
// ## ⚑ ONE KEY FOR EVERY SCENARIO, AND FOR BOTH SURFACES
//
// The switch is set on the deck page and read on the question page — two
// addresses, one browser. `localStorage` is what carries it between them, which
// is also why it is not scoped to a scenario: Marie either wants her answers
// read tonight or she does not, and re-choosing it per story is the friction
// that would stop her using the switch at all.
//
// ## Domain note: OFF is the default, and off is a promise
//
// A browser with no stored value, a browser that has never seen this page, and a
// browser that refuses storage all mean the same thing: OFF, no model call. The
// default is the cautious direction — the state in which nothing she types
// leaves the building — so every failure lands there rather than on a surprise
// call.
//
// ## The Standing Rule 1 carve-out this takes, and its limit
//
// A browser that refuses storage degrades to the default with a `console.warn`
// and no banner. That is the cosmetic-browser-preference carve-out the rule
// names in as many words. It does NOT extend one inch further: whether a read is
// requested rides on the answer REQUEST, which keeps the full `.catch()` and
// error-UI treatment it has always had.

/** The one key. Absent means off; `"1"` means on. Nothing else. */
// STRUCTURAL: a `localStorage` key name is wire vocabulary between this build
// and the reader's own browser, not deployment configuration. There is no server
// in the read path that could supply it, and changing it would orphan every
// reader's stored preference rather than move it — so it cannot vary by
// environment and must not be reachable from one.
export const ANALYSIS_STORAGE_KEY = "colossus.practiceAnswerAnalysis";

/**
 * The only stored value that means on.
 *
 * A named constant rather than a literal in three places because the write and
 * the read have to agree, and a disagreement between them is a switch that
 * appears to do nothing after a reload.
 */
// STRUCTURAL: the sentinel the write and the read must agree on — a protocol
// constant on the same wire as the key above.
export const ANALYSIS_ON = "1";

/**
 * The slice of `Storage` this feature uses.
 *
 * ## Rust Learning: a trait's worth of surface, and no more
 *
 * The whole `Storage` interface has eight members; this names the three it
 * calls. Taking the narrow shape rather than `Storage` is the TypeScript
 * equivalent of accepting `impl Read` instead of `File`: it says what is
 * actually required, and it lets a test hand over three lines of object —
 * including one that THROWS — where the real type would demand a browser.
 */
export type AnalysisStore = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
};

/**
 * What a stored value means. The whole vocabulary, in one place.
 *
 * Exactly one string turns the analysis on. Anything else — `null` because the
 * key was never written, `"0"`, `"false"`, a value left by some future version
 * of this build — means OFF, which is the design's default.
 *
 * Written as an equality rather than a truthiness check on purpose: `"0"` and
 * `"false"` are both truthy strings, and either would have sent an answer to a
 * model on a page where the switch read "off".
 */
export function decodeAnalysis(raw: string | null): boolean {
  return raw === ANALYSIS_ON;
}

/**
 * The browser's own storage, or `undefined` where there is none.
 *
 * Merely NAMING `localStorage` throws in a sandboxed frame and in a browser with
 * site data blocked — the access itself, before any method is called — so the
 * reference is inside the `try` rather than beside it.
 */
export function browserStore(): AnalysisStore | undefined {
  try {
    // best-effort: a COSMETIC preference about a page's own behaviour. A browser
    // with no storage gets the default — off, the safe direction — which is also
    // what it got before this feature existed. `console.warn` keeps it
    // observable; a banner saying "your browser will not remember this switch"
    // would be noise in front of a witness the night before she testifies.
    return typeof localStorage === "undefined" ? undefined : localStorage;
  } catch (e: unknown) {
    console.warn("The browser has no storage; answer analysis stays off.", e);
    return undefined;
  }
}

/**
 * Is the answer analysis on? Never throws; `false` when in doubt.
 *
 * @param store the browser's storage, or `undefined` — [`browserStore`]
 */
export function readAnalysis(store: AnalysisStore | undefined): boolean {
  try {
    return decodeAnalysis(store?.getItem(ANALYSIS_STORAGE_KEY) ?? null);
  } catch (e: unknown) {
    // best-effort, as above: a read that throws is a reader who gets no model
    // call, which is the state that costs nothing and surprises nobody.
    console.warn("Could not read the answer-analysis switch; leaving it off.", e);
    return false;
  }
}

/**
 * Remember the choice. Never throws.
 *
 * ⚑ `false` REMOVES the key rather than writing `"0"`. The design says the value
 * is `"1"` or absent, and the two ways of saying "off" would otherwise both
 * exist — one of which no test asserts and no reader can tell apart. Absent is
 * also what a reader who has never touched the switch has, so flicking it twice
 * returns the store to exactly the state it started in.
 */
export function writeAnalysis(store: AnalysisStore | undefined, on: boolean): void {
  try {
    if (store === undefined) return;
    if (on) store.setItem(ANALYSIS_STORAGE_KEY, ANALYSIS_ON);
    else store.removeItem(ANALYSIS_STORAGE_KEY);
  } catch (e: unknown) {
    // best-effort: the switch works for this visit and is forgotten by the next.
    console.warn("Could not remember the answer-analysis switch.", e);
  }
}

/**
 * The wording key for the state word beside the label.
 *
 * Two keys and not one template: the bar shows the state the switch is IN, and a
 * template with a slot would have the page composing a word out of a boolean
 * with the two halves of that composition living in different places.
 */
export function analysisStateKey(on: boolean): string {
  return on ? "answer_analysis_on" : "answer_analysis_off";
}
