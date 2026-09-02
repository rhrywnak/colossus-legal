// =============================================================================
// compactView.ts — "Dates only", remembered once for every story
// =============================================================================
//
// The sibling of `subsetRows.ts` and `popout.ts`, and it exists for the same
// reason both do: this project has no component-testing tier (CLAUDE.md rule
// 30), so a decision made inside `SubsetWindowBody.tsx` is a decision no test
// can reach. Four of them live here — what a stored value MEANS, how to read
// it, how to write it, and what to do when the browser refuses.
//
// ## ⚑ ONE KEY FOR EVERY SCENARIO, NOT ONE PER SCENARIO
//
// This is deliberately NOT part of `colossus.subsetWindow.<scenarioId>`, which
// remembers where a window sits and which subset it shows — facts about one
// story. "Dates only" is a fact about the READER: Marie is rehearsing rather
// than referring, and she is rehearsing for the whole evening, not for one
// story out of five. Filing it per scenario would make her press the button
// again on every story she opened, which is the friction this view removes.
//
// ## Domain note: why a reading view needs remembering at all
//
// The night before testimony the witness knows the ORDER of events; what she
// is refreshing is the DATES against titles she already knows. The other
// side's play is "she cannot keep the events straight". A view she has to
// re-choose on every story is a view she stops using.
//
// ## The Standing Rule 1 carve-out this takes, and its limit
//
// A browser that refuses storage degrades to the DEFAULT — details — with a
// `console.warn` and no banner. That is the cosmetic-browser-preference
// carve-out the rule names in as many words, and the same one
// `ScenarioTimelineDock`'s window-position helpers take. It does NOT extend
// one inch further: nothing about the subset itself is stored here, and no
// `fetch` in this feature may take it.

/** The one key. Absent means details; `"1"` means compact. Nothing else. */
// STRUCTURAL: a `localStorage` key name is wire vocabulary between this build
// and the reader's own browser, not deployment configuration. There is no
// server in the read path that could supply it, and changing it would orphan
// every reader's stored preference rather than move it — so it cannot vary by
// environment and must not be reachable from one.
export const COMPACT_STORAGE_KEY = "colossus.subsetCompact";

/**
 * The only stored value that means compact.
 *
 * A named constant rather than a literal in three places because the write and
 * the read have to agree, and a disagreement between them is a button that
 * appears to do nothing after a reload.
 */
// STRUCTURAL: the sentinel the write and the read must agree on — a protocol
// constant on the same wire as the key above, and named here so the two sides
// cannot drift. Not a deployment choice: a build that stored a different
// string would simply stop recognising what it had already written.
export const COMPACT_ON = "1";

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
export type CompactStore = {
  getItem: (key: string) => string | null;
  setItem: (key: string, value: string) => void;
  removeItem: (key: string) => void;
};

/**
 * What a stored value means. The whole vocabulary, in one place.
 *
 * Exactly one string is compact. Anything else — `null` because the key was
 * never written, `"0"`, `"true"`, a value left by some future version of this
 * build — means DETAILS, which is the design's default and the view every
 * reader who never presses the button keeps.
 *
 * Written as an equality rather than a truthiness check on purpose: `"0"` and
 * `"false"` are both truthy strings, and either would have flipped the reader
 * into a view they never chose.
 */
export function decodeCompact(raw: string | null): boolean {
  return raw === COMPACT_ON;
}

/**
 * The browser's own storage, or `undefined` where there is none.
 *
 * Merely NAMING `localStorage` throws in a sandboxed frame and in a browser
 * with site data blocked — the access itself, before any method is called — so
 * the reference is inside the `try` rather than beside it.
 */
export function browserStore(): CompactStore | undefined {
  try {
    // best-effort: a COSMETIC reading preference. A browser with no storage
    // gets the design's default view, which is the same view it got before
    // this feature existed. `console.warn` keeps it observable; a banner for
    // "your browser will not remember which view you like" would be noise in
    // front of a witness the night before she testifies.
    return typeof localStorage === "undefined" ? undefined : localStorage;
  } catch (e: unknown) {
    console.warn("The browser has no storage; the timeline window opens with details.", e);
    return undefined;
  }
}

/**
 * Is the reader in the compact view? Never throws; `false` when in doubt.
 *
 * @param store the browser's storage, or `undefined` — [`browserStore`]
 */
export function readCompact(store: CompactStore | undefined): boolean {
  try {
    return decodeCompact(store?.getItem(COMPACT_STORAGE_KEY) ?? null);
  } catch (e: unknown) {
    // best-effort, as above: a read that throws is a reader who gets details.
    console.warn("Could not read which timeline view you prefer; showing details.", e);
    return false;
  }
}

/**
 * Remember the choice. Never throws.
 *
 * ⚑ `false` REMOVES the key rather than writing `"0"`. The design says the
 * value is `"1"` or absent, and the two ways of saying "details" would
 * otherwise both exist — one of which no test asserts and no reader can tell
 * apart. Absent is also what a reader who has never pressed the button has, so
 * pressing it twice returns the store to exactly the state it started in.
 */
export function writeCompact(store: CompactStore | undefined, compact: boolean): void {
  try {
    if (store === undefined) return;
    if (compact) store.setItem(COMPACT_STORAGE_KEY, COMPACT_ON);
    else store.removeItem(COMPACT_STORAGE_KEY);
  } catch (e: unknown) {
    // best-effort: the view works for this visit and is forgotten by the next.
    console.warn("Could not remember which timeline view you prefer.", e);
  }
}
