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

/** Cost label for a run (`—` for a local model with no per-token cost). */
export function costLabel(summary: ThemeScanSummary): string {
  return summary.computed_cost == null ? "—" : `$${summary.computed_cost.toFixed(4)}`;
}

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
