// =============================================================================
// scanPillCaret.test.tsx — the arrow is a click target, not a decoration
// =============================================================================
//
// ## The defect this pins
//
// The `▾` was a flex SIBLING of the `<select>` it decorates. It carried
// `pointer-events: none`, so a click on it fell through — but what it fell
// through to was the pill wrapper, whose `onClick` calls `stopPropagation` to
// stop the header row folding underneath it. The select was NEXT to the caret,
// never under it. So a click on the arrow reached the dropdown never and the
// row never: the single most obvious target on the control did nothing at all.
//
// ## What can be asserted without a DOM
//
// There is no RTL and no jsdom here (CLAUDE.md rule 30), so this cannot fire a
// click and read the dropdown. `renderToStaticMarkup` gives the markup, and the
// markup is where the mechanism lives: the caret is only over the select if it
// is absolutely positioned inside a relative box that also holds the select,
// and the click only lands on the select if the select's own padding reserves
// the space the caret sits in. Those three facts together ARE the overlay —
// each is checked below, and any one of them regressing puts the arrow back
// beside the control where it was dead.
//
// The layout half was measured in a browser and recorded in
// CC_REPORT_SCAN_PILL_CARET_v1: `document.elementFromPoint` at the caret's own
// centre returns the `<select>`.

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import ScanSplitPill from "../ScanSplitPill";
import { model } from "./scanHeaderFixtures";

function markup(): string {
  return renderToStaticMarkup(
    <ScanSplitPill
      label="Scan"
      models={[model()]}
      selectedModel="unsloth/Qwen3.8-27B-NVFP4"
      canRun
      refusal={undefined}
      onOpenConfirm={() => {}}
      onSelect={() => {}}
    />,
  );
}

/** The `style="…"` of the element whose markup contains `needle`. */
function styleOfElementContaining(html: string, needle: string): string {
  const at = html.indexOf(needle);
  expect(at, `'${needle}' is in the markup`).toBeGreaterThan(-1);
  const tagStart = html.lastIndexOf("<", at);
  const tagEnd = html.indexOf(">", tagStart);
  const tag = html.slice(tagStart, tagEnd);
  const style = /style="([^"]*)"/.exec(tag);
  return style === null ? "" : style[1];
}

/** A numeric px value out of an inline style, e.g. `right:11px` → 11. */
function px(style: string, property: string): number {
  const found = new RegExp(`(?:^|;)\\s*${property}:\\s*(-?[\\d.]+)px`).exec(style);
  expect(found, `${property} is set in "${style}"`).not.toBeNull();
  return Number((found as RegExpExecArray)[1]);
}

describe("the caret is drawn OVER the select, not beside it", () => {
  it("takes no pointer events of its own", () => {
    // Necessary but nowhere near sufficient — this was already true while the
    // arrow was dead. It is the other two assertions that decide what the click
    // falls through TO.
    const caret = styleOfElementContaining(markup(), "▾");
    expect(caret).toMatch(/pointer-events:\s*none/);
  });

  it("is absolutely positioned inside a relative box that also holds the select", () => {
    const html = markup();
    expect(styleOfElementContaining(html, "▾")).toMatch(/position:\s*absolute/);

    // The relative box must WRAP the select — a relative ancestor that closed
    // before the select would position the caret against the pill instead, and
    // put it back outside the control's hit area.
    const slot = html.indexOf("position:relative");
    const select = html.indexOf("<select");
    const caret = html.indexOf("▾");
    expect(slot).toBeGreaterThan(-1);
    expect(select).toBeGreaterThan(slot);
    expect(caret).toBeGreaterThan(select);
  });

  it("reserves the caret's room in the SELECT's own padding, so the hit area spans it", () => {
    // This is the assertion that fails if the caret is moved back out. The
    // select's right padding has to reach past the caret's right inset plus the
    // glyph itself, or the arrow overhangs the control and the click misses.
    const html = markup();
    const inset = px(styleOfElementContaining(html, "▾"), "right");
    const paddingRight = px(styleOfElementContaining(html, "<select"), "padding-right");

    expect(
      paddingRight,
      `the select reserves ${paddingRight}px on its right but the caret sits ` +
        `${inset}px in — the glyph is outside the select's box and its click is lost`,
    ).toBeGreaterThan(inset);
  });

  it("keeps the caret out of the accessible name, which the select carries", () => {
    const html = markup();
    expect(html).toMatch(/aria-hidden="true"/);
    expect(html).toContain('aria-label="Scan"');
  });

  it("still renders one arrow, not the platform's as well", () => {
    // `appearance: none` is what removes the native arrow. Without it the pill
    // shows two, which reads as a rendering fault.
    const html = markup();
    expect(styleOfElementContaining(html, "<select")).toMatch(/appearance:\s*none/);
    expect(html.match(/▾/g)).toHaveLength(1);
  });
});
