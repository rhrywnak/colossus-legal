// =============================================================================
// FactsFilterBar.tsx — Included / Candidates / All (v2.1, change D)
// =============================================================================
//
// The pill row at the top of the Scenario facts card. Three buttons, in the
// Documents-page style the mockup names.
//
// ## ⚑ These three words are LITERALS, and here is the accounting
//
// The scenario page's own vocabulary is a settings row wherever a row exists —
// but this control is NEW and there are no rows for it. The nearest existing
// ones belong to the candidate queue's five-facet bar
// (`card_filter_included_label` = "Included", `card_filter_proposed_label` =
// "Proposed", `card_filter_full_pool_label` = "Full pool") and they are NOT the
// same vocabulary: this bar's "All" is included + candidates, while the queue's
// "Full pool" is every card in the pool whatever its ruling. Wiring one row to
// two controls that mean different things is worse than a literal, because the
// day somebody edits the row for one control it changes the other.
//
// So the three words are constants here, matching the mockup exactly, and three
// settings rows are OWED — a migration is Roman's to author (CLAUDE.md rule 25)
// and this task did not authorise one. It is the same accounting the section
// around this one already makes: `ScenarioFactsSection` has carried the literal
// "Scenario facts" heading and its empty-state sentence since 1.7C.
//
// ## The counts, and the one state that is not a number
//
// `null` means the pool has not been read yet, and the pill then shows its word
// with NO number — not `0`. Collapsing those two is what put "No candidates
// gathered yet" over a pool of 148 in .374; see `factsFilter`.

import React from "react";

import { FACTS_FILTERS, type FactsCounts, type FactsFilter } from "./factsFilter";

// ⚑ CONST: the three pill words, and all three are DEBT. See the module header
// for why no settings rows exist yet. Transcribed from MOCKUP_S7_v2.1_3. Owed
// rows, named the way the architecture review suggested:
//   facts_filter_included_label   = "Included"
//   facts_filter_candidates_label = "Candidates"
//   facts_filter_all_label        = "All"
const FILTER_LABELS: Record<FactsFilter, string> = {
  included: "Included",
  candidates: "Candidates",
  all: "All",
};

// STRUCTURAL: the separator between a pill's word and its count. Punctuation,
// not vocabulary — the same interpunct every other row on this page joins with.
const COUNT_SEPARATOR = "·";

/** Mockup `.chip-row`: the pills sit in their own band at the top of the card. */
const rowStyle: React.CSSProperties = {
  display: "flex",
  gap: "0.4rem",
  alignItems: "center",
  flexWrap: "wrap",
  padding: "14px 18px",
};

/**
 * The Documents-page chip, transcribed.
 *
 * ## Why the shape is copied rather than imported
 *
 * `DocumentsPage.chipBase` is a page-local constant in a file this task has no
 * business editing (change G: nothing else moves). Lifting it into a shared
 * module would touch that page's rendering to serve this one, which is the kind
 * of drive-by that turns a scoped change into a regression somewhere nobody
 * looked. The duplication is REPORTED rather than hidden: when a third caller
 * wants this pill, that is the moment it earns a shared home.
 *
 * ## ⚑ The border is LONGHAND here, and that is not a style preference
 *
 * The Documents page writes `border: "1px solid …"` and overrides `borderColor`
 * on its active chip. That is safe THERE because the two objects are never
 * swapped on one element. Here they are — a pill flips between them on every
 * click — and React warns, correctly: mixing shorthand and non-shorthand
 * properties for one value "can lead to styling bugs". What actually happens is
 * that React removes `borderColor` and re-applies the shorthand in an order it
 * does not guarantee, so the active pill can paint with the inactive border.
 * Caught in the browser on 2026-09-07; the longhand triple has no such ordering.
 */
const pillStyle: React.CSSProperties = {
  padding: "0.25rem 0.7rem",
  fontSize: "0.76rem",
  fontWeight: 600,
  borderWidth: "1px",
  borderStyle: "solid",
  borderColor: "var(--border-default)",
  borderRadius: "999px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-muted)",
  cursor: "pointer",
  fontFamily: "inherit",
};

const activePillStyle: React.CSSProperties = {
  ...pillStyle,
  backgroundColor: "var(--accent-primary)",
  color: "var(--bg-surface)",
  borderColor: "var(--accent-primary)",
};

interface Props {
  active: FactsFilter;
  counts: FactsCounts;
  onPick: (filter: FactsFilter) => void;
}

/**
 * The three-way filter over the one facts list.
 *
 * `aria-pressed` rather than a radio group: these are toggles over one list, and
 * a screen reader announcing "radio button, 1 of 3" would describe a form. The
 * pressed state is also carried by the fill colour, so the two agree — colour
 * never stands alone on this page (the v3 rule).
 */
const FactsFilterBar: React.FC<Props> = ({ active, counts, onPick }) => (
  <div style={rowStyle}>
    {FACTS_FILTERS.map((filter) => {
      const count = counts[filter];
      return (
        <button
          key={filter}
          type="button"
          aria-pressed={filter === active}
          style={filter === active ? activePillStyle : pillStyle}
          onClick={() => onPick(filter)}
        >
          {FILTER_LABELS[filter]}
          {/* No number until the pool has been read. A pill claiming `0` before
              its fetch lands is the .374 defect in miniature. */}
          {count !== null && ` ${COUNT_SEPARATOR} ${count}`}
        </button>
      );
    })}
  </div>
);

export default FactsFilterBar;
