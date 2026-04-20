/**
 * AdminModels — Admin tab for managing the `llm_models` registry.
 *
 * Rows come from GET /models (both active and inactive). Toggle the
 * checkbox to flip active/inactive in place. Inline panels handle create
 * and edit; delete refuses at the server if any profile references the
 * model — the 409 body lands in an error box.
 */
import React, { useEffect, useState } from "react";
import {
  createModel,
  CreateModelInput,
  deleteModel,
  listModels,
  LlmModel,
  toggleModel,
  updateModel,
  UpdateModelInput,
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
const fieldGrid: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
  gap: "0.75rem 1rem",
};
const toolbarStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  marginBottom: "0.75rem",
  gap: "0.75rem",
};
const actionBtnRow: React.CSSProperties = {
  display: "flex",
  gap: "0.5rem",
  marginTop: "1rem",
};

const PROVIDERS = ["anthropic", "vllm", "openai"] as const;

// ── Form state ──────────────────────────────────────────────────

type FormMode =
  | { kind: "list" }
  | { kind: "create"; form: ModelForm }
  | { kind: "edit"; id: string; form: ModelForm };

interface ModelForm {
  id: string;
  display_name: string;
  provider: string;
  api_endpoint: string;
  max_context_tokens: string;
  max_output_tokens: string;
  cost_per_input_token: string;
  cost_per_output_token: string;
  notes: string;
}

const emptyForm: ModelForm = {
  id: "",
  display_name: "",
  provider: "anthropic",
  api_endpoint: "",
  max_context_tokens: "",
  max_output_tokens: "",
  cost_per_input_token: "",
  cost_per_output_token: "",
  notes: "",
};

function modelToForm(m: LlmModel): ModelForm {
  return {
    id: m.id,
    display_name: m.display_name,
    provider: m.provider,
    api_endpoint: m.api_endpoint ?? "",
    max_context_tokens: m.max_context_tokens != null ? String(m.max_context_tokens) : "",
    max_output_tokens: m.max_output_tokens != null ? String(m.max_output_tokens) : "",
    cost_per_input_token: m.cost_per_input_token != null ? String(m.cost_per_input_token) : "",
    cost_per_output_token: m.cost_per_output_token != null ? String(m.cost_per_output_token) : "",
    notes: m.notes ?? "",
  };
}

/** Parse a form's optional numeric/string fields into a CreateModelInput. */
function formToCreateInput(f: ModelForm): CreateModelInput {
  const parseOptInt = (s: string): number | undefined =>
    s.trim() === "" ? undefined : Number(s);
  const parseOptFloat = (s: string): number | undefined =>
    s.trim() === "" ? undefined : Number(s);
  const optStr = (s: string): string | undefined =>
    s.trim() === "" ? undefined : s;
  return {
    id: f.id.trim(),
    display_name: f.display_name.trim(),
    provider: f.provider,
    api_endpoint: optStr(f.api_endpoint),
    max_context_tokens: parseOptInt(f.max_context_tokens),
    max_output_tokens: parseOptInt(f.max_output_tokens),
    cost_per_input_token: parseOptFloat(f.cost_per_input_token),
    cost_per_output_token: parseOptFloat(f.cost_per_output_token),
    notes: optStr(f.notes),
  };
}

/** Parse a form into an UpdateModelInput; empty strings become null (clear). */
function formToUpdateInput(f: ModelForm): UpdateModelInput {
  const parseOptInt = (s: string): number | null =>
    s.trim() === "" ? null : Number(s);
  const parseOptFloat = (s: string): number | null =>
    s.trim() === "" ? null : Number(s);
  const strOrNull = (s: string): string | null => (s.trim() === "" ? null : s);
  return {
    display_name: f.display_name.trim(),
    provider: f.provider,
    api_endpoint: strOrNull(f.api_endpoint),
    max_context_tokens: parseOptInt(f.max_context_tokens),
    max_output_tokens: parseOptInt(f.max_output_tokens),
    cost_per_input_token: parseOptFloat(f.cost_per_input_token),
    cost_per_output_token: parseOptFloat(f.cost_per_output_token),
    notes: strOrNull(f.notes),
  };
}

// ── Component ───────────────────────────────────────────────────

const AdminModels: React.FC = () => {
  const [models, setModels] = useState<LlmModel[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [mode, setMode] = useState<FormMode>({ kind: "list" });
  const [opError, setOpError] = useState<string | null>(null);
  const [opSuccess, setOpSuccess] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = () => {
    setLoading(true);
    listModels()
      .then((r) => {
        setModels(r.models);
        setListError(null);
      })
      .catch((e) =>
        setListError(e instanceof Error ? e.message : "Failed to load models"),
      )
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    reload();
  }, []);

  const clearMessages = () => {
    setOpError(null);
    setOpSuccess(null);
  };

  const startCreate = () => {
    clearMessages();
    setMode({ kind: "create", form: emptyForm });
  };

  const startEdit = (m: LlmModel) => {
    clearMessages();
    setMode({ kind: "edit", id: m.id, form: modelToForm(m) });
  };

  const cancel = () => {
    clearMessages();
    setMode({ kind: "list" });
  };

  const saveCreate = async () => {
    if (mode.kind !== "create") return;
    const input = formToCreateInput(mode.form);
    if (input.id.length === 0 || input.display_name.length === 0) {
      setOpError("id and display_name are required");
      return;
    }
    clearMessages();
    setBusy(true);
    try {
      await createModel(input);
      setOpSuccess(`Created ${input.id}`);
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
    const input = formToUpdateInput(mode.form);
    clearMessages();
    setBusy(true);
    try {
      await updateModel(mode.id, input);
      setOpSuccess(`Saved ${mode.id}`);
      setMode({ kind: "list" });
      reload();
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setBusy(false);
    }
  };

  const flipActive = async (m: LlmModel) => {
    clearMessages();
    setBusy(true);
    try {
      await toggleModel(m.id);
      reload();
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Toggle failed");
    } finally {
      setBusy(false);
    }
  };

  const removeModel = async (m: LlmModel) => {
    if (!window.confirm(`Delete model '${m.id}'?`)) return;
    clearMessages();
    setBusy(true);
    try {
      await deleteModel(m.id);
      setOpSuccess(`Deleted ${m.id}`);
      reload();
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Delete failed");
    } finally {
      setBusy(false);
    }
  };

  const updateForm = (patch: Partial<ModelForm>) => {
    if (mode.kind === "create") setMode({ ...mode, form: { ...mode.form, ...patch } });
    else if (mode.kind === "edit") setMode({ ...mode, form: { ...mode.form, ...patch } });
  };

  // ── Render ──

  if (loading && models === null) {
    return <div style={emptyStyle}>Loading models...</div>;
  }
  if (listError) {
    return <div style={{ ...emptyStyle, color: "#dc2626" }}>{listError}</div>;
  }

  const currentForm =
    mode.kind === "create" || mode.kind === "edit" ? mode.form : null;

  return (
    <div>
      {opError && <div style={msgError}>{opError}</div>}
      {opSuccess && <div style={msgSuccess}>{opSuccess}</div>}

      {currentForm && (
        <div style={panelStyle}>
          <div style={toolbarStyle}>
            <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#0f172a" }}>
              {mode.kind === "create" ? "New Model" : `Edit ${mode.kind === "edit" ? mode.id : ""}`}
            </div>
            <button style={btnSecondary} onClick={cancel} disabled={busy}>
              Cancel
            </button>
          </div>
          <div style={fieldGrid}>
            <div>
              <label style={labelStyle}>ID *</label>
              <input
                style={inputStyle}
                value={currentForm.id}
                disabled={mode.kind === "edit"}
                onChange={(e) => updateForm({ id: e.target.value })}
              />
            </div>
            <div>
              <label style={labelStyle}>Display Name *</label>
              <input
                style={inputStyle}
                value={currentForm.display_name}
                onChange={(e) => updateForm({ display_name: e.target.value })}
              />
            </div>
            <div>
              <label style={labelStyle}>Provider</label>
              <select
                style={inputStyle}
                value={currentForm.provider}
                onChange={(e) => updateForm({ provider: e.target.value })}
              >
                {PROVIDERS.map((p) => (
                  <option key={p} value={p}>
                    {p}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label style={labelStyle}>API Endpoint</label>
              <input
                style={inputStyle}
                value={currentForm.api_endpoint}
                onChange={(e) => updateForm({ api_endpoint: e.target.value })}
              />
            </div>
            <div>
              <label style={labelStyle}>Max Context Tokens</label>
              <input
                style={inputStyle}
                value={currentForm.max_context_tokens}
                onChange={(e) => updateForm({ max_context_tokens: e.target.value })}
              />
            </div>
            <div>
              <label style={labelStyle}>Max Output Tokens</label>
              <input
                style={inputStyle}
                value={currentForm.max_output_tokens}
                onChange={(e) => updateForm({ max_output_tokens: e.target.value })}
              />
            </div>
            <div>
              <label style={labelStyle}>Cost / Input Token (USD)</label>
              <input
                style={inputStyle}
                value={currentForm.cost_per_input_token}
                onChange={(e) => updateForm({ cost_per_input_token: e.target.value })}
              />
            </div>
            <div>
              <label style={labelStyle}>Cost / Output Token (USD)</label>
              <input
                style={inputStyle}
                value={currentForm.cost_per_output_token}
                onChange={(e) => updateForm({ cost_per_output_token: e.target.value })}
              />
            </div>
            <div style={{ gridColumn: "1 / -1" }}>
              <label style={labelStyle}>Notes</label>
              <input
                style={inputStyle}
                value={currentForm.notes}
                onChange={(e) => updateForm({ notes: e.target.value })}
              />
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
              {models?.length ?? 0} model{(models?.length ?? 0) === 1 ? "" : "s"}
            </div>
            <button style={btnPrimary} onClick={startCreate} disabled={busy}>
              New Model
            </button>
          </div>
          {models && models.length > 0 ? (
            <div style={tableContainer}>
              <table style={{ width: "100%", borderCollapse: "collapse" }}>
                <thead>
                  <tr>
                    <th style={th}>ID</th>
                    <th style={th}>Display Name</th>
                    <th style={{ ...th, width: "100px" }}>Provider</th>
                    <th style={{ ...th, width: "80px" }}>Active</th>
                    <th style={{ ...th, width: "180px" }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {models.map((m) => (
                    <tr key={m.id}>
                      <td style={{ ...td, fontFamily: "ui-monospace, Menlo, monospace" }}>
                        {m.id}
                      </td>
                      <td style={td}>{m.display_name}</td>
                      <td style={td}>{m.provider}</td>
                      <td style={td}>
                        <input
                          type="checkbox"
                          checked={m.is_active}
                          onChange={() => flipActive(m)}
                          disabled={busy}
                        />
                      </td>
                      <td style={td}>
                        <button
                          style={{ ...btnSecondary, marginRight: "0.35rem" }}
                          onClick={() => startEdit(m)}
                          disabled={busy}
                        >
                          Edit
                        </button>
                        <button
                          style={btnSecondary}
                          onClick={() => removeModel(m)}
                          disabled={busy}
                        >
                          Delete
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div style={emptyStyle}>No models yet.</div>
          )}
        </>
      )}
    </div>
  );
};

export default AdminModels;
