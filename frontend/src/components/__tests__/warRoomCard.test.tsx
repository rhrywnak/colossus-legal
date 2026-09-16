// =============================================================================
// warRoomCard.test.tsx — the status card's rendered markup
// =============================================================================
//
// No RTL and no jsdom here (CLAUDE.md rule 30), so `renderToStaticMarkup` gives
// the markup and the assertions read it. What they pin: three panes, Delete as
// a visible control (no ⋯ menu — ruling Q5), no "Ready" pill, and the badge
// row's two states.

import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";

import WarRoomCard from "../WarRoomCard";
import type { ScenarioSummary } from "../../pages/trialPrepData";
import { s13, warRoomWording } from "./warRoomFixtures";

function markup(scenario: ScenarioSummary): string {
  return renderToStaticMarkup(
    <MemoryRouter>
      <WarRoomCard
        scenario={scenario}
        slug="awad_v_catholic_family_service"
        wording={warRoomWording}
        hasTimeline={false}
        onRequestDelete={() => {}}
      />
    </MemoryRouter>,
  );
}

describe("WarRoomCard markup", () => {
  it("renders exactly three panes, in order", () => {
    const html = markup(s13());
    const panes = [...html.matchAll(/data-pane="([a-z]+)"/g)].map((m) => m[1]);
    expect(panes).toEqual(["identity", "evidence", "prep"]);
  });

  it("puts Delete in the action row as a visible button — no ⋯ menu", () => {
    const html = markup(s13());
    expect(html).toMatch(/<button[^>]*aria-label="Delete S-13"[^>]*>Delete<\/button>/);
    expect(html).not.toContain("⋯");
    expect(html).not.toContain("aria-haspopup");
  });

  it("orders the actions Open scenario, Practice, Delete — Delete last", () => {
    const html = markup(s13());
    const open = html.indexOf(">Open scenario<");
    const practice = html.indexOf(">Practice<");
    const del = html.indexOf(">Delete<");
    expect(open).toBeGreaterThan(-1);
    expect(practice).toBeGreaterThan(open);
    expect(del).toBeGreaterThan(practice);
  });

  it("no longer renders the Ready pill", () => {
    const html = markup(s13());
    expect(html).not.toMatch(/>Ready</);
  });

  it("renders the amber changed badge when Marie has changed questions", () => {
    const html = markup(s13());
    expect(html).toContain('data-badge="changed"');
    expect(html).toContain("3 new or changed for Marie");
    expect(html).not.toContain("Up to date");
  });

  it("renders the green up-to-date badge when nothing is owed", () => {
    const html = markup(s13({ marie_changed: 0 }));
    expect(html).toContain('data-badge="up_to_date"');
    expect(html).toContain("Up to date");
    expect(html).not.toContain("new or changed");
  });

  it("renders both section headers from the store", () => {
    const html = markup(s13());
    expect(html).toContain(">Evidence<");
    expect(html).toContain(">Prep &amp; rehearsal<");
  });
});
