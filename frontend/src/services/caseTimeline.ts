// =============================================================================
// caseTimeline.ts — client + rollup for the static /data/timeline.json file
// -----------------------------------------------------------------------------
// Powers the compact Timeline band restored to the Home page. Reads the same
// bundled, intact `/data/timeline.json` the full Timeline page consumes (do NOT
// modify that data file). This module adds what the band needs that the page
// did inline: a validated loader (the page swallowed errors — Rule 1) and a
// pure rollup that turns the raw phases + events into per-phase pill summaries.
// =============================================================================

import { fetchStaticJson } from "./staticData";

/** A timeline phase as stored in the data file (extra fields ignored here). */
export type TimelinePhase = {
  id: string;
  label: string;
  date_range: string;
  color: string;
};

/** A timeline event; only `phase` matters for the band's per-phase counts. */
export type TimelineEvent = {
  phase: string;
};

/** The slice of `/data/timeline.json` the band relies on. */
export type CaseTimeline = {
  phases: TimelinePhase[];
  events: TimelineEvent[];
};

/**
 * One phase reduced to exactly what a pill renders: its identity, label, date
 * range, accent color, and how many events fall in it. `eventCount` is derived,
 * not stored, so it can never drift from the events array.
 */
export type PhaseSummary = {
  id: string;
  label: string;
  date_range: string;
  color: string;
  eventCount: number;
};

const TIMELINE_PATH = "/data/timeline.json";
const TIMELINE_LABEL = "case timeline";

/**
 * Reduce the raw timeline into per-phase pill summaries.
 *
 * Pure (no fetch, no DOM) so it is independently unit-testable and cannot drift
 * from render. For each phase we count the events whose `phase` matches its id.
 *
 * ## React/TS Learning: derive, don't duplicate
 * The original Home effect stored `eventCount` as separate state alongside the
 * phases; that invites the count and the list getting out of sync. Computing it
 * here from the single events array keeps one source of truth.
 *
 * @param data the validated timeline payload
 * @returns one {@link PhaseSummary} per phase, in the data file's phase order
 */
export function buildPhaseSummaries(data: CaseTimeline): PhaseSummary[] {
  return data.phases.map((phase) => ({
    id: phase.id,
    label: phase.label,
    date_range: phase.date_range,
    color: phase.color,
    eventCount: data.events.filter((event) => event.phase === phase.id).length,
  }));
}

/**
 * Load and validate the timeline document.
 *
 * Asserts both load-bearing arrays are present, throwing a contextual error
 * rather than the original effect's silent `.catch(() => {})` — Standing Rule 1.
 *
 * @returns the validated {@link CaseTimeline}
 * @throws Error on fetch/timeout/non-2xx/invalid-JSON (via {@link fetchStaticJson})
 *   or when `phases` / `events` are not arrays
 */
export async function getCaseTimeline(): Promise<CaseTimeline> {
  const data = await fetchStaticJson(TIMELINE_PATH, TIMELINE_LABEL);

  const parsed = data as Partial<CaseTimeline>;
  if (!Array.isArray(parsed.phases) || !Array.isArray(parsed.events)) {
    throw new Error(
      `${TIMELINE_LABEL} at ${TIMELINE_PATH} is missing required arrays ` +
        `(expected phases[] and events[]). ` +
        `Fix ${TIMELINE_PATH} and redeploy the frontend ` +
        `(reloading the page will not help — the file itself is malformed).`,
    );
  }

  return parsed as CaseTimeline;
}
