// =============================================================================
// practiceAnswerPhase.test.ts — the working state's three claims
// =============================================================================
//
// ## ⚑ WHAT THIS FILE CANNOT DO, STATED FIRST
//
// It cannot hold a request unresolved and assert on a rendered button, because
// this project has NO DOM TEST ENVIRONMENT — no jsdom, no happy-dom, no
// `@testing-library`, and vitest runs in the node environment. React components
// cannot be rendered in a test here at all.
//
// That is the same hole that made `post_practice_answer` reachable only by
// inspection. The claims below were therefore EXTRACTED from the component into
// a pure module so that something can check them; the ORDERING claim — that the
// state is set before the request is awaited — is asserted by a source scan in
// `practiceWorkingState.test.ts`, which is the weaker instrument and says so.
//
// ⚑ THE DAY A DOM TIER EXISTS, replace both with a render that holds the promise
// unresolved. These are substitutes, not the intended tests.

import { describe, expect, it } from "vitest";

import { answerChrome, waitingLineKey, LONG_WAIT_MS } from "../practiceAnswerPhase";

describe("while the read is running", () => {
  // MUTATION: leave the button enabled → `buttonDisabled` false → red.
  it("disables the button so it cannot be pressed twice", () => {
    expect(answerChrome("working").buttonDisabled).toBe(true);
    expect(answerChrome("idle").buttonDisabled).toBe(false);
  });

  // MUTATION: keep the idle label → the two keys match → red.
  it("relabels the button, so the page says what it is doing", () => {
    expect(answerChrome("working").buttonLabelKey).toBe("read_working_label");
    expect(answerChrome("working").buttonLabelKey).not.toBe(
      answerChrome("idle").buttonLabelKey,
    );
  });

  // MUTATION: leave the box editable → `boxLocked` false → red.
  it("locks the answer box", () => {
    expect(answerChrome("working").boxLocked).toBe(true);
    expect(answerChrome("idle").boxLocked).toBe(false);
  });

  // ⚑ MUTATION: render the block only once the promise resolves → this is the
  // defect Roman actually reported, and it is the claim most easily tested into
  // nothing. `critiquePresent` false while working → red.
  it("puts the critique block on screen EMPTY, before anything returns", () => {
    expect(answerChrome("working").critiquePresent).toBe(true);
    expect(answerChrome("idle").critiquePresent).toBe(false);
  });

  it("offers Stop waiting only while there is something to stop", () => {
    expect(answerChrome("working").stopOffered).toBe(true);
    expect(answerChrome("idle").stopOffered).toBe(false);
  });
});

describe("the ten-second line", () => {
  // MUTATION: move the threshold → these fail, because they assert the RIGHT
  // line at the RIGHT time rather than that A line exists.
  it("says 'usually a few seconds' before the threshold", () => {
    expect(waitingLineKey(0)).toBe("read_usually_quick");
    expect(waitingLineKey(LONG_WAIT_MS - 1)).toBe("read_usually_quick");
  });

  it("says her answer is saved either way from the threshold onward", () => {
    // The SAVED half is the fact she needs while waiting, and it is true from
    // the moment the row was written — her answer is the first write.
    expect(waitingLineKey(LONG_WAIT_MS)).toBe("read_still_working");
    expect(waitingLineKey(LONG_WAIT_MS + 1)).toBe("read_still_working");
  });

  it("changes at exactly the threshold, not around it", () => {
    // A `>` instead of `>=` would leave the two lines disagreeing about the
    // instant they swap — invisible on screen, and a test would have to guess.
    expect(waitingLineKey(LONG_WAIT_MS - 1)).not.toBe(waitingLineKey(LONG_WAIT_MS));
  });
});
