// =============================================================================
// DocumentPhaseEditor.tsx — `Phase: <name>` on the document page, click to edit
// -----------------------------------------------------------------------------
// The sibling of DocumentDateEditor, deliberately the same shape: a one-line
// summary that turns into a control when clicked, saves through the same
// endpoint the upload dialog uses, and shows its own failures rather than
// swallowing them.
//
// ## Why this one takes its value as a prop
//
// DocumentDateEditor fetches its own current value. This does not: the document
// page has already loaded the document, and the record now carries `phase`. One
// less request, one less failure mode, and no chance of the header and the
// control disagreeing about the same document.
//
// The label shown is resolved from `/data/timeline.json` through `casePhases` —
// never hardcoded here (ruled 2026-08-17).
// =============================================================================

import React, { useEffect, useState } from "react";

import { getPhaseOptions, phaseLabel, PhaseOption } from "../../services/casePhases";
import { setDocumentPhase } from "../../services/documentPhase";
import PhaseField from "./PhaseField";

interface Props {
  documentId: string;
  /** The stored slug from the loaded document, or undefined when none is set. */
  phase?: string;
  /** Called after a successful save so the page reloads the document. */
  onSaved: () => void;
}

const summaryStyle: React.CSSProperties = {
  background: "none",
  border: "none",
  padding: 0,
  font: "inherit",
  color: "var(--accent-primary)",
  cursor: "pointer",
  textDecoration: "underline",
};
const panelStyle: React.CSSProperties = {
  padding: "0.6rem 0.75rem",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  backgroundColor: "var(--bg-surface)",
  minWidth: "220px",
};
const errorStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--status-dropped-text)",
  marginTop: "0.25rem",
};

const DocumentPhaseEditor: React.FC<Props> = ({ documentId, phase, onSaved }) => {
  const [editing, setEditing] = useState(false);
  const [value, setValue] = useState<string>(phase ?? "");
  const [options, setOptions] = useState<PhaseOption[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Keep the control in step when the page reloads the document after a save.
  useEffect(() => {
    setValue(phase ?? "");
  }, [phase]);

  // Only to render the LABEL for the summary line. The dropdown loads its own
  // options; both go through the same cached fetch, so this is not a second
  // request.
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

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await setDocumentPhase(documentId, value);
      setEditing(false);
      onSaved();
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Could not save the phase");
    } finally {
      setSaving(false);
    }
  };

  if (!editing) {
    // "Not set" rather than a blank: an unanswered field and a broken one must
    // look different, and a bare "Phase:" reads as a rendering fault.
    const label = phaseLabel(options, phase);
    return (
      <div style={{ fontSize: "0.74rem", color: "var(--text-muted)" }}>
        Phase:{" "}
        <button style={summaryStyle} onClick={() => setEditing(true)}>
          {label === "" ? "Not set" : label}
        </button>
        {error && <div style={errorStyle}>{error}</div>}
      </div>
    );
  }

  return (
    <div style={panelStyle}>
      {error && <div style={errorStyle}>{error}</div>}
      <PhaseField value={value} onChange={setValue} disabled={saving} />
      <div style={{ display: "flex", gap: "0.5rem" }}>
        <button
          onClick={() => { void save(); }}
          disabled={saving}
          style={{
            padding: "0.35rem 0.75rem",
            borderRadius: "6px",
            border: "none",
            backgroundColor: "var(--accent-primary)",
            color: "var(--bg-surface)",
            fontSize: "0.78rem",
            fontWeight: 600,
            cursor: saving ? "default" : "pointer",
            fontFamily: "inherit",
          }}
        >
          {saving ? "Saving…" : "Save"}
        </button>
        <button
          onClick={() => {
            setValue(phase ?? "");
            setError(null);
            setEditing(false);
          }}
          disabled={saving}
          style={{
            padding: "0.35rem 0.75rem",
            borderRadius: "6px",
            border: "1px solid var(--border-default)",
            backgroundColor: "var(--bg-surface)",
            color: "var(--text-muted)",
            fontSize: "0.78rem",
            cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          Cancel
        </button>
      </div>
    </div>
  );
};

export default DocumentPhaseEditor;
