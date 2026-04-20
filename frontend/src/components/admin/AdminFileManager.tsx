/**
 * AdminFileManager — generic list/view/edit/create/delete UI for any
 * file-based admin resource (prompt templates, schemas, system prompts).
 *
 * Each caller supplies: list/get/create/update/delete API functions, a
 * human-readable resource label, a filename-extension hint, the columns
 * to render in the list view, and a way to pull the filename off a list
 * row. The three file-manager admin tabs collapse into thin shells over
 * this component.
 */
import React, { useEffect, useState } from "react";
import {
  CreateFileInput,
  FileContent,
  UpdateFileInput,
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
const rowClickable: React.CSSProperties = { cursor: "pointer" };
const panelStyle: React.CSSProperties = {
  backgroundColor: "#ffffff",
  borderRadius: "8px",
  border: "1px solid #e2e8f0",
  padding: "1rem 1.25rem",
  marginBottom: "1rem",
};
const textareaStyle: React.CSSProperties = {
  width: "100%",
  minHeight: "360px",
  padding: "0.6rem 0.75rem",
  border: "1px solid #e2e8f0",
  borderRadius: "6px",
  fontSize: "0.82rem",
  fontFamily: "ui-monospace, Menlo, monospace",
  boxSizing: "border-box",
  resize: "vertical",
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
  marginTop: "0.75rem",
};

// ── Props ───────────────────────────────────────────────────────

export interface FileManagerColumn<T> {
  header: string;
  /** Render a table cell for this row's column. Plain text or nodes. */
  render: (row: T) => React.ReactNode;
  /** Optional style override (e.g. width). */
  style?: React.CSSProperties;
}

export interface AdminFileManagerProps<T> {
  /** Singular resource label, e.g. "Template", "Schema", "System Prompt". */
  resourceLabel: string;
  /** Required filename suffix, e.g. ".md" or ".yaml". */
  extension: string;
  /** Fetch the list of rows. */
  fetchList: () => Promise<T[]>;
  /** Load a single file's full content by filename. */
  fetchItem: (filename: string) => Promise<FileContent>;
  /** Create a new file. */
  createItem: (input: CreateFileInput) => Promise<FileContent>;
  /** Overwrite an existing file. */
  updateItem: (filename: string, input: UpdateFileInput) => Promise<FileContent>;
  /** Delete a file. */
  deleteItem: (filename: string) => Promise<void>;
  /** Table columns to render in list view. */
  columns: FileManagerColumn<T>[];
  /** Pull the filename off a list-view row for click-through + actions. */
  getFilename: (row: T) => string;
}

// ── Component ───────────────────────────────────────────────────

type Mode<T> =
  | { kind: "list" }
  | { kind: "view"; row: T; content: FileContent }
  | { kind: "edit"; row: T; content: FileContent; draft: string }
  | { kind: "create"; filename: string; draft: string };

export default function AdminFileManager<T>(props: AdminFileManagerProps<T>) {
  const {
    resourceLabel,
    extension,
    fetchList,
    fetchItem,
    createItem,
    updateItem,
    deleteItem,
    columns,
    getFilename,
  } = props;

  const [rows, setRows] = useState<T[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [listError, setListError] = useState<string | null>(null);
  const [mode, setMode] = useState<Mode<T>>({ kind: "list" });
  const [opError, setOpError] = useState<string | null>(null);
  const [opSuccess, setOpSuccess] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = () => {
    setLoading(true);
    fetchList()
      .then((r) => {
        setRows(r);
        setListError(null);
      })
      .catch((e) =>
        setListError(e instanceof Error ? e.message : `Failed to load ${resourceLabel.toLowerCase()}s`),
      )
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    reload();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const clearMessages = () => {
    setOpError(null);
    setOpSuccess(null);
  };

  const openRow = async (row: T) => {
    clearMessages();
    setBusy(true);
    try {
      const content = await fetchItem(getFilename(row));
      setMode({ kind: "view", row, content });
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Failed to load file");
    } finally {
      setBusy(false);
    }
  };

  const startEdit = () => {
    if (mode.kind !== "view") return;
    setMode({
      kind: "edit",
      row: mode.row,
      content: mode.content,
      draft: mode.content.content,
    });
  };

  const startCreate = () => {
    clearMessages();
    setMode({ kind: "create", filename: "", draft: "" });
  };

  const cancel = () => {
    clearMessages();
    setMode({ kind: "list" });
  };

  const saveEdit = async () => {
    if (mode.kind !== "edit") return;
    const filename = getFilename(mode.row);
    clearMessages();
    setBusy(true);
    try {
      const updated = await updateItem(filename, { content: mode.draft });
      setOpSuccess(`Saved ${filename}`);
      setMode({ kind: "view", row: mode.row, content: updated });
      reload();
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setBusy(false);
    }
  };

  const saveCreate = async () => {
    if (mode.kind !== "create") return;
    const filename = mode.filename.trim();
    if (filename.length === 0) {
      setOpError("Filename is required");
      return;
    }
    if (!filename.endsWith(extension)) {
      setOpError(`Filename must end with ${extension}`);
      return;
    }
    clearMessages();
    setBusy(true);
    try {
      await createItem({ filename, content: mode.draft });
      setOpSuccess(`Created ${filename}`);
      setMode({ kind: "list" });
      reload();
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Create failed");
    } finally {
      setBusy(false);
    }
  };

  const removeRow = async (row: T) => {
    const filename = getFilename(row);
    if (!window.confirm(`Delete ${filename}?`)) return;
    clearMessages();
    setBusy(true);
    try {
      await deleteItem(filename);
      setOpSuccess(`Deleted ${filename}`);
      setMode({ kind: "list" });
      reload();
    } catch (e) {
      setOpError(e instanceof Error ? e.message : "Delete failed");
    } finally {
      setBusy(false);
    }
  };

  // ── Render ──

  if (loading && rows === null) {
    return <div style={emptyStyle}>Loading {resourceLabel.toLowerCase()}s...</div>;
  }
  if (listError) {
    return <div style={{ ...emptyStyle, color: "#dc2626" }}>{listError}</div>;
  }

  return (
    <div>
      {opError && <div style={msgError}>{opError}</div>}
      {opSuccess && <div style={msgSuccess}>{opSuccess}</div>}

      {mode.kind === "list" && (
        <>
          <div style={toolbarStyle}>
            <div style={{ fontSize: "0.82rem", color: "#64748b" }}>
              {rows?.length ?? 0} {resourceLabel.toLowerCase()}{(rows?.length ?? 0) === 1 ? "" : "s"}
            </div>
            <button style={btnPrimary} onClick={startCreate} disabled={busy}>
              New {resourceLabel}
            </button>
          </div>
          {rows && rows.length > 0 ? (
            <div style={tableContainer}>
              <table style={{ width: "100%", borderCollapse: "collapse" }}>
                <thead>
                  <tr>
                    {columns.map((c) => (
                      <th key={c.header} style={{ ...th, ...(c.style || {}) }}>
                        {c.header}
                      </th>
                    ))}
                    <th style={{ ...th, width: "140px" }}>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((row) => (
                    <tr
                      key={getFilename(row)}
                      style={rowClickable}
                      onClick={() => openRow(row)}
                    >
                      {columns.map((c) => (
                        <td key={c.header} style={td}>
                          {c.render(row)}
                        </td>
                      ))}
                      <td style={td}>
                        <button
                          style={btnSecondary}
                          onClick={(e) => {
                            e.stopPropagation();
                            removeRow(row);
                          }}
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
            <div style={emptyStyle}>No {resourceLabel.toLowerCase()}s yet.</div>
          )}
        </>
      )}

      {(mode.kind === "view" || mode.kind === "edit") && (
        <div style={panelStyle}>
          <div style={toolbarStyle}>
            <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#0f172a" }}>
              {getFilename(mode.row)}
            </div>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              {mode.kind === "view" && (
                <button style={btnPrimary} onClick={startEdit} disabled={busy}>
                  Edit
                </button>
              )}
              <button style={btnSecondary} onClick={cancel} disabled={busy}>
                Close
              </button>
            </div>
          </div>
          {mode.kind === "view" ? (
            <textarea
              style={textareaStyle}
              value={mode.content.content}
              readOnly
            />
          ) : (
            <>
              <textarea
                style={textareaStyle}
                value={mode.draft}
                onChange={(e) =>
                  setMode({ ...mode, draft: e.target.value })
                }
              />
              <div style={actionBtnRow}>
                <button style={btnPrimary} onClick={saveEdit} disabled={busy}>
                  Save
                </button>
                <button
                  style={btnSecondary}
                  onClick={() =>
                    setMode({ kind: "view", row: mode.row, content: mode.content })
                  }
                  disabled={busy}
                >
                  Cancel
                </button>
              </div>
            </>
          )}
        </div>
      )}

      {mode.kind === "create" && (
        <div style={panelStyle}>
          <div style={toolbarStyle}>
            <div style={{ fontSize: "0.95rem", fontWeight: 600, color: "#0f172a" }}>
              New {resourceLabel}
            </div>
            <button style={btnSecondary} onClick={cancel} disabled={busy}>
              Cancel
            </button>
          </div>
          <div style={{ marginBottom: "0.75rem" }}>
            <label style={labelStyle}>Filename (must end with {extension})</label>
            <input
              style={inputStyle}
              type="text"
              value={mode.filename}
              placeholder={`e.g. my_file${extension}`}
              onChange={(e) =>
                setMode({ ...mode, filename: e.target.value })
              }
            />
          </div>
          <div>
            <label style={labelStyle}>Content</label>
            <textarea
              style={textareaStyle}
              value={mode.draft}
              onChange={(e) =>
                setMode({ ...mode, draft: e.target.value })
              }
            />
          </div>
          <div style={actionBtnRow}>
            <button style={btnPrimary} onClick={saveCreate} disabled={busy}>
              Create
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
