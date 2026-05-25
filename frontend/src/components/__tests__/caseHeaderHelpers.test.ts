/**
 * Pure-helper tests for CaseHeader.
 *
 * Locks the display contracts behind the case header: the "v." title split,
 * long-date formatting (timezone-stable), party-label pluralization, the
 * [pending] case-number rule, and the counsel-line formatting (with its
 * omit-when-empty branches). Per §11 test-auditor: every branch — happy path,
 * fallback path, and the boundary between them — has a test.
 *
 * No DOM / RTL — these are pure functions. Pattern matches
 * `configurationPanelHelpers.test.ts`.
 */
import { describe, expect, it } from "vitest";
import {
  formatCounselLine,
  formatFiledDate,
  isCaseNumberPending,
  pluralizePartyLabel,
  splitOnVersus,
} from "../CaseHeader";
import type { CounselContact } from "../../services/caseHeader";

describe("splitOnVersus", () => {
  it("splits a title on the first ' v. '", () => {
    expect(splitOnVersus("Awad v. Catholic Family Service")).toEqual({
      left: "Awad",
      right: "Catholic Family Service",
    });
  });

  it("returns null when there is no ' v. '", () => {
    expect(splitOnVersus("In re Estate of Smith")).toBeNull();
  });

  it("splits only on the first occurrence", () => {
    expect(splitOnVersus("A v. B v. C")).toEqual({ left: "A", right: "B v. C" });
  });
});

describe("formatFiledDate", () => {
  it("formats an ISO date as a long US date", () => {
    expect(formatFiledDate("2013-11-01")).toBe("November 1, 2013");
  });

  it("does not drift across timezones (no zero-pad, no off-by-one)", () => {
    // Parsed as UTC and formatted as UTC, so the day never rolls back.
    expect(formatFiledDate("2020-01-01")).toBe("January 1, 2020");
  });

  it("returns null for null/empty/malformed input", () => {
    expect(formatFiledDate(null)).toBeNull();
    expect(formatFiledDate(undefined)).toBeNull();
    expect(formatFiledDate("")).toBeNull();
    expect(formatFiledDate("not-a-date")).toBeNull();
  });
});

describe("pluralizePartyLabel", () => {
  it("uses the singular for exactly one", () => {
    expect(pluralizePartyLabel("PLAINTIFF", 1)).toBe("PLAINTIFF");
  });

  it("pluralizes for more than one", () => {
    expect(pluralizePartyLabel("DEFENDANT", 2)).toBe("DEFENDANTS");
  });

  it("pluralizes for zero (English treats zero as plural)", () => {
    expect(pluralizePartyLabel("PLAINTIFF", 0)).toBe("PLAINTIFFS");
  });
});

describe("isCaseNumberPending", () => {
  it("is pending for null, undefined, empty, or whitespace", () => {
    expect(isCaseNumberPending(null)).toBe(true);
    expect(isCaseNumberPending(undefined)).toBe(true);
    expect(isCaseNumberPending("")).toBe(true);
    expect(isCaseNumberPending("   ")).toBe(true);
  });

  it("is not pending for a real docket number", () => {
    expect(isCaseNumberPending("13-12345-CZ")).toBe(false);
  });
});

describe("formatCounselLine", () => {
  const base: CounselContact = {
    counsel_id: "counsel-penzien",
    represents_role: "Plaintiff",
    firm_name: "Penzien & McBride, PLLC",
    attorney_name: "Charles M. Penzien",
    bar_number: "P56491",
    address: null,
    phone: null,
    email: null,
  };

  it("renders the full line with bar number and firm", () => {
    expect(formatCounselLine(base)).toBe(
      "Plaintiff's Counsel: Charles M. Penzien (P56491) — Penzien & McBride, PLLC",
    );
  });

  it("omits the parenthetical when bar_number is null/empty", () => {
    expect(formatCounselLine({ ...base, bar_number: null })).toBe(
      "Plaintiff's Counsel: Charles M. Penzien — Penzien & McBride, PLLC",
    );
    expect(formatCounselLine({ ...base, bar_number: "  " })).toBe(
      "Plaintiff's Counsel: Charles M. Penzien — Penzien & McBride, PLLC",
    );
  });

  it("omits the firm suffix when firm_name is null/empty", () => {
    expect(formatCounselLine({ ...base, firm_name: null })).toBe(
      "Plaintiff's Counsel: Charles M. Penzien (P56491)",
    );
  });

  it("omits both when bar number and firm are absent", () => {
    expect(
      formatCounselLine({ ...base, bar_number: null, firm_name: null }),
    ).toBe("Plaintiff's Counsel: Charles M. Penzien");
  });
});
