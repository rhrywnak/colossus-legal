// =============================================================================
// allegationLabel.test.ts — one composer for the chip's words (ruling R11)
// =============================================================================

import { describe, expect, it } from "vitest";

import { allegationLabel, labelForAllegationId } from "../allegationLabel";
import type { AllegationDto } from "../../services/allegations";

function allegation(overrides: Partial<AllegationDto> = {}): AllegationDto {
  return {
    id: "alleg-7",
    title: "Refused amicable division",
    paragraph: "41",
    allegation: "refused every attempt to divide the estate property amicably",
    ...overrides,
  } as AllegationDto;
}

describe("the chip label", () => {
  it("joins the paragraph and the complaint's own sentence", () => {
    expect(allegationLabel(allegation())).toBe(
      "A-41 — refused every attempt to divide the estate property amicably",
    );
  });

  it("prefers the complaint's sentence over the shorter title", () => {
    // The chip is read to decide whether a scenario really bears on this paragraph,
    // and a title rarely says enough to answer that.
    const label = allegationLabel(allegation());
    expect(label).toContain("refused every attempt");
    expect(label).not.toContain("Refused amicable division");
  });

  it("falls back to the title when the complaint sentence is absent", () => {
    expect(allegationLabel(allegation({ allegation: undefined }))).toBe(
      "A-41 — Refused amicable division",
    );
  });

  it("renders the paragraph alone when there is no summary at all", () => {
    expect(allegationLabel(allegation({ allegation: undefined, title: "" }))).toBe("A-41");
  });

  it("falls back to the ID when there is no paragraph either", () => {
    // A poor label, but an HONEST one: it names a thing that exists and can be
    // looked up. A blank chip would be the page showing a link to nothing.
    expect(
      allegationLabel(allegation({ paragraph: undefined, allegation: undefined, title: "" })),
    ).toBe("alleg-7");
  });
});

describe("labelling an id against the catalogue", () => {
  it("uses the catalogue entry when it knows the id", () => {
    expect(labelForAllegationId("alleg-7", [allegation()])).toContain("A-41");
  });

  it("renders an UNKNOWN id as the id rather than dropping the chip", () => {
    // Dropping it would make the "Bears on" row quietly shorter than the scenario's
    // actual anchors — the chip count would disagree with the data and nothing on
    // screen would say why (Standing Rule 1).
    expect(labelForAllegationId("alleg-99", [allegation()])).toBe("alleg-99");
  });

  it("renders every id as itself when the catalogue failed to load", () => {
    expect(labelForAllegationId("alleg-7", [])).toBe("alleg-7");
  });
});
