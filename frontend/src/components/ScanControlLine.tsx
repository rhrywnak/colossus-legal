// =============================================================================
// ScanControlLine — the scan control row (extracted from ThemeScanPanel, 1.7D)
// =============================================================================
//
// Mockup `.scan`: one row — Run scan · model select · last-run meta · candidate
// count · Scan history. This is the row task 1.7D restyled, and it is now the row
// you edit, rather than 80 lines buried two thirds of the way down a 1000-line
// panel.
//
// ## Why it moved out
//
// `ThemeScanPanel` is pre-existing debt at ~980 non-comment lines against a
// 300-line limit; ruling R6 puts its full remediation in task 3.14, but this task
// still may not GROW it. Lifting the row this task actually touched pays for the
// lines 1.7D added (the missing `runButton` style, the candidate-count surfaces)
// and leaves the panel smaller than it started.
//
// Styling is transcribed from the mockup: borderless controls on the chrome fill,
// a ghost Run button carried by its own shadow.

import React from "react";

import type { ScanModel } from "../services/themeScan";
import { chromeInputStyle, ghostButtonStyle } from "./scenarioSectionStyles";

/** Mockup `.scan`: padding 14px 24px, 14px gaps, centred, wrapping. */
const rowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "14px",
  flexWrap: "wrap",
  padding: "14px 24px",
};

const metaStyle: React.CSSProperties = { color: "var(--text-secondary)", fontSize: "13px" };
const mutedStyle: React.CSSProperties = { color: "var(--text-muted)", fontSize: "13px" };

/** The scan-start refusal / model-catalogue failure box. */
const errorBoxStyle: React.CSSProperties = {
  border: "1px solid var(--state-danger-strong)",
  borderRadius: "8px",
  padding: "0.6rem 0.8rem",
  margin: "0 24px",
  color: "var(--state-danger-strong)",
  fontSize: "0.85rem",
};

// ─── SETUP ────────────────────────────────────────────────────────────────────

/**
 * The scan control: ONE LINE (task 1.7B) — Run · model · last-run summary.
 *
 * ## What died here
 *
 * A two-column grid of radio cards and a full-width Run button, which took the
 * vertical space of a small form to express a choice most people make once. The
 * design note calls it a control, not a panel, and a control is a line.
 *
 * ## The model list is ordered and labelled by the SERVER
 *
 * Local models come first and one of them is selected by default, because a scan
 * is one metered call per candidate — 148 of them on S-1 — and the picker must
 * not put a billed model under the cursor. That ordering, the "(API — billed)"
 * label, and which model is default all arrive composed on the payload
 * (`display_label`, `billing_class`): the browser renders them and decides
 * nothing, because which models cost money is a deployment fact, not something a
 * client should infer from a name.
 */
const ScanControlLine: React.FC<{
  models: ScanModel[];
  modelError: string | null;
  modelWarnings: string[];
  selectedModel: string | null;
  lastRun: string | null;
  /** Candidates in the pool right now, or `null` while unknown. */
  candidateCount: number | null;
  /** Whether the candidate-count read FAILED — distinct from a count of zero. */
  countError: boolean;
  onSelect: (id: string) => void;
  onRun: () => void;
  /**
   * The scan-history disclosure, rendered INLINE at the right of the control line.
   *
   * Passed in rather than imported here because the panel owns the run list, the
   * selection and the delete — this component owns the row's layout. The mockup puts
   * the history link on the scan row (after the spacer), not below it, which is why
   * it arrives as a slot instead of being rendered by the caller underneath.
   */
  historySlot?: React.ReactNode;
}> = ({
  models,
  modelError,
  modelWarnings,
  selectedModel,
  lastRun,
  candidateCount,
  countError,
  onSelect,
  onRun,
  historySlot,
}) => (
  <div style={{ display: "flex", flexDirection: "column", gap: "14px" }}>
    {modelError && (
      <div style={errorBoxStyle} role="alert">
        Could not load models — {modelError}
      </div>
    )}
    {modelWarnings.length > 0 && (
      <div style={errorBoxStyle} role="alert">
        {modelWarnings.map((warning) => (
          <div key={warning}>{warning}</div>
        ))}
      </div>
    )}
    <div style={rowStyle}>
      <button
        type="button"
        onClick={onRun}
        disabled={!selectedModel}
        style={selectedModel ? ghostButtonStyle : { ...ghostButtonStyle, ...{ opacity: 0.55, cursor: "not-allowed", boxShadow: "none" } }}
      >
        Run scan
      </button>

      {models.length === 0 && !modelError ? (
        <span style={mutedStyle}>No scan-eligible models available.</span>
      ) : (
        <select
          aria-label="Scan model"
          style={chromeInputStyle}
          value={selectedModel ?? ""}
          onChange={(e) => onSelect(e.target.value)}
        >
          {models.map((m) => (
            <option key={m.model_id} value={m.model_id}>
              {m.display_label}
            </option>
          ))}
        </select>
      )}

      {lastRun && <span style={metaStyle}>{lastRun}</span>}
      {candidateCount != null && (
        <span style={metaStyle}>{candidateCount} candidates gathered</span>
      )}
      {/* The count read FAILED — said, not swallowed. Muted because the scan itself
          still runs without it; the number is context, not a precondition. */}
      {countError && (
        <span style={mutedStyle} role="status">
          candidate count unavailable
        </span>
      )}

      {/* Mockup `.spacer` then the history link, hard right on the same row. */}
      <span style={{ flex: 1 }} />
      {historySlot}
    </div>

    {/* No benchmark toggle. It used to ask "record verdicts without saving
        suggestions to the scenario?" — but a scan never saves suggestions now, so
        every scan is what benchmark mode used to mean. Keeping the toggle would
        keep a second write model on screen, which is the incoherence this change
        exists to remove. */}
  </div>
);


export default ScanControlLine;
