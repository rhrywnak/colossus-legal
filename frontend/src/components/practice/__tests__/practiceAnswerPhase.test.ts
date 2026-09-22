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

import {
  answerChrome,
  savedLabelFor,
  savedLineFor,
  waitingLineKey,
  LONG_WAIT_MS,
} from "../practiceAnswerPhase";

/**
 * The Answer-analysis switch, as these two worlds are named below.
 *
 * Every claim in the first block was written when there was only one world —
 * the read always ran — so they are the ON world, spelled out rather than
 * defaulted. A default here would let the OFF world go untested while every
 * line still read as if it covered both.
 */
const ON = true;
const OFF = false;

describe("while the read is running", () => {
  // MUTATION: leave the button enabled → `buttonDisabled` false → red.
  it("disables the button so it cannot be pressed twice", () => {
    expect(answerChrome("working", ON).buttonDisabled).toBe(true);
    expect(answerChrome("idle", ON).buttonDisabled).toBe(false);
  });

  // MUTATION: keep the idle label → the two keys match → red.
  it("relabels the button, so the page says what it is doing", () => {
    expect(answerChrome("working", ON).buttonLabelKey).toBe("read_working_label");
    expect(answerChrome("working", ON).buttonLabelKey).not.toBe(
      answerChrome("idle", ON).buttonLabelKey,
    );
  });

  // MUTATION: leave the box editable → `boxLocked` false → red.
  it("locks the answer box", () => {
    expect(answerChrome("working", ON).boxLocked).toBe(true);
    expect(answerChrome("idle", ON).boxLocked).toBe(false);
  });

  // ⚑ MUTATION: render the block only once the promise resolves → this is the
  // defect Roman actually reported, and it is the claim most easily tested into
  // nothing. `critiquePresent` false while working → red.
  it("puts the critique block on screen EMPTY, before anything returns", () => {
    expect(answerChrome("working", ON).critiquePresent).toBe(true);
    expect(answerChrome("idle", ON).critiquePresent).toBe(false);
  });

  it("offers Stop waiting only while there is something to stop", () => {
    expect(answerChrome("working", ON).stopOffered).toBe(true);
    expect(answerChrome("idle", ON).stopOffered).toBe(false);
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

// =============================================================================
// The same working state with the analysis OFF
// =============================================================================
//
// CC_TASK_PRACTICE_POLISH_v1 item 3, ruled 2026-09-15. Off means no model is
// asked at all, so three of the five claims above would be the screen saying
// something untrue. The other two still hold: the answer is being SAVED, and a
// save is still a round trip she must not start twice.
describe("while the answer is saving and nothing is being read", () => {
  // ⚑ MUTATION: leave the working label in place → the button says "Reading
  // your answer" while no model was asked → red. This is the whole point of
  // Roman's Q1 ruling: no sixth wording row, the idle label instead.
  it("does not claim to be reading anything", () => {
    expect(answerChrome("working", OFF).buttonLabelKey).toBe("answer_button");
    expect(answerChrome("working", OFF).buttonLabelKey).toBe(
      answerChrome("idle", OFF).buttonLabelKey,
    );
    expect(answerChrome("working", OFF).buttonLabelKey).not.toBe(
      answerChrome("working", ON).buttonLabelKey,
    );
  });

  // MUTATION: keep the block present → an empty bordered box invites her to
  // wait for a read she switched off → red.
  it("draws no critique block, because nothing is coming", () => {
    expect(answerChrome("working", OFF).critiquePresent).toBe(false);
  });

  it("offers no Stop waiting, because nothing is being waited for", () => {
    expect(answerChrome("working", OFF).stopOffered).toBe(false);
  });

  // The two that are about the SAVE, and are true in both worlds. Without
  // these, a change that turned the whole working state off when the analysis
  // was off would pass the three above — and double-press would be back.
  it("still guards the write itself", () => {
    expect(answerChrome("working", OFF).buttonDisabled).toBe(true);
    expect(answerChrome("working", OFF).boxLocked).toBe(true);
  });

  it("is the idle state either way once the write is done", () => {
    expect(answerChrome("idle", OFF)).toEqual(answerChrome("idle", ON));
  });
});

// v2.2.1, Fix 2 — "Did my answer save?"
describe("the answer box says it is saved", () => {
  const loaded = { saved_label: "Your answer — saved Sun 20 Sep · 9:02 am" };
  const pressed = (label: string | null, line: string | null) => ({
    saved_label: label,
    saved_line: line,
  });

  it("keeps the plain label when nothing is saved and nothing was pressed", () => {
    expect(savedLabelFor(null, null)).toBeNull();
  });

  it("says the loaded answer is on file — which is what survives a reload", () => {
    expect(savedLabelFor(loaded, null)).toBe("Your answer — saved Sun 20 Sep · 9:02 am");
  });

  it("moves to the press's label the moment the press returns", () => {
    expect(
      savedLabelFor(loaded, pressed("Your answer — saved Mon 21 Sep · 10:54 pm", null)),
    ).toBe("Your answer — saved Mon 21 Sep · 10:54 pm");
  });

  it("falls back to the loaded label when a press brought none", () => {
    expect(savedLabelFor(loaded, pressed(null, null))).toBe(loaded.saved_label);
  });

  it("shows the confirmation line only when the server sent one (analysis off)", () => {
    const off = "Saved. Answer analysis is off, so no analysis was requested.";
    expect(savedLineFor(null)).toBeNull();
    expect(savedLineFor(pressed("x", null))).toBeNull();
    expect(savedLineFor(pressed("x", off))).toBe(off);
  });
});
