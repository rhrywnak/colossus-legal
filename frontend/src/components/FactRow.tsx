// =============================================================================
// FactRow — one fact CARD, with a fixed anatomy (tasks 2.13, 2.13b)
// =============================================================================
//
// Extracted from `WorkingView` in 2.13 (which was 412 lines, past Rule 17), then
// rebuilt in 2.13b against Roman's own screenshot of S-2 on beta.377:
//
//   "They are not human friendly. everything just blurs together. also no
//    consistency within a card. 1) it is difficult to see where one card ends
//    and the next starts. 2) the Candidate numbers appear in different places
//    cards. the text is light grey and very difficult to read. the tags and
//    buttons are also difficult to see."
//
// Every one of those is a decision this file used to get wrong:
//
// * **Where a card ends.** Rows shared one surface and were divided by a
//   near-invisible line (#eef0f3, 1.14:1). Each fact is now its own card with a
//   `--border-card` hairline, and the gap BETWEEN cards is more than three times
//   the largest gap inside one — proximity separates, the border assists.
// * **Where the number is.** The C-code, the kind and the quote shared one
//   `flex-wrap` row, so the code moved with the length of whatever sat beside it.
//   The header row is now its own line and the code is always its first element.
//   The order comes from `sectionsFor`, which is data and is tested.
// * **Light grey text.** `--text-muted` was 3.10:1 on this surface. It is now
//   4.97:1, and metadata recedes by SIZE rather than by fading (study §2).
//
// ## Typography: one stack, one body size, one bold
//
// There is no per-element font styling below. `--font-sans` at regular weight
// throughout; exactly one body size for the quote with a 1.6 line-height; every
// piece of metadata one step down at `--meta-size`. The ONLY bold on this
// surface is the C-code, which is the card's landmark. That is the study's §2
// binding, and the reason the ad-hoc `fontSize`/`fontWeight` values that used to
// live on individual spans are gone.

import React, { useState } from "react";

import { openViewerWindow } from "./viewerWindow";
import RemoveControl from "./FactRemoveControl";
import { sectionsFor, type CardSection, type WorkingRow } from "./factsTable";
import type { LinkPanelWording } from "../services/evidenceLinks";
import type { FactTier } from "../services/scenarioCards";

import {
  CARD_GAP_PX,
  CARD_PADDING,
  INTRA_GAP,
  META_SIZE,
  metaStyle,
} from "./factRowStyles";
import {
  AllegationRow,
  ExchangeLine,
  HeaderRow,
  nextTier,
  QuoteRow,
  SourceRow,
} from "./FactRowParts";

export { nextTier } from "./FactRowParts";

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
 * Green = evidence a human ruled in, blue = a fact a human wrote. Dimmed in
 * 2.13c: it is decoration until task 2.3 gives it cut meaning, and at full
 * strength it was the loudest thing on a card whose content is the exchange.
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

/** One fact card. */
const FactRow: React.FC<{
  row: WorkingRow;
  justArrived?: boolean;
  wording: LinkPanelWording | null;
  onRemove?: () => void;
  onUnplace?: () => void;
  onSetTier?: (tier: FactTier) => void;
  onDragStart?: () => void;
  onDropOn?: () => void;
  confirm?: LinkPanelWording | null;
}> = ({
  row,
  justArrived = false,
  wording,
  onRemove,
  onUnplace,
  onSetTier,
  onDragStart,
  onDropOn,
  confirm = null,
}) => {
  // A refused popup has to be SAID, not swallowed (Standing Rule 1). Local to the
  // card so the message appears where the human just clicked.
  const [blocked, setBlocked] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);

  const draggable = Boolean(onDragStart && onDropOn);

  /**
   * The renderer for one section. Walking `sectionsFor` rather than writing the
   * rows out inline is what makes the anatomy testable: this switch cannot emit a
   * section the list did not include, and cannot emit them in another order.
   */
  const render = (section: CardSection) => {
    switch (section) {
      case "header":
        return (
          <HeaderRow
            key={section}
            row={row}
            wording={wording}
            onSetTier={onSetTier}
            draggable={draggable}
          />
        );
      case "question":
        return row.question && wording ? (
          <ExchangeLine key={section} prefix={wording.fact_question_label} text={row.question} />
        ) : null;
      case "quote":
        // With a question above it the quote IS the answer and takes the `A:`
        // prefix, so the pair reads as an exchange. Without one there is no
        // exchange to mark, and a lone prefix would assert a question that was
        // lost.
        return row.question && wording ? (
          <ExchangeLine key={section} prefix={wording.fact_answer_label} text={row.text} />
        ) : (
          <QuoteRow key={section} text={row.text} />
        );
      case "allegations":
        return <AllegationRow key={section} bearsOn={row.bearsOn} />;
      case "source":
        return (
          <SourceRow
            key={section}
            row={row}
            wording={wording}
            onBlocked={setBlocked}
            onRemove={onRemove}
            onUnplace={onUnplace}
            confirm={confirm}
          />
        );
    }
  };

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
      <div style={{ display: "flex", flexDirection: "column", gap: INTRA_GAP, flex: 1, minWidth: 0 }}>
        {sectionsFor(row).map(render)}
        {blocked && (
          <div role="alert" style={{ ...metaStyle, color: "var(--state-danger-strong)" }}>
            {blocked}
          </div>
        )}
      </div>
    </div>
  );
};

export default FactRow;
