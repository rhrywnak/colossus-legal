// =============================================================================
// casePhases.ts — the case's four phases, for every surface that shows one
// -----------------------------------------------------------------------------
// Ruled 2026-08-17: the display labels (PRE-PROBATE · PROBATE · COA · COMPLAINT)
// live in `/data/timeline.json` and ONLY there. It is data, not code. The
// backend stores and returns the slug (`estate` · `probate` · `appeals` ·
// `civil_lawsuit`) and never renders a label; this module is the one place the
// frontend turns a slug into a name.
//
// Every surface that shows a phase reads it from here: the upload dialog's
// dropdown, the document page's control, the Documents table and its filter
// chips, the Home page's timeline band, and the Timeline page. Renaming a phase
// is then one line in one JSON file — no deploy, no code change, nothing to keep
// in step.
//
// The alternative — a `PHASE_LABELS` map in TypeScript — was rejected precisely
// because it would have been the second copy, and the second copy is the one
// that goes stale.
// =============================================================================

import { getCaseTimeline } from "./caseTimeline";

/** One phase, as a control renders it. */
export type PhaseOption = {
  /** The stored value: `estate` | `probate` | `appeals` | `civil_lawsuit`. */
  slug: string;
  /** What a human reads. Comes from the data file; never hardcoded here. */
  label: string;
};

/**
 * The in-flight or completed load, shared by every caller.
 *
 * The file is static and small, and a page can easily mount four things that
 * each want the phase list. Caching the PROMISE (rather than the result) means
 * four simultaneous mounts share one request instead of racing four.
 *
 * Deliberately not invalidated: the file changes at deploy time, and a reload is
 * what picks it up.
 */
let cached: Promise<PhaseOption[]> | null = null;

/**
 * The four phases, in the case's chronological order — the order the data file
 * lists them, which is the order the timeline renders and the order a dropdown
 * should offer.
 *
 * Rejects (rather than returning an empty list) when the file cannot be read:
 * an empty dropdown and a broken one must not look the same (Standing Rule 1).
 * Callers surface the failure; none of them silently degrades to no phases.
 */
export function getPhaseOptions(): Promise<PhaseOption[]> {
  if (cached === null) {
    cached = getCaseTimeline()
      .then((timeline) =>
        timeline.phases.map((phase) => ({ slug: phase.id, label: phase.label })),
      )
      .catch((e: unknown) => {
        // Drop the cache so a later mount retries rather than replaying the
        // failure forever.
        cached = null;
        throw e;
      });
  }
  return cached;
}

/**
 * Turn a stored slug into its display label.
 *
 * Pure, so it is testable and so a component that already holds the options does
 * not re-fetch to render a row.
 *
 * ## The two absences, kept distinct
 *
 * `null`/`undefined`/empty slug → `""` — the document has no phase, and the
 * caller decides what to show (the table shows an em dash, the control shows
 * "Not set"). An UNKNOWN slug → the slug itself, verbatim: that is a document
 * carrying a phase this build does not know, and showing the raw value is how an
 * operator finds out. Silently rendering it as "no phase" would hide a real
 * disagreement between the column and the data file.
 */
export function phaseLabel(
  options: PhaseOption[],
  slug: string | null | undefined,
): string {
  if (slug === null || slug === undefined || slug.trim() === "") return "";
  return options.find((o) => o.slug === slug)?.label ?? slug;
}
