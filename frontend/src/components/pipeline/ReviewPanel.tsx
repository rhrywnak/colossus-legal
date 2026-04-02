/**
 * ReviewPanel — Side-by-side extraction item review with PDF viewer.
 *
 * Left pane: scrollable items list with filters, approve/reject/edit actions.
 * Right pane: PdfViewer showing the document, scrolled to the selected item's page.
 * Uses useResizablePanes for a draggable divider.
 */
import React, { useCallback, useEffect, useMemo, useState } from "react";
import PdfViewer from "../shared/PdfViewer";
import { useResizablePanes } from "../../hooks/useResizablePanes";
import {
  fetchDocumentItems, approveItem, rejectItem, editItem, bulkApprove,
  ExtractionItem,
} from "../../services/pipelineApi";

interface ReviewPanelProps {
  documentId: string;
  pdfUrl: string;
}

// ── Styles ──────────────────────────────────────────────────────

const TYPE_COLORS: Record<string, string> = {
  Person: "#2563eb", Evidence: "#059669", Allegation: "#dc2626",
  Claim: "#7c3aed", Document: "#d97706", Event: "#0891b2",
};
const badge = (bg: string, fg: string): React.CSSProperties => ({
  display: "inline-block", padding: "0.1rem 0.4rem", borderRadius: "9999px",
  fontSize: "0.68rem", fontWeight: 600, backgroundColor: bg, color: fg,
});
const REVIEW_BADGE: Record<string, React.CSSProperties> = {
  approved: badge("#dcfce7", "#166534"),
  rejected: badge("#fee2e2", "#991b1b"),
  pending: badge("#f1f5f9", "#64748b"),
};
const filterSel: React.CSSProperties = {
  padding: "0.3rem 0.5rem", fontSize: "0.76rem", borderRadius: "4px",
  border: "1px solid #e2e8f0", fontFamily: "inherit", color: "#334155",
};
const actionBtn = (bg: string, fg: string, border: string): React.CSSProperties => ({
  padding: "0.2rem 0.5rem", fontSize: "0.72rem", fontWeight: 500,
  border: `1px solid ${border}`, borderRadius: "4px", backgroundColor: bg,
  color: fg, cursor: "pointer", fontFamily: "inherit",
});
const cardBase: React.CSSProperties = {
  padding: "0.6rem 0.75rem", borderRadius: "6px", border: "1px solid #e2e8f0",
  backgroundColor: "#fff", cursor: "pointer", marginBottom: "0.4rem",
  transition: "border-color 0.15s",
};

// ── Component ───────────────────────────────────────────────────

const ReviewPanel: React.FC<ReviewPanelProps> = ({ documentId, pdfUrl }) => {
  const [items, setItems] = useState<ExtractionItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [pdfPage, setPdfPage] = useState(1);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editPage, setEditPage] = useState("");
  const [editQuote, setEditQuote] = useState("");

  // Filters
  const [typeFilter, setTypeFilter] = useState("all");
  const [reviewFilter, setReviewFilter] = useState("all");
  const [groundFilter, setGroundFilter] = useState("all");

  const { splitPercent, containerRef, dividerProps, isDragging } = useResizablePanes();

  const loadItems = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetchDocumentItems(documentId, { per_page: 500 });
      setItems(res.items);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load items");
    } finally {
      setLoading(false);
    }
  }, [documentId]);

  useEffect(() => { loadItems(); }, [loadItems]);

  const filtered = useMemo(() => {
    return items.filter((it) => {
      if (typeFilter !== "all" && it.entity_type !== typeFilter) return false;
      if (reviewFilter !== "all" && (it.review_status || "pending") !== reviewFilter) return false;
      if (groundFilter !== "all" && (it.grounding_status || "unknown") !== groundFilter) return false;
      return true;
    });
  }, [items, typeFilter, reviewFilter, groundFilter]);

  const entityTypes = useMemo(() => {
    return Array.from(new Set(items.map((it) => it.entity_type))).sort();
  }, [items]);

  const selectedItem = items.find((it) => it.id === selectedId) ?? null;
  const highlightText = selectedItem?.verbatim_quote ?? null;
  const highlightPage = selectedItem?.grounded_page ?? null;

  // Summary counts
  const pending = items.filter((it) => !it.review_status || it.review_status === "pending").length;
  const approved = items.filter((it) => it.review_status === "approved").length;
  const rejected = items.filter((it) => it.review_status === "rejected").length;

  const handleSelect = (item: ExtractionItem) => {
    setSelectedId(item.id);
    if (item.grounded_page) setPdfPage(item.grounded_page);
  };

  const handleApprove = async (id: number) => {
    try { await approveItem(id); await loadItems(); } catch { /* silent */ }
  };

  const handleReject = async (id: number) => {
    const reason = window.prompt("Rejection reason:");
    if (reason === null) return;
    try { await rejectItem(id, reason); await loadItems(); } catch { /* silent */ }
  };

  const startEdit = (item: ExtractionItem) => {
    setEditingId(item.id);
    setEditPage(item.grounded_page?.toString() ?? "");
    setEditQuote(item.verbatim_quote ?? "");
  };

  const saveEdit = async () => {
    if (editingId === null) return;
    const updates: { grounded_page?: number; verbatim_quote?: string } = {};
    const pg = parseInt(editPage, 10);
    if (!isNaN(pg) && pg > 0) updates.grounded_page = pg;
    if (editQuote.trim()) updates.verbatim_quote = editQuote.trim();
    try { await editItem(editingId, updates); setEditingId(null); await loadItems(); } catch { /* silent */ }
  };

  const handleBulkApprove = async (filter: "grounded" | "all") => {
    try { await bulkApprove(documentId, filter); await loadItems(); } catch { /* silent */ }
  };

  if (loading && items.length === 0) {
    return <div style={{ padding: "2rem", textAlign: "center", color: "#94a3b8" }}>Loading review items...</div>;
  }
  if (error && items.length === 0) {
    return <div style={{ padding: "2rem", textAlign: "center", color: "#dc2626" }}>{error}</div>;
  }

  return (
    <div ref={containerRef} style={{
      display: "flex", height: "calc(100vh - 300px)", minHeight: "400px",
      border: "1px solid #e2e8f0", borderRadius: "8px", overflow: "hidden",
      userSelect: isDragging ? "none" : "auto",
    }}>
      {/* Left pane: items list */}
      <div style={{ width: `${splitPercent}%`, overflow: "auto", padding: "0.75rem", backgroundColor: "#fafbfc" }}>
        {/* Summary + Bulk actions */}
        <div style={{ display: "flex", gap: "0.75rem", alignItems: "center", marginBottom: "0.5rem", flexWrap: "wrap" }}>
          <span style={{ fontSize: "0.76rem", color: "#334155", fontWeight: 600 }}>
            {pending} pending
          </span>
          <span style={{ fontSize: "0.76rem", color: "#166534" }}>{approved} approved</span>
          <span style={{ fontSize: "0.76rem", color: "#991b1b" }}>{rejected} rejected</span>
          <button style={actionBtn("#ecfdf5", "#047857", "#a7f3d0")} onClick={() => handleBulkApprove("grounded")}>
            Approve Grounded
          </button>
          <button style={actionBtn("#eff6ff", "#2563eb", "#bfdbfe")} onClick={() => handleBulkApprove("all")}>
            Approve All
          </button>
        </div>

        {/* Filters */}
        <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.75rem", flexWrap: "wrap" }}>
          <select style={filterSel} value={typeFilter} onChange={(e) => setTypeFilter(e.target.value)}>
            <option value="all">All types</option>
            {entityTypes.map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
          <select style={filterSel} value={reviewFilter} onChange={(e) => setReviewFilter(e.target.value)}>
            <option value="all">All review</option>
            <option value="pending">Pending</option>
            <option value="approved">Approved</option>
            <option value="rejected">Rejected</option>
          </select>
          <select style={filterSel} value={groundFilter} onChange={(e) => setGroundFilter(e.target.value)}>
            <option value="all">All grounding</option>
            <option value="grounded">Grounded</option>
            <option value="ungrounded">Ungrounded</option>
          </select>
          <span style={{ fontSize: "0.72rem", color: "#64748b", alignSelf: "center" }}>
            {filtered.length} / {items.length}
          </span>
        </div>

        {/* Items */}
        {filtered.map((item) => (
          <div key={item.id}
            style={{ ...cardBase, borderColor: selectedId === item.id ? "#2563eb" : "#e2e8f0",
              backgroundColor: selectedId === item.id ? "#eff6ff" : "#fff" }}
            onClick={() => handleSelect(item)}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "0.4rem", marginBottom: "0.25rem" }}>
              <span style={{
                display: "inline-block", padding: "0.1rem 0.4rem", borderRadius: "4px",
                fontSize: "0.66rem", fontWeight: 600, color: "#fff",
                backgroundColor: TYPE_COLORS[item.entity_type] || "#6b7280",
              }}>{item.entity_type}</span>
              <span style={{ fontSize: "0.82rem", fontWeight: 600, color: "#0f172a" }}>{item.label}</span>
              <span style={REVIEW_BADGE[item.review_status || "pending"]}>
                {item.review_status || "pending"}
              </span>
              {item.grounded_page && (
                <span style={{ fontSize: "0.68rem", color: "#64748b" }}>p.{item.grounded_page}</span>
              )}
            </div>

            {item.verbatim_quote && (
              <div style={{ fontSize: "0.74rem", color: "#64748b", fontStyle: "italic", lineHeight: 1.4,
                maxHeight: "2.8em", overflow: "hidden", marginBottom: "0.35rem" }}>
                "{item.verbatim_quote.length > 120 ? item.verbatim_quote.slice(0, 120) + "..." : item.verbatim_quote}"
              </div>
            )}

            {/* Edit form */}
            {editingId === item.id ? (
              <div style={{ display: "flex", gap: "0.4rem", alignItems: "center", flexWrap: "wrap", marginTop: "0.3rem" }}
                onClick={(e) => e.stopPropagation()}>
                <input value={editPage} onChange={(e) => setEditPage(e.target.value)}
                  placeholder="Page" style={{ width: "3.5rem", padding: "0.2rem 0.3rem", fontSize: "0.72rem",
                  border: "1px solid #e2e8f0", borderRadius: "4px" }} />
                <input value={editQuote} onChange={(e) => setEditQuote(e.target.value)}
                  placeholder="Quote" style={{ flex: 1, minWidth: "120px", padding: "0.2rem 0.3rem",
                  fontSize: "0.72rem", border: "1px solid #e2e8f0", borderRadius: "4px" }} />
                <button style={actionBtn("#ecfdf5", "#047857", "#a7f3d0")} onClick={saveEdit}>Save</button>
                <button style={actionBtn("#fff", "#64748b", "#e2e8f0")} onClick={() => setEditingId(null)}>Cancel</button>
              </div>
            ) : (
              <div style={{ display: "flex", gap: "0.3rem", marginTop: "0.2rem" }}
                onClick={(e) => e.stopPropagation()}>
                {(!item.review_status || item.review_status === "pending") && (
                  <>
                    <button style={actionBtn("#ecfdf5", "#047857", "#a7f3d0")} onClick={() => handleApprove(item.id)}>Approve</button>
                    <button style={actionBtn("#fef2f2", "#dc2626", "#fecaca")} onClick={() => handleReject(item.id)}>Reject</button>
                  </>
                )}
                <button style={actionBtn("#fff", "#64748b", "#e2e8f0")} onClick={() => startEdit(item)}>Edit</button>
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Divider */}
      <div {...dividerProps}>
        <div style={{ width: "2px", height: "24px", borderRadius: "1px", backgroundColor: "#94a3b8" }} />
      </div>

      {/* Right pane: PDF viewer */}
      <div style={{ width: `${100 - splitPercent}%`, overflow: "hidden", display: "flex", flexDirection: "column" }}>
        <PdfViewer
          src={pdfUrl}
          page={pdfPage}
          onPageChange={setPdfPage}
          highlightText={highlightText}
          highlightPage={highlightPage}
        />
      </div>
    </div>
  );
};

export default ReviewPanel;
