// =============================================================================
// scenarioDeleteCopy.test.ts — the last words before something irreversible
// =============================================================================
//
// Two surfaces ask this question now: the scenario page's header kebab and every
// card on the Trial Prep dashboard. What is tested here is not the strings —
// Roman's standing law rules out pinning text for its own sake — but the two
// properties a human's decision actually rests on: that the dialog says WHICH
// scenario, and that it says what survives.
//
// ## What is deliberately NOT tested here
//
// A third test asserted that both surfaces ask the same question, by calling
// this builder twice and comparing. The test auditor was right to reject it: it
// calls one pure function with identical arguments, so it cannot fail — and it
// would still pass on the very day someone gave the scenario page its own
// hardcoded copy, which is the only thing it claimed to guard.
//
// That consistency is structural, not testable from here: there is ONE builder,
// and both surfaces import it. If that stops being true the guarantee is gone,
// and no assertion in this file would have noticed.

import { describe, expect, it } from "vitest";

import { scenarioDeleteCopy } from "../scenarioDeleteCopy";

describe("the delete confirmation", () => {
  it("names the scenario it is about to delete", () => {
    // On a grid of cards that differ only by title, a dialog that does not name
    // its scenario is how the wrong one gets deleted. The name must reach the
    // message, not merely the props.
    const copy = scenarioDeleteCopy("Refused to divide property amicably");
    expect(copy.message).toContain("Refused to divide property amicably");
  });

  it("says what survives, not only what is destroyed", () => {
    // A scenario is a curation artifact; the evidence it points at lives in the
    // case graph and is untouched. Without that sentence a reasonable person
    // hesitates over a reversible-feeling act that reads as unbounded.
    const copy = scenarioDeleteCopy("An attack");
    expect(copy.message).toContain("cannot be undone");
    expect(copy.message).toContain("case graph is not affected");
  });
});
