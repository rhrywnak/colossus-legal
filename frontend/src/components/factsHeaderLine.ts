// =============================================================================
// factsHeaderLine — the ONE sentence under the Scenario facts heading
// =============================================================================
//
// This module exists because two sentences used to compete for that slot and
// could both be wrong at once.
//
// ## The defect it closes (PROD S-13, 2026-09-11)
//
// The header rendered the served "No scan has run yet" notice whenever the
// backend reported no COMPLETED run, and otherwise a summary composed from the
// run history. A scenario whose only run was CANCELLED satisfied the first
// condition, so the page said nothing had ever been scanned — three lines above
// a history table listing the cancelled run. Neither half was lying on its own.
// The screen was.
//
// The fix is not a better sentence, it is one decision. Both answers now come out
// of this function, from one array, so the contradictory pair is not a state this
// code can reach — there is no branch that produces both and no branch that
// produces the notice while `runs` has a row in it.
//
// ## Why the caller formats the date
//
// `formatWhen` is passed in rather than imported so this module stays pure and
// testable with a stub. The date is the one part of the sentence the SERVER
// cannot compose — it is rendered in the reader's locale — which is the same
// carve-out the history's `{run}` slot has carried since 2.15.

import type { ScanRunHeader, ScanRunState, ScanWording } from "../services/themeScan";

/** Everything the sentence is decided from. */
export interface FactsHeaderLineInput {
  /**
   * The run history, newest first, or `null` while it has not been READ yet.
   *
   * The distinction is load-bearing and is why this is not just an array. An
   * empty array means "this scenario has never been scanned" and earns the
   * notice; `null` means "we do not know yet" and earns silence. Collapsing them
   * would flash "No scan has run yet" onto every scenario for as long as its
   * history takes to arrive — which is the defect above, reintroduced as a
   * race instead of as a rule.
   */
  runs: ScanRunHeader[] | null;
  /** The stored sentences, or `null` until they load. */
  wording: ScanWording | null;
  /** The served never-scanned notice, or `null` when the payload had none. */
  neverScannedNotice: string | null;
  /** Renders an ISO-8601 instant in the reader's locale. */
  formatWhen: (isoDate: string) => string;
}

/**
 * The status word for a run state, in the last-scan line's lowercase register.
 *
 * Exhaustive over `ScanRunState` on purpose: TypeScript's `Record` of a union
 * makes a new state a COMPILE error here, rather than a run silently described
 * by whichever arm a lookup fell through to. If a fifth state is ever added to
 * the backend, this line is where the compiler stops and asks what it is called.
 */
function statusWord(state: ScanRunState, wording: ScanWording): string {
  const words: Record<ScanRunState, string> = {
    running: wording.header_status_running,
    completed: wording.header_status_completed,
    cancelled: wording.header_status_cancelled,
    failed: wording.header_status_failed,
  };
  return words[state];
}

/**
 * The sentence to show beneath the heading, or `null` for no sentence at all.
 *
 * Returns `null` — rather than something reassuring — in the two states where
 * this build genuinely does not know what to say: the history has not been read,
 * and the words have not loaded. Both are brief and both are honest; a sentence
 * invented to fill the gap would be a claim about a scan nobody has looked up.
 */
export function factsHeaderLine(input: FactsHeaderLineInput): string | null {
  const { runs, wording, neverScannedNotice, formatWhen } = input;

  // Nothing read yet, or no words to say it with. Silence, not a guess.
  if (runs === null || wording === null) return null;

  // An empty history is the ONLY state the never-scanned notice can describe.
  // This is the whole guarantee: below this line `runs` is known non-empty, so
  // the notice is unreachable from here on.
  const newest = runs[0];
  if (newest === undefined) return neverScannedNotice;

  const when = formatWhen(newest.started_at);
  const status = statusWord(newest.status, wording);

  // `candidates_read === 0` means the run never reached the pool read — it did
  // not look. Saying "0 candidates" would report that it looked and found
  // nothing, which is a different run. See `ScanRunHeader.candidates_read`.
  if (newest.candidates_read === 0) {
    return wording.header_last_scan_no_count_template
      .replace("{when}", when)
      .replace("{status}", status);
  }

  return wording.header_last_scan_template
    .replace("{when}", when)
    .replace("{status}", status)
    .replace("{count}", String(newest.candidates_read));
}
