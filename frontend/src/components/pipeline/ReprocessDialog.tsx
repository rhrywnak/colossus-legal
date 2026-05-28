/**
 * ReprocessDialog — choose how to re-process a document.
 *
 * Offers three options: same settings, new settings, or delete and re-extract.
 */
import React, { useState } from "react";
import { processDocument } from "../../services/pipelineApi";

interface ReprocessDialogProps {
  open: boolean;
  documentId: string;
  onClose: () => void;
  onSuccess: () => void;
}

type ReprocessOption = "same_settings" | "new_settings" | "delete_and_reextract";

const overlay: React.CSSProperties = {
  position: "fixed", inset: 0, backgroundColor: "rgba(0,0,0,0.4)",
  display: "flex", alignItems: "center", justifyContent: "center", zIndex: 1000,
};
const card: React.CSSProperties = {
  backgroundColor: "var(--bg-surface)", borderRadius: "12px", padding: "1.75rem",
  maxWidth: "480px", width: "90%", boxShadow: "0 20px 60px rgba(0,0,0,0.15)",
};
const optionStyle = (selected: boolean): React.CSSProperties => ({
  padding: "0.75rem 1rem", border: `2px solid ${selected ? "var(--accent-primary)" : "var(--border-default)"}`,
  borderRadius: "8px", cursor: "pointer", marginBottom: "0.5rem",
  backgroundColor: selected ? "var(--accent-bg-soft)" : "var(--bg-surface)",
  transition: "all 0.15s ease",
});
const optionTitle: React.CSSProperties = {
  fontSize: "0.88rem", fontWeight: 600, color: "var(--text-primary)",
};
const optionDesc: React.CSSProperties = {
  fontSize: "0.76rem", color: "var(--text-muted)", marginTop: "0.15rem",
};
const btnRow: React.CSSProperties = {
  display: "flex", justifyContent: "flex-end", gap: "0.5rem", marginTop: "1rem",
};
const btnCancel: React.CSSProperties = {
  padding: "0.45rem 1rem", fontSize: "0.84rem", fontWeight: 500, border: "1px solid var(--border-default)",
  borderRadius: "6px", backgroundColor: "var(--bg-surface)", color: "var(--text-muted)", cursor: "pointer",
  fontFamily: "inherit",
};
const btnContinue = (enabled: boolean): React.CSSProperties => ({
  padding: "0.45rem 1rem", fontSize: "0.84rem", fontWeight: 600, border: "none",
  borderRadius: "6px", backgroundColor: enabled ? "var(--accent-primary)" : "var(--text-disabled)",
  color: "var(--bg-surface)", cursor: enabled ? "pointer" : "default", fontFamily: "inherit",
});

const OPTIONS: { value: ReprocessOption; title: string; description: string }[] = [
  {
    value: "same_settings",
    title: "Re-extract with same settings",
    description: "Re-run extraction using the same model, schema, and template.",
  },
  {
    value: "new_settings",
    title: "Re-extract with new settings",
    description: "Choose a different model, schema, or template before re-running.",
  },
  {
    value: "delete_and_reextract",
    title: "Delete graph data and re-extract",
    description: "Remove this document's entities from the knowledge graph, then re-extract from scratch.",
  },
];

const ReprocessDialog: React.FC<ReprocessDialogProps> = ({ open, documentId, onClose, onSuccess }) => {
  const [selected, setSelected] = useState<ReprocessOption>("same_settings");
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const handleContinue = async () => {
    if (running) return;
    setRunning(true);
    setError(null);
    try {
      await processDocument(documentId, selected);
      onSuccess();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Re-process failed");
    } finally {
      setRunning(false);
    }
  };

  return (
    <div style={overlay} onClick={onClose}>
      <div style={card} onClick={(e) => e.stopPropagation()}>
        <h2 style={{ margin: "0 0 1rem", fontSize: "1.1rem", fontWeight: 700, color: "var(--text-primary)" }}>
          Re-process Document
        </h2>
        <p style={{ fontSize: "0.84rem", color: "var(--text-muted)", marginBottom: "1rem" }}>
          Choose how to re-process this document:
        </p>

        {OPTIONS.map((opt) => (
          <div
            key={opt.value}
            style={optionStyle(selected === opt.value)}
            onClick={() => setSelected(opt.value)}
          >
            <div style={optionTitle}>{opt.title}</div>
            <div style={optionDesc}>{opt.description}</div>
          </div>
        ))}

        {error && (
          <div style={{ padding: "0.5rem 0.75rem", backgroundColor: "var(--state-danger-bg-soft)", border: "1px solid var(--state-danger-border)", borderRadius: "6px", color: "var(--status-dropped-text)", fontSize: "0.76rem", marginTop: "0.5rem" }}>
            {error}
          </div>
        )}

        <div style={btnRow}>
          <button style={btnCancel} onClick={onClose}>Cancel</button>
          <button style={btnContinue(!running)} onClick={handleContinue} disabled={running}>
            {running ? "Processing..." : "Continue"}
          </button>
        </div>
      </div>
    </div>
  );
};

export default ReprocessDialog;
