// practiceCritiqueBlock.test.tsx — what each abstain-shaped critique DRAWS.
//
// Static markup (rule 30: no DOM tier here). ADDENDUM_3's claim is visual — a
// decline and a failure get the neutral rail and no "older analysis" hint, while
// a true older analysis keeps both its rail and its hint — so it is checked on
// the rendered block, not only on `critiqueFor`'s return value.

import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import Critique from "../PracticeCritiqueBlock";
import * as c from "../practiceCritiqueStyles";
import type { PracticeWording } from "../../../services/practice";

/** The migration's value for the one row these arms can read (ADDENDUM_2). */
const HINT = "This is an older analysis. Press Answer again for a fuller one.";
const wording: PracticeWording = { read_plain_hint: HINT };

const draw = (view: Parameters<typeof Critique>[0]["view"]) =>
  renderToStaticMarkup(<Critique view={view} wording={wording} />);

/** The rail colour a style object sets, as React writes it into markup. */
const rail = (style: { borderLeftColor?: unknown }) => `border-left-color:${String(style.borderLeftColor)}`;

describe("the critique block's abstain-shaped arms", () => {
  it("draws a model decline on the neutral rail, with its two sentences and no hint", () => {
    const html = draw({
      kind: "declined",
      text: "The analysis couldn't judge this answer. That looks like a test entry.",
    });
    expect(html).toContain('data-critique="declined"');
    expect(html).toContain("That looks like a test entry.");
    expect(html).toContain(rail(c.neutral));
    expect(html).not.toContain(rail(c.fine));
    expect(html).not.toContain(HINT);
  });

  it("draws a failure the same neutral, hint-free way", () => {
    const html = draw({ kind: "failed", text: "Your answer is saved, but the answer analysis didn't come back." });
    expect(html).toContain('data-critique="failed"');
    expect(html).toContain(rail(c.neutral));
    expect(html).not.toContain(HINT);
  });

  it("still gives a true older analysis its rail and its hint", () => {
    const html = draw({ kind: "sentence", text: "Fine. Short, and yours.", ok: true });
    expect(html).toContain('data-critique="sentence"');
    expect(html).toContain(rail(c.fine));
    expect(html).toContain(HINT);
  });
});
