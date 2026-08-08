// =============================================================================
// CandidateList — the scrollable card list (task 1.7E-a, items 1 and 3)
// =============================================================================
//
// The queue was one card at a time: no skip forward, no skip back, no way to see
// where the rulable candidates sat in a 148-candidate pool. This is the list that
// replaces it — every card in the current filter, in a scrolling region, with one
// of them selected.
//
// ## No windowing, and the measurement that settled it (ruling R3)
//
// Measured on the real pool before any of this was built: 148 full cards = 4046
// DOM nodes, 39.3 ms worst-case full relayout, 0.8 ms per scroll frame. Well
// inside the ~100 ms interaction budget, so the list is plain — no virtualization,
// no page size, and therefore no settings row for one. R3's condition was evidence
// first; this is the evidence, and it says the simplest thing works.
//
// ## Scrolling follows selection; selection does not follow scrolling
//
// Item 3, and the two halves are not symmetrical on purpose. Moving the selection
// with ↑/↓ or j/k must bring the card into view, or the keyboard would be aimed
// off-screen. Scrolling with the wheel must NOT move the selection, because
// browsing to check something two cards down is not a decision to rule that card.

import React, { useEffect, useRef } from "react";

import { CandidateCard } from "./CandidateCard";
import { candidateState } from "./candidateFilters";
import type { RulingKey } from "./cardTriage";
import type { ScenarioCard } from "../services/scenarioCards";
import type { AllegationOptions, LinkCut } from "../services/evidenceLinks";

/**
 * The scroll region.
 *
 * ## Why a viewport-relative height and why it is not a setting
 *
 * The region has to be tall enough to read a card in and short enough that the
 * page's other sections stay reachable — that is layout geometry, the same class
 * of value as the mockup's paddings and radii which live in style objects
 * throughout this feature. Rule 13 governs a PAGE SIZE (how many records a human
 * is served at a time); this serves all of them and only chooses how much glass
 * they are seen through. Ruling R3 settled that there is no page size here.
 */
const scrollRegionStyle: React.CSSProperties = {
  maxHeight: "70vh",
  overflowY: "auto",
  // The scrollbar is deliberately not hidden: it is the only thing that tells a
  // human there are 120 more cards below the fold.
  display: "flex",
  flexDirection: "column",
  gap: "0.75rem",
  padding: "4px 2px",
};

/**
 * The card list.
 *
 * `cards` is ALREADY filtered — the filtering is pure and lives in
 * `candidateFilters`, so this component neither knows nor asks which facet is
 * active. It renders what it is given, in the order it is given.
 */
const CandidateList: React.FC<{
  cards: ScenarioCard[];
  /** The selected card's node id, or `null` when the filter leaves nothing. */
  selectedId: string | null;
  /** The reducer's refusal message, shown on the selected card. */
  notice: string | null;
  /**
   * Whether any filter is narrowing the list.
   *
   * Only used for the empty state, and load-bearing there: "no candidates match
   * this filter" over an empty POOL sends a human hunting for the filter that is
   * hiding their work, and there isn't one. Two states, two sentences.
   */
  filtered: boolean;
  onSelect: (graphNodeId: string) => void;
  /**
   * Rule one NAMED card (1.7G, ruling R1).
   *
   * The id is the second argument rather than something the queue infers, and
   * every card below binds it from its own render scope. That is the whole fix:
   * the target travels with the button, so no shared position can decide it.
   */
  onRule: (key: RulingKey, graphNodeId: string) => void;
  /** Save a correction of one NAMED card's question (task 1.7F Part B). Bound
   *  per card below, on the same principle as `onRule`: the target travels with
   *  the control, never through a shared position. */
  onCorrectQuestion: (graphNodeId: string, text: string) => Promise<void>;
  /** Restore the machine's question on one named card. */
  onRevertQuestion: (graphNodeId: string) => Promise<void>;
  /** The accusations the link panels offer, and their words (task 2.10). `null`
   *  while loading or after a failed read — no panel is rendered then. */
  linkOptions: AllegationOptions | null;
  /** Save one NAMED card's links (task 2.10, ruling R1). Bound per card below,
   *  on the same principle as `onRule`: the target travels with the control, and
   *  no shared position can decide it. */
  onSaveLinks: (graphNodeId: string, allegationIds: string[], cut: LinkCut) => Promise<void>;
  /** Take one link back, on one named card. */
  onUnlink: (graphNodeId: string, allegationId: string) => void;
  /**
   * "Proposed by the Aug 7 scan", or `null` when nothing is being proposed
   * (2026-08-08).
   *
   * ONE sentence for the whole list, composed once by the queue: only the latest
   * completed run projects (R-b), so every proposed card in a payload shares it.
   * Composing it per card would be thirty chances to format one date thirty ways.
   */
  proposedAttribution: string | null;
}> = ({
  cards,
  selectedId,
  notice,
  filtered,
  onSelect,
  onRule,
  onCorrectQuestion,
  onRevertQuestion,
  linkOptions,
  onSaveLinks,
  onUnlink,
  proposedAttribution,
}) => {
  // One ref for the selected row, re-pointed on every render. A map of refs would
  // let the effect scroll a card that is no longer selected.
  const selectedRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    // `block: "nearest"` scrolls only when the card is actually out of view, so a
    // selection that is already visible does not jerk the list under the reader.
    selectedRef.current?.scrollIntoView({ block: "nearest" });
  }, [selectedId]);

  if (cards.length === 0) {
    return (
      <div style={{ padding: "1rem", color: "var(--text-muted)", fontSize: "13px" }}>
        {filtered
          ? "No candidates match this filter. The counts above say how many there are of each kind."
          : "This scenario has no candidates yet. Running a scan is what puts them here."}
      </div>
    );
  }

  return (
    <div
      style={scrollRegionStyle}
      // The region names itself and its selected row for assistive technology:
      // the cards themselves are not focusable (see `CandidateCard`), so this is
      // where "which one is the keyboard aimed at" has to be said.
      aria-label="Candidates, one selected — move with the arrow keys or J and K"
    >
      {cards.map((card) => {
        const selected = card.graph_node_id === selectedId;
        return (
          <div
            key={card.graph_node_id}
            aria-current={selected ? "true" : undefined}
            // The wrapper exists to carry the ref: `scrollIntoView` needs a real
            // box, and `display: contents` — the tempting way to keep the DOM flat
            // — removes exactly that. As a plain block it becomes the flex item and
            // the card fills it, so the list's gap is unchanged.
            ref={selected ? selectedRef : undefined}
          >
            <CandidateCard
              card={card}
              selected={selected}
              // A ruled card collapses UNLESS it is selected: selecting it is how a
              // human reaches U to take the ruling back, and U needs the buttons.
              // Unruled cards never collapse, so under "Rulable now" — the default
              // view — every card on screen shows its own four controls.
              compact={candidateState(card) !== "not_ruled" && !selected}
              onSelect={() => onSelect(card.graph_node_id)}
              // THE FIX, in one line: the handler closes over THIS card's id, in
              // this iteration's scope. Every card in the list gets live controls
              // and each one can only ever rule itself.
              onRule={(key) => onRule(key, card.graph_node_id)}
              onCorrectQuestion={(text) => onCorrectQuestion(card.graph_node_id, text)}
              onRevertQuestion={() => onRevertQuestion(card.graph_node_id)}
              linkOptions={linkOptions}
              // THE SAME FIX AS `onRule`, one line lower: the handler closes over
              // THIS card's id, in this iteration's scope. Every stuck card in the
              // list carries a live panel and none of them can link another card.
              onSaveLinks={(allegationIds, cut) =>
                onSaveLinks(card.graph_node_id, allegationIds, cut)
              }
              onUnlink={(allegationId) => onUnlink(card.graph_node_id, allegationId)}
              // Rendered only on a card that IS a proposal — the card checks its
              // own `proposed` field before showing it, so a shared sentence
              // cannot leak onto a ruled card.
              proposedAttribution={proposedAttribution}
              keyboardRefused={selected && notice !== null}
            />
          </div>
        );
      })}
    </div>
  );
};

export default CandidateList;
