// =============================================================================
// IncludePickerRow — the two questions Include asks before it files
// =============================================================================
//
// Mockup: `INCLUDE_PICKER_MOCKUP_v1_2026-09-14.jpg`, drawn on live PROD C-127.
// A row inside the card, under everything the card says about itself: an accent
// stripe down its left edge, "Include under:", the accusation select, a
// two-state Helps us / Helps them control, Save, Cancel.
//
// ## Why the accent is a STRIPE and not a border
//
// Roman's standing rule (2026-08-31, fenced by `accentHairline.test.ts`):
// `--accent-primary` is INK or FILL, never a 1-px hairline. The stripe is a
// 3-px filled bar — the same construction `ScanConfirmBar` uses and the rule's
// own note exempts — and the row's other sides take `--border-default`.
//
// ## Why this is its own component
//
// `CandidateCard.tsx` is at 272 non-comment lines against Rule 17's 300. This
// row is ~120 of its own, and it is a self-contained surface: its own words, its
// own two answers, its own two buttons.
//
// ## Domain note: "Helps us" is not "supports"
//
// What a curator answers is whether the fact helps us or helps them. What the
// graph stores is `supports` / `rebuts` — 802 and 142 edges measured, and the
// word Job B wrote into the 59 drafted cards on disk. Those tokens are never
// renamed. The mapping between the two lives in `includePickerModel.stanceLabels`
// and nowhere else, so a re-wording moves one settings row and touches no wire.

import React from "react";

import { stanceLabels, type IncludeOption } from "./includePickerModel";
import type { CardGrammarWording } from "../services/evidenceLinks";
import type { CardFactStance } from "../services/scenarioCards";

/** The row. Hairline box, accent stripe, card radius — `ScanConfirmBar`'s shape. */
const rowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  flexWrap: "wrap",
  gap: "12px",
  padding: "10px 14px",
  marginBottom: "12px",
  border: "1px solid var(--border-default)",
  borderLeft: "3px solid var(--accent-primary)",
  borderRadius: "var(--radius-card)",
  background: "var(--bg-page)",
};

const labelStyle: React.CSSProperties = {
  fontSize: "13.5px",
  fontWeight: 600,
  color: "var(--text-primary)",
  flexShrink: 0,
};

const selectStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13px",
  padding: "6px 10px",
  borderRadius: "6px",
  border: "1px solid var(--border-default)",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
  // Flexible, with a floor: the accusation is the longest thing on the row and
  // the one a human has to READ before answering.
  flex: 1,
  minWidth: "220px",
  maxWidth: "460px",
};

/** The two-state control: one group, two halves, the chosen one filled. */
const segmentStyle = (chosen: boolean, first: boolean): React.CSSProperties => ({
  fontFamily: "inherit",
  fontSize: "13px",
  fontWeight: chosen ? 600 : 400,
  padding: "7px 14px",
  border: "1px solid var(--border-default)",
  // The halves share an edge, so the inner border is not doubled.
  borderLeftWidth: first ? "1px" : 0,
  borderRadius: first ? "6px 0 0 6px" : "0 6px 6px 0",
  background: chosen ? "var(--accent-primary)" : "var(--bg-surface)",
  color: chosen ? "var(--v3-on-fill)" : "var(--text-primary)",
  cursor: "pointer",
  flexShrink: 0,
});

/** Commits the pair. Accent FILL with the on-fill foreground — a token pair. */
const saveStyle = (enabled: boolean): React.CSSProperties => ({
  fontFamily: "inherit",
  fontSize: "13px",
  fontWeight: 600,
  padding: "7px 18px",
  borderRadius: "999px",
  border: "1px solid var(--accent-primary)",
  background: "var(--accent-primary)",
  color: "var(--v3-on-fill)",
  cursor: enabled ? "pointer" : "not-allowed",
  // Dimmed rather than hidden: a control that vanishes tells a human nothing
  // about what it wanted. This one stays, and the empty select says why.
  opacity: enabled ? 1 : 0.5,
  flexShrink: 0,
});

/** The retreat. Same shape, no fill — as easy to hit, not as loud. */
const cancelStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "13px",
  padding: "7px 16px",
  borderRadius: "999px",
  border: "1px solid var(--border-default)",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
  cursor: "pointer",
  flexShrink: 0,
};

/**
 * Everything the row needs except the words.
 *
 * Exported so `CandidateCard` can take it as ONE prop: the state and the four
 * handlers are meaningless apart, and a card holding half of them is a shape
 * worth making unrepresentable. `wording` is NOT here — the card already holds
 * the grammar and supplies it, so no caller has to thread it twice.
 */
export interface IncludePickerProps {
  /** The accusations this card may be filed under, in the order they render. */
  options: IncludeOption[];
  /** The chosen accusation, or `null` on a card that bears on nothing. */
  allegationId: string | null;
  stance: CardFactStance;
  selectRef?: React.RefObject<HTMLSelectElement>;
  onAllegation: (allegationId: string) => void;
  onStance: (stance: CardFactStance) => void;
  /** Files the include. The only control on this row that sends anything. */
  onSave: () => void;
  onCancel: () => void;
}

/**
 * The picker row.
 *
 * ## Why every click stops propagating
 *
 * The card's own `onClick` SELECTS it. Answering a question on this row is not a
 * selection, and letting it become one would move the keyboard's aim mid-answer
 * — the same class of defect the mode's captured `graphNodeId` exists to stop,
 * arriving through the pointer instead. `DeferReasonForm` above does the same.
 */
const IncludePickerRow: React.FC<IncludePickerProps & { wording: CardGrammarWording }> = ({
  options,
  allegationId,
  stance,
  wording,
  selectRef,
  onAllegation,
  onStance,
  onSave,
  onCancel,
}) => {
  const canSave = allegationId !== null;

  return (
    <div style={rowStyle} onClick={(e) => e.stopPropagation()}>
      <span style={labelStyle}>{wording.include_picker_label}</span>

      <select
        ref={selectRef}
        style={selectStyle}
        value={allegationId ?? ""}
        aria-label={wording.include_picker_label}
        onChange={(e) => onAllegation(e.target.value)}
      >
        {/* The empty option exists ONLY while nothing is chosen — a card that
            bears on something opens with it selected, and an empty row in that
            list would be a way to un-choose into a state Save refuses. */}
        {allegationId === null && <option value="">{wording.include_picker_choose_prompt}</option>}
        {options.map((option) => (
          <option key={option.allegationId} value={option.allegationId}>
            {option.label}
          </option>
        ))}
      </select>

      <div style={{ display: "flex", flexShrink: 0 }}>
        {stanceLabels(wording).map((choice, i) => (
          <button
            key={choice.stance}
            type="button"
            aria-pressed={stance === choice.stance}
            style={segmentStyle(stance === choice.stance, i === 0)}
            onClick={() => onStance(choice.stance)}
          >
            {choice.label}
          </button>
        ))}
      </div>

      <button type="button" disabled={!canSave} style={saveStyle(canSave)} onClick={onSave}>
        {wording.include_picker_save_label}
      </button>
      <button type="button" style={cancelStyle} onClick={onCancel}>
        {wording.include_picker_cancel_label}
      </button>
    </div>
  );
};

export default IncludePickerRow;
