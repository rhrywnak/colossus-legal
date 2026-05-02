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
import React, { useEffect, useMemo, useState } from "react";
import {
  getDocumentConfig,
  getProfile,
  listModels,
  listProfiles,
  listSchemas,
  listTemplates,
  LlmModel,
  patchDocumentConfig,
  previewPrompt,
  ProcessingProfile,
  PromptPreviewResponse,
  SchemaInfo,
  TemplateInfo,
} from "../../services/configApi";
import {
  buildPatchInput,
  diffConfigFromProfile,
  isMapKeyModified,
  Overrides,
  resolveClientSide,
  TOOLTIPS,
  truncateHash,
} from "./configurationPanelHelpers";

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

const subKeyEditorContainer: React.CSSProperties = {
  marginTop: "0.85rem",
  padding: "0.6rem 0.75rem",
  border: "1px solid #e2e8f0",
  borderRadius: "6px",
  backgroundColor: "#fafbfc",
};
const subKeyEditorHeader: React.CSSProperties = {
  fontSize: "0.78rem",
  fontWeight: 600,
  color: "#334155",
  marginBottom: "0.4rem",
};
const subKeyRow: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "minmax(140px, auto) 1fr auto",
  alignItems: "center",
  columnGap: "0.6rem",
  rowGap: "0.4rem",
};
const resetBtn: React.CSSProperties = {
  padding: "0.15rem 0.5rem",
  fontSize: "0.72rem",
  border: "1px solid #cbd5e1",
  borderRadius: "4px",
  backgroundColor: "#ffffff",
  color: "#64748b",
  cursor: "pointer",
  fontFamily: "inherit",
};
const resetBtnHidden: React.CSSProperties = {
  ...resetBtn,
  visibility: "hidden",
};
const resolvedSectionStyle: React.CSSProperties = {
  marginTop: "1rem",
  border: "1px solid #e2e8f0",
  borderRadius: "6px",
  backgroundColor: "#fafbfc",
};
const resolvedSummaryStyle: React.CSSProperties = {
  padding: "0.5rem 0.75rem",
  cursor: "pointer",
  fontSize: "0.82rem",
  fontWeight: 600,
  color: "#334155",
  userSelect: "none",
};
const resolvedBodyStyle: React.CSSProperties = {
  padding: "0.6rem 0.85rem",
  borderTop: "1px solid #e2e8f0",
  fontSize: "0.78rem",
  color: "#334155",
  lineHeight: 1.5,
};
const resolvedKey: React.CSSProperties = {
  display: "inline-block",
  minWidth: "150px",
  color: "#64748b",
};
const resolvedHashStyle: React.CSSProperties = {
  fontFamily: "ui-monospace, Menlo, monospace",
  fontSize: "0.74rem",
  color: "#0f172a",
  padding: "0 0.25rem",
  backgroundColor: "#f1f5f9",
  borderRadius: "3px",
};
const resolvedMapStyle: React.CSSProperties = {
  margin: "0.25rem 0 0.6rem 0.5rem",
  padding: "0.4rem 0.6rem",
  backgroundColor: "#ffffff",
  border: "1px solid #e2e8f0",
  borderRadius: "4px",
  fontFamily: "ui-monospace, Menlo, monospace",
  fontSize: "0.74rem",
  whiteSpace: "pre-wrap" as const,
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

const CHUNKING_MODES = ["full", "structured", "chunked"] as const;

const CHUNKING_MODE_LABELS: Record<(typeof CHUNKING_MODES)[number], string> = {
  full: "Full document",
  structured: "Structured",
  chunked: "Legacy Chunked",
};

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

// ── Sub-components ──────────────────────────────────────────────

/**
 * Editor for one of the profile's `Record<string, unknown>` config
 * maps (chunking_config or context_config). One labeled input per
 * key the profile declares; type-aware (number for numeric defaults,
 * checkbox for booleans, text otherwise).
 *
 * The `mode` sub-key is rendered read-only when present — operators
 * change the chunking mode via the main "Chunking" dropdown above
 * (which dual-writes both the legacy `chunking_mode` field and
 * `chunking_config.mode`). Letting them edit `mode` here would be a
 * second-class entry path that drifts from the dropdown.
 *
 * ## Rust Learning analogue
 *
 * `Record<string, unknown>` is TS's parallel to Rust's
 * `HashMap<String, serde_json::Value>` — the value type is "anything
 * JSON-shaped." We narrow at render time based on the *profile's*
 * default type for each key (the operator can't change the type).
 */
const SubKeyEditor: React.FC<{
  label: string;
  profileMap: Record<string, unknown>;
  overrideMap: Record<string, unknown> | undefined;
  onSetKey: (key: string, value: unknown) => void;
  onClearKey: (key: string) => void;
}> = ({ label, profileMap, overrideMap, onSetKey, onClearKey }) => {
  const keys = Object.keys(profileMap).sort();
  return (
    <div style={subKeyEditorContainer}>
      <div style={subKeyEditorHeader}>{label}</div>
      <div style={subKeyRow}>
        {keys.map((key) => {
          const isOverridden =
            overrideMap !== undefined &&
            Object.prototype.hasOwnProperty.call(overrideMap, key);
          const profileValue = profileMap[key];
          const effectiveValue = isOverridden ? overrideMap![key] : profileValue;
          const labelStyle: React.CSSProperties = isOverridden
            ? fieldLabelModified
            : fieldLabel;

          // The legacy "mode" sub-key is owned by the main Chunking
          // dropdown (which dual-writes). Render read-only here so
          // operators have one clear entry path.
          const isReadOnlyMode = key === "mode";

          // Pick widget shape from the *profile's* value type.
          const isNumber = typeof profileValue === "number";
          const isBool = typeof profileValue === "boolean";

          return (
            <React.Fragment key={key}>
              <label style={labelStyle}>
                {key}
                {isOverridden && !isReadOnlyMode && (
                  <span style={modifiedBadge}>modified</span>
                )}
              </label>
              {isReadOnlyMode ? (
                <input
                  style={{ ...inputStyle, opacity: 0.6, cursor: "not-allowed" }}
                  type="text"
                  value={String(effectiveValue ?? "")}
                  readOnly
                  disabled
                  aria-label={`${key} (set via Chunking dropdown above)`}
                  title="Edit via the Chunking dropdown above (kept in sync with the legacy chunking_mode field)."
                />
              ) : isBool ? (
                <input
                  type="checkbox"
                  aria-label={key}
                  checked={Boolean(effectiveValue)}
                  onChange={(e) => onSetKey(key, e.target.checked)}
                />
              ) : isNumber ? (
                <input
                  style={inputStyle}
                  type="number"
                  step={1}
                  aria-label={key}
                  value={Number(effectiveValue ?? 0)}
                  onChange={(e) => onSetKey(key, Number(e.target.value))}
                />
              ) : (
                <input
                  style={inputStyle}
                  type="text"
                  aria-label={key}
                  value={String(effectiveValue ?? "")}
                  onChange={(e) => onSetKey(key, e.target.value)}
                />
              )}
              {isReadOnlyMode ? (
                <span style={resetBtnHidden} aria-hidden="true">
                  Reset
                </span>
              ) : (
                <button
                  type="button"
                  style={isOverridden ? resetBtn : resetBtnHidden}
                  onClick={() => onClearKey(key)}
                  title={TOOLTIPS.resetSubKey}
                  aria-label={`Reset ${key} to profile default`}
                  disabled={!isOverridden}
                >
                  Reset
                </button>
              )}
            </React.Fragment>
          );
        })}
      </div>
    </div>
  );
};

/**
 * Collapsed-by-default audit-trail section. Shows the fully-resolved
 * view of what will run (per [`resolveClientSide`]) including content
 * hashes for audit reproducibility — Gap 4 / Gap 5 / Gap 11
 * fingerprints surface here so an operator can verify before
 * processing.
 *
 * Hashes display as the leading 8 hex chars with the full value on a
 * `title` tooltip — operators don't usually need the full 64 chars,
 * but they're one hover away when they do.
 */
const ResolvedConfigSection: React.FC<{
  resolved: import("./configurationPanelHelpers").ResolvedView;
}> = ({ resolved }) => {
  const formatMap = (m: Record<string, unknown>): string =>
    Object.keys(m).length === 0 ? "(empty)" : JSON.stringify(m, null, 2);
  return (
    <details style={resolvedSectionStyle}>
      <summary style={resolvedSummaryStyle} title={TOOLTIPS.resolvedSection}>
        Resolved Configuration (audit-trail view)
      </summary>
      <div style={resolvedBodyStyle}>
        <div>
          <span style={resolvedKey}>profile_name:</span>
          {resolved.profile_name}{" "}
          <span style={resolvedHashStyle} title={resolved.profile_hash}>
            {truncateHash(resolved.profile_hash) || "(no hash)"}
          </span>
        </div>
        <div>
          <span style={resolvedKey}>pass1_template:</span>
          {resolved.template_file}
        </div>
        <div>
          <span style={resolvedKey}>pass2_template:</span>
          {resolved.run_pass2
            ? resolved.pass2_template_file ?? "(none — Pass 2 will fail)"
            : "(Pass 2 disabled)"}
        </div>
        <div>
          <span style={resolvedKey}>system_prompt:</span>
          {resolved.system_prompt_file ?? "(none)"}
        </div>
        <div>
          <span style={resolvedKey}>global_rules:</span>
          {resolved.global_rules_file ?? "(none)"}
        </div>
        <div>
          <span style={resolvedKey}>schema:</span>
          {resolved.schema_file}
        </div>
        <div>
          <span style={resolvedKey}>pass1_model:</span>
          {resolved.model}
        </div>
        <div>
          <span style={resolvedKey}>pass2_model:</span>
          {resolved.run_pass2
            ? resolved.pass2_model ?? `${resolved.model} (reuses Pass-1)`
            : "(Pass 2 disabled)"}
        </div>
        <div>
          <span style={resolvedKey}>chunking_mode:</span>
          {resolved.chunking_mode}
        </div>
        <div>
          <span style={resolvedKey}>chunking_config:</span>
        </div>
        <div style={resolvedMapStyle}>{formatMap(resolved.chunking_config)}</div>
        <div>
          <span style={resolvedKey}>context_config:</span>
        </div>
        <div style={resolvedMapStyle}>{formatMap(resolved.context_config)}</div>
        <div>
          <span style={resolvedKey}>max_tokens:</span>
          {resolved.max_tokens}
        </div>
        <div>
          <span style={resolvedKey}>temperature:</span>
          {resolved.temperature}
        </div>
      </div>
    </details>
  );
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

  /**
   * Set a single sub-key of the chunking_config override map.
   *
   * The operator edits, say, `units_per_chunk = 3` in the sub-key
   * editor. We add the entry to `overrides.chunking_config` so it
   * round-trips into the PATCH payload and the backend's resolver
   * sees a per-document override on just that key (other keys still
   * inherit from the profile).
   */
  const setChunkingSubKey = (key: string, value: unknown) => {
    setOverrides((cur) => ({
      ...cur,
      chunking_config: { ...(cur.chunking_config ?? {}), [key]: value },
    }));
  };

  /**
   * Clear a single sub-key from the chunking_config override map —
   * the panel's "Reset to profile default" button.
   *
   * If clearing this key empties the map, we leave the empty `{}`
   * behind in `Overrides`; `buildPatchInput` sends `null` in that
   * case so the column resets to NULL (full re-inherit from the
   * profile). The empty-vs-undefined distinction matches the
   * backend's three-state contract documented on
   * `PipelineConfigOverrides.chunking_config`.
   *
   * ## Why this dance instead of `null` per-key
   *
   * The PATCH endpoint uses COALESCE to preserve unchanged columns,
   * which means there's no JSON-Merge-Patch-style "delete this key"
   * primitive. To clear a single key while preserving others, the
   * panel must compute the new full override map and send it whole.
   * Tracked as tech debt; eventual fix is RFC 7396 JSON Merge Patch.
   */
  const clearChunkingSubKey = (key: string) => {
    setOverrides((cur) => {
      const map = { ...(cur.chunking_config ?? {}) };
      delete map[key];
      return { ...cur, chunking_config: map };
    });
  };

  /**
   * Chunking-mode dropdown handler — dual-write to both the legacy
   * `chunking_mode` field and the newer `chunking_config.mode` key.
   *
   * Why the duplicate write: the runtime resolver
   * (`backend/src/pipeline/steps/llm_extract.rs:resolve_effective_mode`)
   * prefers `chunking_config["mode"]` when present, falling back to
   * `chunking_mode`. If we wrote only the legacy field, the runtime
   * could silently disagree with the operator's selection (Gap 7 in
   * the audit). Writing both keeps them in lockstep until a future
   * cleanup removes the legacy `chunking_mode` column entirely.
   */
  const setChunkingMode = (mode: string) => {
    setOverrides((cur) => ({
      ...cur,
      chunking_mode: mode,
      chunking_config: { ...(cur.chunking_config ?? {}), mode },
    }));
  };

  /**
   * The fully resolved view of `profile + overrides` — what would
   * actually run if the operator clicks Process now. Memoised so the
   * audit-trail section and the field widgets read from the same
   * value.
   *
   * `null` when no profile loaded — the "no profile" branch below
   * handles that case.
   */
  const resolved = useMemo(
    () => (baseProfile ? resolveClientSide(baseProfile, overrides) : null),
    [baseProfile, overrides],
  );

  const runPreview = async () => {
    setPreviewBusy(true);
    setPreviewError(null);
    try {
      const resp = await previewPrompt({
        document_id: documentId,
        profile_name: overrides.profile_name,
        template_file: overrides.template_file,
        // Schema is profile-level (Gap 8 — disabled UI); pass the
        // profile's value so the preview matches what will run.
        schema_file: baseProfile?.schema_file,
      });
      setPreview(resp);
    } catch (e) {
      setPreviewError(e instanceof Error ? e.message : "Preview failed");
    } finally {
      setPreviewBusy(false);
    }
  };

  const saveAndProcess = async () => {
    setSaveError(null);
    const payload = buildPatchInput(overrides);
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

  if (!baseProfile || !resolved) {
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
            aria-label="Profile"
            value={resolved.profile_name}
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
            aria-label="Pass-1 extraction model"
            value={resolved.model}
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
            aria-label="Pass-1 template"
            value={resolved.template_file}
            onChange={(e) => setOverride("template_file", e.target.value)}
          >
            {templates.map((t) => (
              <option key={t.filename} value={t.filename}>
                {t.filename}
              </option>
            ))}
          </select>

          {/*
           * Schema dropdown — disabled per Gap 8 in
           * AUDIT_PIPELINE_CONFIG_GAPS.md. Pre-Instruction-D the
           * widget rendered editable but the PATCH builder silently
           * dropped the value — a lie about acceptance. Now disabled
           * with a tooltip explaining the profile-level constraint.
           */}
          <label style={fieldLabel}>Schema</label>
          <select
            style={{ ...inputStyle, opacity: 0.6, cursor: "not-allowed" }}
            aria-label="Schema (read-only — profile-level only)"
            value={resolved.schema_file}
            disabled
            title={TOOLTIPS.schemaFileDisabled}
            onChange={() => {
              /* disabled — no-op */
            }}
          >
            {schemas.map((s) => (
              <option key={s.filename} value={s.filename}>
                {s.filename}
              </option>
            ))}
            {/* The profile's schema_file may not be in the dropdown's
                option list when the schemas API hasn't loaded yet or
                when the profile references a schema that has been
                renamed. Add it as a fallback option so the disabled
                widget always displays the correct value. */}
            {!schemas.some((s) => s.filename === resolved.schema_file) && (
              <option value={resolved.schema_file}>{resolved.schema_file}</option>
            )}
          </select>

          <label style={isModified("chunking_mode") ? fieldLabelModified : fieldLabel}>
            Chunking
            {isModified("chunking_mode") && <span style={modifiedBadge}>modified</span>}
          </label>
          <select
            style={inputStyle}
            aria-label="Chunking mode"
            value={resolved.chunking_mode}
            onChange={(e) => setChunkingMode(e.target.value)}
          >
            {CHUNKING_MODES.map((m) => (
              <option key={m} value={m}>
                {CHUNKING_MODE_LABELS[m]}
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
            min={1}
            step={1}
            aria-label="Max output tokens per LLM call"
            value={resolved.max_tokens}
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
            min={0}
            max={2}
            aria-label="LLM temperature"
            value={resolved.temperature ?? 0}
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
                checked={resolved.run_pass2}
                onChange={(e) => setOverride("run_pass2", e.target.checked)}
                style={{ marginRight: "0.4rem" }}
                aria-label="Enable Pass 2 synthesis"
              />
              Enable synthesis pass
            </label>
            {resolved.run_pass2 && (
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
                  aria-label="Pass-2 extraction model"
                  value={resolved.pass2_model ?? resolved.model}
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

                {/*
                 * Pass-2 template — read-only display per Gap 2 +
                 * Roman's Path 2 decision. The profile sets the
                 * Pass-2 template; per-document override is
                 * deliberately not plumbed (would require migration
                 * + repo plumbing — separate instruction if needed).
                 * Visibility was the audit's stated concern.
                 */}
                <label
                  style={{ ...fieldLabel, marginTop: "0.6rem", marginBottom: "0.25rem" }}
                >
                  Pass 2 Template
                </label>
                <input
                  style={{ ...inputStyle, opacity: 0.6, cursor: "not-allowed" }}
                  type="text"
                  readOnly
                  disabled
                  aria-label="Pass-2 template (read-only — profile-level only)"
                  title={TOOLTIPS.pass2TemplateReadOnly}
                  value={resolved.pass2_template_file ?? "(none configured)"}
                />
                <div
                  style={{
                    marginTop: "0.25rem",
                    fontSize: "0.75rem",
                    color: "#64748b",
                  }}
                >
                  Set by the profile. To change, edit the profile YAML
                  or use a different profile.
                </div>
              </div>
            )}
          </div>
        </div>

        {/*
         * §2C: chunking_config sub-key editor.
         *
         * Renders one input per key the profile's chunking_config
         * declares. Each input is type-aware (number for numeric
         * defaults, text otherwise). A "modified" badge appears next
         * to a sub-key when its value differs from the profile's
         * value; a "Reset" button clears that sub-key's override.
         *
         * Hidden when the profile's chunking_config is empty (no
         * keys to override). The context_config editor below it
         * follows the same pattern.
         */}
        {Object.keys(baseProfile.chunking_config).length > 0 && (
          <SubKeyEditor
            label="Chunking parameters"
            profileMap={baseProfile.chunking_config}
            overrideMap={overrides.chunking_config}
            onSetKey={setChunkingSubKey}
            onClearKey={clearChunkingSubKey}
          />
        )}

        {Object.keys(baseProfile.context_config).length > 0 && (
          <SubKeyEditor
            label="Context parameters"
            profileMap={baseProfile.context_config}
            overrideMap={overrides.context_config}
            onSetKey={(k, v) =>
              setOverrides((cur) => ({
                ...cur,
                context_config: { ...(cur.context_config ?? {}), [k]: v },
              }))
            }
            onClearKey={(k) =>
              setOverrides((cur) => {
                const map = { ...(cur.context_config ?? {}) };
                delete map[k];
                return { ...cur, context_config: map };
              })
            }
          />
        )}

        {/*
         * §2F: Resolved Configuration audit-trail section.
         *
         * Collapsed by default. Shows the fully-merged view of what
         * will run, including profile/template/rules content hashes
         * for audit reproducibility. Operators expand it to verify
         * the resolved view before clicking Process.
         */}
        <ResolvedConfigSection resolved={resolved} />

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
