// =============================================================================
// PhaseField.tsx — the "Phase" dropdown, shared by intake and the document page
// -----------------------------------------------------------------------------
// One control, two callers, mirroring DocumentDateField: the upload dialog asks
// at intake, the document page corrects afterwards. A second copy would drift in
// its option order or its blank handling, and the two screens would disagree
// about what "no phase" looks like.
//
// The options come from `casePhases`, which reads them from
// `/data/timeline.json`. Nothing here hardcodes a label — see that module for
// why (ruled 2026-08-17).
// =============================================================================

import React, { useEffect, useState } from "react";

import { getPhaseOptions, PhaseOption } from "../../services/casePhases";

interface Props {
  /** The stored slug, or `""` for a document with no phase. */
  value: string;
  onChange: (next: string) => void;
  disabled?: boolean;
  /** Rendered above the control. Omitted on surfaces that label it themselves. */
  showLabel?: boolean;
}

const labelStyle: React.CSSProperties = {
  display: "block",
  fontSize: "0.8rem",
  fontWeight: 600,
  color: "var(--text-secondary)",
  marginBottom: "0.3rem",
};
const selectStyle: React.CSSProperties = {
  width: "100%",
  padding: "0.45rem 0.6rem",
  fontSize: "0.84rem",
  border: "1px solid var(--border-default)",
  borderRadius: "4px",
  fontFamily: "inherit",
  boxSizing: "border-box",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-primary)",
};
const errorStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--status-dropped-text)",
  marginTop: "0.25rem",
};

/**
 * The Phase dropdown.
 *
 * Blank is a real option and it is FIRST with no pre-selection: the phase is
 * never required, and pre-selecting one would have every uploaded document
 * silently claim to be pre-probate.
 *
 * A failure to load the phase list is shown, not swallowed — an empty dropdown
 * and a broken one must not look the same (Standing Rule 1).
 */
const PhaseField: React.FC<Props> = ({ value, onChange, disabled, showLabel = true }) => {
  const [options, setOptions] = useState<PhaseOption[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getPhaseOptions()
      .then((opts) => {
        if (!cancelled) setOptions(opts);
      })
      .catch((e: unknown) => {
        if (cancelled) return;
        setError(e instanceof Error ? e.message : "Could not load the case phases");
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <div style={{ marginBottom: "1rem" }}>
      {showLabel && <label style={labelStyle} htmlFor="document-phase">Phase</label>}
      <select
        id="document-phase"
        style={selectStyle}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled || error !== null}
        aria-label="Phase"
      >
        {/* No pre-selection: a phase is never required. */}
        <option value="">—</option>
        {options.map((o) => (
          <option key={o.slug} value={o.slug}>
            {o.label}
          </option>
        ))}
      </select>
      {error !== null && (
        <div style={errorStyle}>
          The phase list could not be loaded ({error}). Phase cannot be set here until it is.
        </div>
      )}
    </div>
  );
};

export default PhaseField;
