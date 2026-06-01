// =============================================================================
// CountSelector.tsx — selectable strip of Counts for the Proof Matrix page (PM4)
// -----------------------------------------------------------------------------
// A thin presentational selector: the case's Counts as a horizontal strip of
// selectable chips. Each chip shows "COUNT {roman}", the Count name, and the
// deduped allegation total. Selecting a chip reports the Count number up via
// `onSelect`; the parent (ProofMatrixPage) owns `selectedCountNumber` state and
// swaps the displayed Count in place — no route navigation.
//
// Why a new component instead of reusing CountCard: CountCard is a `role="link"`
// dashboard card (it navigates on click, renders a serif description + a
// "N Elements" metrics line, and has no selected state). Forcing a `selected`
// affordance and selection semantics into it would complicate a frozen-PROD
// layout component shared with Home. This selector instead mirrors PM3's
// Element-list selection idiom (`role="tab"` + `aria-selected`) and reuses the
// shared `toRomanNumeral` helper + design tokens.
// =============================================================================

import React from "react";
import { CountDetail } from "../services/causesOfAction";
import { toRomanNumeral } from "./CountCard";

export interface CountSelectorProps {
  /** Counts to offer, already sorted by the parent (ascending count_number). */
  counts: CountDetail[];
  /** The currently-selected Count's number. */
  selectedCountNumber: number;
  /**
   * Deduped allegation totals keyed by count_number, from the proof-matrix
   * rollup. A count_number absent from the map (pending or failed rollup fetch)
   * renders a muted `—` — the same graceful-degrade Home's CountCard uses.
   */
  allegationTotals: Record<number, number>;
  /** Fired with the chosen Count's number on click or Enter/Space. */
  onSelect: (countNumber: number) => void;
}

/**
 * One selectable Count chip. Extracted from `CountSelector` so each function
 * stays within the 50-line limit (CLAUDE.md Rule 18) and so the per-chip
 * selection affordance reads as a single unit.
 *
 * `total` is the deduped allegation count for this Count, or `undefined` while
 * the rollup is pending/failed (→ muted `—`).
 */
const CountChip: React.FC<{
  count: CountDetail;
  selected: boolean;
  total: number | undefined;
  onSelect: (countNumber: number) => void;
}> = ({ count, selected, total, onSelect }) => (
  <div
    role="tab"
    tabIndex={0}
    aria-selected={selected}
    onClick={() => onSelect(count.count_number)}
    onKeyDown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onSelect(count.count_number);
      }
    }}
    style={{
      ...CHIP_STYLE,
      borderColor: selected ? "var(--accent-primary)" : "var(--border-default)",
      backgroundColor: selected ? "var(--accent-bg-soft)" : "var(--bg-surface)",
    }}
  >
    <div style={EYEBROW_STYLE}>COUNT {toRomanNumeral(count.count_number)}</div>
    {count.count_name && <div style={NAME_STYLE}>{count.count_name}</div>}
    <div style={TOTAL_STYLE}>
      {total != null ? (
        <span style={{ color: "var(--text-secondary)" }}>{total} allegations</span>
      ) : (
        <span style={{ color: "var(--text-muted)" }}>— allegations</span>
      )}
    </div>
  </div>
);

/**
 * Render the Counts as a selectable, keyboard-navigable strip.
 *
 * ## React/TS Learning: a presentational selector
 * It holds no state and fetches nothing — `selectedCountNumber` comes in and the
 * chosen number goes out via `onSelect`, so the single source of truth for "which
 * Count is selected" stays in the page. This keeps the selector reusable and
 * trivially reasoned about.
 */
const CountSelector: React.FC<CountSelectorProps> = ({
  counts,
  selectedCountNumber,
  allegationTotals,
  onSelect,
}) => (
  <div role="tablist" aria-label="Counts" style={STRIP_STYLE}>
    {counts.map((count) => (
      <CountChip
        key={count.count_number}
        count={count}
        selected={count.count_number === selectedCountNumber}
        total={allegationTotals[count.count_number]}
        onSelect={onSelect}
      />
    ))}
  </div>
);

// ─── Styles (tokens only; no hardcoded colors) ──────────────────────────────

const STRIP_STYLE: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: "12px",
};

const CHIP_STYLE: React.CSSProperties = {
  flex: "1 1 200px",
  minWidth: "180px",
  border: "1px solid var(--border-default)",
  borderRadius: "10px",
  padding: "12px 14px",
  cursor: "pointer",
  transition: "border-color 0.15s ease, background-color 0.15s ease",
};

const EYEBROW_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "0.68rem",
  fontWeight: 700,
  color: "var(--accent-primary)",
  textTransform: "uppercase",
  letterSpacing: "0.05em",
  marginBottom: "0.2rem",
};

const NAME_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "0.92rem",
  fontWeight: 600,
  color: "var(--text-primary)",
  lineHeight: 1.3,
  marginBottom: "0.3rem",
};

const TOTAL_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "0.78rem",
  color: "var(--text-secondary)",
};

export default CountSelector;
