// =============================================================================
// compactBody.test.ts — what "Dates only" hides, and the one thing it may not
// =============================================================================
//
// C4 asks for it by name: "a test that reads `SubsetWindowBody.tsx` and asserts
// the five hidden elements are each behind the compact flag and the gap badge
// is NOT".
//
// ## ⚑ WHY THE BADGE IS THE ASSERTION THAT MATTERS
//
// The other four are a reading preference: hide a fact paragraph by mistake and
// a reader presses `Show details`. The GAP BADGE is different in kind. It marks
// an event the chronology soft-deleted — the story saying "this happened and it
// is not on our timeline yet" — and design §11 calls the visible gap half a
// subset's value. A compact view that swallowed it would hand a witness a story
// with a hole in it and no mark where the hole is, the night before she is
// asked about exactly that. Nothing else in this build would notice: `tsc`
// passes, the build passes, and the row simply renders one span shorter.
//
// ## What this CANNOT prove
//
// That the compact row LOOKS right — one line, the date still bold, the title
// beside it. A source scan reads structure, not pixels, and Roman pressing the
// button on DEV is what knows that. Same technique and same limit as
// `dockPopoutCallSite.test.ts` and `badgeRetirement.test.ts` before it.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

/**
 * Source with its comments removed.
 *
 * The standing hazard this directory has hit twice: the file now carries a
 * header paragraph naming all five hidden elements and the badge that stays. A
 * scanner that did not strip comments would find every name it looks for in
 * the PROSE and pass on a file that had lost the code.
 */
function code(source: string): string {
  return source
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
}

const BODY = code(readFileSync(join(__dirname, "..", "SubsetWindowBody.tsx"), "utf8"));

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

describe("the five things `Dates only` hides (C2)", () => {
  it("hides the subset description strip", () => {
    expect(BODY).toContain("!compact && subset.description");
  });

  it("hides the tag pills", () => {
    expect(BODY).toContain("!compact &&\n                    event.tags.map");
  });

  it("hides the sentence under a gap row", () => {
    // `subsets_removed_event_line` — the paragraph, not the badge above it.
    expect(BODY).toContain("!compact && row.removed &&");
  });

  it("hides each row's fact paragraph", () => {
    expect(BODY).toContain("!compact && !row.removed &&");
  });

  it("hides the story note", () => {
    expect(BODY).toContain("!compact && row.subset_note");
  });

  it("hides FIVE things and no sixth", () => {
    // The fence in the other direction. A future edit that puts a sixth element
    // behind the flag has changed what this view means, and should have to say
    // so here rather than arriving as a row that quietly lost something.
    expect(occurrences(BODY, "!compact &&")).toHaveLength(5);
  });
});

describe("what `Dates only` KEEPS (C2)", () => {
  it("keeps the gap badge — it is NOT behind the compact flag", () => {
    // Read as the compiler reads it: the guard that actually encloses the
    // badge, not merely the absence of the word nearby.
    const badge = BODY.indexOf("ws.gapBadge");
    expect(badge).toBeGreaterThan(-1);
    const guard = BODY.slice(BODY.lastIndexOf("{row.removed", badge), badge);
    expect(guard).not.toContain("compact");
  });

  it("keeps the year / phase dividers", () => {
    expect(BODY).toContain("{divider !== null && (");
    const divider = BODY.indexOf("{divider !== null && (");
    expect(BODY.slice(Math.max(0, divider - 120), divider)).not.toContain("compact");
  });

  it("keeps the whole date column, caption and all", () => {
    expect(BODY).toContain("ws.eventDate(rule, event.approximate)");
    expect(BODY).toContain('{caption !== "" && <small style={ws.eventDateCaption}>');
    const date = BODY.indexOf("ws.eventDate(rule");
    expect(BODY.slice(Math.max(0, date - 200), date)).not.toContain("compact");
  });

  it("keeps the title, struck when the event was removed", () => {
    expect(BODY).toContain("row.removed ? ws.removedTitle : ws.eventTitle");
  });

  it("keeps the click that opens the event in a new tab", () => {
    expect(BODY).toContain("onClick={() => onOpenEvent(event.id)}");
  });

  it("keeps the footer and its count", () => {
    expect(BODY).toContain("footerLine(subset.events, wording)");
  });
});

describe("the control (C1)", () => {
  it("is one button in the FOOTER, in the existing footLink style", () => {
    // Footer and not the title bar: this file is the one tree both containers
    // render, so a control here reaches the in-page window and the popped-out
    // window for free — and `ScenarioTimelineDock`, already over Rule 17, is
    // not touched at all.
    const foot = BODY.indexOf("ws.foot}");
    const toggle = BODY.indexOf("onClick={toggleCompact}");
    expect(foot).toBeGreaterThan(-1);
    expect(toggle).toBeGreaterThan(foot);
    expect(BODY.slice(foot, toggle)).toContain("ws.footLink");
  });

  it("names BOTH stored rows, and names them literally", () => {
    // Literal `cw(wording, "…")` calls, because that is what the backend's
    // chronology reach test scans this directory for. A key assembled from a
    // variable would be a word no scanner can see.
    expect(BODY).toContain('cw(wording, "subsets_window_show_details")');
    expect(BODY).toContain('cw(wording, "subsets_window_dates_only")');
  });

  it("says `Dates only` when showing details and `Show details` when compact", () => {
    // The labels are the way round that names what pressing the button DOES.
    // Inverted, each would name the state the reader is already in.
    const ternary = BODY.slice(BODY.indexOf("{compact"), BODY.indexOf("onClick={toggleCompact}") + 400);
    const showDetails = ternary.indexOf("subsets_window_show_details");
    const datesOnly = ternary.indexOf("subsets_window_dates_only");
    expect(showDetails).toBeGreaterThan(-1);
    expect(datesOnly).toBeGreaterThan(showDetails);
  });
});

describe("the choice is remembered where C3 says", () => {
  it("reads and writes through `compactView`, never localStorage directly", () => {
    // Every decision about the stored value lives in the pure module, where a
    // test can reach it. A direct `localStorage` call here would be a decision
    // made where nothing can check it — and an unguarded one, in a browser
    // that throws on the access itself.
    expect(BODY).toContain('from "./compactView"');
    expect(BODY).not.toContain("localStorage");
  });

  it("reads ONCE at mount, which is what survives a reload and a pop-out", () => {
    // A lazy `useState` initialiser: it runs on the first render of this tree
    // and never again. A reload rebuilds the component; popping the window out
    // mounts it in the other container. Both re-read the stored value, and
    // neither needs the dock to carry the flag.
    expect(BODY).toContain("useState<boolean>(() => readCompact(browserStore()))");
  });

  it("holds no per-scenario key of its own", () => {
    // C3: one key for every scenario. The window's own record is
    // `colossus.subsetWindow.<scenarioId>` and this view is deliberately not
    // part of it — see `compactView.ts`.
    expect(BODY).not.toContain("subsetWindow");
    expect(BODY).not.toContain("scenarioId");
  });
});
