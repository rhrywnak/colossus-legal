// =============================================================================
// CandidateFilterBar — five chips and one honest progress line (Piece 1)
// =============================================================================
//
//     [Proposed (8)] [Deferred (2)] [Included (19)] [Excluded (1)] [Full pool (148)] ⓘ
//     2 of 8 Proposed ruled
//
// ## What died here, and why (ONE_CARD_GRAMMAR §4, ruling R5)
//
// * **The Status dropdown.** 1.7G replaced two rows of count-pills with a
//   labelled `<select>`, and that was right about the pills and wrong about the
//   click: five facets behind a closed control is five states nobody can compare
//   without opening it. The chips carry their counts on the face, which is what
//   the select's own defence claimed as its win.
// * **"Rulable now" and "All".** Two of six options were subsets of a third, and
//   the bar made a human read a taxonomy before a count. A locked card now rides
//   inside Proposed, stating its own condition with the type-ahead ready
//   (Piece 4b), rather than being sorted out of it by a facet.
// * **"Clear filters".** With five chips and one always active, "clear" means
//   "press Full pool" — and a control whose only job is to press another control
//   is a click tax.
// * **"Filtered: 30 of 148 candidates".** Replaced by the progress line, which
//   answers the question a curator actually has. See below.
//
// ## The progress line tracks the FILTER, never the pool (Piece 1c)
//
// It read "23 of 148 ruled" over a progress bar of 125 nobody owes.
// Rule-the-promising is the ratified triage model — a curator works the
// proposals and leaves the rest — so a bar measuring the whole gathered pool was
// reporting a debt the method says does not exist.
//
// ## Every string here is a stored word
//
// The five names and the ⓘ text are card-grammar rows (ruling R6). The bar is
// not rendered until they load: there is no fallback vocabulary to render it
// with, and a filter row with invented names would be worse than none.

import React, { useEffect, useRef, useState } from "react";

import type { CandidateCounts, CandidateFilters, StateFacet } from "./candidateFilters";
import { filterChips } from "./candidateFilters";
import { fillSlots, type CardGrammarWording } from "../services/evidenceLinks";

const barStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "8px",
  padding: "12px 0 14px",
};

const rowStyle: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: "8px",
  alignItems: "center",
};

/** §2c: hairline chips on a white surface, one accent for the active one. */
const chipStyle = (active: boolean): React.CSSProperties => ({
  fontFamily: "inherit",
  fontSize: "12.5px",
  border: `1px solid ${active ? "var(--accent-primary)" : "var(--border-default)"}`,
  borderRadius: "999px",
  padding: "5px 13px",
  background: active ? "var(--state-info-bg-soft)" : "var(--bg-surface)",
  color: active ? "var(--accent-primary)" : "var(--text-secondary)",
  cursor: "pointer",
  fontWeight: active ? 600 : 400,
});

const infoStyle: React.CSSProperties = {
  width: "16px",
  height: "16px",
  lineHeight: "15px",
  textAlign: "center",
  fontSize: "10.5px",
  border: "1px solid var(--border-default)",
  borderRadius: "50%",
  color: "var(--text-muted)",
  cursor: "pointer",
  background: "var(--bg-surface)",
  fontFamily: "inherit",
  padding: 0,
  flexShrink: 0,
};

const popStyle: React.CSSProperties = {
  position: "absolute",
  right: 0,
  top: "24px",
  width: "280px",
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "10px",
  boxShadow: "var(--shadow-raised)",
  padding: "10px 12px",
  fontSize: "12.5px",
  color: "var(--text-secondary)",
  lineHeight: 1.5,
  zIndex: 60,
  textAlign: "left",
};

const progressStyle: React.CSSProperties = {
  fontSize: "0.85rem",
  color: "var(--text-secondary)",
  marginLeft: "auto",
  whiteSpace: "nowrap",
};

/**
 * The chip row and the active filter's progress.
 *
 * `counts` arrives already derived (one pass, one source) rather than being
 * computed here — a bar that counted its own chips would be the second
 * derivation ruling R1 exists to prevent, and that half of R1 survives every
 * redesign of the control.
 *
 * `progress` is likewise computed by the queue over the VISIBLE list, because
 * the queue is what owns the filtering; a bar that re-derived it would be a
 * second answer to "how much of this have I done".
 */
const CandidateFilterBar: React.FC<{
  counts: CandidateCounts;
  filters: CandidateFilters;
  /** Ruled and total, over the ACTIVE filter's cards (Piece 1c). */
  progress: { ruled: number; total: number };
  wording: CardGrammarWording;
  onChange: (next: CandidateFilters) => void;
}> = ({ counts, filters, progress, wording, onChange }) => {
  const [explaining, setExplaining] = useState(false);
  const explainerRef = useRef<HTMLSpanElement | null>(null);

  /**
   * A way OUT of the ⓘ popup (filed 2026-08-09: "it would not close").
   *
   * The ⓘ itself always toggled — what the popup had was no other exit. It is
   * `position: absolute` with a z-index, so it sat over whatever rendered below,
   * which on this surface is the ruling acknowledgment; a human who clicked
   * elsewhere expecting it to go away watched it stay on top of the thing they
   * had just done.
   *
   * Both exits, because they are the two a reader reaches for: click away, or
   * press Escape. `mousedown` rather than `click` so the popup is gone before the
   * click lands on whatever is underneath it — otherwise dismissing it would also
   * press the control it was covering.
   */
  useEffect(() => {
    if (!explaining) return;
    const onAway = (event: MouseEvent) => {
      if (!explainerRef.current?.contains(event.target as Node)) setExplaining(false);
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setExplaining(false);
    };
    document.addEventListener("mousedown", onAway);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onAway);
      document.removeEventListener("keydown", onKey);
    };
  }, [explaining]);
  const chips = filterChips(counts, wording);

  return (
    <div style={barStyle}>
      <div style={rowStyle}>
        {chips.map((chip) => (
          <React.Fragment key={chip.facet}>
            <button
              type="button"
              style={chipStyle(chip.facet === filters.state)}
              aria-pressed={chip.facet === filters.state}
              onClick={() => onChange({ state: chip.facet as StateFacet })}
            >
              {chip.label} ({chip.count})
            </button>
            {/* The ⓘ rides beside the ONE chip whose meaning a reader cannot
                infer from its name. Roman's addition — Marie and Chuck will not
                know the term "full pool", and a filter nobody understands is a
                filter nobody presses. */}
            {chip.explainer && (
              <span
                ref={explainerRef}
                style={{ position: "relative", display: "inline-flex" }}
              >
                <button
                  type="button"
                  style={infoStyle}
                  aria-label={chip.explainer}
                  aria-expanded={explaining}
                  onClick={() => setExplaining(!explaining)}
                >
                  i
                </button>
                {explaining && <span style={popStyle}>{chip.explainer}</span>}
              </span>
            )}
          </React.Fragment>
        ))}

        {/* THE PROGRESS COUNT, on the chip row (Roman's cleanup ruling,
            2026-08-10). It was a line of its own under the chips; five lines of
            frame sat above a queue people came here to work.

            It no longer names a filter, because it no longer measures one: the
            denominator is every candidate the scans put forward, and it does not
            move when a chip is clicked. `marginLeft: auto` puts it at the far
            end of the row, away from the controls, so it reads as a status rather
            than as a sixth chip. */}
        <span style={progressStyle}>
          {fillSlots(wording.filter_progress_template, {
            ruled: String(progress.ruled),
            total: String(progress.total),
          })}
        </span>
      </div>

    </div>
  );
};

export default CandidateFilterBar;
