import { describe, it, expect } from "vitest";
import { attentionBannerText } from "../documentsPageHelpers";

describe("attentionBannerText", () => {
  it("agrees the verb with the number", () => {
    // The bug: the old inline expression pluralised the noun and left the verb
    // alone, so one document read "1 document need attention".
    expect(attentionBannerText(1)).toBe("1 document needs attention — click to filter");
    expect(attentionBannerText(2)).toBe("2 documents need attention — click to filter");
    expect(attentionBannerText(11)).toBe("11 documents need attention — click to filter");
  });

  it("renders nothing when nothing needs attention", () => {
    // Which is the state DEV is in once the banner stops counting a
    // retried-then-completed step.
    expect(attentionBannerText(0)).toBeNull();
  });

  it("renders nothing for a negative or non-finite count", () => {
    // A count that arrived broken must not become a banner.
    expect(attentionBannerText(-1)).toBeNull();
    expect(attentionBannerText(NaN)).toBeNull();
  });
});
