/**
 * Pure-helper tests for the not-found page.
 *
 * The page's one piece of shaping is the path it echoes back, and that string
 * comes from the URL bar — i.e. from outside the app. These lock the two
 * properties that keep it safe to render: an empty path still reads as a path,
 * and an arbitrarily long one cannot exceed the display budget. No DOM / RTL —
 * pure functions only (CLAUDE.md §30), mirroring proofReviewHelpers.test.ts.
 */
import { describe, expect, it } from "vitest";

import { formatUnknownPath, MAX_DISPLAYED_PATH_LENGTH } from "../notFoundHelpers";

describe("formatUnknownPath", () => {
  it("returns a realistic unmatched path unchanged", () => {
    // The common case: a typo or a retired route. Nothing is shaped away, because
    // the whole point of showing it is that the reader recognises it.
    expect(formatUnknownPath("/analysis")).toBe("/analysis");
    expect(formatUnknownPath("/cases/awad_v_cfs/decomposition")).toBe(
      "/cases/awad_v_cfs/decomposition",
    );
  });

  it("renders an empty pathname as the root, not as nothing", () => {
    // React Router always supplies at least "/", so this is defensive — but an
    // empty gap where the page promises a path reads as a broken render.
    expect(formatUnknownPath("")).toBe("/");
    expect(formatUnknownPath("   ")).toBe("/");
  });

  it("truncates an over-long path to the budget, ellipsis included", () => {
    const long = `/${"x".repeat(500)}`;
    const shown = formatUnknownPath(long);

    // The cap is inclusive of the marker: appending the ellipsis instead of
    // substituting for the last character would overshoot the very limit this
    // function exists to enforce.
    expect(shown.length).toBe(MAX_DISPLAYED_PATH_LENGTH);
    expect(shown.endsWith("…")).toBe(true);
    expect(shown.startsWith("/xxx")).toBe(true);
  });

  it("leaves a path of exactly the budget length intact", () => {
    // The boundary: at the limit nothing is lost, so a path that genuinely ends
    // there is never mistaken for a truncated one.
    const exact = `/${"y".repeat(MAX_DISPLAYED_PATH_LENGTH - 1)}`;
    expect(exact.length).toBe(MAX_DISPLAYED_PATH_LENGTH);
    expect(formatUnknownPath(exact)).toBe(exact);
    expect(formatUnknownPath(exact).endsWith("…")).toBe(false);
  });
});
