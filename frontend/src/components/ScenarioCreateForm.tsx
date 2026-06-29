// =============================================================================
// ScenarioCreateForm.tsx — minimal create-scenario entry form (Chunk 3)
// -----------------------------------------------------------------------------
// A functional inline form, NOT the final designed surface. POSTs to the
// chunk-1 create route via `createScenario`. Anchor allegation ids are entered
// as free text here (comma/newline separated); a graph-backed allegation picker
// is a later chunk. React state only — no localStorage/sessionStorage.
// =============================================================================

import React, { useState } from "react";

import type { ScenarioStatus } from "../pages/trialPrepData";
import { statusMeta } from "../pages/trialPrepHelpers";
import {
  createScenario,
  type ScenarioDirection,
} from "../services/scenarioCrud";

/** The three real statuses, labelled via the shared `statusMeta` (one source). */
const STATUS_OPTIONS: ScenarioStatus[] = ["draft", "needs_evidence", "ready"];
const DIRECTION_OPTIONS: { value: ScenarioDirection; label: string }[] = [
  { value: "offense", label: "Offense" },
  { value: "defense", label: "Defense" },
];

interface ScenarioCreateFormProps {
  /** Case slug the new scenario is created under (from the URL). */
  slug: string;
  /** Called after a successful create so the parent can refresh the dashboard. */
  onCreated: () => void;
  /** Called when the user dismisses the form without creating. */
  onCancel: () => void;
}

/**
 * Inline create-scenario form. Validates a non-empty name client-side, splits
 * the anchor-ids textarea on commas/newlines, and surfaces any thrown error
 * (including the backend's field-named BadRequest) inline — never swallowed
 * (Standing Rule 1).
 */
const ScenarioCreateForm: React.FC<ScenarioCreateFormProps> = ({
  slug,
  onCreated,
  onCancel,
}) => {
  const [name, setName] = useState("");
  const [direction, setDirection] = useState<ScenarioDirection>("defense");
  const [status, setStatus] = useState<ScenarioStatus>("draft");
  const [anchorsText, setAnchorsText] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmedName = name.trim();
    if (trimmedName.length === 0) {
      setError("Name is required.");
      return;
    }

    // Crude-but-functional anchor entry: split on comma or newline, trim, drop
    // empties. An empty list is sent as `undefined` (the backend treats it as
    // "no anchors yet", a valid state).
    const anchorIds = anchorsText
      .split(/[,\n]/)
      .map((s) => s.trim())
      .filter((s) => s.length > 0);

    setSubmitting(true);
    setError(null);
    try {
      await createScenario(slug, {
        name: trimmedName,
        direction,
        status,
        anchor_allegation_ids: anchorIds.length > 0 ? anchorIds : undefined,
      });
      // Clear the form, then let the parent refresh so the new card appears.
      setName("");
      setAnchorsText("");
      setDirection("defense");
      setStatus("draft");
      onCreated();
    } catch (err: unknown) {
      setError(
        err instanceof Error
          ? err.message
          : "Failed to create the scenario. Please try again.",
      );
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} style={panelStyle}>
      <div style={fieldStyle}>
        <label htmlFor="scenario-name" style={labelStyle}>
          Name
        </label>
        <input
          id="scenario-name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="e.g. Marie is obstructive and uncooperative"
          style={inputStyle}
        />
      </div>

      <div style={rowStyle}>
        <div style={fieldStyle}>
          <label htmlFor="scenario-direction" style={labelStyle}>
            Direction
          </label>
          <select
            id="scenario-direction"
            value={direction}
            onChange={(e) => setDirection(e.target.value as ScenarioDirection)}
            style={inputStyle}
          >
            {DIRECTION_OPTIONS.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </div>

        <div style={fieldStyle}>
          <label htmlFor="scenario-status" style={labelStyle}>
            Status
          </label>
          <select
            id="scenario-status"
            value={status}
            onChange={(e) => setStatus(e.target.value as ScenarioStatus)}
            style={inputStyle}
          >
            {STATUS_OPTIONS.map((value) => (
              <option key={value} value={value}>
                {statusMeta(value).label}
              </option>
            ))}
          </select>
        </div>
      </div>

      <div style={fieldStyle}>
        <label htmlFor="scenario-anchors" style={labelStyle}>
          Anchor allegation id(s)
        </label>
        <textarea
          id="scenario-anchors"
          value={anchorsText}
          onChange={(e) => setAnchorsText(e.target.value)}
          placeholder="doc-…:allegation:<hash>"
          rows={2}
          style={{ ...inputStyle, resize: "vertical", fontFamily: "var(--font-mono)" }}
        />
        <div style={helperStyle}>
          Optional. Paste allegation node ids, comma- or newline-separated (a
          graph-backed picker comes later).
        </div>
      </div>

      {error && <div style={errorStyle}>{error}</div>}

      <div style={actionsStyle}>
        <button
          type="button"
          onClick={onCancel}
          disabled={submitting}
          style={secondaryButtonStyle}
        >
          Cancel
        </button>
        <button type="submit" disabled={submitting} style={primaryButtonStyle}>
          {submitting ? "Creating…" : "Create scenario"}
        </button>
      </div>
    </form>
  );
};

// ─── Styles (design tokens only — Rule 2: no bespoke colors) ─────────────────

const panelStyle: React.CSSProperties = {
  margin: "1rem 0 1.5rem",
  padding: "1.25rem",
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  display: "flex",
  flexDirection: "column",
  gap: "1rem",
  maxWidth: "640px",
};
const rowStyle: React.CSSProperties = {
  display: "flex",
  gap: "1rem",
  flexWrap: "wrap",
};
const fieldStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "4px",
  flex: "1 1 200px",
};
const labelStyle: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  fontWeight: 600,
  color: "var(--text-secondary)",
};
const inputStyle: React.CSSProperties = {
  padding: "8px 10px",
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  color: "var(--text-primary)",
  backgroundColor: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
};
const helperStyle: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  color: "var(--text-muted)",
};
const errorStyle: React.CSSProperties = {
  padding: "0.75rem",
  backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
};
const actionsStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "flex-end",
  gap: "0.75rem",
};
const baseButtonStyle: React.CSSProperties = {
  padding: "8px 16px",
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  fontWeight: 600,
  borderRadius: "6px",
  cursor: "pointer",
};
const primaryButtonStyle: React.CSSProperties = {
  ...baseButtonStyle,
  color: "var(--accent-primary)",
  backgroundColor: "var(--accent-bg-soft)",
  border: "1px solid var(--accent-primary)",
};
const secondaryButtonStyle: React.CSSProperties = {
  ...baseButtonStyle,
  color: "var(--text-secondary)",
  backgroundColor: "transparent",
  border: "1px solid var(--border-default)",
};

export default ScenarioCreateForm;
