/**
 * Which tab the URL asks for.
 *
 * The tab is an ADDRESS, not component state — that is what lets Proof Review
 * be bookmarked, reached by Back, and redirected to from its old route. So the
 * function that reads it has to be right about the two cases a URL can be
 * wrong in: absent, and nonsense.
 */
import { describe, expect, it } from "vitest";

import { activeTab, TAB_PARAM, type PageTab } from "../PageTabs";

const TABS: PageTab[] = [
  { id: "matrix", label: "Matrix" },
  { id: "review", label: "Proof Review" },
];

const search = (value?: string) =>
  new URLSearchParams(value === undefined ? "" : `${TAB_PARAM}=${value}`);

describe("activeTab", () => {
  it("returns the tab the URL names", () => {
    expect(activeTab(TABS, search("review"))).toBe("review");
  });

  it("falls back to the first tab when the parameter is absent", () => {
    // The canonical address of the matrix carries no query at all — the first
    // tab's id is deliberately left OUT of the URL, so "absent" is the normal
    // case and not an error.
    expect(activeTab(TABS, search())).toBe("matrix");
  });

  it("falls back to the first tab on a value nothing matches", () => {
    // A URL is user input and a bookmark can go stale. "The tab you asked for
    // does not exist" is not worth a screen when the sensible thing is to show
    // the page's default half.
    expect(activeTab(TABS, search("nonsense"))).toBe("matrix");
    expect(activeTab(TABS, search(""))).toBe("matrix");
  });

  it("is not fooled by a DIFFERENT query parameter", () => {
    // `?section=review` is not `?tab=review`. Reading any parameter that
    // happened to hold a tab id would make an unrelated link change the page.
    expect(activeTab(TABS, new URLSearchParams("section=review"))).toBe("matrix");
  });
});
