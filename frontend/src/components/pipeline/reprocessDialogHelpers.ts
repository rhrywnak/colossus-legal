/**
 * Pure guard logic for ReprocessDialog.
 *
 * Extracted for the same reason `configurationPanelHelpers.ts` was: this repo has
 * no component-testing setup (no RTL, no jsdom), so anything left inside a .tsx
 * cannot be asserted. The decision that matters here — *may this operator start a
 * re-extraction yet?* — is the one thing in the dialog worth testing, so it lives
 * where a test can reach it.
 */

/** What the curated-row lookup came back with. */
export type CuratedState =
  | { kind: "loading" }
  | { kind: "loaded"; total: number }
  /** The count could not be read. NOT the same as zero. */
  | { kind: "failed" };

export interface GuardInput {
  curated: CuratedState;
  /** What the operator has typed into the confirmation box. */
  typed: string;
  /** The document id they must type to proceed. */
  documentId: string;
  /** A request is already in flight. */
  running: boolean;
}

export interface GuardResult {
  /** Show the typed-id confirmation box. */
  needsTypedId: boolean;
  /** Enable the Re-extract button. */
  canRun: boolean;
}

/**
 * Decide whether the re-extraction may start.
 *
 * ## The rule, and the one non-obvious case
 *
 * A document carrying rulings demands the typed id. A document carrying none
 * gets no friction. The case worth stating: when the count **failed to load**,
 * the answer is the SAME as "carries rulings" — an unmeasured document is not an
 * empty one, and defaulting a failed read to "no friction" would put the
 * riskiest documents behind the weakest guard exactly when the system is already
 * misbehaving.
 *
 * While loading, nothing may start: the operator would be committing before the
 * dialog can tell them what is at stake.
 */
export function evaluateGuard({
  curated,
  typed,
  documentId,
  running,
}: GuardInput): GuardResult {
  const needsTypedId = curated.kind === "failed" || (curated.kind === "loaded" && curated.total > 0);

  const idConfirmed = typed.trim() === documentId;
  const canRun =
    !running && curated.kind !== "loading" && (!needsTypedId || idConfirmed);

  return { needsTypedId, canRun };
}
