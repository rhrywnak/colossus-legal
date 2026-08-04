// =============================================================================
// CandidateFilterBar — the candidate list's filter pills (task 1.7E, ruling R1)
// =============================================================================
//
// Roman's instruction was to REUSE the filter component from the Bias Analysis
// and document-review pages. Ruling R1 honoured that where reuse has value and
// declined it where it would defeat the task:
//
//   * REUSED — the §9 honesty rule behind the Bias Explorer's counter line, now
//     lifted into `shared/filteredCounter` and consumed by both bars.
//   * REUSED — the `DocumentsPage` single-`useMemo` counts-derivation shape, so
//     no two facet counts can disagree (`candidateFilters.candidateCounts`).
//   * DECLINED — the `<select>` dropdowns. A count inside a closed dropdown is a
//     count nobody can see, and this task exists because Roman cannot see where
//     the rulable candidates are. "Rulable now (24)" has to be visible before any
//     click and reachable in ONE.
//
// So the control is v3 pills. Every pill wears its count; the active one is
// filled; the rest are chrome.
//
// ## Every string here names a control, not a case
//
// The labels and the hints come from `candidateFilters`, which owns them for the
// reason set out in its header: they are the list's own vocabulary, the same class
// of word as the Include button. No case vocabulary is composed anywhere on this
// surface.

import React from "react";

import type {
  CandidateCounts,
  CandidateFilters,
  FilterPill,
  ScannedFacet,
  StateFacet,
} from "./candidateFilters";
import { candidateCounterLine, scannedPills, statePills } from "./candidateFilters";

/** The bar sits on the card surface, divided from the queue meta above it. */
const barStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "8px",
  padding: "12px 0 14px",
};

const rowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "8px",
  flexWrap: "wrap",
};

/** Mockup `.seg` label: the small uppercase word that names a facet. */
const facetLabelStyle: React.CSSProperties = {
  fontSize: "11px",
  fontWeight: 700,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  marginRight: "2px",
};

/** An inactive pill: chrome fill, secondary text, radius 999 (v3 `.chip`). */
const pillStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "12.5px",
  fontWeight: 500,
  padding: "5px 13px",
  borderRadius: "999px",
  border: "none",
  cursor: "pointer",
  background: "var(--v3-chrome)",
  color: "var(--text-secondary)",
  whiteSpace: "nowrap",
};

/** The ACTIVE pill: the one accent, filled, with `--v3-on-fill` text. */
const activePillStyle: React.CSSProperties = {
  ...pillStyle,
  background: "var(--accent-primary)",
  color: "var(--v3-on-fill)",
  fontWeight: 600,
};

const counterStyle: React.CSSProperties = {
  fontSize: "12.5px",
  color: "var(--text-secondary)",
};

/**
 * One row of pills.
 *
 * ## TS learning: a generic component over the facet type
 *
 * `<F,>` (the trailing comma is what tells the .tsx parser this is a type
 * parameter and not a JSX tag) lets the same row render the state facets and the
 * scanned facets while keeping `onPick` typed to the facet it actually hands
 * back. The alternative — `facet: string` — would compile happily and let a
 * scanned facet be passed to the state setter.
 */
function PillRow<F extends string>({
  label,
  pills,
  active,
  onPick,
}: {
  label: string;
  pills: FilterPill<F>[];
  active: F;
  onPick: (facet: F) => void;
}) {
  return (
    <div style={rowStyle}>
      <span style={facetLabelStyle}>{label}</span>
      {pills.map((pill) => (
        <button
          key={pill.facet}
          type="button"
          onClick={() => onPick(pill.facet)}
          style={pill.facet === active ? activePillStyle : pillStyle}
          // The hint says what the facet MEANS. A pill reading "Rulable now (24)"
          // beside "Not ruled (100)" otherwise invites the reader to add them up,
          // and one is a subset of the other (§9).
          title={pill.hint}
          aria-pressed={pill.facet === active}
        >
          {pill.label} ({pill.count})
        </button>
      ))}
    </div>
  );
}

/**
 * The filter bar: state pills, scan-history pills, and the honest counter line.
 *
 * `counts` arrives already derived (one pass, one source) rather than being
 * computed here — a bar that counted its own pills would be the second derivation
 * ruling R1 exists to prevent.
 */
const CandidateFilterBar: React.FC<{
  counts: CandidateCounts;
  filters: CandidateFilters;
  shown: number;
  onChange: (next: CandidateFilters) => void;
}> = ({ counts, filters, shown, onChange }) => (
  <div style={barStyle}>
    <PillRow<StateFacet>
      label="Show"
      pills={statePills(counts)}
      active={filters.state}
      onPick={(state) => onChange({ ...filters, state })}
    />
    <PillRow<ScannedFacet>
      label="Scan"
      pills={scannedPills(counts)}
      active={filters.scanned}
      onPick={(scanned) => onChange({ ...filters, scanned })}
    />
    {/* The §9 line: what is on screen, out of what exists, worded by intent. */}
    <div style={counterStyle}>{candidateCounterLine(shown, counts.all, filters)}</div>
  </div>
);

export default CandidateFilterBar;
