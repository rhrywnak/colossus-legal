// =============================================================================
// documentDateField.test.ts — the mandatory-with-override rule, on the UI side
// =============================================================================
//
// The backend refuses a bad pair; these cover the half that decides whether the
// user can even press the button. Both helpers are pure, which is why they are
// helpers: this repo has no component-test infrastructure, so behaviour that
// matters has to be reachable without rendering.

import { describe, expect, it } from "vitest";

import {
  isDocumentDateComplete,
  valueForPrecision,
} from "../pipeline/DocumentDateField";
import { summarise } from "../pipeline/DocumentDateEditor";

// The vocabulary as the backend serves it (domain::date_precision).
const PRECISIONS = [
  { value: "day", label: "Exact date", requires_date: true },
  { value: "month", label: "Month and year only", requires_date: true },
  { value: "year", label: "Year only", requires_date: true },
  { value: "unknown", label: "No date on the document", requires_date: false },
];

describe("isDocumentDateComplete", () => {
  it("refuses an unanswered question", () => {
    // The select starts empty on purpose. Until it is answered, upload is
    // blocked — that is what "mandatory" means here.
    expect(isDocumentDateComplete({ date: null, precision: "" }, PRECISIONS)).toBe(
      false,
    );
  });

  it("refuses a real precision with no date", () => {
    expect(
      isDocumentDateComplete({ date: null, precision: "day" }, PRECISIONS),
    ).toBe(false);
    expect(isDocumentDateComplete({ date: "", precision: "day" }, PRECISIONS)).toBe(
      false,
    );
  });

  it("accepts a real precision with a date", () => {
    for (const precision of ["day", "month", "year"]) {
      expect(
        isDocumentDateComplete({ date: "2009-11-05", precision }, PRECISIONS),
      ).toBe(true);
    }
  });

  it("accepts the override with no date — it is an answer", () => {
    expect(
      isDocumentDateComplete({ date: null, precision: "unknown" }, PRECISIONS),
    ).toBe(true);
  });

  it("refuses a precision the backend did not offer", () => {
    // A stale build sending "moth" must be blocked here rather than 400ing
    // after the file has already uploaded.
    expect(
      isDocumentDateComplete({ date: "2009-11-05", precision: "moth" }, PRECISIONS),
    ).toBe(false);
  });

  it("refuses everything while the precision list is still loading", () => {
    expect(isDocumentDateComplete({ date: "2009-11-05", precision: "day" }, [])).toBe(
      false,
    );
  });
});

describe("valueForPrecision", () => {
  it("keeps the typed date when switching between real precisions", () => {
    const next = valueForPrecision(
      { date: "2009-11-05", precision: "day" },
      "month",
      PRECISIONS,
    );
    expect(next).toEqual({ date: "2009-11-05", precision: "month" });
  });

  it("clears the date when the override is chosen", () => {
    // Otherwise a stale date is submitted alongside "this document has no
    // date" — a contradiction the backend refuses and the user cannot see.
    const next = valueForPrecision(
      { date: "2009-11-05", precision: "day" },
      "unknown",
      PRECISIONS,
    );
    expect(next).toEqual({ date: null, precision: "unknown" });
  });

  it("clears the date for a precision it does not recognise", () => {
    const next = valueForPrecision(
      { date: "2009-11-05", precision: "day" },
      "moth",
      PRECISIONS,
    );
    expect(next.date).toBeNull();
  });
});

describe("summarise", () => {
  it("distinguishes never-asked from answered-unknown", () => {
    // The load-bearing distinction: all nine ingested documents start in the
    // first state, and showing them as the second would make the intake
    // question look already answered.
    expect(summarise(undefined, undefined, undefined)).toBe("Date not set");
    expect(summarise(undefined, "No date on the document", "unknown")).toBe(
      "No date on the document",
    );
  });

  it("shows a day-precision date as itself", () => {
    expect(summarise("2009-11-05", "Exact date", "day")).toBe("2009-11-05");
  });

  it("hides the padding day on a coarser precision", () => {
    // The stored day is padding the source never stated; showing it would make
    // a fabricated day indistinguishable from a real one.
    expect(summarise("2009-11-01", "Month and year only", "month")).toBe(
      "2009-11 (month only)",
    );
    expect(summarise("2009-01-01", "Year only", "year")).toBe("2009 (year only)");
  });

  it("renders a precision this build does not know without pretending", () => {
    expect(summarise("2009-11-05", "Unrecognised precision 'moth'", "moth")).toBe(
      "2009-11-05 (Unrecognised precision 'moth')",
    );
  });
});
