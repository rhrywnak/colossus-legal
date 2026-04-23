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
  getDocumentConfig,
  getProfile,
  listModels,
  listProfiles,
  listSchemas,
  listTemplates,
  LlmModel,
  patchDocumentConfig,
  PatchConfigInput,
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
  /**
   * Raw backend status — e.g. `"NEW"`, `"TEXT_EXTRACTED"`. The Preview
   * button is gated on this because the preview endpoint needs
   * `document_text` pages and returns an error before ExtractText runs.
   */
  documentStatus?: string;
  /** PDF classification fields (populated at upload time). Absent on
   *  legacy rows predating the classifier — the panel renders no
   *  content line in that case. */
  contentType?: string;
  pageCount?: number;
  textPages?: number;
  scannedPages?: number;
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
  /** Pass-2 relationship-extraction model. `undefined` means "unchanged". */
  pass2_extraction_model?: string;
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

/**
 * Seed the panel's `overrides` state from the per-document pipeline_config
 * row, skipping any field whose value matches the currently-loaded profile.
 *
 * At upload time the backend auto-populates pipeline_config from the
 * matched profile, so a freshly-uploaded doc's DB values equal the
 * profile's values. Marking every matching field as "modified" would be
 * wrong — `isModified()` means "user has overridden the profile". Only
 * fields whose DB value genuinely differs from the profile belong here.
 *
 * `system_prompt_file` is in the DB payload but not tracked by this panel
 * (no UI surface yet); it's ignored. `schema_file` is tracked in the
 * panel's `Overrides` but has no corresponding pipeline_config column, so
 * the DB payload never sets it.
 */
function diffConfigFromProfile(
  docConfig: PatchConfigInput | null,
  profile: ProcessingProfile,
): Overrides {
  if (!docConfig) return {};
  const out: Overrides = {};
  if (docConfig.profile_name != null && docConfig.profile_name !== profile.name) {
    out.profile_name = docConfig.profile_name;
  }
  if (
    docConfig.extraction_model != null &&
    docConfig.extraction_model !== profile.extraction_model
  ) {
    out.extraction_model = docConfig.extraction_model;
  }
  if (
    docConfig.pass2_extraction_model != null &&
    docConfig.pass2_extraction_model !== profile.pass2_extraction_model
  ) {
    out.pass2_extraction_model = docConfig.pass2_extraction_model;
  }
  if (
    docConfig.template_file != null &&
    docConfig.template_file !== profile.template_file
  ) {
    out.template_file = docConfig.template_file;
  }
  if (
    docConfig.chunking_mode != null &&
    docConfig.chunking_mode !== profile.chunking_mode
  ) {
    out.chunking_mode = docConfig.chunking_mode;
  }
  if (
    docConfig.chunk_size != null &&
    docConfig.chunk_size !== profile.chunk_size
  ) {
    out.chunk_size = docConfig.chunk_size;
  }
  if (
    docConfig.chunk_overlap != null &&
    docConfig.chunk_overlap !== profile.chunk_overlap
  ) {
    out.chunk_overlap = docConfig.chunk_overlap;
  }
  if (
    docConfig.max_tokens != null &&
    docConfig.max_tokens !== profile.max_tokens
  ) {
    out.max_tokens = docConfig.max_tokens;
  }
  if (
    docConfig.temperature != null &&
    docConfig.temperature !== profile.temperature
  ) {
    out.temperature = docConfig.temperature;
  }
  if (
    docConfig.run_pass2 != null &&
    docConfig.run_pass2 !== profile.run_pass2
  ) {
    out.run_pass2 = docConfig.run_pass2;
  }
  return out;
}

// ── Component ───────────────────────────────────────────────────

/**
 * Format the PDF-classification summary for the top of the panel.
 * Returns `null` when no classification data is available.
 */
function formatContentLine(
  contentType?: string,
  pageCount?: number,
  textPages?: number,
  scannedPages?: number,
): string | null {
  if (!contentType) return null;
  switch (contentType) {
    case "text_based":
      return pageCount != null
        ? `${pageCount} text-based page${pageCount === 1 ? "" : "s"}`
        : "Text-based document";
    case "scanned":
      return pageCount != null
        ? `Scanned document (${pageCount} page${pageCount === 1 ? "" : "s"} need OCR)`
        : "Scanned document (OCR required)";
    case "mixed":
      return `Mixed (${textPages ?? 0} text, ${scannedPages ?? 0} scanned)`;
    case "unknown":
      return "Content type unknown";
    default:
      return null;
  }
}

const contentInfoStyle: React.CSSProperties = {
  fontSize: "0.82rem",
  color: "#334155",
  marginBottom: "0.85rem",
  padding: "0.5rem 0.75rem",
  backgroundColor: "#f8fafc",
  border: "1px solid #e2e8f0",
  borderRadius: "6px",
};

const ConfigurationPanel: React.FC<ConfigurationPanelProps> = ({
  documentId,
  documentType,
  documentStatus,
  contentType,
  pageCount,
  textPages,
  scannedPages,
  onProcess,
  busy,
}) => {
  // Preview calls previewPrompt, which reads document_text pages. Before
  // ExtractText runs those pages don't exist yet, so previewing from the
  // NEW state errors out. The button stays visible but disabled with a
  // small hint; Process Document stays enabled (it runs ExtractText as
  // the first step of the pipeline).
  const previewDisabled = documentStatus === "NEW";
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

  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

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
        const [profile, allProfiles, modelsResp, templatesResp, schemasResp, docConfig] =
          await Promise.all([
            loadProfile(),
            listProfiles().catch(() => ({ profiles: [] })),
            listModels().catch(() => ({ models: [] })),
            listTemplates().catch(() => ({ templates: [] })),
            listSchemas().catch(() => ({ schemas: [] })),
            getDocumentConfig(documentId).catch(() => null),
          ]);
        if (cancelled) return;
        setBaseProfile(profile);
        setProfiles(allProfiles.profiles);
        setModels(modelsResp.models.filter((m) => m.is_active));
        setTemplates(templatesResp.templates);
        setSchemas(schemasResp.schemas);
        if (profile) {
          setOverrides(diffConfigFromProfile(docConfig, profile));
        }
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
  }, [documentType, documentId]);

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
    // Pass-2 model resolution mirrors the backend's fall-back chain:
    //   user override → profile's pass2_extraction_model → pass-1 model.
    // If a user picks a model here it becomes an override; leaving it
    // alone keeps backend behavior consistent with the profile default.
    const pass1Model = overrides.extraction_model ?? p.extraction_model;
    const pass2Model =
      overrides.pass2_extraction_model
        ?? p.pass2_extraction_model
        ?? pass1Model;
    return {
      name: overrides.profile_name ?? p.name,
      extraction_model: pass1Model,
      pass2_extraction_model: pass2Model,
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

  /**
   * Build the PATCH payload from the user-tracked overrides. Only fields
   * the user actually changed are included — the server treats omitted
   * fields as "preserve existing column value".
   */
  const buildPatchInput = (): PatchConfigInput => {
    const out: PatchConfigInput = {};
    if (overrides.profile_name !== undefined) out.profile_name = overrides.profile_name;
    if (overrides.extraction_model !== undefined) out.extraction_model = overrides.extraction_model;
    if (overrides.pass2_extraction_model !== undefined) {
      out.pass2_extraction_model = overrides.pass2_extraction_model;
    }
    if (overrides.template_file !== undefined) out.template_file = overrides.template_file;
    if (overrides.schema_file !== undefined) {
      // schema_file is not yet a pipeline_config override column — skip it
      // silently; schema switching must go via the profile.
    }
    if (overrides.chunking_mode !== undefined) out.chunking_mode = overrides.chunking_mode;
    if (overrides.chunk_size !== undefined) out.chunk_size = overrides.chunk_size;
    if (overrides.chunk_overlap !== undefined) out.chunk_overlap = overrides.chunk_overlap;
    if (overrides.max_tokens !== undefined) out.max_tokens = overrides.max_tokens;
    if (overrides.temperature !== undefined) out.temperature = overrides.temperature;
    if (overrides.run_pass2 !== undefined) out.run_pass2 = overrides.run_pass2;
    return out;
  };

  const saveAndProcess = async () => {
    setSaveError(null);
    const payload = buildPatchInput();
    const hasChanges = Object.keys(payload).length > 0;
    if (hasChanges) {
      setSaving(true);
      try {
        await patchDocumentConfig(documentId, payload);
      } catch (e) {
        setSaveError(e instanceof Error ? e.message : "Failed to save configuration");
        setSaving(false);
        return;
      }
      setSaving(false);
    }
    await onProcess();
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
              style={btnPrimary(!busy && !saving)}
              disabled={busy || saving}
              onClick={() => { void saveAndProcess(); }}
            >
              {saving
                ? "Saving configuration..."
                : busy
                  ? "Starting..."
                  : "Process Document"}
            </button>
          </div>
          {saveError && (
            <div style={{ ...errorBox, marginTop: "0.75rem", marginBottom: 0 }}>
              {saveError}
            </div>
          )}
        </div>
      </div>
    );
  }

  return (
    <div style={containerStyle}>
      <div style={headerStyle}>Processing Configuration</div>
      <div style={bodyStyle}>
        {(() => {
          const line = formatContentLine(contentType, pageCount, textPages, scannedPages);
          return line ? (
            <div style={contentInfoStyle}>
              <strong>Content:</strong> {line}
            </div>
          ) : null;
        })()}

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
                {m.display_name}
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
          <div>
            <label style={{ fontSize: "0.82rem", color: "#334155" }}>
              <input
                type="checkbox"
                checked={effective.run_pass2}
                onChange={(e) => setOverride("run_pass2", e.target.checked)}
                style={{ marginRight: "0.4rem" }}
              />
              Enable synthesis pass
            </label>
            {effective.run_pass2 && (
              <div style={{ marginTop: "0.5rem" }}>
                <label
                  style={
                    isModified("pass2_extraction_model")
                      ? { ...fieldLabelModified, marginBottom: "0.25rem" }
                      : { ...fieldLabel, marginBottom: "0.25rem" }
                  }
                >
                  Pass 2 Model
                  {isModified("pass2_extraction_model") && (
                    <span style={modifiedBadge}>modified</span>
                  )}
                </label>
                <select
                  style={inputStyle}
                  value={effective.pass2_extraction_model}
                  onChange={(e) =>
                    setOverride("pass2_extraction_model", e.target.value)
                  }
                >
                  {models.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.display_name}
                    </option>
                  ))}
                </select>
                <div
                  style={{
                    marginTop: "0.25rem",
                    fontSize: "0.75rem",
                    color: "#64748b",
                  }}
                >
                  Defaults to the Pass 1 model when the profile doesn't set one.
                </div>
              </div>
            )}
          </div>
        </div>

        <div style={btnRow}>
          <button
            style={btnSecondary(!previewBusy && !saving && !previewDisabled)}
            disabled={previewBusy || saving || previewDisabled}
            onClick={() => { void runPreview(); }}
            title={previewDisabled ? "Preview available after text extraction." : undefined}
          >
            {previewBusy ? "Previewing..." : "Preview Prompt"}
          </button>
          <button
            style={btnPrimary(!busy && !saving)}
            disabled={busy || saving}
            onClick={() => { void saveAndProcess(); }}
          >
            {saving
              ? "Saving configuration..."
              : busy
                ? "Starting..."
                : "Process Document"}
          </button>
        </div>
        {previewDisabled && (
          <div style={{ marginTop: "0.4rem", fontSize: "0.76rem", color: "#64748b" }}>
            Preview available after text extraction.
          </div>
        )}

        {saveError && (
          <div style={{ ...errorBox, marginTop: "0.75rem", marginBottom: 0 }}>
            {saveError}
          </div>
        )}

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
