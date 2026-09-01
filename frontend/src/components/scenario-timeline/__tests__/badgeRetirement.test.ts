// =============================================================================
// badgeRetirement.test.ts — the ⚑ came off two surfaces, and stays off
// =============================================================================
//
// Roman retired the "date to confirm" badge on 2026-08-31, reversing his own T4
// ruling: it could only read `approximate`, so it claimed four of the case's
// thirty-one events needed a date confirmed — including two nobody has ever
// flagged. A badge that makes a false claim about the record is worse than no
// badge.
//
// ## ⚑ WHY THIS FILE EXISTS AT ALL
//
// A removal has no natural test. The pure tests beside `footerLine` and
// `dateCell` prove the two MODELS produce no ⚑, and that is most of it — but
// the badge was JSX, and JSX is where it would come back. `test-auditor` named
// exactly that as the one thing it believed could not be asserted without a
// component tier this project does not have (CLAUDE.md rule 30).
//
// It can be asserted, the way this codebase asserts everything else about
// components: by reading the source. Precedent `subsetModalStructure.test.ts`,
// and `onePageSurface.test.ts` before it.
//
// ## What this CANNOT prove
//
// That the retired pill looked right when it existed, or that the rows read
// well without it. A source scan reads structure, not pixels. The screenshots
// in the T6 round-two report are what know that.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const SCENARIO = join(__dirname, "..");
const TIMELINE = join(__dirname, "..", "..", "timeline");

/**
 * Source with its comments removed.
 *
 * The standing hazard: this codebase documents its rules beside its rules, and
 * both files below now carry a paragraph explaining what was removed and why.
 * A scanner that did not strip comments would find the word "⚑" in the
 * explanation and report the badge as still present — passing, or failing, for
 * entirely the wrong reason.
 */
function code(source: string): string {
  return source
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
}

const WINDOW_ROW = code(readFileSync(join(SCENARIO, "SubsetWindowBody.tsx"), "utf8"));
const PICKER_CELL = code(readFileSync(join(TIMELINE, "pickerDateCell.ts"), "utf8"));
const STYLES = code(readFileSync(join(SCENARIO, "windowStyles.ts"), "utf8"));
const ROWS = code(readFileSync(join(SCENARIO, "subsetRows.ts"), "utf8"));

describe("the window row draws no badge (T4.1, retired)", () => {
  it("renders no ⚑ anywhere", () => {
    expect(WINDOW_ROW).not.toContain("⚑");
  });

  it("does not import the predicate — the import IS the fence", () => {
    // Neither surface can render the badge again without re-adding this
    // import, which is a visible code change rather than a silent data one.
    expect(WINDOW_ROW).not.toContain("isDateToConfirm");
  });

  it("asks the store for no badge row", () => {
    expect(WINDOW_ROW).not.toContain("subsets_date_to_confirm_badge");
  });

  it("still marks what is TRUE — the gap badge and the date caption stay", () => {
    // The ruling took the false claim, not the true ones. A scan that only
    // asserted absence would pass just as happily on a file that had lost the
    // gap badge too.
    expect(WINDOW_ROW).toContain('cw(wording, "subsets_gap_badge_label")');
    expect(WINDOW_ROW).toContain("dateCaption(event");
    expect(WINDOW_ROW).toContain("ws.eventDate(rule, event.approximate)");
  });
});

describe("the picker cell draws no badge (T6.2, retired)", () => {
  it("renders no ⚑ and asks for no badge row", () => {
    expect(PICKER_CELL).not.toContain("⚑");
    expect(PICKER_CELL).not.toContain("subsets_date_to_confirm_badge");
    expect(PICKER_CELL).not.toContain("isDateToConfirm");
  });

  it("still reads the two precision captions, which state what the source said", () => {
    expect(PICKER_CELL).toContain('cw(wording, "subsets_precision_month_label")');
    expect(PICKER_CELL).toContain('cw(wording, "subsets_precision_year_label")');
  });
});

describe("what went with the badge, and what deliberately did not", () => {
  it("drops the pill style it dressed", () => {
    // Unlike the predicate, a style nothing renders is just a style nothing
    // renders. `gapBadge` — the OTHER amber pill, marking a different fact —
    // survives, and the two used to be drawn unalike on purpose.
    expect(STYLES).not.toContain("dateFlag");
    expect(STYLES).toContain("export const gapBadge");
  });

  it("drops the footer's counter", () => {
    expect(ROWS).not.toContain("flagCount");
  });

  it("KEEPS the predicate, exported and unread, on Roman's instruction", () => {
    // The exception to "delete what nothing calls": a recorded decision with a
    // named successor. When a real `date_to_confirm` column lands on
    // `chronology_events`, this is the ONE place that changes, and both
    // surfaces get the badge back together or not at all.
    expect(ROWS).toContain("export function isDateToConfirm");
  });

  it("leaves the footer saying only how many events", () => {
    // ⚑ THE FIRST DRAFT OF THIS ASSERTION DID NOT WORK, and the way it failed
    // is worth keeping. It cut the function at the first `}` — which is the one
    // closing `{ count: rows.length }`, three tokens in — so it inspected a
    // fragment that could not contain a second half whatever the code said. A
    // mutation that put "· 2 ⚑" back walked straight through it.
    //
    // The body is cut at the closing brace in column ONE instead, which is the
    // only brace in this file's style that ends a top-level function.
    expect(ROWS).toContain('cw(wording, "subsets_window_footer_events_template")');
    const from = ROWS.indexOf("export function footerLine");
    const body = ROWS.slice(from, ROWS.indexOf("\n}", from) + 2);
    expect(body).toContain("return");
    expect(body).not.toContain("⚑");
    // One statement: the fill IS the line. A second half would need a second.
    expect(body.match(/return/g)).toHaveLength(1);
    expect(body).not.toContain("`");
  });
});
