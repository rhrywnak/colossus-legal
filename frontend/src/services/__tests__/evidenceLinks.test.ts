// =============================================================================
// evidenceLinks.test.ts — filling a stored sentence's one slot (task 2.10)
// =============================================================================
//
// `fillDetail` is the only pure function in this client, and it is the one place
// the BROWSER touches a stored sentence. Its backend twin, `domain::wording::
// render`, carries a full set of tests for the same reason: a substitution that
// silently does nothing produces a message with the fact missing, and nothing
// downstream can tell.

import { describe, expect, it } from "vitest";

import {
  fillCode,
  fillCodeAndReason,
  fillCount,
  fillCounts,
  fillDetail,
} from "../evidenceLinks";

describe("fillDetail", () => {
  it("drops the failure's own text into the slot the sentence leaves", () => {
    expect(
      fillDetail("That link did not save: {detail} The queue has been reloaded.", "HTTP 500"),
    ).toBe("That link did not save: HTTP 500 The queue has been reloaded.");
  });

  it("returns a template with no slot unchanged", () => {
    // A human editing this row could drop the placeholder — the backend's write
    // path refuses that (`REQUIRED_PLACEHOLDERS`), so reaching here means the
    // store was edited around the API. Returning the sentence as stored is the
    // honest outcome: it is what somebody asked for, and the missing detail is
    // visible by its absence rather than replaced by an empty gap.
    expect(fillDetail("That link did not save.", "HTTP 500")).toBe("That link did not save.");
  });

  it("fills only the FIRST slot, so a sentence cannot repeat the failure", () => {
    // `String.replace` with a string needle replaces once. That is the behaviour
    // wanted here: a template that named {detail} twice would otherwise print a
    // stack trace twice in one banner.
    expect(fillDetail("{detail} — {detail}", "boom")).toBe("boom — {detail}");
  });

  it("does not re-scan the value it just substituted", () => {
    // The same property `render` is tested for on the backend. A failure message
    // is arbitrary text — it can legitimately contain braces — and it must never
    // be treated as a template in its own right.
    expect(fillDetail("saving: {detail}", "the {detail} field was rejected")).toBe(
      "saving: the {detail} field was rejected",
    );
  });

  it("handles an empty detail without mangling the sentence", () => {
    // A thrown value that stringifies to nothing still leaves a readable sentence
    // rather than a `{detail}` token on screen.
    expect(fillDetail("did not save: {detail}", "")).toBe("did not save: ");
  });
});

describe("fillCode", () => {
  const template = "Remove {code} from this scenario? It goes back to the queue as not ruled.";

  it("names the card the confirmation is about", () => {
    // A confirmation that does not name what it is about is not a confirmation —
    // which is why {code} is a REQUIRED placeholder on that stored key.
    expect(fillCode(template, "C-111")).toBe(
      "Remove C-111 from this scenario? It goes back to the queue as not ruled.",
    );
  });

  it("keeps the second sentence, which is the one that matters", () => {
    // Removing is not excluding. The human has to be told the item comes back
    // rather than being judged bad evidence, and that is in the stored words.
    expect(fillCode(template, "C-111")).toContain("goes back to the queue as not ruled");
  });

  it("returns a template with no slot unchanged", () => {
    expect(fillCode("Remove this fact?", "C-111")).toBe("Remove this fact?");
  });

  it("does not re-scan the code it just substituted", () => {
    expect(fillCode("{code} — {code}", "C-{code}")).toBe("C-{code} — {code}");
  });
});

// ── Task 2.13: the two-slot templates ───────────────────────────────────────

describe("fillCodeAndReason", () => {
  it("fills both slots of a failure message", () => {
    expect(
      fillCodeAndReason("Could not save the weight for {code}. {reason}", "C-108", "HTTP 409"),
    ).toBe("Could not save the weight for C-108. HTTP 409");
  });

  it("does not re-scan a value it has already substituted", () => {
    // THE bug two chained `.replace()` calls would have. The backend's refusal
    // text is prose we do not control; if a reason happened to contain "{code}",
    // a second pass would chew it out and the message would lie about which fact
    // failed. One walk over the template means a substituted value is never
    // looked at again — the same argument `domain::wording::render` makes.
    expect(
      fillCodeAndReason("for {code}: {reason}", "C-1", "the server said {code} is gone"),
    ).toBe("for C-1: the server said {code} is gone");
  });

  it("leaves a template that lost a slot unchanged rather than guessing", () => {
    // The backend refuses an edit that drops either placeholder, so reaching here
    // means the store was edited around the API. Showing the sentence as stored
    // is how a reader finds out; inventing the missing half would not be.
    expect(fillCodeAndReason("no slots here", "C-1", "HTTP 500")).toBe("no slots here");
  });
});

describe("fillCount", () => {
  it("drops the pile's size into its stored line", () => {
    expect(fillCount("{count} in the background · show", 7)).toBe(
      "7 in the background · show",
    );
  });

  it("renders a zero honestly rather than as an empty slot", () => {
    // A pile of zero should not render at all — but if it ever does, it must say
    // zero rather than leave a hole where the number goes.
    expect(fillCount("{count} in the background · show", 0)).toBe(
      "0 in the background · show",
    );
  });
});

describe("fillCounts", () => {
  it("fills both counts, each into its own slot", () => {
    expect(fillCounts("{shown} shown · {background} in background", 36, 10)).toBe(
      "36 shown · 10 in background",
    );
  });

  it("does not swap the two numbers", () => {
    // Two integers in one template is exactly the shape where an argument-order
    // slip is invisible on review and wrong on screen. The asymmetric values are
    // the point: 36/10 reads differently from 10/36.
    const filled = fillCounts("{shown}|{background}", 36, 10);
    expect(filled).toBe("36|10");
    expect(filled).not.toBe("10|36");
  });

  it("fills slots in either order in the template", () => {
    // A human may reword this row to lead with the folded count. The template
    // decides the sentence; the function must not assume a fixed order.
    expect(fillCounts("{background} hidden, {shown} visible", 36, 10)).toBe(
      "10 hidden, 36 visible",
    );
  });

  it("leaves a token it does not own alone", () => {
    // NOT the re-scan bug — with `number` arguments a substituted value cannot
    // contain a token, so that scenario is unreachable here; `fillCodeAndReason`
    // above is where it is real and where it is tested. What this pins is
    // narrower and still worth pinning: the regex replaces only the two names it
    // owns and leaves every other brace token in the stored text untouched.
    expect(fillCounts("{shown} and {background}", 1, 2)).toBe("1 and 2");
    // A template whose literal text happens to contain a brace token is left
    // exactly as written once its own slots are filled.
    expect(fillCounts("{shown} of {background} — see {other}", 3, 4)).toBe(
      "3 of 4 — see {other}",
    );
  });

  it("returns a template with no slots unchanged", () => {
    // The write path refuses an edit that drops either placeholder, so reaching
    // here means the store was edited around the API. Showing the sentence as
    // stored is how a reader finds out.
    expect(fillCounts("no counts here", 36, 10)).toBe("no counts here");
  });
});
