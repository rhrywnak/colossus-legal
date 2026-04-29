/**
 * AdminProfiles — Admin tab for managing processing-profile YAML files.
 *
 * Rows come from GET /profiles. The edit/create form populates its
 * `extraction_model`, `template_file`, and `schema_file` dropdowns from
 * the live models / templates / schemas lists so operators cannot
 * accidentally reference something that doesn't exist.
 */
import React, { useEffect, useState } from "react";
import {
  createProfile,
  deactivateProfile,
  getProfile,
  listModels,
  listProfiles,
  listSchemas,
  listSystemPrompts,
  listTemplates,
  LlmModel,
  ProcessingProfile,
  SchemaInfo,
  SystemPromptInfo,
  TemplateInfo,
  updateProfile,
} from "../../services/configApi";
import {
  btnPrimary,
  btnSecondary,
  inputStyle,
  labelStyle,
  msgError,
  msgSuccess,
} from "./adminStyles";

// ── Styles ──────────────────────────────────────────────────────

const tableContainer: React.CSSProperties = {
  backgroundColor: "#ffffff",
  borderRadius: "8px",
  border: "1px solid #e2e8f0",
  overflow: "hidden",
};
const th: React.CSSProperties = {
  padding: "0.6rem 1rem",
  textAlign: "left",
  fontSize: "0.76rem",
  fontWeight: 600,
  color: "#64748b",
  borderBottom: "1px solid #e2e8f0",
  backgroundColor: "#f8fafc",
};
const td: React.CSSProperties = {
  padding: "0.6rem 1rem",
  fontSize: "0.84rem",
  color: "#334155",
  borderBottom: "1px solid #f1f5f9",
};
const emptyStyle: React.CSSProperties = {
  padding: "3rem",
  textAlign: "center",
  color: "#94a3b8",
  fontSize: "0.9rem",
};
const panelStyle: React.CSSProperties = {
  backgroundColor: "#ffffff",
  borderRadius: "8px",
  border: "1px solid #e2e8f0",
  padding: "1rem 1.25rem",
  marginBottom: "1rem",
};
const toolbarStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  marginBottom: "0.75rem",
  gap: "0.75rem",
};
const fieldGrid: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
  gap: "0.75rem 1rem",
};
const actionBtnRow: React.CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  marginTop: "1rem",
};
const detailRowStyle: React.CSSProperties = {
  ...td,
  backgroundColor: "#f8fafc",
  whiteSpace: "pre-wrap",
  fontFamily: "ui-monospace, Menlo, monospace",
  fontSize: "0.78rem",
};

const CHUNKING_MODES = ["full", "structured", "chunked"] as const;

const CHUNKING_MODE_LABELS: Record<(typeof CHUNKING_MODES)[number], string> = {
  full: "Full document",
  structured: "Structured",
  chunked: "Legacy Chunked",
};

// ── Helpers ─────────────────────────────────────────────────────

function blankProfile(): ProcessingProfile {
  return {
    name: "",
    display_name: "",
    description: "",
    schema_file: "",
    template_file: "",
    system_prompt_file: null,
    extraction_model: "",
    synthesis_model: null,
    chunking_mode: "chunked",
    chunk_size: null,
    chunk_overlap: null,
    max_tokens: 8000,
    temperature: 0.0,
    auto_approve_grounded: true,
    run_pass2: false,
    is_default: false,
  };
}

type Mode =
  | { kind: "list" }
  | { kind: "view"; name: string; profile: ProcessingProfile }
  | { kind: "edit"; originalName: string; profile: ProcessingProfile }
  | { kind: "create"; profile: ProcessingProfile };

// ── Component ───────────────────────────────────────────────────

const AdminProfiles: React.FC = () => {
  const [rows, setRows] = useState<ProcessingProfile[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);

  // Dropdown source data
  const [models, setModels] = useState<LlmModel[]>([]);
  const [templates, setTemplates] = useState<TemplateInfo[]>([]);
  const [schemas, setSchemas] = useState<SchemaInfo[]>([]);
  const [systemPrompts, setSystemPrompts] = useState<SystemPromptInfo[]>([]);

  const [mode, setMode] = useState<Mode>({ kind: "list" });
  const [expandedName, setExpandedName] = useState<string | null>(null);
  const [opError, setOpError] = useState<string | null>(null);
  const [opSuccess, setOpSuccess] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = () => {
    setLoading(true);
    listProfiles()
      .then((r) => {
        setRows(r.profiles);
        setListError(null);
      })
      .catch((e) =>
        setListError(e instanceof Error ? e.message : "Failed to load profiles"),
      )
      .finally(() => setLoading(false));
  };

  const loadReferences = () => {
    listModels()
      .then((r) => setModels(r.models.filter((m) => m.is_active)))
      .catch(() => {});
    listTemplates().then((r) => setTemplates(r.templates)).catch(() => {});
    listSchemas().then((r) => setSchemas(r.schemas)).catch(() => {});
    listSystemPrompts()
      .then((r) => setSystemPrompts(r.system_prompts))
      .catch(() => {});
  };

  useEffect(() => {
    reload();
    loadReferences();
  }, []);

  const clearMessages = () => {
    setOpError(null);
    setOpSuccess(null);
  };

  const toggleExpand = (name: string) => {
    setExpandedName((cur) => (cur === name ? null : name));
  };

  const startCreate = () => {
    clearMessages();
    setMode({ kind: "create", profile: blankProfile() });
  };

  const startEdit = (row: ProcessingProfile) => {
    clearMessages();
    setBusy(true);
    getProfile(row.name)
      .then((p) =>
        setMode({ kind: "edit", originalName: row.name, profile: p }),
      )
      .catch((e) =>
        setOpError(e instanceof Error ? e.message : "Failed to load profile"),
      )
      .finally(() => setBusy(false));
  };

  const cancel = () => {
    clearMessages();
    setMode({ kind: "list" });
  };

  const saveCreate = async () => {
    if (mode.kind !== "create") return;
    if (mode.profile.name.trim() === "") {
      setOpError("Profile name is required");
      return;
    }
    clearMessages();
    setBusy(true);
    try {
      await createProfile(mode.profile);
      setOpSuccess(`Created ${mode.profile.name}`);
      setMode({ kind: "list" });
      reload();
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Create failed");
    } finally {
      setBusy(false);
    }
  };

  const saveEdit = async () => {
    if (mode.kind !== "edit") return;
    clearMessages();
    setBusy(true);
    try {
      await updateProfile(mode.originalName, mode.profile);
      setOpSuccess(`Saved ${mode.originalName}`);
      setMode({ kind: "list" });
      reload();
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setBusy(false);
    }
  };

  const deactivate = async (row: ProcessingProfile) => {
    if (!window.confirm(`Deactivate profile '${row.name}'? This renames the file to .yaml.inactive.`)) return;
    clearMessages();
    setBusy(true);
    try {
      await deactivateProfile(row.name);
      setOpSuccess(`Deactivated ${row.name}`);
      reload();
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Deactivate failed");
    } finally {
      setBusy(false);
    }
  };

  const updateField = <K extends keyof ProcessingProfile>(
    key: K,
    value: ProcessingProfile[K],
  ) => {
    if (mode.kind === "create" || mode.kind === "edit") {
      setMode({
        ...mode,
        profile: { ...mode.profile, [key]: value },
      });
    }
  };

  // ── Render ──

  if (loading && rows === null) {
    return <div style={emptyStyle}>Loading profiles...</div>;
  }
  if (listError) {
    return <div style={{ ...emptyStyle, color: "#dc2626" }}>{listError}</div>;
  }

  const formProfile =
    mode.kind === "create" || mode.kind === "edit" ? mode.profile : null;

  return (
    <div>
      {opError && <div style={msgError}>{opError}</div>}
      {opSuccess && <div style={msgSuccess}>{opSuccess}</div>}

      {formProfile && (
        <div style={panelStyle}>
          <div style={toolbarStyle}>
            <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#0f172a" }}>
              {mode.kind === "create"
                ? "New Profile"
                : `Edit ${mode.kind === "edit" ? mode.originalName : ""}`}
            </div>
            <button style={btnSecondary} onClick={cancel} disabled={busy}>
              Cancel
            </button>
          </div>
          <div style={fieldGrid}>
            <div>
              <label style={labelStyle}>Name *</label>
              <input
                style={inputStyle}
                value={formProfile.name}
                disabled={mode.kind === "edit"}
                onChange={(e) => updateField("name", e.target.value)}
              />
            </div>
            <div>
              <label style={labelStyle}>Display Name</label>
              <input
                style={inputStyle}
                value={formProfile.display_name}
                onChange={(e) => updateField("display_name", e.target.value)}
              />
            </div>
            <div style={{ gridColumn: "1 / -1" }}>
              <label style={labelStyle}>Description</label>
              <input
                style={inputStyle}
                value={formProfile.description}
                onChange={(e) => updateField("description", e.target.value)}
              />
            </div>
            <div>
              <label style={labelStyle}>Schema File</label>
              <select
                style={inputStyle}
                value={formProfile.schema_file}
                onChange={(e) => updateField("schema_file", e.target.value)}
              >
                <option value="">(select)</option>
                {schemas.map((s) => (
                  <option key={s.filename} value={s.filename}>
                    {s.filename}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label style={labelStyle}>Template File</label>
              <select
                style={inputStyle}
                value={formProfile.template_file}
                onChange={(e) => updateField("template_file", e.target.value)}
              >
                <option value="">(select)</option>
                {templates.map((t) => (
                  <option key={t.filename} value={t.filename}>
                    {t.filename}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label style={labelStyle}>System Prompt File (optional)</label>
              <select
                style={inputStyle}
                value={formProfile.system_prompt_file ?? ""}
                onChange={(e) =>
                  updateField(
                    "system_prompt_file",
                    e.target.value === "" ? null : e.target.value,
                  )
                }
              >
                <option value="">(none)</option>
                {systemPrompts.map((s) => (
                  <option key={s.filename} value={s.filename}>
                    {s.filename}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label style={labelStyle}>Extraction Model</label>
              <select
                style={inputStyle}
                value={formProfile.extraction_model}
                onChange={(e) => updateField("extraction_model", e.target.value)}
              >
                <option value="">(select)</option>
                {models.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.id}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label style={labelStyle}>Chunking Mode</label>
              <select
                style={inputStyle}
                value={formProfile.chunking_mode}
                onChange={(e) => updateField("chunking_mode", e.target.value)}
              >
                {CHUNKING_MODES.map((m) => (
                  <option key={m} value={m}>
                    {CHUNKING_MODE_LABELS[m]}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label style={labelStyle}>Chunk Size</label>
              <input
                style={inputStyle}
                value={formProfile.chunk_size ?? ""}
                onChange={(e) =>
                  updateField(
                    "chunk_size",
                    e.target.value === "" ? null : Number(e.target.value),
                  )
                }
              />
            </div>
            <div>
              <label style={labelStyle}>Chunk Overlap</label>
              <input
                style={inputStyle}
                value={formProfile.chunk_overlap ?? ""}
                onChange={(e) =>
                  updateField(
                    "chunk_overlap",
                    e.target.value === "" ? null : Number(e.target.value),
                  )
                }
              />
            </div>
            <div>
              <label style={labelStyle}>Max Tokens</label>
              <input
                style={inputStyle}
                value={String(formProfile.max_tokens)}
                onChange={(e) =>
                  updateField("max_tokens", Number(e.target.value) || 0)
                }
              />
            </div>
            <div>
              <label style={labelStyle}>Temperature</label>
              <input
                style={inputStyle}
                value={String(formProfile.temperature)}
                onChange={(e) =>
                  updateField("temperature", Number(e.target.value) || 0)
                }
              />
            </div>
            <div>
              <label style={labelStyle}>
                <input
                  type="checkbox"
                  checked={formProfile.auto_approve_grounded}
                  onChange={(e) =>
                    updateField("auto_approve_grounded", e.target.checked)
                  }
                  style={{ marginRight: "0.5rem" }}
                />
                Auto-approve grounded
              </label>
            </div>
            <div>
              <label style={labelStyle}>
                <input
                  type="checkbox"
                  checked={formProfile.run_pass2}
                  onChange={(e) => updateField("run_pass2", e.target.checked)}
                  style={{ marginRight: "0.5rem" }}
                />
                Run pass 2
              </label>
            </div>
            <div>
              <label style={labelStyle}>
                <input
                  type="checkbox"
                  checked={formProfile.is_default}
                  onChange={(e) => updateField("is_default", e.target.checked)}
                  style={{ marginRight: "0.5rem" }}
                />
                Default profile
              </label>
            </div>
          </div>
          <div style={actionBtnRow}>
            <button
              style={btnPrimary}
              onClick={mode.kind === "create" ? saveCreate : saveEdit}
              disabled={busy}
            >
              {mode.kind === "create" ? "Create" : "Save"}
            </button>
          </div>
        </div>
      )}

      {mode.kind === "list" && (
        <>
          <div style={toolbarStyle}>
            <div style={{ fontSize: "0.82rem", color: "#64748b" }}>
              {rows?.length ?? 0} profile{(rows?.length ?? 0) === 1 ? "" : "s"}
            </div>
            <button style={btnPrimary} onClick={startCreate} disabled={busy}>
              New Profile
            </button>
          </div>
          {rows && rows.length > 0 ? (
            <div style={tableContainer}>
              <table style={{ width: "100%", borderCollapse: "collapse" }}>
                <thead>
                  <tr>
                    <th style={th}>Name</th>
                    <th style={th}>Display Name</th>
                    <th style={th}>Template</th>
                    <th style={th}>Schema</th>
                    <th style={th}>Model</th>
                    <th style={{ ...th, width: "100px" }}>Mode</th>
                    <th style={{ ...th, width: "220px" }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((p) => (
                    <React.Fragment key={p.name}>
                      <tr
                        style={{ cursor: "pointer" }}
                        onClick={() => toggleExpand(p.name)}
                      >
                        <td style={{ ...td, fontFamily: "ui-monospace, Menlo, monospace" }}>
                          {p.name}
                        </td>
                        <td style={td}>{p.display_name}</td>
                        <td style={td}>{p.template_file}</td>
                        <td style={td}>{p.schema_file}</td>
                        <td style={td}>{p.extraction_model}</td>
                        <td style={td}>{p.chunking_mode}</td>
                        <td style={td}>
                          <button
                            style={{ ...btnSecondary, marginRight: "0.35rem" }}
                            onClick={(e) => {
                              e.stopPropagation();
                              startEdit(p);
                            }}
                            disabled={busy}
                          >
                            Edit
                          </button>
                          <button
                            style={btnSecondary}
                            onClick={(e) => {
                              e.stopPropagation();
                              deactivate(p);
                            }}
                            disabled={busy}
                          >
                            Deactivate
                          </button>
                        </td>
                      </tr>
                      {expandedName === p.name && (
                        <tr>
                          <td colSpan={7} style={detailRowStyle}>
                            {JSON.stringify(p, null, 2)}
                          </td>
                        </tr>
                      )}
                    </React.Fragment>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div style={emptyStyle}>No profiles yet.</div>
          )}
        </>
      )}
    </div>
  );
};

export default AdminProfiles;
