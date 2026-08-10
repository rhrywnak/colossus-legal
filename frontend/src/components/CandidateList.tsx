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
import type { RulingReceipt } from "./rulingAcknowledgment";
import type { ChipFilter } from "./evidenceCardModel";
import type { AllegationOptions, LinkCut } from "../services/evidenceLinks";

/**
 * The card list's own box — NO LONGER A SCROLLPORT (task R2, Roman's ruling).
 *
 * ## What `maxHeight: 70vh; overflowY: auto` was doing to this page
 *
 * It made the queue a second scrollport in the middle of a scrolling document,
 * and on MacBook-class hardware — the height this product is used at — the queue
 * was most of the viewport. A wheel event over it scrolled the INNER region; the
 * page underneath only moved once the inner region hit its end, and with 148
 * cards it never did. The outer page appeared to jam, which is exactly what
 * Roman reported.
 *
 * It also broke the two things built on top of it. The ruling bar's
 * `position: sticky; top: 0` was calibrated to this region, so it pinned while
 * you scrolled the queue and did nothing while you scrolled the page — one
 * control, two behaviours, depending on where the pointer happened to be. And
 * the mount-time `scrollIntoView` below had a document to move instead of a
 * small box, so arriving on the page threw the reader down to the queue.
 *
 * The page scrolls as one document now. The queue is long; that is what the
 * filters are for.
 */
const scrollRegionStyle: React.CSSProperties = {
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
  /** Narrow the queue to a chip's value (Piece 7). */
  onFilterChip?: (filter: ChipFilter) => void;
  /** Save one NAMED card's links (task 2.10, ruling R1). Bound per card below,
   *  on the same principle as `onRule`: the target travels with the control, and
   *  no shared position can decide it. */
  onSaveLinks: (graphNodeId: string, allegationIds: string[], cut: LinkCut) => Promise<void>;
  /** Take one link back, on one named card. */
  onUnlink: (graphNodeId: string, allegationId: string) => void;
  /**
   * What the last ruling did, or `null`. Rendered on the ONE card it names — the
   * acknowledgment belongs where the human's eye already is, not in a banner
   * they have to go looking for.
   */
  receipt: RulingReceipt | null;
  /**
   * The open defer prompt, or `null` (architect ruling R1, 2026-08-08).
   *
   * The reason input renders on the card being deferred, under its action row.
   * It used to render at the BOTTOM of the queue, below the 70vh scroll window
   * (retired in .391),
   * where it could open entirely outside the human's view — §7 says a card is
   * rulable from the card alone, and collecting the reason elsewhere broke that.
   */
  deferring: { graphNodeId: string; draft: string } | null;
  deferInputRef: React.RefObject<HTMLInputElement>;
  onDeferDraft: (draft: string) => void;
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
  onFilterChip,
  onSaveLinks,
  onUnlink,
  proposedAttribution,
  receipt,
  deferring,
  deferInputRef,
  onDeferDraft,
}) => {
  // One ref for the selected row, re-pointed on every render. A map of refs would
  // let the effect scroll a card that is no longer selected.
  const selectedRef = useRef<HTMLDivElement | null>(null);

  // Whether this component has finished arriving. `scrollIntoView` is for MOVING
  // between cards, not for landing on the page.
  const arrived = useRef(false);

  useEffect(() => {
    // THE FIRST RUN IS SKIPPED (task R2). This effect fires on mount with a
    // selection already made, and `scrollIntoView` adjusts EVERY scrollable
    // ancestor — `block: "nearest"` limits how far each one moves, not how many
    // of them move. With the 70vh box gone the nearest ancestor is the document,
    // so on arrival the page scrolled itself down to the first card and took the
    // identity header off screen. That was the "opens scrolled to Scan &
    // candidates" defect.
    //
    // Every LATER run still scrolls, because that is the keyboard's whole
    // contract: J/K and the arrows move the selection and the list follows.
    if (!arrived.current) {
      arrived.current = true;
      return;
    }
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
              onFilterChip={onFilterChip}
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
              // Both of these name their card, so a receipt or an open prompt
              // can never appear on a card it is not about.
              receipt={receipt?.graphNodeId === card.graph_node_id ? receipt : null}
              deferring={deferring?.graphNodeId === card.graph_node_id ? deferring : null}
              deferInputRef={deferInputRef}
              onDeferDraft={onDeferDraft}
              keyboardRefused={selected && notice !== null}
            />
          </div>
        );
      })}
    </div>
  );
};

export default CandidateList;
