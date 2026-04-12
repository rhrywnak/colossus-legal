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
  backgroundColor: "#ffffff", borderRadius: "12px", padding: "1.75rem",
  maxWidth: "480px", width: "90%", boxShadow: "0 20px 60px rgba(0,0,0,0.15)",
};
const optionStyle = (selected: boolean): React.CSSProperties => ({
  padding: "0.75rem 1rem", border: `2px solid ${selected ? "#2563eb" : "#e2e8f0"}`,
  borderRadius: "8px", cursor: "pointer", marginBottom: "0.5rem",
  backgroundColor: selected ? "#eff6ff" : "#ffffff",
  transition: "all 0.15s ease",
});
const optionTitle: React.CSSProperties = {
  fontSize: "0.88rem", fontWeight: 600, color: "#0f172a",
};
const optionDesc: React.CSSProperties = {
  fontSize: "0.76rem", color: "#64748b", marginTop: "0.15rem",
};
const btnRow: React.CSSProperties = {
  display: "flex", justifyContent: "flex-end", gap: "0.5rem", marginTop: "1rem",
};
const btnCancel: React.CSSProperties = {
  padding: "0.45rem 1rem", fontSize: "0.84rem", fontWeight: 500, border: "1px solid #e2e8f0",
  borderRadius: "6px", backgroundColor: "#ffffff", color: "#64748b", cursor: "pointer",
  fontFamily: "inherit",
};
const btnContinue = (enabled: boolean): React.CSSProperties => ({
  padding: "0.45rem 1rem", fontSize: "0.84rem", fontWeight: 600, border: "none",
  borderRadius: "6px", backgroundColor: enabled ? "#2563eb" : "#94a3b8",
  color: "#ffffff", cursor: enabled ? "pointer" : "default", fontFamily: "inherit",
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
        <h2 style={{ margin: "0 0 1rem", fontSize: "1.1rem", fontWeight: 700, color: "#0f172a" }}>
          Re-process Document
        </h2>
        <p style={{ fontSize: "0.84rem", color: "#64748b", marginBottom: "1rem" }}>
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
          <div style={{ padding: "0.5rem 0.75rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca", borderRadius: "6px", color: "#991b1b", fontSize: "0.76rem", marginTop: "0.5rem" }}>
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
