// =============================================================================
// subsetModalStructure.test.ts — what the modal renders, and what it must not
// =============================================================================
//
// T6.5's "component" tier, met the way this codebase meets it: by scanning the
// source. No jsdom, no `@testing-library/*` — CLAUDE.md rule 30, and Roman's
// T5 ruling repeated it. Precedent: `onePageSurface.test.ts`.
//
// ## ⚑ THE ABSENCE ASSERTIONS ARE THE VALUABLE HALF
//
// Two of the three things T6 asks for are REMOVALS — no date input, no order
// arrows on an unpicked row — and a removal has no natural test. A date input
// that crept back would break no build and fail nothing. It would simply be
// wrong again, in the exact way Roman reported.
//
// ## What these CANNOT prove
//
// That the box is legible, that the drag feels right, that Save is reachable at
// 700px. A source scan reads structure, not pixels. Roman's walk and the two
// screenshots in the report are what know those. Stated here so nobody reads a
// green suite as a claim it does not make.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const HERE = join(__dirname, "..");
const read = (file: string): string => readFileSync(join(HERE, file), "utf8");

/**
 * Source with its comments removed.
 *
 * The standing hazard in this codebase: rules are documented beside the rules,
 * so a scanner searching for a forbidden string finds the paragraph explaining
 * why it is forbidden. Both comment forms go — the `//` lines and the JSX
 * `{/* … *\/}` blocks the modal is full of.
 */
function code(source: string): string {
  return source
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
}

const MODAL = code(read("SubsetModal.tsx"));
const STYLES = code(read("subsetModalStyles.ts"));

/**
 * Just `renderRow` — one picker row and everything in it.
 *
 * Cut at the modal's OWN return and not at the first `return (`, because
 * `renderRow` has one of those in the middle of it. Getting that boundary wrong
 * is how a fence ends up asserting over three lines and passing for the wrong
 * reason, which is what the first draft of this file did.
 */
const ROW = MODAL.slice(
  MODAL.indexOf("const renderRow"),
  MODAL.indexOf('return (\n    <div style={m.scrim}'),
);

describe("the picker's rows carry no editor for the event (T6.2, D1/D9)", () => {
  it("renders NO date input anywhere in the modal", () => {
    // A subset is a list of REFERENCES to events that already exist. A date
    // input here would be an edit control for something this screen does not
    // own — and, before T6.2, one that showed "2009-04-01" for a source that
    // said only "April 2009".
    expect(MODAL).not.toContain('type="date"');
    expect(MODAL).not.toMatch(/type=\{?["']date["']\}?/);
  });

  it("renders the date through the shared cell model, not a raw field", () => {
    // `dateCell` is where the format, the caption and the amber decision are
    // made — and where a test can reach them.
    expect(MODAL).toContain("dateCell(event, wording)");
    expect(MODAL).not.toContain("{event.event_date}");
  });

  it("leaves exactly three controls in a row: the tick, the arrows and the note", () => {
    // Every `<input`, `<select`, `<textarea` and `<button` inside `renderRow`.
    const controls = ROW.match(/<(input|select|textarea|button)\b/g) ?? [];
    expect(controls.sort()).toEqual(["<button", "<button", "<input", "<input"]);
    // …and the two inputs are the checkbox and the note, in that order.
    expect(ROW).toMatch(/type="checkbox"[\s\S]*type="text"/);
  });
});

describe("the order arrows are for picked rows only", () => {
  it("guards BOTH arrows behind the picked flag", () => {
    // The arrows live inside one `{on && ( … )}` block. Asserting the guard
    // and the two glyphs are in that block is what says an unpicked row draws
    // no arrows — the mockup's rule, and the reason an unpicked row has no
    // number to move.
    const guard = ROW.slice(ROW.indexOf("{on && ("), ROW.indexOf("</>"));
    expect(guard).toContain("▲");
    expect(guard).toContain("▼");
    expect((ROW.match(/[▲▼]/g) ?? []).length).toBe(2);
  });

  it("disables the note on an unpicked row instead of hiding it", () => {
    // The author must see what is being left out — the picker's whole premise.
    expect(ROW).toContain("disabled={!on}");
  });
});

describe("the box is movable and never jammed (T6.3, D7)", () => {
  it("drags by its title bar, and by nothing else", () => {
    expect(MODAL).toContain("dragHandleClassName={DRAG_HANDLE_CLASS}");
    // The class goes on the HEAD and on nothing else — the body scrolls, and a
    // box draggable by a scrolling body cannot be scrolled.
    expect((MODAL.match(/DRAG_HANDLE_CLASS/g) ?? []).length).toBe(3);
    expect(MODAL).toContain("style={m.head} className={DRAG_HANDLE_CLASS}");
  });

  it("stays inside the browser window and cannot be resized", () => {
    expect(MODAL).toContain('bounds="window"');
    expect(MODAL).toContain("enableResizing={false}");
  });

  it("draws the grip with its STORED name, not an English one", () => {
    expect(MODAL).toContain('title={cw(wording, "subsets_modal_drag_label")}');
    expect(MODAL).toContain("⠿");
  });

  it("does NOT persist where the reader dragged it", () => {
    // A modal reopens centred. Persisting a position would also mean a box that
    // reopens off-screen after a window resize, which is the defect again.
    expect(MODAL).not.toContain("onDragStop");
    expect(MODAL).not.toContain("localStorage");
  });

  it("opens 48px down, 860 wide, capped at the viewport less 96", () => {
    expect(STYLES).toContain("export const MODAL_WIDTH = 860");
    expect(STYLES).toContain("export const MODAL_TOP = 48");
    expect(STYLES).toContain("export const MODAL_MARGIN = 96");
    expect(STYLES).toContain("maxHeight: `calc(100vh - ${MODAL_MARGIN}px)`");
    // The LIST scrolls, not the box: header, form and footer stay put.
    expect(STYLES).toMatch(/export const body: CSSProperties = \{[\s\S]*?overflowY: "auto"/);
  });
});

describe("the banner tells the truth in halves (T6.4, D2)", () => {
  it("builds both halves in the pure model and renders them in ONE box", () => {
    expect(MODAL).toContain("bannerModel(wording, failure)");
    // One `m.banner` element, with the green half nested inside it.
    expect((MODAL.match(/style=\{m\.banner\}/g) ?? []).length).toBe(1);
    expect(MODAL).toContain("style={m.bannerSaved}");
  });

  it("no longer says one flat sentence for a save that half-landed", () => {
    // The withdrawn line. `write_failed_template` is still the right thing for
    // a single failed write elsewhere; it is the wrong thing for two calls.
    expect(MODAL).not.toContain("write_failed_template");
  });
});

describe("the standing colour ruling holds in this file (three rejections)", () => {
  it("uses --accent-primary as an ink and a fill, never as a hairline", () => {
    // Every `border`/`borderBottom`/`borderTop`/`borderLeft` declaration in the
    // modal's stylesheet, checked for the accent token. A phase header's
    // `borderLeft` carries a phase COLOUR from the payload and is exempt by
    // being a coloured rule rather than a hairline — it is matched and shown to
    // be a template literal, not the token.
    const borders = STYLES.match(/border[A-Za-z]*: *[`"][^`"]*[`"]/g) ?? [];
    expect(borders.length).toBeGreaterThan(5);
    expect(borders.filter((d) => d.includes("--accent-primary"))).toEqual([]);
  });
});
