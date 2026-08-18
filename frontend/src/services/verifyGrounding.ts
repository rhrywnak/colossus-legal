// =============================================================================
// verifyGrounding.ts — read the verify step's OWN grounding numbers
// -----------------------------------------------------------------------------
// The Completed card used to compute "Grounding: 92% (76 grounded, 7
// ungrounded)" from `entities_written` and `entities_flagged`. Those are ingest
// counters, not verify counters: `entities_written` includes the Document node
// and the Party nodes, neither of which is a grounded quote, and neither of
// which verify ever looked at. The percentage was therefore a number about a
// different question than the label claimed.
//
// The honest source is the verify step's `result_summary`, which the backend
// already writes with `grounded` / `total` keys (see
// `pipeline/workflow_steps/verify.rs::build_result_summary`). This module reads
// it and nothing else.
//
// Pure so it can be tested without a component harness — the frontend testing
// pattern in this repo (CLAUDE.md §4.30).
// =============================================================================

import { PipelineStep } from "./pipelineApi";

/** The verify step's own count of grounded items. */
export interface VerifyGrounding {
  /** Items whose quote was located in the canonical text. */
  grounded: number;
  /** Items verify examined. */
  total: number;
  /** `grounded / total`, rounded to a whole percent. */
  pct: number;
}

/** The step name the backend records for verification. */
const VERIFY_STEP = "verify";

/**
 * Read one non-negative integer out of an untyped `result_summary`.
 *
 * `result_summary` is `Record<string, unknown>` because it is JSON the backend
 * shapes per step. A missing key, a null, a string, or a negative all mean the
 * same thing here — this summary cannot answer the question — and all return
 * `null` so the caller can decline to render rather than render a zero that
 * looks like a real measurement.
 */
function readCount(summary: Record<string, unknown>, key: string): number | null {
  const raw = summary[key];
  if (typeof raw !== "number" || !Number.isFinite(raw) || raw < 0) return null;
  return Math.trunc(raw);
}

/**
 * The grounding numbers from the most recent completed `verify` step, or `null`
 * when the history carries none.
 *
 * `null` is a real answer and the caller must honour it: a document that has
 * never been verified, and a document verified with zero grounded items, are
 * different states and must not render identically (Standing Rule 1). The
 * caller shows the line only when this returns a value.
 *
 * "Most recent" is taken as the LAST matching entry in the array, which is the
 * order the history endpoint returns (oldest first) — the same assumption
 * `ExecutionHistory` already renders under. A re-verify therefore supersedes
 * the original run, which is exactly what the operator just did.
 */
export function verifyGroundingFromHistory(
  history: PipelineStep[] | undefined,
): VerifyGrounding | null {
  if (!history || history.length === 0) return null;

  for (let i = history.length - 1; i >= 0; i -= 1) {
    const step = history[i];
    if (step.step_name !== VERIFY_STEP) continue;
    if (!step.result_summary) continue;

    const grounded = readCount(step.result_summary, "grounded");
    const total = readCount(step.result_summary, "total");
    // Both keys or nothing: a summary carrying one of the two is a backend
    // shape we do not recognise, and guessing the other half would invent a
    // number. Keep looking at older runs instead.
    if (grounded === null || total === null) continue;
    if (grounded > total) continue; // impossible pair — not a summary we trust

    return {
      grounded,
      total,
      // A verify run over zero items is 0%, not a division by zero. It also
      // renders as "0 of 0 items grounded", which reads as the empty run it is.
      pct: total > 0 ? Math.round((grounded / total) * 100) : 0,
    };
  }

  return null;
}
