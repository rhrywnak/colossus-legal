// =============================================================================
// warRoomSummaryCard.test.tsx — the summary card's markup
// =============================================================================

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import WarRoomSummaryCard from "../WarRoomSummaryCard";
import { s1, s11, warRoomWording } from "./warRoomFixtures";

const html = renderToStaticMarkup(
  <WarRoomSummaryCard
    dashboard={{ metrics: { scenarios: 11, ready: 11, drafted_or_review: 0 }, scenarios: [s1(), s11()] }}
    wording={warRoomWording}
  />,
);

describe("WarRoomSummaryCard markup", () => {
  it("renders the top row and three owned cells, in order", () => {
    expect(html).toContain("data-summary-top");
    expect(html).toContain("data-summary-answered");
    const owners = [...html.matchAll(/data-owner="([a-z]+)"/g)].map((m) => m[1]);
    expect(owners).toEqual(["marie", "reviewer", "roman"]);
  });

  it("puts an owner chip on every cell — stored names, never usernames", () => {
    const chips = [...html.matchAll(/data-chip="true">([^<]+)</g)].map((m) => m[1]);
    expect(chips).toEqual(["Marie", "Chuck", "Roman"]);
    expect(html).not.toContain("cpenzien");
  });

  it("draws each cell's coloured stripe", () => {
    expect((html.match(/box-shadow:inset 4px 0 0 var\(--/g) ?? []).length).toBe(3);
  });

  it("ships no MOCKUP annotation and no stat strip", () => {
    expect(html).not.toMatch(/MOCKUP|New answers for you|Waiting for Marie/);
  });
});
