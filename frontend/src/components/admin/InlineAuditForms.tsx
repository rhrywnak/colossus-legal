/**
 * InlineAuditForms — Verify and Flag inline forms for the Document Workspace.
 *
 * Extracted from DocumentWorkspace.tsx to keep that page under 300 lines.
 * These forms appear at the top of the evidence panel when the user clicks
 * "Verify" or "Flag" on an evidence card.
 *
 * TODO: B-4 — v1 dead code. Part of the manual evidence audit workflow
 * superseded by the v2 pipeline. Remove when v1 is fully deprecated.
 */
import React from "react";
import { DocumentEvidence } from "../../services/documentEvidence";
import { cardStyle, btnPrimary, btnSecondary, inputStyle, labelStyle } from "./adminStyles";

// ── Verify form ──────────────────────────────────────────────────

interface VerifyFormProps {
  evidence: DocumentEvidence;
  status: string;
  notes: string;
  submitting: boolean;
  onStatusChange: (status: string) => void;
  onNotesChange: (notes: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
}

export const InlineVerifyForm: React.FC<VerifyFormProps> = ({
  evidence, status, notes, submitting,
  onStatusChange, onNotesChange, onSubmit, onCancel,
}) => (
  <div style={{ ...cardStyle, marginBottom: "0.75rem", border: "2px solid #a7f3d0" }}>
    <div style={{ fontSize: "0.84rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.5rem" }}>
      Verify: {evidence.title || evidence.id}
    </div>
    <div style={{ marginBottom: "0.5rem" }}>
      <label style={labelStyle}>Status</label>
      <select style={inputStyle} value={status} onChange={(e) => onStatusChange(e.target.value)}>
        <option value="verified">Verified</option>
        <option value="rejected">Rejected</option>
        <option value="pending">Pending (undo)</option>
      </select>
    </div>
    <div style={{ marginBottom: "0.5rem" }}>
      <label style={labelStyle}>Notes</label>
      <textarea
        style={{ ...inputStyle, minHeight: "60px", resize: "vertical" }}
        value={notes}
        onChange={(e) => onNotesChange(e.target.value)}
        placeholder="e.g. Confirmed on page 5, paragraph 2"
      />
    </div>
    <div style={{ display: "flex", gap: "0.4rem" }}>
      <button style={btnPrimary} onClick={onSubmit} disabled={submitting}>
        {submitting ? "Saving..." : "Save"}
      </button>
      <button style={btnSecondary} onClick={onCancel}>Cancel</button>
    </div>
  </div>
);

// ── Flag form ────────────────────────────────────────────────────

interface FlagFormProps {
  evidence: DocumentEvidence;
  severity: string;
  description: string;
  submitting: boolean;
  onSeverityChange: (severity: string) => void;
  onDescriptionChange: (description: string) => void;
  onSubmit: () => void;
  onCancel: () => void;
}

export const InlineFlagForm: React.FC<FlagFormProps> = ({
  evidence, severity, description, submitting,
  onSeverityChange, onDescriptionChange, onSubmit, onCancel,
}) => (
  <div style={{ ...cardStyle, marginBottom: "0.75rem", border: "2px solid #fecaca" }}>
    <div style={{ fontSize: "0.84rem", fontWeight: 600, color: "#0f172a", marginBottom: "0.5rem" }}>
      Flag: {evidence.title || evidence.id}
    </div>
    <div style={{ marginBottom: "0.5rem" }}>
      <label style={labelStyle}>Severity</label>
      <select style={inputStyle} value={severity} onChange={(e) => onSeverityChange(e.target.value)}>
        <option value="low">Low</option>
        <option value="medium">Medium</option>
        <option value="high">High</option>
        <option value="critical">Critical</option>
      </select>
    </div>
    <div style={{ marginBottom: "0.5rem" }}>
      <label style={labelStyle}>Description</label>
      <textarea
        style={{ ...inputStyle, minHeight: "60px", resize: "vertical" }}
        value={description}
        onChange={(e) => onDescriptionChange(e.target.value)}
        placeholder="Describe the issue..."
      />
    </div>
    <div style={{ display: "flex", gap: "0.4rem" }}>
      <button style={{ ...btnPrimary, backgroundColor: "#dc2626" }} onClick={onSubmit} disabled={submitting}>
        {submitting ? "Saving..." : "Submit Flag"}
      </button>
      <button style={btnSecondary} onClick={onCancel}>Cancel</button>
    </div>
  </div>
);
