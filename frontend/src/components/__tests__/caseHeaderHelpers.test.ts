/**
 * Pure-helper tests for CaseHeader.
 *
 * Locks the display contracts behind the case header: title resolution
 * (full → short → placeholder), long-date formatting (timezone-stable),
 * party-label pluralization, the [pending] case-number rule, the dropped-
 * defendant "(Dropped)" label, and the counsel-line formatting (with its
 * omit-when-empty branches). Per §11 test-auditor: every branch — happy path,
 * fallback path, and the boundary between them — has a test.
 *
 * No DOM / RTL — these are pure functions.
 */
import { describe, expect, it } from "vitest";
import {
  formatCounselLine,
  formatDroppedDefendant,
  formatFiledDate,
  isCaseNumberPending,
  pluralizePartyLabel,
  resolveTitle,
} from "../CaseHeader";
import type {
  CaseHeaderResponse,
  CounselContact,
} from "../../services/caseHeader";

describe("resolveTitle", () => {
  const base: CaseHeaderResponse = {
    case_id: "case-1",
    case_slug: "awad_v_catholic_family_service",
    display_title: "Awad v. CFS",
    display_title_full: "Marie Awad v. Catholic Family Service & George Phillips",
    court: {
      name: null,
      jurisdiction: null,
      case_number: null,
      filed_date: null,
      transferred_from: null,
      transfer_date: null,
    },
    status: "active",
    complaint_document_id: null,
    parties: { plaintiffs: [], active_defendants: [], dropped_defendants: [] },
    counsel: [],
  };

  it("prefers display_title_full when present", () => {
    expect(resolveTitle(base)).toBe(
      "Marie Awad v. Catholic Family Service & George Phillips",
    );
  });

  it("falls back to display_title when full is null/blank", () => {
    expect(resolveTitle({ ...base, display_title_full: null })).toBe("Awad v. CFS");
    expect(resolveTitle({ ...base, display_title_full: "   " })).toBe("Awad v. CFS");
  });

  it("falls back to the placeholder when both are blank", () => {
    expect(
      // @ts-ignore — exercising the defensive empty-string path
      resolveTitle({ ...base, display_title_full: null, display_title: "" }),
    ).toBe("Case title unavailable");
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

describe("formatDroppedDefendant", () => {
  it("appends the (Dropped) marker", () => {
    expect(formatDroppedDefendant("Archdiocese of Detroit")).toBe(
      "Archdiocese of Detroit (Dropped)",
    );
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
