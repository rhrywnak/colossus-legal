// =============================================================================
// scanConfirmModel.test.ts — clicking Scan must not start a scan
// =============================================================================
//
// The mutation proof. Until 2026-09-11 the header's `Scan again` button was
// wired straight to `scan.onRun`: one click and 313 metered calls were under
// way, unasked and uncancellable.
//
// ## Why this is a reducer test and not a click test
//
// There is no RTL and no jsdom in this tree (CLAUDE.md rule 30), so a click
// cannot be fired. That constraint produced a better proof than the click would
// have been: the property worth asserting is a NEGATIVE — "nothing except Run
// scan starts a run" — and over a component that is only as good as the list of
// interactions somebody remembered to simulate. Over a closed union of actions
// it is exhaustive, and a control added later that forgets to go through the
// machine is a control this file's first test will not cover — which is why
// `ScenarioFactsHeader` routes even the two no-op actions through it.

import { describe, expect, it } from "vitest";

import {
  confirmSentence,
  estimatedMinutes,
  flattenConfirmSentence,
  idleConfirm,
  scanConfirmStep,
} from "../scanConfirmModel";
import type { ScanConfirmAction, ScanConfirmState } from "../scanConfirmModel";
import { model, wording } from "./scanHeaderFixtures";

const MODEL = "unsloth/Qwen3.8-27B-NVFP4";
const OTHER = "claude-opus-4-8";

/** Every action a human can take on this header. */
const EVERY_ACTION: ScanConfirmAction[] = [
  { type: "openConfirm" },
  { type: "chooseModel", modelId: OTHER },
  { type: "cancel" },
  { type: "confirmRun" },
  { type: "toggleHistory" },
  { type: "toggleCollapse" },
];

const confirming: ScanConfirmState = { phase: "confirming", modelId: MODEL };

describe("no control but Run scan starts a scan", () => {
  it("starts nothing from idle, whatever is done", () => {
    // Including `confirmRun` itself: a Run scan that somehow fired with no bar
    // open must still not run. The button cannot exist in that state today, and
    // "cannot exist" is the kind of claim that survives until the next refactor
    // moves the button.
    for (const action of EVERY_ACTION) {
      expect(
        scanConfirmStep(idleConfirm, action, MODEL).run,
        `${action.type} started a scan from idle`,
      ).toBeNull();
    }
  });

  it("starts nothing from an OPEN confirmation except on Run scan", () => {
    for (const action of EVERY_ACTION) {
      const { run } = scanConfirmStep(confirming, action, MODEL);
      if (action.type === "confirmRun") {
        expect(run).toBe(MODEL);
      } else {
        expect(run, `${action.type} started a scan`).toBeNull();
      }
    }
  });

  it("opens the bar when Scan is clicked, and only opens it", () => {
    const step = scanConfirmStep(idleConfirm, { type: "openConfirm" }, MODEL);
    expect(step.state).toEqual({ phase: "confirming", modelId: MODEL });
    expect(step.run).toBeNull();
  });
});

describe("the run that starts is the run that was confirmed", () => {
  it("runs the model the OPEN bar holds, not whatever the picker now says", () => {
    // The sentence a human agreed to named a model. Re-reading the dropdown
    // afterwards could start a different — and billed — one.
    expect(scanConfirmStep(confirming, { type: "confirmRun" }, OTHER).run).toBe(MODEL);
  });

  it("re-aims an open bar when the dropdown changes", () => {
    const step = scanConfirmStep(confirming, { type: "chooseModel", modelId: OTHER }, MODEL);
    expect(step.state).toEqual({ phase: "confirming", modelId: OTHER });
    expect(step.run).toBeNull();
  });

  it("refuses to open with nothing selected", () => {
    expect(scanConfirmStep(idleConfirm, { type: "openConfirm" }, null).state).toEqual(
      idleConfirm,
    );
  });

  it("closes on Cancel and on a completed run, so one confirmation is one scan", () => {
    expect(scanConfirmStep(confirming, { type: "cancel" }, MODEL).state).toEqual(idleConfirm);
    expect(scanConfirmStep(confirming, { type: "confirmRun" }, MODEL).state).toEqual(
      idleConfirm,
    );
  });

  it("survives the history and the fold", () => {
    // Both are things a human does WHILE deciding — the history is where they
    // check how long the last scan took. Closing the question they are part-way
    // through answering would be the page taking the decision back.
    for (const action of [{ type: "toggleHistory" }, { type: "toggleCollapse" }] as const) {
      expect(scanConfirmStep(confirming, action, MODEL).state).toEqual(confirming);
    }
  });
});

describe("the estimate is measured or it is absent", () => {
  it("is absent when nothing has timed this model", () => {
    expect(estimatedMinutes(313, undefined)).toBeNull();
  });

  it("is absent rather than zero for a nonsense rate", () => {
    // A stored 0 would render "about 0 minutes" for 313 metered calls.
    expect(estimatedMinutes(313, 0)).toBeNull();
    expect(estimatedMinutes(313, -4)).toBeNull();
    expect(estimatedMinutes(313, Number.NaN)).toBeNull();
  });

  it("is absent when there is no pool to scan", () => {
    expect(estimatedMinutes(0, 10.54)).toBeNull();
  });

  it("reproduces the mockup's own arithmetic", () => {
    // 313 candidates at the 10.54 s/card the mockup was drawn against.
    expect(estimatedMinutes(313, 10.54)).toBe(55);
  });

  it("never rounds down to nothing", () => {
    // A twenty-second scan reads "about 1 minute": overstated by forty seconds,
    // where "about 0 minutes" reads as a fault.
    expect(estimatedMinutes(2, 10)).toBe(1);
  });
});

describe("the confirmation sentence", () => {
  const words = wording();

  /** The whole question, as a screen reader hears it. */
  const flat = (model: Parameters<typeof confirmSentence>[0]["model"], count: number | null) => {
    const parts = confirmSentence({ wording: words, model, candidateCount: count });
    return parts === null ? null : flattenConfirmSentence(parts);
  };

  it("names the model with its cost, and the pool, and the time", () => {
    expect(flat(model({ measured_seconds_per_candidate: 10.54 }), 313)).toBe(
      "Run a theme scan with Qwen3.8 27B (NVFP4, local) (local · $0)? 313 candidates, " +
        "about 55 minutes.",
    );
  });

  it("splits at the template's OWN slot, so the bold falls on the model", () => {
    // Not a search of the rendered text for the model's name — that would break
    // the day a model is called "Scan". The slot is where the template says the
    // name goes, and the emphasis follows it.
    const parts = confirmSentence({
      wording: words,
      model: model({ measured_seconds_per_candidate: 10.54 }),
      candidateCount: 313,
    });
    expect(parts).toEqual({
      before: "Run a theme scan with ",
      model: "Qwen3.8 27B (NVFP4, local) (local · $0)",
      after: "? 313 candidates, about 55 minutes.",
    });
  });

  it("emphasises nothing rather than guessing when a template has no slot", () => {
    const parts = confirmSentence({
      wording: wording({ header_confirm_no_count_template: "Run a theme scan?" }),
      model: model(),
      candidateCount: null,
    });
    expect(parts).toEqual({ before: "Run a theme scan?", model: "", after: "" });
  });

  it("drops the time clause entirely when nothing has been measured", () => {
    const sentence = flat(model(), 313);
    expect(sentence).toBe(
      "Run a theme scan with Qwen3.8 27B (NVFP4, local) (local · $0)? 313 candidates.",
    );
    expect(sentence).not.toContain("about");
  });

  it("drops the count and the time together when the count could not be read", () => {
    // The estimate IS the count times the rate, so a sentence with no count
    // cannot honestly carry one.
    const sentence = flat(model({ measured_seconds_per_candidate: 10.54 }), null);
    expect(sentence).toBe("Run a theme scan with Qwen3.8 27B (NVFP4, local) (local · $0)?");
  });

  it("asks nothing when there is no model to ask about", () => {
    expect(
      confirmSentence({ wording: words, model: undefined, candidateCount: 313 }),
    ).toBeNull();
  });

  it("leaves no placeholder unfilled in any of the three sentences", () => {
    for (const [m, count] of [
      [model({ measured_seconds_per_candidate: 10.54 }), 313],
      [model(), 313],
      [model(), null],
    ] as const) {
      expect(flat(m, count)).not.toMatch(/\{[a-z]+\}/);
    }
  });
});
