// =============================================================================
// themeScanFormat.ts — pure formatting/derivation helpers for ThemeScanPanel.
// -----------------------------------------------------------------------------
// Extracted from the component so they can be unit-tested without rendering
// (CLAUDE.md frontend test pattern: pure-helper tests + service tests).
// =============================================================================

import type { ThemeScanSummary } from "../services/themeScan";

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
 */
export function lastRunSummary(
  runs: {
    candidates_total: number | null;
    started_at: string;
    pool_delta?: number | null;
    model_id?: string;
  }[],
  modelName?: (modelId: string) => string,
): string | null {
  const latest = runs[0];
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
