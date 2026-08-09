// =============================================================================
// FactStack — the fact cards themselves, and the folded background pile
// =============================================================================
//
// Extracted from `WorkingView` when ONE_CARD_GRAMMAR pushed that file past the
// 300-line limit (Rule 17, and ruling R8's "no file this task touches ends over
// it"). The seam was already there and 2.13b had drawn it once: `WorkingView`
// owns the search header, the arrival bookkeeping, the empty states and the
// writes, and this owns the STACK — which cards render, in which half, and what
// each one's controls are bound to.
//
// Pure presentation. Every decision it makes was made upstream.

import React from "react";

import FactRow from "./FactRow";
import { neighboursForDrop, type WorkingRow } from "./factsTable";
import { ghostButtonStyle } from "./scenarioSectionStyles";
import type { ChipFilter } from "./evidenceCardModel";
import { fillCount, type AllegationOptions, type LinkPanelWording } from "../services/evidenceLinks";
import type { FactTier } from "../services/scenarioCards";

/**
 * The cards themselves: the shown stack, then the folded background pile.
 *
 * Extracted from `WorkingView` in task 2.13b. That component was carrying the
 * search header, the arrival bookkeeping, the empty states AND every card's drag
 * wiring in one body; the card stack is the part with its own rules and it reads
 * better alone. Pure presentation — every decision it makes was made upstream.
 */
export const FactStack: React.FC<{
  shown: WorkingRow[];
  background: WorkingRow[];
  showBackground: boolean;
  onToggleBackground: () => void;
  /** The FULL ordered list, which a drop needs to name its neighbours from. */
  rows: WorkingRow[];
  arrived: Set<string>;
  wording: LinkPanelWording | null;
  dragging: string | null;
  setDragging: (id: string | null) => void;
  onRemoveFact: (graphNodeId: string) => void;
  onRemoveHumanFact: (factId: string) => void;
  onSetTier: (graphNodeId: string, tier: FactTier) => void;
  onMoveFact: (graphNodeId: string, after: string | null, before: string | null) => void;
  /** The card's own words and fold thresholds. */
  options: AllegationOptions | null;
  /** The last weight change and the row it was about, or `null`. */
  weightNotice: { text: string; nodeId: string; previous: FactTier } | null;
  /** Put the previous weight back, through the same write path. */
  onUndoWeight: () => void;
  onNoticeCleared: () => void;
  /** The chip narrowing this list, or `null` (Piece 7). */
  chipFilter: ChipFilter | null;
  onFilterChip: (filter: ChipFilter | null) => void;
}> = ({
  shown,
  background,
  showBackground,
  onToggleBackground,
  rows,
  arrived,
  wording,
  dragging,
  setDragging,
  onRemoveFact,
  onRemoveHumanFact,
  onSetTier,
  onMoveFact,
  options,
  weightNotice,
  onUndoWeight,
  onNoticeCleared,
  chipFilter,
  onFilterChip,
}) => {
  /** One card's props — identical for the shown stack and the background pile. */
  const cardFor = (row: WorkingRow) => ({
    key: row.graphNodeId,
    row,
    wording,
    justArrived: arrived.has(row.graphNodeId),
    // Item G: EVERY card can be taken out from where it is now. A human fact is
    // deleted outright (it exists nowhere else); an evidence fact returns to the
    // queue as not ruled. Two different acts, which is why the confirmation
    // names what will happen. The evidence control is WITHHELD until the wording
    // loads — `undefined`, not a control with an invented label (R4).
    onRemove: row.isHuman
      ? () => onRemoveHumanFact(row.graphNodeId.replace(/^human:/, ""))
      : wording
        ? () => onRemoveFact(row.graphNodeId)
        : undefined,
    // A human fact carries no weight tier (§8 — it is not evidence), so it gets
    // no weight control rather than a disabled one.
    onSetTier: row.isHuman
      ? undefined
      : (tier: FactTier) => onSetTier(row.graphNodeId, tier),
    onDragStart: row.isHuman ? undefined : () => setDragging(row.graphNodeId),
    onDropOn: row.isHuman
      ? undefined
      : () => {
          if (!dragging) return;
          const pair = neighboursForDrop(rows, dragging, row.graphNodeId);
          setDragging(null);
          // A drop that cannot name a position (onto itself, or onto a card that
          // has gone) is not sent: the server would only have to refuse it, and
          // the human meant nothing by it.
          if (pair) onMoveFact(dragging, pair.after, pair.before);
        },
    confirm: row.isHuman ? null : wording,
    options,
    onFilterChip,
  });

  return (
    <>
      {shown.map((row) => (
        <FactRow {...cardFor(row)} />
      ))}

      {/* Piece 5a: EVERY weight change says what it did, and offers the way
          back. Task 2.13c said it only for a card folded into the pile; the
          undo is new, and it is the half that was missing when Roman was moved
          to the background by a control he had signed off three days earlier.

          The card it names is still in place above: `splitBackground` holds a
          just-demoted row in the shown half while this sentence is on screen, so
          the list does not slide out from under the cursor at the moment of the
          click (the C-91 defect). */}
      {weightNotice && options && (
        <div
          role="status"
          style={{
            padding: "0.45rem 0.7rem",
            fontSize: "0.8rem",
            color: "var(--text-primary)",
            background: "var(--state-warning-bg-soft)",
            borderRadius: "6px",
            display: "flex",
            gap: "0.6rem",
            alignItems: "center",
          }}
        >
          <span>{weightNotice.text}</span>
          <button
            type="button"
            onClick={onUndoWeight}
            style={{
              border: "none",
              background: "none",
              padding: 0,
              color: "var(--accent-primary)",
              cursor: "pointer",
              fontFamily: "inherit",
              fontSize: "0.8rem",
              textDecoration: "underline",
            }}
          >
            {options.card_grammar.weight_undo_label}
          </button>
        </div>
      )}

      {/* The background pile: folded, never hidden. The count is always on
          screen, so a curated fact can never silently vanish — which is the
          whole reason this tier is a fold and not a filter. */}
      {background.length > 0 && wording && (
        <div style={{ padding: "0.2rem 0" }}>
          <button
            type="button"
            onClick={() => {
              onNoticeCleared();
              onToggleBackground();
            }}
            style={{ ...ghostButtonStyle, fontSize: "0.8rem" }}
          >
            {showBackground
              ? wording.fact_background_hide_label
              : fillCount(wording.fact_background_count_template, background.length)}
          </button>
        </div>
      )}

      {/* Same anatomy inside the pile (ruling 3) — these are the same cards, not
          a reduced rendering of them. */}
      {showBackground && background.map((row) => <FactRow {...cardFor(row)} />)}
    </>
  );
};


export default FactStack;
