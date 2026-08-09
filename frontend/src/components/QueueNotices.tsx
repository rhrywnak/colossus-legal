// =============================================================================
// QueueNotices — the three strips above the candidate list
// =============================================================================
//
// Extracted from `CardQueue` when ONE_CARD_GRAMMAR pushed that file past the
// 300-line limit (Rule 17, and ruling R8). All three are the same KIND of thing
// — a sentence between the filter chips and the cards, saying what just happened
// or what the list is currently showing — which is the seam.
//
// Each one exists because its absence was a measured defect:
//
// * **The link failure.** A write that vanished with nothing said.
// * **The vanish receipt.** On beta.385 a defer wrote an anchor, a reference row
//   and its provenance, and said nothing at all — the card left the Proposed
//   filter (correctly) and the screen read exactly like a dead button. A RULING's
//   outcome normally rides on the card it was about; this is the fallback for
//   when that card has left the list, which is itself one of the things it says.
// * **The chip filter.** A list narrowed with no visible exit is how a human ends
//   up reporting that candidates have gone missing.

import React from "react";

import { chipStyle } from "./candidateCardStyles";
import type { ChipFilter } from "./evidenceCardModel";
import type { RulingReceipt } from "./rulingAcknowledgment";
import type { AllegationOptions } from "../services/evidenceLinks";

const SURFACE = "var(--bg-surface)";
const HAIRLINE = "1px solid var(--border-default)";

/** The receipt strip: quiet for a landing, loud only for a refusal. */
const receiptStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "8px",
  padding: "0.55rem 0.8rem",
  margin: "0.5rem 0",
  fontSize: "0.85rem",
  color: "var(--text-secondary)",
  background: "var(--v3-chrome)",
};

const receiptFailedStyle: React.CSSProperties = {
  ...receiptStyle,
  borderColor: "var(--state-danger-strong)",
  color: "var(--state-danger-strong)",
  background: SURFACE,
};

export const QueueNotices: React.FC<{
  linkError: string | null;
  onDismissLinkError: () => void;
  /** Shown ONLY when the card it names has left the visible list. */
  orphanedReceipt: RulingReceipt | null;
  chipFilter: ChipFilter | null;
  onClearChipFilter: () => void;
  options: AllegationOptions | null;
}> = ({
  linkError,
  onDismissLinkError,
  orphanedReceipt,
  chipFilter,
  onClearChipFilter,
  options,
}) => (
  <>
    {/* A LINK write's failure. A RULING's outcome is not shown here — it goes on
        the card it was about, and only falls back to this strip when that card
        has left the list (which is itself one of the things it says). */}
    {linkError && (
      <div
        role="alert"
        style={{
          border: HAIRLINE,
          borderColor: "var(--state-danger-strong)",
          borderRadius: "8px",
          padding: "0.6rem 0.8rem",
          margin: "0.5rem 0",
          color: "var(--state-danger-strong)",
          fontSize: "0.85rem",
        }}
      >
        {linkError}
        <button
          type="button"
          onClick={() => onDismissLinkError()}
          style={{
            ...chipStyle,
            marginLeft: "0.6rem",
            cursor: "pointer",
            background: SURFACE,
          }}
        >
          Dismiss
        </button>
      </div>
    )}

    {/* THE VANISH, said out loud. When a ruling takes its card out of the list,
        the card is no longer there to carry its own receipt — so it is reported
        here, above the list, where the human is already looking. This is the
        sentence whose absence made a working defer read as a dead button on
        beta.385. */}
    {orphanedReceipt && (
      <div
        role={orphanedReceipt.failed ? "alert" : "status"}
        style={orphanedReceipt.failed ? receiptFailedStyle : receiptStyle}
      >
        {orphanedReceipt.text}
      </div>
    )}

    {/* Piece 7: a list narrowed by a chip SAYS what it is narrowed to and
        offers the way out. A filter with no visible exit is how a human ends
        up reporting that candidates have gone missing. */}
    {chipFilter && options && (
      <div role="status" style={{ margin: "0.4rem 0" }}>
        <button
          type="button"
          onClick={() => onClearChipFilter()}
          style={{
            border: "1px solid var(--accent-primary)",
            background: "var(--state-info-bg-soft)",
            color: "var(--accent-primary)",
            borderRadius: "999px",
            padding: "3px 12px",
            cursor: "pointer",
            fontFamily: "inherit",
            fontSize: "0.8rem",
          }}
        >
          {options.card_grammar.chip_filter_clear_template.replace(
            "{value}",
            chipFilter.value,
          )}
        </button>
      </div>
    )}
  </>
);

export default QueueNotices;
