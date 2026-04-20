/**
 * ConfigurationPanel — Pre-process configuration editor for a document.
 *
 * Renders above the existing "Processing" card on a new document's Process
 * tab. Pulls the profile matched by `documentType` (or `"default"`), lets
 * the operator preview overrides via `previewPrompt`, and triggers the
 * actual process action through the parent-supplied `onProcess` callback.
 *
 * IMPORTANT: Per CC Task 3E decision (c), override PERSISTENCE is not
 * wired in this task — the backend `/process` endpoint doesn't yet accept
 * override payload. The UI makes this explicit so users aren't misled.
 */
import React, { useEffect, useState } from "react";
import {
  getProfile,
  listModels,
  listProfiles,
  listSchemas,
  listTemplates,
  LlmModel,
  previewPrompt,
  ProcessingProfile,
  PromptPreviewResponse,
  SchemaInfo,
  TemplateInfo,
} from "../../services/configApi";

// ── Styles ──────────────────────────────────────────────────────

const containerStyle: React.CSSProperties = {
  backgroundColor: "#ffffff",
  borderRadius: "8px",
  border: "1px solid #e2e8f0",
  overflow: "hidden",
  marginBottom: "1rem",
};
const headerStyle: React.CSSProperties = {
  padding: "0.6rem 0.85rem",
  fontWeight: 600,
  fontSize: "0.84rem",
  color: "#334155",
  backgroundColor: "#f8fafc",
  borderBottom: "1px solid #e2e8f0",
};
const bodyStyle: React.CSSProperties = { padding: "1rem 0.85rem" };
const fieldGrid: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(140px, auto) 1fr",
  rowGap: "0.5rem",
  columnGap: "0.85rem",
  alignItems: "center",
};
const fieldLabel: React.CSSProperties = {
  fontSize: "0.82rem",
  color: "#64748b",
  fontWeight: 500,
};
const fieldLabelModified: React.CSSProperties = {
  ...fieldLabel,
  color: "#0f172a",
  fontWeight: 700,
};
const inputStyle: React.CSSProperties = {
  width: "100%",
  padding: "0.35rem 0.55rem",
  border: "1px solid #e2e8f0",
  borderRadius: "6px",
  fontSize: "0.82rem",
  fontFamily: "inherit",
  boxSizing: "border-box",
};
const modifiedBadge: React.CSSProperties = {
  display: "inline-block",
  marginLeft: "0.4rem",
  padding: "0.05rem 0.4rem",
  fontSize: "0.68rem",
  fontWeight: 600,
  color: "#2563eb",
  backgroundColor: "#dbeafe",
  borderRadius: "4px",
};
const btnPrimary = (enabled: boolean): React.CSSProperties => ({
  padding: "0.35rem 0.85rem",
  fontSize: "0.8rem",
  fontWeight: 600,
  border: "1px solid #2563eb",
  borderRadius: "6px",
  cursor: enabled ? "pointer" : "default",
  backgroundColor: enabled ? "#2563eb" : "#e2e8f0",
  color: enabled ? "#ffffff" : "#94a3b8",
  fontFamily: "inherit",
});
const btnSecondary = (enabled: boolean): React.CSSProperties => ({
  padding: "0.35rem 0.85rem",
  fontSize: "0.8rem",
  fontWeight: 500,
  border: "1px solid #cbd5e1",
  borderRadius: "6px",
  cursor: enabled ? "pointer" : "default",
  backgroundColor: enabled ? "#ffffff" : "#f1f5f9",
  color: enabled ? "#334155" : "#94a3b8",
  fontFamily: "inherit",
});
const btnRow: React.CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  marginTop: "0.85rem",
};
const noteStyle: React.CSSProperties = {
  fontSize: "0.76rem",
  color: "#92400e",
  backgroundColor: "#fffbeb",
  border: "1px solid #fde68a",
  borderRadius: "6px",
  padding: "0.45rem 0.65rem",
  marginTop: "0.75rem",
};
const errorBox: React.CSSProperties = {
  padding: "0.6rem 0.85rem",
  backgroundColor: "#fef2f2",
  border: "1px solid #fecaca",
  borderRadius: "6px",
  color: "#991b1b",
  fontSize: "0.82rem",
  marginBottom: "0.75rem",
};
const previewContainer: React.CSSProperties = {
  marginTop: "1rem",
  border: "1px solid #e2e8f0",
  borderRadius: "6px",
  overflow: "hidden",
};
const previewHeader: React.CSSProperties = {
  padding: "0.5rem 0.75rem",
  fontSize: "0.8rem",
  fontWeight: 600,
  color: "#334155",
  backgroundColor: "#f8fafc",
  borderBottom: "1px solid #e2e8f0",
};
const previewMeta: React.CSSProperties = {
  padding: "0.5rem 0.75rem",
  fontSize: "0.78rem",
  color: "#334155",
  backgroundColor: "#fdfdfd",
  borderBottom: "1px solid #f1f5f9",
};
const previewTextarea: React.CSSProperties = {
  width: "100%",
  minHeight: "280px",
  maxHeight: "480px",
  padding: "0.6rem 0.75rem",
  border: "none",
  fontSize: "0.76rem",
  fontFamily: "ui-monospace, Menlo, monospace",
  boxSizing: "border-box",
  resize: "vertical",
};

// ── Types ───────────────────────────────────────────────────────

export interface ConfigurationPanelProps {
  documentId: string;
  documentType: string;
  /** Trigger the existing process flow. Called when "Process Document" is clicked. */
  onProcess: () => Promise<void>;
  /** Whether the parent is currently processing an action. */
  busy?: boolean;
}

/**
 * Override fields tracked in local state. Only the subset the UI exposes —
 * matches the editable dropdowns / inputs below.
 */
interface Overrides {
  profile_name?: string;
  extraction_model?: string;
  template_file?: string;
  schema_file?: string;
  chunking_mode?: string;
  chunk_size?: number | null;
  chunk_overlap?: number | null;
  max_tokens?: number;
  temperature?: number;
  run_pass2?: boolean;
}

const CHUNKING_MODES = ["chunked", "full"] as const;

// ── Component ───────────────────────────────────────────────────

const ConfigurationPanel: React.FC<ConfigurationPanelProps> = ({
  documentId,
  documentType,
  onProcess,
  busy,
}) => {
  const [baseProfile, setBaseProfile] = useState<ProcessingProfile | null>(null);
  const [profiles, setProfiles] = useState<ProcessingProfile[]>([]);
  const [models, setModels] = useState<LlmModel[]>([]);
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [schemas, setSchemas] = useState<SchemaInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);

  const [overrides, setOverrides] = useState<Overrides>({});
  const [preview, setPreview] = useState<PromptPreviewResponse | null>(null);
  const [previewBusy, setPreviewBusy] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const loadProfile = async (): Promise<ProcessingProfile | null> => {
      try {
        return await getProfile(documentType);
      } catch {
        try {
          return await getProfile("default");
        } catch {
          return null;
        }
      }
    };

    async function bootstrap() {
      setLoading(true);
      setLoadError(null);
      try {
        const [profile, allProfiles, modelsResp, templatesResp, schemasResp] =
          await Promise.all([
            loadProfile(),
            listProfiles().catch(() => ({ profiles: [] })),
            listModels().catch(() => ({ models: [] })),
            listTemplates().catch(() => ({ templates: [] })),
            listSchemas().catch(() => ({ schemas: [] })),
          ]);
        if (cancelled) return;
        setBaseProfile(profile);
        setProfiles(allProfiles.profiles);
        setModels(modelsResp.models.filter((m) => m.is_active));
        setTemplates(templatesResp.templates);
        setSchemas(schemasResp.schemas);
      } catch (e) {
        if (cancelled) return;
        setLoadError(e instanceof Error ? e.message : "Failed to load configuration");
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    bootstrap();
    return () => {
      cancelled = true;
    };
  }, [documentType]);

  /**
   * Handle profile change — switch the base profile and clear other overrides
   * so the new profile's defaults take effect.
   */
  const switchProfile = async (newName: string) => {
    if (newName === baseProfile?.name) {
      const { profile_name: _omit, ...rest } = overrides;
      void _omit;
      setOverrides(rest);
      return;
    }
    setOverrides({ profile_name: newName });
    try {
      const p = await getProfile(newName);
      setBaseProfile(p);
    } catch (e) {
      setLoadError(e instanceof Error ? e.message : "Failed to load profile");
    }
  };

  const setOverride = <K extends keyof Overrides>(key: K, value: Overrides[K]) => {
    setOverrides((cur) => {
      const next = { ...cur, [key]: value };
      return next;
    });
  };

  const isModified = <K extends keyof Overrides>(key: K): boolean =>
    overrides[key] !== undefined;

  const effective = (() => {
    const p = baseProfile;
    if (!p) return null;
    return {
      name: overrides.profile_name ?? p.name,
      extraction_model: overrides.extraction_model ?? p.extraction_model,
      template_file: overrides.template_file ?? p.template_file,
      schema_file: overrides.schema_file ?? p.schema_file,
      chunking_mode: overrides.chunking_mode ?? p.chunking_mode,
      chunk_size:
        overrides.chunk_size !== undefined
          ? overrides.chunk_size
          : p.chunk_size,
      chunk_overlap:
        overrides.chunk_overlap !== undefined
          ? overrides.chunk_overlap
          : p.chunk_overlap,
      max_tokens: overrides.max_tokens ?? p.max_tokens,
      temperature:
        overrides.temperature !== undefined
          ? overrides.temperature
          : p.temperature,
      run_pass2: overrides.run_pass2 ?? p.run_pass2,
    };
  })();

  const runPreview = async () => {
    setPreviewBusy(true);
    setPreviewError(null);
    try {
      const resp = await previewPrompt({
        document_id: documentId,
        profile_name: overrides.profile_name,
        template_file: overrides.template_file,
        schema_file: overrides.schema_file,
      });
      setPreview(resp);
    } catch (e) {
      setPreviewError(e instanceof Error ? e.message : "Preview failed");
    } finally {
      setPreviewBusy(false);
    }
  };

  if (loading) {
    return (
      <div style={containerStyle}>
        <div style={headerStyle}>Processing Configuration</div>
        <div style={{ ...bodyStyle, color: "#64748b" }}>Loading configuration...</div>
      </div>
    );
  }

  if (loadError) {
    return (
      <div style={containerStyle}>
        <div style={headerStyle}>Processing Configuration</div>
        <div style={bodyStyle}>
          <div style={errorBox}>{loadError}</div>
        </div>
      </div>
    );
  }

  if (!baseProfile || !effective) {
    return (
      <div style={containerStyle}>
        <div style={headerStyle}>Processing Configuration</div>
        <div style={bodyStyle}>
          <div style={{ color: "#64748b", fontSize: "0.84rem" }}>
            No profile found for document type "{documentType}" or "default".
            You can still process with system defaults, or create a profile in
            Admin › Profiles.
          </div>
          <div style={btnRow}>
            <button
              style={btnPrimary(!busy)}
              disabled={busy}
              onClick={() => { void onProcess(); }}
            >
              {busy ? "Starting..." : "Process Document"}
            </button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div style={containerStyle}>
      <div style={headerStyle}>Processing Configuration</div>
      <div style={bodyStyle}>
        <div style={fieldGrid}>
          <label style={isModified("profile_name") ? fieldLabelModified : fieldLabel}>
            Profile
            {isModified("profile_name") && <span style={modifiedBadge}>modified</span>}
          </label>
          <select
            style={inputStyle}
            value={effective.name}
            onChange={(e) => switchProfile(e.target.value)}
          >
            {profiles.map((p) => (
              <option key={p.name} value={p.name}>
                {p.display_name || p.name}
              </option>
            ))}
          </select>

          <label style={isModified("extraction_model") ? fieldLabelModified : fieldLabel}>
            Model
            {isModified("extraction_model") && <span style={modifiedBadge}>modified</span>}
          </label>
          <select
            style={inputStyle}
            value={effective.extraction_model}
            onChange={(e) => setOverride("extraction_model", e.target.value)}
          >
            {models.map((m) => (
              <option key={m.id} value={m.id}>
                {m.display_name} ({m.id})
              </option>
            ))}
          </select>

          <label style={isModified("template_file") ? fieldLabelModified : fieldLabel}>
            Template
            {isModified("template_file") && <span style={modifiedBadge}>modified</span>}
          </label>
          <select
            style={inputStyle}
            value={effective.template_file}
            onChange={(e) => setOverride("template_file", e.target.value)}
          >
            {templates.map((t) => (
              <option key={t.filename} value={t.filename}>
                {t.filename}
              </option>
            ))}
          </select>

          <label style={isModified("schema_file") ? fieldLabelModified : fieldLabel}>
            Schema
            {isModified("schema_file") && <span style={modifiedBadge}>modified</span>}
          </label>
          <select
            style={inputStyle}
            value={effective.schema_file}
            onChange={(e) => setOverride("schema_file", e.target.value)}
          >
            {schemas.map((s) => (
              <option key={s.filename} value={s.filename}>
                {s.filename}
              </option>
            ))}
          </select>

          <label style={isModified("chunking_mode") ? fieldLabelModified : fieldLabel}>
            Chunking
            {isModified("chunking_mode") && <span style={modifiedBadge}>modified</span>}
          </label>
          <select
            style={inputStyle}
            value={effective.chunking_mode}
            onChange={(e) => setOverride("chunking_mode", e.target.value)}
          >
            {CHUNKING_MODES.map((m) => (
              <option key={m} value={m}>
                {m === "full" ? "Full document" : "Chunked"}
              </option>
            ))}
          </select>

          <label style={isModified("max_tokens") ? fieldLabelModified : fieldLabel}>
            Max Tokens
            {isModified("max_tokens") && <span style={modifiedBadge}>modified</span>}
          </label>
          <input
            style={inputStyle}
            type="number"
            value={effective.max_tokens}
            onChange={(e) =>
              setOverride("max_tokens", Number(e.target.value) || 0)
            }
          />

          <label style={isModified("temperature") ? fieldLabelModified : fieldLabel}>
            Temperature
            {isModified("temperature") && <span style={modifiedBadge}>modified</span>}
          </label>
          <input
            style={inputStyle}
            type="number"
            step="0.1"
            value={effective.temperature ?? 0}
            onChange={(e) =>
              setOverride("temperature", Number(e.target.value) || 0)
            }
          />

          <label style={isModified("run_pass2") ? fieldLabelModified : fieldLabel}>
            Pass 2
            {isModified("run_pass2") && <span style={modifiedBadge}>modified</span>}
          </label>
          <label style={{ fontSize: "0.82rem", color: "#334155" }}>
            <input
              type="checkbox"
              checked={effective.run_pass2}
              onChange={(e) => setOverride("run_pass2", e.target.checked)}
              style={{ marginRight: "0.4rem" }}
            />
            Enable synthesis pass
          </label>
        </div>

        <div style={noteStyle}>
          Preview reflects the persisted profile + file-level overrides
          (profile / template / schema). Other parameter changes (model,
          chunking, tokens, temperature) are UI-only for now — override
          persistence requires a future backend update.
        </div>

        <div style={btnRow}>
          <button
            style={btnSecondary(!previewBusy)}
            disabled={previewBusy}
            onClick={() => { void runPreview(); }}
          >
            {previewBusy ? "Previewing..." : "Preview Prompt"}
          </button>
          <button
            style={btnPrimary(!busy)}
            disabled={busy}
            onClick={() => { void onProcess(); }}
          >
            {busy ? "Starting..." : "Process Document"}
          </button>
        </div>

        {previewError && (
          <div style={{ ...errorBox, marginTop: "0.75rem", marginBottom: 0 }}>
            {previewError}
          </div>
        )}

        {preview && (
          <div style={previewContainer}>
            <div style={previewHeader}>
              Preview — {preview.model} / {preview.chunking_mode}
            </div>
            <div style={previewMeta}>
              <div>
                Estimated input tokens:{" "}
                <strong>{preview.estimated_input_tokens.toLocaleString()}</strong>
                {preview.estimated_cost_usd != null && (
                  <>
                    {" "}· Estimated cost:{" "}
                    <strong>${preview.estimated_cost_usd.toFixed(4)}</strong>
                  </>
                )}
                {preview.estimated_cost_usd == null && (
                  <> · Cost: unavailable</>
                )}
                {preview.chunk_count != null && (
                  <> · Chunks: <strong>{preview.chunk_count}</strong></>
                )}
              </div>
              {preview.notes.length > 0 && (
                <ul style={{ margin: "0.35rem 0 0", paddingLeft: "1.1rem", color: "#64748b" }}>
                  {preview.notes.map((n, i) => (
                    <li key={i}>{n}</li>
                  ))}
                </ul>
              )}
            </div>
            <textarea
              style={previewTextarea}
              value={preview.assembled_prompt}
              readOnly
            />
          </div>
        )}
      </div>
    </div>
  );
};

export default ConfigurationPanel;
