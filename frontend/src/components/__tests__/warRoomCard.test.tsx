// =============================================================================
// warRoomCard.test.tsx — the status card's rendered markup
// =============================================================================
//
// No RTL and no jsdom here (CLAUDE.md rule 30), so `renderToStaticMarkup` gives
// the markup and the assertions read it. What they pin: three panes; the
// accessible-card click zones (a real link per zone, stretched by CSS, with
// Delete and Timeline raised above it); and NO hint text (ruled mockup v3).

import { renderToStaticMarkup } from "react-dom/server";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";

import WarRoomCard from "../WarRoomCard";
import { WAR_ROOM_CARD_CSS } from "../trialPrepCardStyles";
import type { ScenarioSummary } from "../../pages/trialPrepData";
import { s1, s11, warRoomWording } from "./warRoomFixtures";

function markup(scenario: ScenarioSummary): string {
  return renderToStaticMarkup(
    <MemoryRouter>
      <WarRoomCard
        scenario={scenario}
        slug="awad_v_catholic_family_service"
        wording={warRoomWording}
        hasTimeline={false}
        mayReview
        onRequestDelete={() => {}}
      />
    </MemoryRouter>,
  );
}

/** The markup of one pane, from its opening `<section …data-pane="name"` to its close. */
function pane(html: string, name: string): string {
  const start = html.indexOf(`data-pane="${name}"`);
  const end = html.indexOf("</section>", start);
  return html.slice(start, end);
}

describe("WarRoomCard markup", () => {
  it("renders exactly three panes, in order", () => {
    const panes = [...markup(s1()).matchAll(/data-pane="([a-z]+)"/g)].map((m) => m[1]);
    expect(panes).toEqual(["identity", "evidence", "prep"]);
  });

  it("makes the TITLE the scenario link, in the left zone", () => {
    const left = pane(markup(s1()), "identity");
    expect(left).toContain("data-zone");
    expect(left).toMatch(/<h2[^>]*><a[^>]*data-zone-link="scenario"[^>]*>Marie is obstructive and uncooperative<\/a><\/h2>/);
  });

  it("makes Practice → the prep zone's link", () => {
    const prep = pane(markup(s1()), "prep");
    expect(prep).toContain("data-zone");
    expect(prep).toMatch(/<a[^>]*data-zone-link="practice"[^>]*>Practice →<\/a>/);
  });

  it("leaves the EVIDENCE pane inert — no zone, no link", () => {
    const evidence = pane(markup(s1()), "evidence");
    expect(evidence).not.toContain("data-zone");
    expect(evidence).not.toContain("<a ");
  });

  it("stretches each zone link over its pane, with Delete/Timeline raised above", () => {
    expect(WAR_ROOM_CARD_CSS).toMatch(/\[data-zone\]\s*\{[^}]*position:\s*relative/);
    expect(WAR_ROOM_CARD_CSS).toMatch(/\[data-zone-link\]::after\s*\{[^}]*position:\s*absolute;[^}]*inset:\s*0;[^}]*z-index:\s*1/);
    expect(WAR_ROOM_CARD_CSS).toMatch(/\[data-zone-above\]\s*\{[^}]*z-index:\s*2/);
    // Delete lives INSIDE the raised row, so the pseudo-element never covers it.
    const left = pane(markup(s1()), "identity");
    const above = left.indexOf("data-zone-above");
    expect(above).toBeGreaterThan(-1);
    expect(left.indexOf('aria-label="Delete S-1"')).toBeGreaterThan(above);
  });

  it("renders Delete as a real button, not inside any link", () => {
    const html = markup(s1());
    expect(html).toMatch(/<button[^>]*aria-label="Delete S-1"[^>]*>Delete<\/button>/);
    expect(html).not.toMatch(/<a[^>]*>[^<]*<button/);
  });

  it("ships NO hint text anywhere on the card (M)", () => {
    for (const html of [markup(s1()), markup(s11())]) {
      expect(html.toLowerCase()).not.toMatch(/click anywhere|opens the scenario|click here/);
    }
  });

  it("renders the review queue's amber pill, and Not started on an unanswered deck", () => {
    expect(markup(s1())).toContain('data-badge="review"');
    expect(markup(s1())).toContain("42 answers awaiting your review");
    const idle = markup(s11());
    expect(idle).toContain('data-badge="not_started"');
    expect(idle).not.toContain("Up to date");
  });

  it("drops the talking points and watch items rows, and the scan model", () => {
    const html = markup(s11());
    expect(html).not.toMatch(/Talking points|Watch items|relevant of/);
    expect(html).toContain("Last scan ");
  });
});
