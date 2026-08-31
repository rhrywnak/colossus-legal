// =============================================================================
// timelineFilters.ts — every decision the timeline page makes, as pure functions
// =============================================================================
//
// This project has no component-testing tier (no RTL, no jsdom rendering), so
// anything decided inside a component is decided where no test can reach it.
// Everything here is a plain function over plain data: what the filters keep,
// how events group into phases, which events name a phase that does not exist,
// and how one date, one badge and one link read. The components below become
// arrangement, and the judgement lives where `vitest` can see it.

import type {
  CaseTimeline,
  ChronologyWording,
  TimelineEvent,
  TimelineLink,
  TimelinePhase,
  TimelineTag,
} from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";

/** Everything the filter bar can narrow by. Empty strings mean "not set". */
export type TimelineFilters = {
  /** A tag id, or null for every tag. */
  tag: string | null;
  /** A phase id when one phase owns the page (design R16), else null. */
  phase: string | null;
  /** Free text, matched against title and fact. */
  search: string;
  /** ISO `YYYY-MM-DD`, inclusive. */
  from: string;
  /** ISO `YYYY-MM-DD`, inclusive. */
  to: string;
};

export const NO_FILTERS: TimelineFilters = {
  tag: null,
  phase: null,
  search: "",
  from: "",
  to: "",
};

/** True when any filter would narrow the list. */
export function isFiltered(filters: TimelineFilters): boolean {
  return (
    filters.tag !== null ||
    filters.phase !== null ||
    filters.search.trim() !== "" ||
    filters.from !== "" ||
    filters.to !== ""
  );
}

/**
 * The events the active filters keep, in the order they arrived.
 *
 * Filters COMPOSE (design R7): each one narrows what the previous left, so a
 * tag and a date range together mean both, never either. Order is preserved
 * because the API already sorted by `(event_date, id)` and re-sorting here
 * would be a second opinion about chronology.
 */
export function applyFilters(
  events: TimelineEvent[],
  filters: TimelineFilters,
): TimelineEvent[] {
  const needle = filters.search.trim().toLowerCase();
  return events.filter((event) => {
    if (filters.phase !== null && event.phase !== filters.phase) return false;
    if (filters.tag !== null && !event.tags.includes(filters.tag)) return false;
    if (filters.from !== "" && event.event_date < filters.from) return false;
    if (filters.to !== "" && event.event_date > filters.to) return false;
    if (needle !== "") {
      const haystack = `${event.title} ${event.fact ?? ""}`.toLowerCase();
      if (!haystack.includes(needle)) return false;
    }
    return true;
  });
}

/** One phase and the events that belong to it. */
export type PhaseGroup = { phase: TimelinePhase; events: TimelineEvent[] };

/**
 * Group events under their phase, in the phases' stored order.
 *
 * Phases with no matching event are KEPT: design R6 says every phase stays
 * visible, so a phase the filters emptied says so rather than disappearing and
 * leaving a reader wondering where it went.
 */
export function groupByPhase(
  phases: TimelinePhase[],
  events: TimelineEvent[],
): PhaseGroup[] {
  return phases.map((phase) => ({
    phase,
    events: events.filter((event) => event.phase === phase.id),
  }));
}

/**
 * Events naming a phase that has no row.
 *
 * ## ⚑ These are the ones that used to vanish
 *
 * `groupByPhase` can only place an event under a phase it has. Anything else
 * would fall out of the render entirely — which is exactly what the home band
 * did until Phase B. The page renders these loudly instead, so the row can be
 * corrected by whoever can see it.
 */
export function unknownPhaseEvents(
  phases: TimelinePhase[],
  events: TimelineEvent[],
): TimelineEvent[] {
  const known = new Set(phases.map((phase) => phase.id));
  return events.filter((event) => !known.has(event.phase));
}

/**
 * How one event's date reads.
 *
 * `~` prefixes an approximate date, unchanged from the page this replaces. A
 * month- or year-precision date prints only the parts the source actually
 * stated — printing "1 March 2010" for "March 2010" would be a fabricated day,
 * which is the class of mistake the precision vocabulary exists to prevent.
 */
export function formatEventDate(
  isoDate: string,
  approximate: boolean,
  precision: string,
): string {
  const parsed = new Date(`${isoDate}T00:00:00`);
  if (Number.isNaN(parsed.getTime())) return isoDate;

  const options: Intl.DateTimeFormatOptions =
    precision === "year"
      ? { year: "numeric" }
      : precision === "month"
        ? { month: "short", year: "numeric" }
        : { month: "short", day: "numeric", year: "numeric" };

  const formatted = parsed.toLocaleDateString("en-US", options);
  return approximate ? `~ ${formatted}` : formatted;
}

/**
 * The same date, split into the two lines the subset window stacks.
 *
 * ## ⚑ Why this lives HERE and not beside the window that renders it
 *
 * The window draws the date over two lines — "Aug 18" above, "2008" below
 * (mockup v2 Screen 2) — and `formatEventDate` returns one string with no seam
 * to cut. The obvious move was a second formatter in `scenario-timeline/`. It
 * would have been a SECOND opinion about what a month-precision date says, in a
 * different directory, and the two would have drifted the first time somebody
 * changed a locale option in one of them. So the split lives against the
 * formatter it must agree with, sharing its parse, its fallback, and its `~`.
 *
 * `lead` is the big line, `year` the small one under it:
 *
 * | precision | approximate | lead      | year   |
 * |-----------|-------------|-----------|--------|
 * | `day`     | no          | `Aug 18`  | `2008` |
 * | `day`     | yes         | `~ May 3` | `2009` |
 * | `month`   | yes         | `~ Apr`   | `2009` |
 * | `year`    | yes         | `~ 2009`  | `""`   |
 *
 * A year-precision date returns an EMPTY `year` rather than repeating itself:
 * the year IS the lead, and "2009" stacked under "~ 2009" reads as two facts
 * where there is one. An unparseable date returns the raw string as the lead
 * and an empty year, the same degradation `formatEventDate` makes — a date the
 * browser cannot parse is still shown, never blanked.
 */
export type SplitEventDate = { lead: string; year: string };

export function splitEventDate(
  isoDate: string,
  approximate: boolean,
  precision: string,
): SplitEventDate {
  const parsed = new Date(`${isoDate}T00:00:00`);
  if (Number.isNaN(parsed.getTime())) return { lead: isoDate, year: "" };

  const tilde = (text: string): string => (approximate ? `~ ${text}` : text);
  const year = parsed.toLocaleDateString("en-US", { year: "numeric" });

  if (precision === "year") return { lead: tilde(year), year: "" };
  if (precision === "month") {
    return { lead: tilde(parsed.toLocaleDateString("en-US", { month: "short" })), year };
  }
  // Everything else is day precision — the same fall-through `formatEventDate`
  // makes, so an unrecognised precision string renders a full date in both
  // rather than one surface silently dropping the day.
  return {
    lead: tilde(parsed.toLocaleDateString("en-US", { month: "short", day: "numeric" })),
    year,
  };
}

/** The tag row for an id, or undefined when the vocabulary does not have it. */
export function tagOf(tags: TimelineTag[], id: string): TimelineTag | undefined {
  return tags.find((tag) => tag.id === id);
}

/**
 * The colour of an event's dot — its first known tag's, or a neutral fallback.
 *
 * A fallback and not a throw: an event tagged with something the vocabulary has
 * not caught up with is still an event, and it must still render.
 */
export function dotColor(tags: TimelineTag[], event: TimelineEvent, fallback: string): string {
  for (const id of event.tags) {
    const found = tagOf(tags, id);
    if (found) return found.color;
  }
  return fallback;
}

/** The note badge, or null when there are no notes to badge. */
export function noteBadge(count: number, wording: ChronologyWording): string | null {
  if (count <= 0) return null;
  if (count === 1) return cw(wording, "note_count_one");
  return fill(cw(wording, "note_count_template"), { count });
}

/** How one link renders: a live link, or a marked state that is not a link. */
export type LinkRendering =
  | { kind: "link"; label: string; pinpoint: string | null }
  | { kind: "missing"; label: string }
  | { kind: "unchecked"; label: string };

/**
 * One link's rendering, decided from its resolution.
 *
 * THREE states, never two. `missing` is an answer — looked for, not there;
 * `unchecked` is the absence of one. A dead link is never rendered as live and
 * never dropped from the list, which is the defect this whole redesign was
 * written after: ten of eleven links in the old JSON pointed at ids that did
 * not exist and every one of them rendered blue.
 */
export function linkRendering(
  link: TimelineLink,
  wording: ChronologyWording,
): LinkRendering {
  if (link.resolution === "resolves") {
    return {
      kind: "link",
      label: link.label ?? link.target_id,
      pinpoint: link.pinpoint ?? null,
    };
  }
  if (link.resolution === "missing") {
    return { kind: "missing", label: cw(wording, "no_document_label") };
  }
  return { kind: "unchecked", label: cw(wording, "link_unchecked_label") };
}

/** The subtitle under the page title — filtered or not. */
export function subtitleOf(
  data: CaseTimeline,
  filters: TimelineFilters,
  shown: number,
): string {
  if (filters.phase !== null) {
    const phase = data.phases.find((p) => p.id === filters.phase);
    return fill(cw(data.wording, "filtered_count_template"), {
      phase: phase?.label ?? filters.phase,
      shown,
      total: data.events.length,
    });
  }
  if (isFiltered(filters)) {
    return fill(cw(data.wording, "filtered_count_template"), {
      phase: cw(data.wording, "all_tags_label"),
      shown,
      total: data.events.length,
    });
  }
  return fill(cw(data.wording, "count_template"), {
    events: data.events.length,
    phases: data.phases.length,
  });
}
