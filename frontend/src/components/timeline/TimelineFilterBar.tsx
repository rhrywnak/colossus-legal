// =============================================================================
// TimelineFilterBar.tsx — search, the tag chips, and the date range
// =============================================================================
//
// Mockup v2 Screen 1's `.filters`. THE CHIPS ARE THE STORED VOCABULARY: they
// are drawn from `chronology_tags` (ruling R-F), so a sixth tag is a row and not
// a build, and each chip wears that tag's own colour.
//
// Ruled out of this phase and therefore ABSENT rather than dead: the People
// filter and the "Spine only" toggle. Neither has data behind it yet, and the
// honest-gap law says a control that cannot work is not drawn.

import React from "react";

import type {
  ChronologyWording,
  TimelinePhase,
  TimelineTag,
} from "../../services/caseTimeline";
import { cw } from "../../services/caseTimeline";
import type { TimelineFilters } from "./timelineFilters";
import * as s from "./timelineStyles";

type Props = {
  tags: TimelineTag[];
  /** The phases, so an active phase filter can wear its own label and colour. */
  phases: TimelinePhase[];
  wording: ChronologyWording;
  filters: TimelineFilters;
  onChange: (next: TimelineFilters) => void;
};

const TimelineFilterBar: React.FC<Props> = ({ tags, phases, wording, filters, onChange }) => {
  const activePhase = phases.find((phase) => phase.id === filters.phase);
  return (
  <div style={s.filters}>
    <label style={s.search}>
      <span aria-hidden="true">🔍</span>
      <input
        style={s.searchInput}
        placeholder={cw(wording, "search_placeholder")}
        aria-label={cw(wording, "search_placeholder")}
        value={filters.search}
        onChange={(e) => onChange({ ...filters, search: e.target.value })}
      />
    </label>

    {/* The active phase, as a chip that can be dismissed. Mockup Screen 1b
        offers TWO ways out of an expanded phase — this ✕ and the header's
        "Show all phases" — and they are deliberately the same action, because a
        reader who reached the state from the header may look for the way out in
        the filter bar. */}
    {activePhase && (
      <button
        type="button"
        style={s.chip(activePhase.color, true)}
        onClick={() => onChange({ ...filters, phase: null })}
      >
        {activePhase.label} ✕
      </button>
    )}

    <button
      type="button"
      style={s.allChip(filters.tag === null)}
      aria-pressed={filters.tag === null}
      onClick={() => onChange({ ...filters, tag: null })}
    >
      {cw(wording, "all_tags_label")}
    </button>
    {tags.map((tag) => (
      <button
        key={tag.id}
        type="button"
        style={s.chip(tag.color, filters.tag === tag.id)}
        aria-pressed={filters.tag === tag.id}
        // Pressing the active chip clears it — the same affordance as the ✕ on
        // the phase chip, so "turn this off" is one gesture everywhere.
        onClick={() => onChange({ ...filters, tag: filters.tag === tag.id ? null : tag.id })}
      >
        {tag.label}
      </button>
    ))}

    <span style={s.dateControl}>
      <b>{cw(wording, "dates_label")}</b>
      <label>
        <span style={{ marginRight: "0.25rem" }}>{cw(wording, "date_from_label")}</span>
        <input
          type="date"
          style={s.dateInput}
          aria-label={cw(wording, "date_from_label")}
          value={filters.from}
          onChange={(e) => onChange({ ...filters, from: e.target.value })}
        />
      </label>
      <label>
        <span style={{ marginRight: "0.25rem" }}>{cw(wording, "date_to_label")}</span>
        <input
          type="date"
          style={s.dateInput}
          aria-label={cw(wording, "date_to_label")}
          value={filters.to}
          onChange={(e) => onChange({ ...filters, to: e.target.value })}
        />
      </label>
    </span>
    </div>
  );
};

export default TimelineFilterBar;
