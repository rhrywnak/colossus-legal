// =============================================================================
// scanHeaderAnatomy.test.tsx — the header, checked against RENDERED MARKUP
// =============================================================================
//
// ## Why this file can exist when component testing "is not set up"
//
// CLAUDE.md Rule 30 records that RTL + jsdom are not configured here, and that
// remains true — nothing below mounts a component, fires an event or reads a
// layout. `renderToStaticMarkup` is a pure function from React elements to an
// HTML string and needs no DOM at all. The same carve-out `factCardAnatomy`
// took, for the same reason: some things are only true of the OUTPUT.
//
// `scanConfirmModel.test.ts` proves the machine will not start a scan. What a
// data test cannot reach is whether this component is WIRED to that machine —
// a header is free to ignore the reducer it imports, and only its markup shows
// whether it did. That is the half this file covers.

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import ScenarioFactsHeader from "../ScenarioFactsHeader";
import type { ScanHeaderApi } from "../scanHeaderApi";
import type { CardGrammarWording } from "../../services/evidenceLinks";
import { optionsFixture } from "./cardFixtures";
import { model, wording } from "./scanHeaderFixtures";

/** Every scan the header could be handed. Runnable and timed by default. */
function scan(overrides: Partial<ScanHeaderApi> = {}): ScanHeaderApi {
  return {
    headerLine: "Last scan Sep 11, 12:06 AM · cancelled · 313 candidates",
    running: false,
    canRun: true,
    onRun: () => {},
    history: null,
    models: [model({ measured_seconds_per_candidate: 10.54 })],
    selectedModel: "unsloth/Qwen3.8-27B-NVFP4",
    onSelect: () => {},
    candidateCount: 313,
    wording: wording(),
    ...overrides,
  };
}

const grammar = optionsFixture().card_grammar as CardGrammarWording;

function markup(overrides: Partial<ScanHeaderApi> = {}): string {
  return renderToStaticMarkup(
    <ScenarioFactsHeader
      scan={scan(overrides)}
      historyOpen={false}
      onToggleHistory={() => {}}
      grammar={grammar}
      onResetOrder={() => {}}
      open
      onToggleOpen={() => {}}
    />,
  );
}

describe("the header row, as the mockup draws it", () => {
  it("stacks the heading over the last-scan line", () => {
    // The mockup's two-line left block. They were inline siblings in a flex row
    // before, which is a different shape and reads as a title with a footnote.
    const html = markup();
    const heading = html.indexOf("Scenario facts");
    const line = html.indexOf("Last scan Sep 11");
    expect(heading).toBeGreaterThan(-1);
    expect(line).toBeGreaterThan(heading);
    expect(html).toContain("flex-direction:column");
  });

  it("carries the split pill, History, the ⋯ overflow and a bare chevron", () => {
    const html = markup();
    expect(html).toContain("Scan");
    expect(html).toContain("History");
    expect(html).toContain("⋯");
    expect(html).toContain("▾");
  });

  it("names the ⋯ from a stored row, because a glyph has no name", () => {
    expect(markup()).toContain(`aria-label="${grammar.more_actions_label}"`);
  });

  it("makes the WHOLE row the collapse target, not just the chevron", () => {
    // Task 394 P6 gave the fold a word because a bare arrow "read as furniture:
    // nobody found it". The bare chevron is only defensible while this holds.
    const html = markup();
    expect(html).toContain('role="button"');
    expect(html).toContain('aria-expanded="true"');
    expect(html).toContain('aria-labelledby="scenario-facts-heading"');
    expect(html).toContain('id="scenario-facts-heading"');
  });
});

describe("the scan control asks rather than runs", () => {
  it("renders NO confirmation bar until one is asked for", () => {
    // The header's initial state. Run scan does not exist on the page yet, so
    // there is nothing on this row that could start a scan.
    const html = markup();
    expect(html).not.toContain("Run scan");
    expect(html).not.toContain("Run a theme scan with");
  });

  it("does not render the old one-click control", () => {
    // "Scan again" was the button wired straight to `onRun`. Its absence is the
    // change; its presence would mean the rebuild shipped beside the defect
    // rather than over it.
    expect(markup()).not.toContain("Scan again");
  });

  it("offers the model picker the header lost in v2.1.0", () => {
    const html = markup();
    expect(html).toContain("<select");
    expect(html).toContain("Qwen3.8 27B (NVFP4, local)");
  });
});

describe("what the header withholds, and why", () => {
  it("draws no scan control at all until its words arrive", () => {
    // The absent-not-fake law: a control with no stored label is not rendered
    // with a compiled-in fallback.
    const html = markup({ wording: null });
    expect(html).not.toContain("History");
    expect(html).not.toContain("<select");
    // The heading and the fold are still there — the section is still foldable
    // while the scan's words are in flight.
    expect(html).toContain("Scenario facts");
    expect(html).toContain('role="button"');
  });

  it("says nothing about a scan when it has been told nothing", () => {
    expect(markup({ headerLine: null })).not.toContain("Last scan");
  });

  it("explains a refusal rather than presenting a dead control", () => {
    const running = markup({ running: true, canRun: false });
    expect(running).toContain("A scan is running");
    const noModels = markup({ canRun: false, models: [], selectedModel: null });
    expect(noModels).toContain("No scan-eligible model is available.");
  });
});
