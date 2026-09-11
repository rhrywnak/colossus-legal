// =============================================================================
// themeScanFormat.ts — pure formatting/derivation helpers for ThemeScanPanel.
// -----------------------------------------------------------------------------
// Extracted from the component so they can be unit-tested without rendering
// (CLAUDE.md frontend test pattern: pure-helper tests + service tests).
// =============================================================================

import type { ScanRunHeader, ScanWording, ThemeScanSummary } from "../services/themeScan";

/** Format elapsed milliseconds as `m:ss` for the running timer / durations. */
export function formatElapsed(ms: number): string {
  const totalSec = Math.floor(Math.max(0, ms) / 1000);
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

/** Format a computed dollar cost (`—` when null: a local model with no per-token
 *  cost, or a run where no token usage was reported). Shared by the completed-run
 *  card and the history list so both render cost identically. */
export function formatCost(cost: number | null): string {
  return cost == null ? "—" : `$${cost.toFixed(4)}`;
}

/** Cost label for a completed run summary (delegates to [`formatCost`]). */
export function costLabel(summary: ThemeScanSummary): string {
  return formatCost(summary.computed_cost);
}

/** Format a run's ISO-8601 `started_at` as a compact local date + time for the
 *  history row (e.g. `Jul 16, 14:32`). An unparseable value degrades to the raw
 *  string rather than throwing (Standing Rule 1 — the row still renders and the
 *  bad value is visible, not swallowed). */
export function formatRunTimestamp(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/**
 * The last-run summary the scan control line shows.
 *
 * `"148 candidates · +54 since the previous scan · Aug 2, 09:14 · Claude Opus 4.8"`
 *
 * `null` when there is nothing to report — no runs at all, or none that got far
 * enough to have a count. Rendering "0 candidates" for a scenario that has never
 * been scanned would say something false about a scan that never happened; the
 * absence of the phrase is the honest signal, and the history disclosure below is
 * where a human goes for detail either way.
 *
 * Reads the FIRST run because the backend serves them newest-first — the ordering
 * is server-owned (task D2b) and this must not re-sort and quietly disagree.
 *
 * ## Task 1.7C (§2.3, ruling R2) added two clauses
 *
 * * **The delta**, when there is one. `pool_delta` is `null` on the first
 *   measurable run, and the clause is then OMITTED rather than rendered as "+0" —
 *   "there is nothing to compare against" is not "the pool did not change". The
 *   delta itself is computed by the backend (Rule 12); this only words it.
 * * **The model**, which §2.3's example meta line carries and which the panel
 *   already knows. Resolved through the caller's catalogue rather than a lookup
 *   here, so this module still holds no vocabulary of its own.
 *
 * `modelName` is optional so the existing callers and their tests keep working
 * unchanged; without it the clause is simply absent.
 *
 * "Last run" means the latest COMPLETED run. A failed or still-running one is
 * not summarised here — see the comment on the selection itself.
 */
export function lastRunSummary(
  runs: {
    status: string;
    candidates_total: number | null;
    started_at: string;
    pool_delta?: number | null;
    model_id?: string;
  }[],
  modelName?: (modelId: string) => string,
): string | null {
  // THE STATUS FILTER, and why this line needed one (audit defect 10).
  //
  // This used to read `runs[0]` — the newest run of ANY status. The list arrives
  // `ORDER BY started_at DESC` with no status fence, `fail_scan_run` changes only
  // `status` and `error`, and `candidates_total` is written at PROMOTE time. So a
  // run that died mid-judging kept a non-null total, sat at index 0, and rendered
  // here as "104 candidates · Aug 9 · Claude Opus 5" — a sentence describing a
  // scan that worked, about one that did not.
  //
  // The collapsed card eleven lines away in `ThemeScanPanel` already made this
  // distinction (`latestCompleted` / `latestSettled` / `latestFailed`), which
  // meant one screen could carry two different runs under two labels. This adopts
  // the same selection, so both surfaces name the same run or say nothing.
  //
  // A failed or running run is not described here AT ALL rather than described
  // with a qualifier: the failed run has its own sentence on the collapsed card
  // (`collapsedFailedSummary`), and two sentences about one failure on one screen
  // is how the .389 card came to contradict itself.
  const latest = runs.find((run) => run.status === "completed");
  if (!latest || latest.candidates_total == null) return null;

  // Built as parts and joined, so an absent clause leaves no orphaned " · ".
  const parts: string[] = [`${latest.candidates_total} candidates`];
  if (latest.pool_delta != null) {
    const signed = latest.pool_delta > 0 ? `+${latest.pool_delta}` : String(latest.pool_delta);
    parts.push(`${signed} since the previous scan`);
  }
  parts.push(formatRunTimestamp(latest.started_at));
  if (modelName && latest.model_id) {
    parts.push(modelName(latest.model_id));
  }
  return parts.join(" · ");
}

/**
 * NOTE: `formatMergeState` was REMOVED.
 *
 * It rendered "merged 2× · last Jul 18, 14:00" from a run's merge counters. Both
 * the counters and the display belonged to the run-level merge model: when a RUN
 * was the unit of merge, "how many times was this run merged" was a real question.
 * Merge is pick-keyed now, so the provenance the human needs is per-fact (the
 * judgment strip on the card) and per-pick (a suggestion's applied state) — never
 * per-run. The backend no longer emits `merge_count` / `last_merged_at` at all.
 */
/** Relevant-set agreement between two completed runs, from their full relevant
 *  sets (`suggestions`).
 *
 *  This is a PARTIAL agreement: irrelevant verdicts are only sampled in the
 *  summary, so the full agreement (incl. irrelevant-on-irrelevant) needs the
 *  `scan_run_verdicts` join (a backend query, out of scope here).
 *  - `relevantPct` = Jaccard of the two relevant sets (|A∩B| / |A∪B|).
 *  - `rolePct`     = role agreement on their intersection.
 *  - `sharedCount` = size of the intersection. */
export function computeAgreement(
  a: ThemeScanSummary,
  b: ThemeScanSummary,
): { relevantPct: number; rolePct: number; sharedCount: number } {
  const roleOf = (s: ThemeScanSummary) =>
    new Map(s.suggestions.map((x) => [x.graph_node_id, x.proposed_role]));
  const ra = roleOf(a);
  const rb = roleOf(b);
  const union = new Set<string>([...ra.keys(), ...rb.keys()]);
  const shared = [...ra.keys()].filter((id) => rb.has(id));
  const relevantPct = union.size === 0 ? 100 : Math.round((shared.length / union.size) * 100);
  const roleMatches = shared.filter((id) => ra.get(id) === rb.get(id)).length;
  const rolePct = shared.length === 0 ? 0 : Math.round((roleMatches / shared.length) * 100);
  return { relevantPct, rolePct, sharedCount: shared.length };
}

// ─── The collapsed scan card (piece 4a, 2026-08-08) ─────────────────────────

/**
 * The one line a FOLDED scan card shows: when, which model, how many proposed.
 *
 * ## Why this is a function and not three `.replace()` calls in the JSX
 *
 * CLAUDE.md rule 30 records that component-test infrastructure is deliberately not
 * set up, so a sentence composed inside a React tree is a sentence nothing can
 * assert. The card folds by default — this line is the ONLY thing most readers
 * will see of a completed scan — and "reports the run, the model and the proposed
 * count" is exactly the kind of claim that should be a test rather than a promise.
 *
 * Every part is supplied by the caller: the template from the settings store, the
 * model's display name from the panel's catalogue, and the date already formatted
 * in the reader's locale (the server owns the sentence, the browser owns the date
 * format — the same split the delete confirmation makes).
 *
 * `proposedCount` is `null` when nothing is proposed, which renders as `0` rather
 * than an em dash: on a collapsed card "0 proposed" is the true and useful
 * statement — the run finished and there is nothing waiting — while a dash would
 * read as "not measured".
 */
export function collapsedScanSummary(
  template: string,
  when: string,
  model: string,
  proposedCount: number | null,
): string {
  return template
    .replace("{when}", when)
    .replace("{model}", model)
    .replace("{count}", String(proposedCount ?? 0));
}

/**
 * The one line a FOLDED scan card shows when the latest run FAILED.
 *
 * ## Domain note: the line that lied (ruling R4, 2026-08-09)
 *
 * The collapsed card is what Roman read first, and on 2026-08-09 it said "Last
 * scan 2:57 PM · Claude Opus 5 · 0 proposed" about a run whose 104 judge calls
 * had all returned 400 inside five seconds. "0 proposed" is a claim about a scan
 * that WORKED and found nothing; that run judged nothing at all. A scenario
 * cannot be told apart from a scanned-and-empty one by that sentence, which is
 * why a failed run gets a different sentence rather than the same one with a
 * zero in it.
 *
 * Same shape as its sibling above and for the same reason: composed here, where a
 * test can assert it, rather than inside a React tree where nothing can.
 * `{count}` is the FAILED count — the number that was recorded all along and had
 * nowhere to appear.
 */
export function collapsedFailedSummary(
  template: string,
  when: string,
  model: string,
  failedCount: number,
): string {
  return template
    .replace("{when}", when)
    .replace("{model}", model)
    .replace("{count}", String(failedCount));
}

/**
 * The ONE line a collapsed scan card shows, or `null` when there is nothing to
 * fold — no settled run yet, or the words have not loaded.
 *
 * Lifted out of `ThemeScanPanel` on 2026-09-11. It belongs here beside the two
 * templates it fills, and it had to leave: that component is pre-existing debt
 * at 853 non-comment lines against a 300-line limit (ruling R6 → task 3.14),
 * and the facts-header rebuild added lines to it. This is what pays them back,
 * so the panel comes out of the change no larger than it went in.
 *
 * ## Why a FAILED run outranks a completed one
 *
 * They are found separately and the failed branch wins. A run whose every judged
 * call failed records `failed` (ruling R3), so it is invisible to a search for
 * the latest COMPLETED run — correctly, because it projects nothing. But it is
 * the run the human just watched, and a card that quietly described the one
 * before it would report a scan that worked to someone who had just seen one
 * fail. Only the most recent SETTLED run can claim the line.
 */
export function collapsedCardSummary(input: {
  runs: ScanRunHeader[];
  wording: ScanWording | null;
  proposedCount: number | null;
  formatWhen: (isoDate: string) => string;
  modelName: (modelId: string) => string;
}): string | null {
  const { runs, wording, proposedCount, formatWhen, modelName } = input;
  if (wording === null) return null;

  const settled = runs.find((r) => r.status !== "running") ?? null;
  if (settled?.status === "failed") {
    return collapsedFailedSummary(
      wording.card_collapsed_failed_template,
      formatWhen(settled.started_at),
      modelName(settled.model_id),
      settled.failed_count,
    );
  }

  const completed = runs.find((r) => r.status === "completed") ?? null;
  if (completed === null) return null;
  return collapsedScanSummary(
    wording.card_collapsed_summary_template,
    formatWhen(completed.started_at),
    modelName(completed.model_id),
    proposedCount,
  );
}
