import { describe, it, expect } from "vitest";
import { phaseLabel, PhaseOption } from "../casePhases";

// The four as the data file defines them after the 2026-08-17 rename.
const OPTIONS: PhaseOption[] = [
  { slug: "estate", label: "PRE-PROBATE" },
  { slug: "probate", label: "PROBATE" },
  { slug: "appeals", label: "COA" },
  { slug: "civil_lawsuit", label: "COMPLAINT" },
];

describe("phaseLabel", () => {
  it("maps every stored slug to its display label", () => {
    expect(phaseLabel(OPTIONS, "estate")).toBe("PRE-PROBATE");
    expect(phaseLabel(OPTIONS, "probate")).toBe("PROBATE");
    expect(phaseLabel(OPTIONS, "appeals")).toBe("COA");
    expect(phaseLabel(OPTIONS, "civil_lawsuit")).toBe("COMPLAINT");
  });

  it("returns empty for a document with no phase", () => {
    // The caller decides how to render the absence — an em dash in the table,
    // "Not set" in the control — so this must not invent either.
    expect(phaseLabel(OPTIONS, null)).toBe("");
    expect(phaseLabel(OPTIONS, undefined)).toBe("");
    expect(phaseLabel(OPTIONS, "")).toBe("");
    expect(phaseLabel(OPTIONS, "   ")).toBe("");
  });

  it("shows an UNKNOWN slug verbatim rather than hiding it", () => {
    // A document carrying a phase this build does not know is a real
    // disagreement between the column and the data file. Rendering it as "no
    // phase" would conceal exactly the thing an operator needs to see.
    expect(phaseLabel(OPTIONS, "mediation")).toBe("mediation");
    expect(phaseLabel(OPTIONS, "COA")).toBe("COA");
  });

  it("does not match a label as if it were a slug", () => {
    // Guards the direction of the map: labels are output, never input.
    expect(phaseLabel(OPTIONS, "PRE-PROBATE")).toBe("PRE-PROBATE");
    expect(phaseLabel([{ slug: "estate", label: "PRE-PROBATE" }], "PRE-PROBATE")).toBe(
      "PRE-PROBATE",
    );
  });

  it("survives an empty option list without throwing", () => {
    // The list is empty only when the data file failed to load; the caller shows
    // the error, and any row still rendering must not crash the page.
    expect(phaseLabel([], "estate")).toBe("estate");
    expect(phaseLabel([], null)).toBe("");
  });
});
