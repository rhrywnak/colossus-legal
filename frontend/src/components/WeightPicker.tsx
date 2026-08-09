// =============================================================================
// WeightPicker — the labelled three-tier weight control (Piece 5a)
// =============================================================================
//
// ONE_CARD_GRAMMAR §5. The star that cycled carries/backup/background dies here,
// and the reason is on the record: Roman was moved to the background pile by a
// control he had signed off three days earlier. His ruling — "if it confused me,
// Marie has no chance."
//
// ## What was wrong with cycling, precisely
//
// 2.13c defended it on the grounds that the CURRENT tier was written out beside
// the glyph, so nothing was hidden. That is true and it is not the problem. A
// cycling control discloses where you ARE and never where a click will take you,
// so the only way to learn that ☆ → background makes the card leave the list is
// to make it leave the list. A picker shows all three before you choose one.
//
// ## Every word is stored
//
// The three tier NAMES are existing curation rows (they predate this task and are
// unchanged); the label beside them is a new card-grammar row. No literal here.

import React from "react";

import type { FactTier } from "../services/scenarioCards";
import type { CardGrammarWording, LinkPanelWording } from "../services/evidenceLinks";
import { META_SIZE } from "./factRowStyles";

/** The three weights, heaviest first — the order the picker lists them in. */
const TIERS: FactTier[] = ["carries", "backup", "background"];

const selectStyle: React.CSSProperties = {
  font: "inherit",
  fontSize: META_SIZE,
  padding: "3px 8px",
  border: "1px solid var(--border-default)",
  borderRadius: "7px",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
  fontFamily: "inherit",
};

/**
 * The stored name of each tier, as one lookup.
 *
 * Exported because the acknowledgment has to NAME the tier it just set, and a
 * second mapping there would be a second place the three names live — which is
 * how a renamed tier ends up renamed on the control and not in the sentence
 * announcing it.
 */
export function tierLabels(wording: LinkPanelWording): Record<FactTier, string> {
  return {
    carries: wording.fact_tier_carries_label,
    backup: wording.fact_tier_backup_label,
    background: wording.fact_tier_background_label,
  };
}

/**
 * "Weight: [Backup ▾]" — the three choices spelled out.
 *
 * ## Why a native `<select>` rather than a custom menu
 *
 * It is the Bias Analysis page's own pattern (the precedent Roman named twice),
 * it is keyboard- and screen-reader-correct without a line of code here, and on
 * a MacBook it opens as a system menu that cannot be clipped by the card's own
 * scroll region — which a hand-rolled dropdown inside a `maxHeight` container
 * would be.
 */
export const WeightPicker: React.FC<{
  current: FactTier;
  wording: LinkPanelWording;
  cardWording: CardGrammarWording;
  onSetTier: (tier: FactTier) => void;
}> = ({ current, wording, cardWording, onSetTier }) => {
  const labels = tierLabels(wording);

  return (
    <label
      style={{
        display: "flex",
        alignItems: "center",
        gap: "0.35rem",
        fontSize: META_SIZE,
        color: "var(--text-secondary)",
      }}
      // The control sits inside a draggable card; a click on it is a choice, not
      // the start of a drag and not a selection of the card behind it.
      onClick={(event) => event.stopPropagation()}
      onDragStart={(event) => event.preventDefault()}
    >
      {cardWording.weight_picker_label}
      <select
        value={current}
        style={selectStyle}
        title={wording.fact_tier_prompt}
        aria-label={wording.fact_tier_prompt}
        onChange={(event) => {
          // ## Rust parallel: narrowing at the parse boundary
          //
          // A `<select>` hands back `string`. The cast is safe HERE and only
          // here, because the options were generated from the closed `TIERS`
          // list four lines down — the boundary is one function wide, which is
          // the property that makes it checkable. Same argument as
          // `CandidateFilterBar.FacetSelect`.
          onSetTier(event.target.value as FactTier);
        }}
      >
        {TIERS.map((tier) => (
          <option key={tier} value={tier}>
            {labels[tier]}
          </option>
        ))}
      </select>
    </label>
  );
};

export default WeightPicker;
