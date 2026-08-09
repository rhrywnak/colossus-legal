// =============================================================================
// FactsResetOrder — the confirmation and receipt for "Reset order" (Piece 5b)
// =============================================================================
//
// Extracted from `ScenarioFactsSection` when this control pushed that file past
// the 300-line limit (Rule 17, and ruling R8). The seam is the ordinary one:
// that file owns the section's WRITES and this owns the one control whose
// interaction has three states of its own — asking, done, and neither.
//
// ## Why this control asks first, unlike the per-card one it replaces
//
// "Clear my order" sat on every placed card and cleared exactly that card. This
// clears the whole scenario's placement, and the sequence IS the argument — so
// it is confirmed, and both buttons NAME the act rather than saying Yes and No,
// so a human reading only the buttons still knows what each one does.
//
// Every word is a stored row; the control is withheld until they load, because a
// destructive control that cannot state what it does must not be offered at all.

import React from "react";

import { ghostButtonStyle } from "./scenarioSectionStyles";
import type { CardGrammarWording } from "../services/evidenceLinks";

export const FactsResetOrder: React.FC<{
  wording: CardGrammarWording;
  /** Whether the confirmation is open. */
  asking: boolean;
  /** What the last reset did, or `null`. Every action acknowledges itself. */
  notice: string | null;
  onConfirm: () => void;
  onCancel: () => void;
}> = ({ wording, asking, notice, onConfirm, onCancel }) => (
  <>
    {asking && (
      <div
        role="alertdialog"
        aria-label={wording.reset_order_confirm}
        style={{
          border: "1px solid var(--state-warning-strong)",
          background: "var(--state-warning-bg-soft)",
          borderRadius: "8px",
          padding: "10px 12px",
          marginBottom: "0.6rem",
          display: "flex",
          gap: "0.6rem",
          alignItems: "center",
          flexWrap: "wrap",
          fontSize: "0.82rem",
        }}
      >
        <span>{wording.reset_order_confirm}</span>
        <button type="button" onClick={onConfirm} style={ghostButtonStyle}>
          {wording.reset_order_confirm_yes}
        </button>
        <button type="button" onClick={onCancel} style={ghostButtonStyle}>
          {wording.reset_order_confirm_cancel}
        </button>
      </div>
    )}

    {notice && (
      <div
        role="status"
        style={{
          fontSize: "0.82rem",
          color: "var(--text-secondary)",
          marginBottom: "0.5rem",
        }}
      >
        {notice}
      </div>
    )}
  </>
);

export default FactsResetOrder;
