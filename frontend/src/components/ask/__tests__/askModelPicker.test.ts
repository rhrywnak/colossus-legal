// =============================================================================
// askModelPicker.test.ts — the picker's four states read differently
// =============================================================================
//
// CC_TASK_CHAT_DEFAULT_MODEL_v1, ruling 4b. Pure helpers, per the frontend's
// test pattern (CLAUDE.md rule 30 — there is no component-testing
// infrastructure in this project).
//
// The whole of 4b is WHICH SENTENCE APPEARS WHEN. Before this task the failed
// state did not exist on screen at all: the service swallowed its own failure
// and handed back one invented model, so an unreachable server and a one-model
// deployment looked identical. These tests are that distinction.

import { describe, expect, it } from "vitest";

import { isDisabled, offers, placeholderFor, type ModelCatalogue } from "../AskModelPicker";

const loading: ModelCatalogue = { state: "loading" };
const failed: ModelCatalogue = { state: "failed", detail: "HTTP 503" };
const empty: ModelCatalogue = { state: "ready", models: [] };
const ready: ModelCatalogue = {
  state: "ready",
  models: [
    { model_id: "claude-opus-5", display_name: "Claude Opus 5", is_default: true },
    { model_id: "claude-sonnet-5", display_name: "Claude Sonnet 5", is_default: false },
  ],
};

describe("what the picker says", () => {
  it("says it is LOADING while the catalogue is in flight", () => {
    expect(placeholderFor(loading)).toBe("Loading models…");
  });

  it("says it FAILED when the catalogue could not be loaded", () => {
    expect(placeholderFor(failed)).toBe("Models unavailable");
  });

  it("says NOTHING IS CONFIGURED for an empty catalogue that did arrive", () => {
    // A deployment with no active Anthropic row. Real, and not a failure.
    expect(placeholderFor(empty)).toBe("No models configured");
  });

  it("renders no placeholder at all once there are models", () => {
    expect(placeholderFor(ready)).toBeNull();
  });

  it("gives the three unusable states three DIFFERENT sentences", () => {
    // The point of 4b. Two of these reading alike would be the old behaviour
    // wearing new words: an unreachable server looking like a configuration.
    const said = [placeholderFor(loading), placeholderFor(failed), placeholderFor(empty)];
    expect(new Set(said).size).toBe(3);
    expect(said.every((s) => s !== null && s.trim() !== "")).toBe(true);
  });

  it("names no model of its own in any state", () => {
    // The fabricated entry this task deleted lived in the service, but the
    // picker is where it SHOWED. Nothing here may name a model.
    for (const catalogue of [loading, failed, empty, ready]) {
      expect(placeholderFor(catalogue) ?? "").not.toMatch(/claude/i);
    }
  });
});

describe("when the picker is usable", () => {
  it("is disabled in every state but a non-empty ready one", () => {
    expect(isDisabled(loading)).toBe(true);
    expect(isDisabled(failed)).toBe(true);
    expect(isDisabled(empty)).toBe(true);
    expect(isDisabled(ready)).toBe(false);
  });
});

describe("whether a model is still on offer", () => {
  it("is true for a model in a ready catalogue", () => {
    expect(offers(ready, "claude-sonnet-5")).toBe(true);
  });

  it("is false for a model that has left the catalogue", () => {
    // A history entry answered on a model since deactivated. Moving the picker
    // to it would leave a <select> whose value is not one of its options —
    // blank on screen, and sent as the chosen model on the next ask.
    expect(offers(ready, "claude-sonnet-4-6")).toBe(false);
  });

  it("is false while loading and false after a failure", () => {
    // Neither state knows what is on offer, and "I do not know" must not be
    // read as "yes" (Standing Rule 1).
    expect(offers(loading, "claude-opus-5")).toBe(false);
    expect(offers(failed, "claude-opus-5")).toBe(false);
  });

  it("is false for an empty catalogue", () => {
    expect(offers(empty, "claude-opus-5")).toBe(false);
  });
});
