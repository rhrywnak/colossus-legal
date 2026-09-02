// =============================================================================
// emptyStory.test.ts — the window's body says something when there is nothing
// =============================================================================
//
// P1, and the defect it closes was seen on DEV: `SubsetWindowBody` maps the
// subset's events into the scrolling body, so a subset with NO events rendered
// a blank padded band. The title bar named the story, the footer said "0
// events", and the space between them said nothing at all.
//
// ## ⚑ WHY THAT IS A BUG AND NOT A TIDY EMPTY STATE
//
// Loading has a sentence and a failed read has one. An empty story had none, so
// the three states of one slot produced two observables — the Standing Rule 1
// failure in as many words. A reader cannot tell "this story has nothing in it
// yet" from "this window did not load", and the second is the one that would
// send them looking for help they do not need.
//
// ## What this CANNOT prove
//
// That the sentence LOOKS right in the band — one muted line where fifteen rows
// would be. A source scan reads structure, not pixels; Roman previewing a subset
// with no events on DEV is what knows that. Technique and limit as in
// `badgeRetirement.test.ts` and `dockPopoutCallSite.test.ts`.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

/**
 * Source with its comments removed.
 *
 * The standing hazard for every scanner in this directory: the file now carries
 * a paragraph explaining the empty state, naming the guard and the wording key.
 * A scanner that read comments would find both in the PROSE and pass on a file
 * that had lost the code.
 */
function code(source: string): string {
  return source
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
}

const BODY = code(readFileSync(join(__dirname, "..", "SubsetWindowBody.tsx"), "utf8"));

describe("a story with no events says so (P1)", () => {
  it("renders the stored sentence behind a zero guard", () => {
    expect(BODY).toContain("subset.events.length === 0 &&");
    expect(BODY).toContain('cw(wording, "subsets_window_no_events")');
  });

  it("names the key LITERALLY, so the backend's reach scan can see it", () => {
    // `chronology_wording_reach_tests.rs` scans this directory for
    // `cw(<wording>, "key")`. A key assembled from a variable is a word no
    // scanner can see and no boot check can guard.
    const at = BODY.indexOf('cw(wording, "subsets_window_no_events")');
    expect(at).toBeGreaterThan(-1);
  });

  it("puts the sentence inside the scrolling body, where the rows would be", () => {
    // Not above the body and not in the footer: it occupies the slot the map
    // leaves empty, which is the slot the reader is looking at.
    const body = BODY.indexOf("ws.body}");
    const sentence = BODY.indexOf("subsets_window_no_events");
    const map = BODY.indexOf("subset.events.map(");
    expect(body).toBeGreaterThan(-1);
    expect(sentence).toBeGreaterThan(body);
    expect(sentence).toBeLessThan(map);
  });

  it("is a GUARD beside the map, not a branch around it", () => {
    // Deliberate, and it is what keeps this change mergeable: with zero events
    // the map renders nothing anyway, so the rows below are untouched and the
    // three-way merge with the compact branch — which edits inside that same
    // map — stays clean. A ternary would have re-indented every row line.
    expect(BODY).not.toContain("subset.events.length === 0 ?");
  });

  it("does NOT special-case the footer, which still counts", () => {
    // P1 says so in as many words. "0 events" and "This story has no events
    // yet." answer different questions — how many, and what to do about it —
    // and neither replaces the other.
    expect(BODY).toContain("footerLine(subset.events, wording)");
    const footer = BODY.indexOf("footerLine(subset.events, wording)");
    expect(BODY.slice(Math.max(0, footer - 200), footer)).not.toContain("length === 0");
  });
});
