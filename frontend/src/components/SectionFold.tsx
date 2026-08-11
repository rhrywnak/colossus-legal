// =============================================================================
// SectionFold — the ▸/▾ control on a section heading (task R4, P1b)
// =============================================================================
//
// A section's own collapse arrow, sitting at the end of its header row.
//
// ## Why this is a component and not thirty lines inside the facts section
//
// The queue already has a fold, in `ScanSection`, built against the queue's own
// region model — and it is the ONLY one on the page. The moment a second section
// grew one by hand, the page would have two arrows that looked alike, behaved
// alike, and shared no code, which is how they end up disagreeing about what a
// closed section shows. One control, one behaviour, one place to change it.
//
// ## What a folded section still says
//
// The heading and its count line stay on screen; only the body goes. That is the
// whole point of folding rather than hiding: a reader who has closed the facts
// list can still see how many facts are in it, so a collapsed section can never
// be mistaken for an empty one.
//
// ## Why the state is NOT remembered between visits
//
// Ruling R7 declined to persist the queue's collapsed state, for a reason that
// applies here without change: a section that remembers "closed" greets the next
// arrival with its work hidden and nothing on screen explaining why. This holds
// the state for as long as the page is open and no longer, so a reload is always
// the honest full view.
//
// The caller owns the state rather than this component, because the caller is
// what decides which of its children the fold governs.

import React from "react";

/**
 * Mockup parity with the queue's chevron (`ScanSection`), minus the
 * `marginLeft: auto` — a section header can carry controls to the fold's right,
 * so the caller decides the spacing rather than this button claiming the gap.
 */
const buttonStyle: React.CSSProperties = {
  width: "30px",
  height: "30px",
  borderRadius: "8px",
  background: "var(--v3-chrome)",
  border: "none",
  cursor: "pointer",
  color: "var(--text-secondary)",
  fontSize: "13px",
  fontFamily: "inherit",
  flexShrink: 0,
};

interface Props {
  /** Whether the body this fold governs is currently shown. */
  open: boolean;
  onToggle: () => void;
  /**
   * What this fold opens and closes, in words — "the scenario facts".
   *
   * Read into the accessible label and the tooltip, so the control names its
   * target rather than being a bare arrow. A screen reader on a page with two
   * folds would otherwise hear "collapse" twice with nothing to tell them apart.
   */
  names: string;
}

const SectionFold: React.FC<Props> = ({ open, onToggle, names }) => {
  const label = open ? `Collapse ${names}` : `Expand ${names}`;
  return (
    <button
      type="button"
      onClick={onToggle}
      aria-expanded={open}
      aria-label={label}
      title={label}
      style={buttonStyle}
    >
      {open ? "▾" : "▸"}
    </button>
  );
};

export default SectionFold;
