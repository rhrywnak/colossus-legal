// =============================================================================
// CandidateFilterBar — the candidate list's filter row (task 1.7G, Fix 2)
// =============================================================================
//
// Two labelled dropdowns, a Clear filters link, and one honest counter line:
//
//     Status: [Rulable now (42) ▾]   Scan: [Any (148) ▾]   Clear filters
//     Filtered: 42 of 148 candidates
//
// ## Why this replaced the pills, in Roman's words
//
// 1.7E's ruling R1 kept the count-pills and declined the `<select>` controls,
// reasoning that a count inside a closed dropdown is a count nobody can see.
// Roman overruled it (SCENARIO_QUEUE_FIX_DESIGN_v1, signed 2026-08-03): he had
// named the Bias Analysis and document-review pages' filters TWICE as the pattern
// to reuse, and got "two anonymous rows of count-pills — a pattern I never
// approved". His ruling R3 answers R1's objection rather than ignoring it: the
// OPTIONS carry their counts, the same way the Bias Analysis page's speaker and
// subject options do, so the numbers survive the move into the dropdown.
//
// ## What is reused, concretely
//
//   * The control STYLE — uppercase micro-label above the control, bordered
//     select, bordered accent-text clear button — from `BiasExplorerFilters`.
//   * The §9 counter rule — `shared/filteredCounter` via `candidateCounterLine`,
//     which says "Filtered:" only when a filter is actually active (ruling R4:
//     the shared helper's wording, not a third variant invented here).
//   * The single counts derivation — `candidateCounts`, computed by the queue and
//     passed in, so no two numbers on this bar can disagree.
//
// ## Every string here names a control, not a case
//
// The labels and hints come from `candidateFilters`, which owns them for the
// reason set out in its header: they are the list's own vocabulary, the same class
// of word as the Include button. No case vocabulary is composed on this surface.

import React from "react";

import type {
  CandidateCounts,
  CandidateFilters,
  FilterOption,
  ScannedFacet,
  StateFacet,
} from "./candidateFilters";
import {
  candidateCounterLine,
  scannedOptions,
  stateOptions,
  UNFILTERED,
} from "./candidateFilters";

/** The bar sits on the card surface, divided from the queue meta above it. */
const barStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "8px",
  padding: "12px 0 14px",
};

/** The controls sit on one line and wrap as a unit on a narrow window. */
const rowStyle: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: "1rem",
  alignItems: "flex-end",
};

/** One labelled control. Bias Analysis `fieldStyle`, narrowed: these two lists
 *  are short words rather than the Bias page's person names. */
const fieldStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  minWidth: "180px",
};

/** Bias Analysis `labelStyle`: the small uppercase word naming the facet. */
const labelStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  fontWeight: 700,
  color: "var(--text-disabled)",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
  marginBottom: "0.3rem",
};

/** Bias Analysis `selectStyle`, verbatim. */
const selectStyle: React.CSSProperties = {
  padding: "0.45rem 0.6rem",
  fontSize: "0.88rem",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-primary)",
  fontFamily: "inherit",
};

/** Bias Analysis `clearAllBtnStyle`, verbatim. */
const clearBtnStyle: React.CSSProperties = {
  padding: "0.5rem 0.9rem",
  fontSize: "0.82rem",
  fontWeight: 500,
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--accent-primary)",
  cursor: "pointer",
  fontFamily: "inherit",
  flexShrink: 0,
};

const counterStyle: React.CSSProperties = {
  fontSize: "0.85rem",
  color: "var(--text-secondary)",
  fontWeight: 500,
};

/**
 * One labelled dropdown.
 *
 * ## TS learning: a generic component over the facet type
 *
 * `<F,>` (the trailing comma is what tells the .tsx parser this is a type
 * parameter and not a JSX tag) lets the same control render the Status facets and
 * the Scan facets while keeping `onPick` typed to the facet it actually hands
 * back. The alternative — `facet: string` — would compile happily and let a scan
 * facet be passed to the status setter.
 *
 * ## Rust parallel: the closed enum at the parse boundary
 *
 * A `<select>` hands back `string`, which is the same problem the backend solves
 * with a `#[serde(rename_all)]` enum: the value has to be narrowed back to the
 * closed set before it can mean anything. The cast is safe HERE and only here,
 * because the options were generated from that same closed set two lines up — the
 * boundary is one function wide, which is the property that makes it checkable.
 */
function FacetSelect<F extends string>({
  id,
  label,
  options,
  active,
  onPick,
}: {
  id: string;
  label: string;
  options: FilterOption<F>[];
  active: F;
  onPick: (facet: F) => void;
}) {
  return (
    <div style={fieldStyle}>
      <label htmlFor={id} style={labelStyle}>
        {label}
      </label>
      <select
        id={id}
        style={selectStyle}
        value={active}
        onChange={(event) => onPick(event.target.value as F)}
        // The hint of the ACTIVE option, so hovering the closed control explains
        // what it is currently showing — the pills' tooltips said this per pill,
        // and the sentence is worth more than the hover target it lost.
        title={options.find((option) => option.facet === active)?.hint}
      >
        {options.map((option) => (
          <option key={option.facet} value={option.facet} title={option.hint}>
            {option.label} ({option.count})
          </option>
        ))}
      </select>
    </div>
  );
}

/**
 * The filter row: the two selects, Clear filters, and the honest counter line.
 *
 * `counts` arrives already derived (one pass, one source) rather than being
 * computed here — a bar that counted its own options would be the second
 * derivation ruling R1 exists to prevent, and that half of R1 survives 1.7G
 * untouched.
 */
const CandidateFilterBar: React.FC<{
  counts: CandidateCounts;
  filters: CandidateFilters;
  shown: number;
  onChange: (next: CandidateFilters) => void;
}> = ({ counts, filters, shown, onChange }) => (
  <div style={barStyle}>
    <div style={rowStyle}>
      <FacetSelect<StateFacet>
        id="candidate-filter-status"
        label="Status"
        options={stateOptions(counts)}
        active={filters.state}
        onPick={(state) => onChange({ ...filters, state })}
      />
      <FacetSelect<ScannedFacet>
        id="candidate-filter-scan"
        label="Scan"
        options={scannedOptions(counts)}
        active={filters.scanned}
        onPick={(scanned) => onChange({ ...filters, scanned })}
      />
      {/* Always visible, like the Bias Analysis page's — a control that appears
          only once a filter is active is a control you have to discover twice.
          It resets to All / Any, which is the honest denominator's view, NOT the
          "Rulable now" the page opens on: clearing a filter means showing
          everything, not returning to a default that hides 106 candidates. */}
      <button type="button" style={clearBtnStyle} onClick={() => onChange(UNFILTERED)}>
        Clear filters
      </button>
    </div>
    {/* The §9 line: what is on screen, out of what exists, worded by intent. */}
    <div style={counterStyle}>{candidateCounterLine(shown, counts.all, filters)}</div>
  </div>
);

export default CandidateFilterBar;
