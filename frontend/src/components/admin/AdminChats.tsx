import React, { useEffect, useState } from "react";
import {
  AdminQAEntry,
  getAdminQAEntries,
  bulkDeleteQAEntries,
  deleteAllQAEntries,
} from "../../services/admin";

// ── Styles ────────────────────────────────────────────────────────────────────

const cardStyle: React.CSSProperties = {
  backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "10px",
  padding: "1.25rem 1.5rem",
};

const btnDanger: React.CSSProperties = {
  backgroundColor: "#dc2626", color: "#fff", border: "none", borderRadius: "6px",
  padding: "0.4rem 0.85rem", fontSize: "0.8rem", fontWeight: 600, cursor: "pointer",
  fontFamily: "inherit",
};

const btnSecondary: React.CSSProperties = {
  backgroundColor: "#f1f5f9", color: "#334155", border: "1px solid #e2e8f0",
  borderRadius: "6px", padding: "0.4rem 0.85rem", fontSize: "0.8rem", fontWeight: 500,
  cursor: "pointer", fontFamily: "inherit",
};

const inputStyle: React.CSSProperties = {
  padding: "0.4rem 0.65rem", border: "1px solid #e2e8f0", borderRadius: "6px",
  fontSize: "0.84rem", fontFamily: "inherit",
};

const msgSuccess: React.CSSProperties = {
  padding: "0.65rem 1rem", backgroundColor: "#ecfdf5", border: "1px solid #a7f3d0",
  borderRadius: "6px", fontSize: "0.84rem", color: "#047857", marginBottom: "1rem",
};

const msgError: React.CSSProperties = {
  padding: "0.65rem 1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
  borderRadius: "6px", fontSize: "0.84rem", color: "#dc2626", marginBottom: "1rem",
};

// ── Component ─────────────────────────────────────────────────────────────────

const PAGE_SIZE = 25;

const AdminChats: React.FC = () => {
  const [entries, setEntries] = useState<AdminQAEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [offset, setOffset] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [userFilter, setUserFilter] = useState("");
  const [confirmDeleteAll, setConfirmDeleteAll] = useState(false);
  const [deleteConfirmText, setDeleteConfirmText] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const loadEntries = async (newOffset = 0) => {
    setLoading(true);
    try {
      const data = await getAdminQAEntries(
        PAGE_SIZE, newOffset,
        userFilter || undefined
      );
      setEntries(data.entries);
      setTotal(data.total);
      setOffset(newOffset);
      setSelected(new Set());
      setError("");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { loadEntries(0); }, [userFilter]);

  const toggleSelect = (id: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };

  const toggleSelectAll = () => {
    if (selected.size === entries.length) {
      setSelected(new Set());
    } else {
      setSelected(new Set(entries.map((e) => e.id)));
    }
  };

  const handleBulkDelete = async () => {
    if (selected.size === 0) return;
    setSubmitting(true);
    setError("");
    setSuccess("");
    try {
      const res = await bulkDeleteQAEntries(Array.from(selected));
      setSuccess(`Deleted ${res.deleted} entries`);
      loadEntries(offset);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSubmitting(false);
    }
  };

  const handleDeleteAll = async () => {
    if (deleteConfirmText !== "DELETE") return;
    setSubmitting(true);
    setError("");
    setSuccess("");
    try {
      const res = await deleteAllQAEntries();
      setSuccess(`Deleted all ${res.deleted} entries`);
      setConfirmDeleteAll(false);
      setDeleteConfirmText("");
      loadEntries(0);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSubmitting(false);
    }
  };

  const totalPages = Math.ceil(total / PAGE_SIZE);
  const currentPage = Math.floor(offset / PAGE_SIZE) + 1;

  const formatDate = (iso: string) => {
    try { return new Date(iso).toLocaleDateString(); } catch { return iso; }
  };

  return (
    <div>
      {success && <div style={msgSuccess}>{success}</div>}
      {error && <div style={msgError}>{error}</div>}

      {/* Controls bar */}
      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "1rem", alignItems: "center", flexWrap: "wrap" }}>
        <input style={inputStyle} placeholder="Filter by user..." value={userFilter}
          onChange={(e) => setUserFilter(e.target.value)} />
        <button style={btnDanger} onClick={handleBulkDelete}
          disabled={selected.size === 0 || submitting}>
          Delete Selected ({selected.size})
        </button>
        <button style={{ ...btnDanger, backgroundColor: "#7f1d1d" }}
          onClick={() => setConfirmDeleteAll(true)} disabled={submitting}>
          Delete All
        </button>
        <div style={{ marginLeft: "auto", fontSize: "0.82rem", color: "#64748b" }}>
          {total} total entries
        </div>
      </div>

      {/* Delete all confirmation */}
      {confirmDeleteAll && (
        <div style={{ ...cardStyle, marginBottom: "1rem", borderColor: "#fecaca" }}>
          <div style={{ fontSize: "0.84rem", color: "#dc2626", fontWeight: 600, marginBottom: "0.5rem" }}>
            This will permanently delete ALL chat entries. Type DELETE to confirm:
          </div>
          <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
            <input style={inputStyle} value={deleteConfirmText}
              onChange={(e) => setDeleteConfirmText(e.target.value)} placeholder="Type DELETE" />
            <button style={btnDanger} onClick={handleDeleteAll}
              disabled={deleteConfirmText !== "DELETE" || submitting}>
              {submitting ? "Deleting..." : "Confirm Delete All"}
            </button>
            <button style={btnSecondary}
              onClick={() => { setConfirmDeleteAll(false); setDeleteConfirmText(""); }}>
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Entries table */}
      <div style={cardStyle}>
        {loading ? (
          <div style={{ textAlign: "center", padding: "2rem", color: "#64748b" }}>Loading...</div>
        ) : entries.length === 0 ? (
          <div style={{ textAlign: "center", padding: "2rem", color: "#64748b" }}>No entries found</div>
        ) : (
          <>
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: "0.84rem" }}>
              <thead>
                <tr style={{ borderBottom: "2px solid #e2e8f0" }}>
                  <th style={{ width: "32px", padding: "0.5rem 0.5rem 0.5rem 0" }}>
                    <input type="checkbox" checked={selected.size === entries.length && entries.length > 0}
                      onChange={toggleSelectAll} />
                  </th>
                  <th style={{ textAlign: "left", padding: "0.5rem", color: "#64748b", fontWeight: 600 }}>Question</th>
                  <th style={{ textAlign: "left", padding: "0.5rem", color: "#64748b", fontWeight: 600 }}>User</th>
                  <th style={{ textAlign: "left", padding: "0.5rem", color: "#64748b", fontWeight: 600 }}>Date</th>
                  <th style={{ textAlign: "left", padding: "0.5rem", color: "#64748b", fontWeight: 600 }}>Model</th>
                </tr>
              </thead>
              <tbody>
                {entries.map((e) => (
                  <tr key={e.id} style={{ borderBottom: "1px solid #f1f5f9" }}>
                    <td style={{ padding: "0.5rem 0.5rem 0.5rem 0" }}>
                      <input type="checkbox" checked={selected.has(e.id)} onChange={() => toggleSelect(e.id)} />
                    </td>
                    <td style={{ padding: "0.5rem", color: "#0f172a", maxWidth: "400px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                      {e.question_preview}
                    </td>
                    <td style={{ padding: "0.5rem", color: "#475569" }}>{e.asked_by}</td>
                    <td style={{ padding: "0.5rem", color: "#475569", whiteSpace: "nowrap" }}>{formatDate(e.asked_at)}</td>
                    <td style={{ padding: "0.5rem", color: "#475569", fontSize: "0.78rem" }}>{e.model}</td>
                  </tr>
                ))}
              </tbody>
            </table>

            {/* Pagination */}
            {totalPages > 1 && (
              <div style={{ display: "flex", justifyContent: "center", gap: "0.5rem", marginTop: "1rem", alignItems: "center" }}>
                <button style={btnSecondary} onClick={() => loadEntries(offset - PAGE_SIZE)}
                  disabled={offset === 0}>Prev</button>
                <span style={{ fontSize: "0.82rem", color: "#64748b" }}>
                  Page {currentPage} of {totalPages}
                </span>
                <button style={btnSecondary} onClick={() => loadEntries(offset + PAGE_SIZE)}
                  disabled={currentPage >= totalPages}>Next</button>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
};

export default AdminChats;
