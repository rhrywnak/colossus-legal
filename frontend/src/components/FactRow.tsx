// =============================================================================
// FactRow — the FACT WRAPPER around one evidence card (2.13, then ONE_CARD)
// =============================================================================
//
// The other half of the one-card law. This wrapper carries what may be done to a
// fact — drag, weight, Remove — and mounts `EvidenceCardBody` over a view built
// by `evidenceCardView`: the SAME component over the SAME builder over the SAME
// payload the candidate wrapper uses. The two cards cannot show different fields.
//
// ## What the fact card gets back (ONE_CARD_GRAMMAR §5c)
//
// Everything the old projection threw away: the speaker's provenance tag, the
// kind chip, the confidence band, every element chip compressed behind "+N more",
// the count chip, the anchor highlight, the surrounding context behind one click,
// and the scan's reason — which until ruling R3 was deleted the moment a human
// pressed Include.
//
// ## The header row PINS (Piece 3)
//
// The facts list scrolls inside its own 60vh region, so `position: sticky` on
// this row keeps the code, the drag handle and the weight picker on screen while
// the card's own body scrolls under them. On a MacBook-sized window the controls
// never scroll away — the acceptance surface.
//
// ## A human fact renders its own shape, and always did
//
// It has no card payload (`row.card === null`): no anchor, no pinpoint, no
// allegations, no verdict, because §8 keeps it uncited by design. That row shows
// its text and its provenance line, exactly as before.

import React, { useState } from "react";

import RemoveControl from "./FactRemoveControl";
import EvidenceCardBody from "./EvidenceCardBody";
import FactCardBody from "./FactCardBody";
import { evidenceCardView, type ChipFilter } from "./evidenceCardModel";
import type { WorkingRow } from "./factsTable";
import type { AllegationOptions, LinkPanelWording } from "../services/evidenceLinks";
import type { FactTier } from "../services/scenarioCards";

import {
  bodyStyle,
  CARD_GAP_PX,
  CARD_PADDING,
  INTRA_GAP,
  META_SIZE,
  metaStyle,
} from "./factRowStyles";

export { CARD_GAP_PX, MAX_INTRA_GAP_PX } from "./factRowStyles";

const cardStyle = (justArrived: boolean, isDropTarget: boolean): React.CSSProperties => ({
  display: "flex",
  gap: "12px",
  alignItems: "stretch",
  padding: CARD_PADDING,
  // The separating space is the CARD's, not the container's — a flex gap on the
  // scroll region has no drop handler, which turned every seam into dead space
  // and is half of why Roman's drag did nothing (task 2.13c).
  marginBottom: `${CARD_GAP_PX}px`,
  border: "1px solid var(--border-card)",
  borderRadius: "var(--radius-card)",
  // The drop indicator REPLACES the top border rather than adding to it, so a
  // card never changes height while being dragged over.
  borderTop: isDropTarget ? "2px solid var(--accent-primary)" : "1px solid var(--border-card)",
  background: justArrived ? "var(--state-warning-bg-soft)" : "var(--bg-surface)",
  transition: "background 600ms ease-out",
});

/**
 * The coloured left spine, running the FULL height of the card.
 *
 * Green = evidence a human ruled in, blue = a fact a human wrote. Dimmed: it is
 * decoration until task 2.3 gives it cut meaning, and at full strength it was
 * the loudest thing on a card whose content is the exchange.
 *
 * A cue, never the only signal — a human fact's provenance line still says which
 * it is in words, for a colourblind reader and for greyscale print.
 */
const spineStyle = (isHuman: boolean): React.CSSProperties => ({
  width: "4px",
  borderRadius: "2px",
  flexShrink: 0,
  alignSelf: "stretch",
  opacity: 0.4,
  background: isHuman ? "var(--accent-primary)" : "var(--state-success-strong)",
});

/**
 * The header row — the card's landmark line, PINNED to the scroll viewport.
 *
 * `top: 0` inside the facts list's own `overflowY: auto` region: the row sticks
 * to the top of that region while its card is on screen and releases when the
 * card leaves. The opaque background is load-bearing — a transparent sticky row
 * would have the card's own text scroll through it.
 *
 * C-code first, ALWAYS, and at a fixed position: `flexShrink: 0` and its own
 * element, so nothing beside it can push it anywhere. That is the direct answer
 * to "the Candidate numbers appear in different places cards".
 */
const HeaderRow: React.FC<{
  row: WorkingRow;
  wording: LinkPanelWording;
  draggable: boolean;
}> = ({ row, wording, draggable }) => (
  <div
    style={{
      display: "flex",
      alignItems: "center",
      gap: "0.5rem",
      minHeight: "24px",
      position: "sticky",
      top: 0,
      zIndex: 2,
      background: "var(--bg-surface)",
      // The pinned row needs to end somewhere the eye can see, or the body's
      // first line looks like part of it while scrolling under.
      borderBottom: "1px solid var(--border-card)",
      paddingBottom: "6px",
      marginBottom: "2px",
    }}
  >
    {draggable && (
      <span
        aria-label={wording.fact_order_drag_hint}
        title={wording.fact_order_drag_hint}
        style={{ cursor: "grab", color: "var(--text-secondary)", fontSize: META_SIZE }}
      >
        ⠿
      </span>
    )}
    {/* The landmark. A human fact has no candidate number and shows none — an
        em-dash placeholder would imply a missing value where there is none. */}
    {row.code && (
      <span
        data-fact-code
        style={{
          fontSize: META_SIZE,
          fontWeight: 600,
          color: "var(--text-primary)",
          flexShrink: 0,
        }}
      >
        {row.code}
      </span>
    )}

    {/* THE WEIGHT PICKER IS GONE (FACT_CARD_v2 §2).
        
        "Position number (replaces weight; weight control removed)". A three-way
        weight is a curator's control, and this deck is read by a witness — the
        number she needs is which card she is on, and it stands in the card's own
        title bar where the picker used to be.
        
        The weight itself is NOT retired: `sort_ordinal` and `tier` still ride the
        payload and still order the list. What is removed is the control on this
        surface. `onSetTier` stays on the props so the caller's wiring is
        undisturbed and a future curator surface can mount one again. */}
  </div>
);

/** One fact card. */
const FactRow: React.FC<{
  row: WorkingRow;
  justArrived?: boolean;
  wording: LinkPanelWording | null;
  /** The card's own words and fold thresholds. `null` until they load. */
  options?: AllegationOptions | null;
  onRemove?: () => void;
  /** Fired by the weight picker with the chosen tier.
   *
   *  UNRENDERED since FACT_CARD_v2 §2 removed the control from this surface — a
   *  three-way weight is a curator's judgment and this deck is read by a witness.
   *  The prop stays so `FactStack`'s wiring is undisturbed and a curator surface
   *  can mount a picker again without re-threading it through three components. */
  onSetTier?: (tier: FactTier) => void;
  onDragStart?: () => void;
  onDropOn?: () => void;
  confirm?: LinkPanelWording | null;
  /** Narrow the facts list to a chip's value (Piece 7). */
  onFilterChip?: (filter: ChipFilter) => void;
  // ── FACT_CARD_v2 §2 ───────────────────────────────────────────────────────
  /** The case and scenario a field edit is written against. */
  slug?: string;
  scenarioId?: string;
  /** Collapsed cards show the title bar and source line only. */
  collapsed?: boolean;
  onToggleCard?: () => void;
  /** Re-read the deck after a field edit. */
  onCardEdited?: () => void;
}> = ({
  row,
  justArrived = false,
  wording,
  options = null,
  onRemove,
  onDragStart,
  onDropOn,
  confirm = null,
  onFilterChip,
  slug,
  scenarioId,
  collapsed = false,
  onToggleCard,
  onCardEdited,
}) => {
  const [dragOver, setDragOver] = useState(false);

  const draggable = Boolean(onDragStart && onDropOn);

  // The SHARED view. Built here from the same payload and the same builder the
  // candidate wrapper uses, which is the whole of the one-card law in one line.
  const view =
    row.card && options
      ? evidenceCardView(row.card, options.card_grammar, {
          questionChars: options.card_question_truncate_chars,
          elementK: options.card_element_chips_visible_k,
        })
      : null;

  return (
    <div
      style={cardStyle(justArrived, dragOver)}
      draggable={draggable}
      onDragStart={(event) => {
        // Firefox CANCELS a drag whose `dragstart` sets no data — the drag simply
        // never begins, with no event, no error and nothing on screen. That is
        // half of Roman's repro: real mouse, real gesture, nothing happened.
        // Chrome does not require it, which is why it worked under test and not
        // for him. The payload is unused (the dragged id lives in React state);
        // what matters is that data exists at all.
        event.dataTransfer?.setData("text/plain", "fact");
        if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
        onDragStart?.();
      }}
      onDragOver={(event) => {
        if (!draggable) return;
        // Without `preventDefault` the browser refuses the drop outright — this
        // is what makes the card a legal target, not a styling concern.
        event.preventDefault();
        setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={(event) => {
        if (!draggable) return;
        event.preventDefault();
        setDragOver(false);
        onDropOn?.();
      }}
    >
      <span style={spineStyle(row.isHuman)} aria-hidden="true" />
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: INTRA_GAP,
          flex: 1,
          minWidth: 0,
        }}
      >
        {wording && (
          <HeaderRow row={row} wording={wording} draggable={draggable} />
        )}

        {row.card && options && slug && scenarioId ? (
          // FACT_CARD_v2 §2: what a WITNESS reads. The curator's `EvidenceCardBody`
          // is still below for a row this surface cannot write a card against —
          // it has no scenario to key one by — and for a human fact, which has no
          // card at all.
          <FactCardBody
            card={row.card}
            wording={options.fact_card}
            slug={slug}
            scenarioId={scenarioId}
            position={row.card.display_ordinal ?? null}
            collapsed={collapsed}
            onToggle={() => onToggleCard?.()}
            onEdited={() => onCardEdited?.()}
          />
        ) : view && options ? (
          <EvidenceCardBody
            view={view}
            wording={options.card_grammar}
            onFilterChip={onFilterChip}
          />
        ) : (
          // A human fact, or an evidence row whose words have not loaded. Both
          // render the text; only the first renders a provenance line, because
          // only the first HAS one — "Added by Roman · Around Mar 2010" is that
          // row's authority and the words backing its coloured spine.
          <>
            <div style={bodyStyle}>{row.text}</div>
            {row.isHuman && <span style={metaStyle}>{row.statusLabel}</span>}
          </>
        )}

        {/* Piece 5b: the footer keeps ONLY Remove. "Clear my order" left for the
            section header, where one control does the job it was doing on
            forty-six cards. */}
        {onRemove && (
          <div style={{ display: "flex", justifyContent: "flex-end" }}>
            <RemoveControl row={row} onRemove={onRemove} confirm={confirm} />
          </div>
        )}
      </div>
    </div>
  );
};

export default FactRow;
