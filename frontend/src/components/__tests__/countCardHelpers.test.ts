/**
 * Pure-helper tests for CountCard.
 *
 * Locks the display contracts behind the Causes of Action tables: Roman-
 * numeral headers, the "{count}.{order}" Element ordinal, and the §7 sort
 * (order_in_count asc, null last, element_name secondary). Per §11 test-
 * auditor: happy path, fallback path, and the boundary between them are all
 * covered. No DOM / RTL — pure functions only.
 *
 * Note (E2): the §8 click-through URL helper (`buildEvidenceUrl`) was removed
 * — clicking an Element row now opens the Element detail panel via an
 * `onElementClick` callback rather than navigating to /evidence.
 */
import { describe, expect, it } from "vitest";
import {
  formatElementNumber,
  sortElements,
  toRomanNumeral,
} from "../CountCard";
import type { ElementDetail } from "../../services/causesOfAction";

const makeElement = (overrides: Partial<ElementDetail> = {}): ElementDetail => ({
  element_id: "el-1",
  order_in_count: 1,
  element_name: "Existence of fiduciary duty",
  what_plaintiff_must_prove: "That a fiduciary relationship existed.",
  controlling_authority: null,
  theory_variant: null,
  allegation_count: 0,
  ...overrides,
});

describe("toRomanNumeral", () => {
  it("converts the four case Counts", () => {
    expect(toRomanNumeral(1)).toBe("I");
    expect(toRomanNumeral(2)).toBe("II");
    expect(toRomanNumeral(3)).toBe("III");
    expect(toRomanNumeral(4)).toBe("IV");
  });

  it("handles subtractive and larger values", () => {
    expect(toRomanNumeral(9)).toBe("IX");
    expect(toRomanNumeral(14)).toBe("XIV");
    expect(toRomanNumeral(40)).toBe("XL");
    expect(toRomanNumeral(2020)).toBe("MMXX");
  });

  it("returns non-positive input as a plain string (defensive)", () => {
    expect(toRomanNumeral(0)).toBe("0");
    expect(toRomanNumeral(-3)).toBe("-3");
  });
});

describe("formatElementNumber", () => {
  it("joins count and order with a dot", () => {
    expect(formatElementNumber(1, 1)).toBe("1.1");
    expect(formatElementNumber(2, 11)).toBe("2.11");
  });
});

describe("sortElements", () => {
  it("sorts by order_in_count ascending", () => {
    const input = [
      makeElement({ element_id: "c", order_in_count: 3 }),
      makeElement({ element_id: "a", order_in_count: 1 }),
      makeElement({ element_id: "b", order_in_count: 2 }),
    ];
    expect(sortElements(input).map((e) => e.element_id)).toEqual(["a", "b", "c"]);
  });

  it("places null order_in_count last", () => {
    const input = [
      makeElement({ element_id: "z", order_in_count: null }),
      makeElement({ element_id: "a", order_in_count: 1 }),
    ];
    expect(sortElements(input).map((e) => e.element_id)).toEqual(["a", "z"]);
  });

  it("breaks ties on element_name alphabetically", () => {
    const input = [
      makeElement({ element_id: "beta", order_in_count: 1, element_name: "Beta" }),
      makeElement({ element_id: "alpha", order_in_count: 1, element_name: "Alpha" }),
    ];
    expect(sortElements(input).map((e) => e.element_id)).toEqual(["alpha", "beta"]);
  });

  it("does not mutate the input array (purity)", () => {
    const input = [
      makeElement({ element_id: "b", order_in_count: 2 }),
      makeElement({ element_id: "a", order_in_count: 1 }),
    ];
    sortElements(input);
    expect(input.map((e) => e.element_id)).toEqual(["b", "a"]);
  });
});

