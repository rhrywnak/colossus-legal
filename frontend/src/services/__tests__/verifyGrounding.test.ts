import { describe, expect, it } from "vitest";

import type { PipelineStep } from "../pipelineApi";
import { verifyGroundingFromHistory } from "../verifyGrounding";

/** A history row with only the fields this helper reads. */
function step(
  step_name: string,
  result_summary: Record<string, unknown>,
): PipelineStep {
  return {
    id: 1,
    document_id: "doc-x",
    step_name,
    status: "completed",
    started_at: "2026-08-17T00:00:00Z",
    completed_at: "2026-08-17T00:01:00Z",
    duration_secs: 60,
    triggered_by: "roman",
    input_params: {},
    result_summary,
    error_message: null,
    step_label: null,
  };
}

describe("verifyGroundingFromHistory", () => {
  it("reads the verify step's own grounded/total, not an ingest counter", () => {
    const history = [
      step("llm_extract", { entities: 83 }),
      step("verify", { grounded: 76, total: 83, grounding_pct: 91.6 }),
      step("ingest", { entities_written: 76, relationships_written: 317 }),
    ];
    expect(verifyGroundingFromHistory(history)).toEqual({
      grounded: 76,
      total: 83,
      pct: 92,
    });
  });

  it("returns null when the document has never been verified", () => {
    expect(verifyGroundingFromHistory([step("llm_extract", {})])).toBeNull();
    expect(verifyGroundingFromHistory([])).toBeNull();
    expect(verifyGroundingFromHistory(undefined)).toBeNull();
  });

  it("distinguishes 'never verified' from 'verified, nothing grounded'", () => {
    // The whole point of returning null above: this case is NOT null, and the
    // panel must be able to say 0 of 12 rather than showing nothing.
    expect(verifyGroundingFromHistory([step("verify", { grounded: 0, total: 12 })])).toEqual({
      grounded: 0,
      total: 12,
      pct: 0,
    });
  });

  it("prefers the most recent verify run, so a re-verify supersedes the first", () => {
    const history = [
      step("verify", { grounded: 70, total: 83 }),
      step("ingest", { entities_written: 70 }),
      step("verify", { grounded: 76, total: 83 }),
    ];
    expect(verifyGroundingFromHistory(history)?.grounded).toBe(76);
  });

  it("skips a summary it cannot read rather than inventing a number", () => {
    const history = [
      step("verify", { grounded: 70, total: 83 }),
      step("verify", { grounded: "seventy-six" }),
      step("verify", { total: 83 }),
      step("verify", { grounded: 90, total: 83 }), // impossible pair
    ];
    expect(verifyGroundingFromHistory(history)).toEqual({
      grounded: 70,
      total: 83,
      pct: 84,
    });
  });

  it("does not divide by zero on an empty verify run", () => {
    expect(verifyGroundingFromHistory([step("verify", { grounded: 0, total: 0 })])).toEqual({
      grounded: 0,
      total: 0,
      pct: 0,
    });
  });
});
