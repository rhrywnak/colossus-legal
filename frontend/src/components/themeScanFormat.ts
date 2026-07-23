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
