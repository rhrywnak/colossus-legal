/**
 * ExtractionConfigDialog — Select schema/model/instructions before
 * triggering LLM extraction. Fetches options from the backend on mount
 * and returns a config object the caller posts to /documents/:id/extract.
 */
import React, { useEffect, useState } from "react";
import {
  fetchSchemas, fetchModels, SchemaInfo, ModelInfo,
} from "../../services/pipelineApi";

interface ExtractionConfigDialogProps {
  documentId: string;
  currentSchemaFile?: string;
  onSubmit: (config: Record<string, unknown>) => void;
  onCancel: () => void;
}

// ── Styles ──────────────────────────────────────────────────────

const S = {
  overlay: {
    position: "fixed", top: 0, left: 0, right: 0, bottom: 0,
    backgroundColor: "rgba(0, 0, 0, 0.5)", display: "flex",
    alignItems: "center", justifyContent: "center", zIndex: 9999,
  } as React.CSSProperties,
  dialog: {
    backgroundColor: "#fff", borderRadius: "8px", padding: "1.5rem",
    maxWidth: "560px", width: "90%", maxHeight: "85vh", overflowY: "auto",
    boxShadow: "0 4px 24px rgba(0,0,0,0.2)",
  } as React.CSSProperties,
  title: {
    fontSize: "1.1rem", fontWeight: 700, color: "#0f172a", marginBottom: "0.75rem",
  } as React.CSSProperties,
  label: {
    display: "block", fontSize: "0.8rem", fontWeight: 600, color: "#475569",
    marginBottom: "0.3rem",
  } as React.CSSProperties,
  hint: {
    fontSize: "0.72rem", color: "#64748b", marginTop: "0.2rem",
  } as React.CSSProperties,
  select: {
    width: "100%", padding: "0.45rem 0.6rem", fontSize: "0.84rem",
    border: "1px solid #cbd5e1", borderRadius: "4px", fontFamily: "inherit",
    boxSizing: "border-box", backgroundColor: "#fff",
  } as React.CSSProperties,
  textarea: {
    width: "100%", padding: "0.45rem 0.6rem", fontSize: "0.84rem",
    border: "1px solid #cbd5e1", borderRadius: "4px", fontFamily: "inherit",
    boxSizing: "border-box", resize: "vertical" as const, minHeight: "70px",
  } as React.CSSProperties,
  fieldGroup: { marginBottom: "1rem" } as React.CSSProperties,
  buttons: {
    display: "flex", justifyContent: "flex-end", gap: "0.5rem", marginTop: "1.25rem",
  } as React.CSSProperties,
  cancelBtn: {
    padding: "0.4rem 1rem", fontSize: "0.84rem", fontWeight: 500,
    border: "1px solid #cbd5e1", borderRadius: "4px", backgroundColor: "#fff",
    color: "#334155", cursor: "pointer", fontFamily: "inherit",
  } as React.CSSProperties,
  submitBtn: (enabled: boolean) => ({
    padding: "0.4rem 1rem", fontSize: "0.84rem", fontWeight: 600,
    border: "none", borderRadius: "4px",
    backgroundColor: enabled ? "#2563eb" : "#94a3b8",
    color: "#fff", cursor: enabled ? "pointer" : "not-allowed",
    fontFamily: "inherit",
  } as React.CSSProperties),
  loading: {
    padding: "1.5rem", textAlign: "center" as const,
    color: "#64748b", fontSize: "0.84rem",
  } as React.CSSProperties,
  error: {
    padding: "0.5rem 0.75rem", backgroundColor: "#fef2f2",
    border: "1px solid #fecaca", borderRadius: "4px", color: "#991b1b",
    fontSize: "0.76rem", marginBottom: "0.75rem",
  } as React.CSSProperties,
};

// ── Component ───────────────────────────────────────────────────

const ExtractionConfigDialog: React.FC<ExtractionConfigDialogProps> = ({
  currentSchemaFile, onSubmit, onCancel,
}) => {
  const [schemas, setSchemas] = useState<SchemaInfo[]>([]);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [schemaFile, setSchemaFile] = useState(currentSchemaFile ?? "");
  const [modelId, setModelId] = useState("");
  const [instructions, setInstructions] = useState("");

  useEffect(() => {
    let cancelled = false;
    Promise.all([fetchSchemas(), fetchModels()])
      .then(([sch, mod]) => {
        if (cancelled) return;
        setSchemas(sch);
        setModels(mod);
        if (!currentSchemaFile && sch.length > 0) {
          setSchemaFile(sch[0].filename);
        }
      })
      .catch((e) => {
        if (!cancelled) setLoadError(e instanceof Error ? e.message : "Failed to load options");
      })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [currentSchemaFile]);

  const selectedModel = models.find((m) => m.id === modelId);
  const canSubmit = !loading && schemaFile.length > 0;

  const handleSubmit = () => {
    if (!canSubmit) return;
    const config: Record<string, unknown> = { schema_file: schemaFile };
    if (modelId) config.model = modelId;
    if (instructions.trim()) config.admin_instructions = instructions.trim();
    onSubmit(config);
  };

  return (
    <div style={S.overlay} onClick={onCancel}>
      <div style={S.dialog} onClick={(e) => e.stopPropagation()}>
        <div style={S.title}>Run Extraction</div>

        {loadError && <div style={S.error}>{loadError}</div>}
        {loading ? (
          <div style={S.loading}>Loading options...</div>
        ) : (
          <>
            <div style={S.fieldGroup}>
              <label style={S.label}>Schema (required)</label>
              <select
                style={S.select}
                value={schemaFile}
                onChange={(e) => setSchemaFile(e.target.value)}
              >
                <option value="">— Select a schema —</option>
                {schemas.map((s) => (
                  <option key={s.filename} value={s.filename}>
                    {s.document_type} — {s.filename} ({s.entity_type_count} types)
                  </option>
                ))}
              </select>
              {schemaFile && (() => {
                const s = schemas.find((x) => x.filename === schemaFile);
                return s ? <div style={S.hint}>{s.description}</div> : null;
              })()}
            </div>

            <div style={S.fieldGroup}>
              <label style={S.label}>Model (optional override)</label>
              <select
                style={S.select}
                value={modelId}
                onChange={(e) => setModelId(e.target.value)}
              >
                <option value="">Use default</option>
                {models.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.display_name} ({m.provider})
                  </option>
                ))}
              </select>
              {selectedModel && (
                <div style={S.hint}>
                  ${selectedModel.input_cost_per_mtok.toFixed(2)}/MTok in,
                  ${selectedModel.output_cost_per_mtok.toFixed(2)}/MTok out
                </div>
              )}
            </div>

            <div style={S.fieldGroup}>
              <label style={S.label}>Custom instructions (optional)</label>
              <textarea
                style={S.textarea}
                placeholder="Additional guidance for the LLM..."
                value={instructions}
                onChange={(e) => setInstructions(e.target.value)}
              />
            </div>
          </>
        )}

        <div style={S.buttons}>
          <button style={S.cancelBtn} onClick={onCancel}>Cancel</button>
          <button
            style={S.submitBtn(canSubmit)}
            disabled={!canSubmit}
            onClick={handleSubmit}
          >
            Run Extraction
          </button>
        </div>
      </div>
    </div>
  );
};

export default ExtractionConfigDialog;
