/**
 * DocumentWorkspaceTabs — Tabbed workspace for a single pipeline document.
 *
 * Tabs: Document (PDF), Content (extracted items), Processing (admin),
 * Review (placeholder), People & Links (placeholder).
 *
 * Loaded at /documents/:id. The previous PipelineDocumentDetail page is
 * now the Processing tab via ProcessingPanel.
 */
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useAuth } from "../context/AuthContext";
import { API_BASE_URL } from "../services/api";
import DocumentStatusBadge from "../components/pipeline/DocumentStatusBadge";
import PdfViewer from "../components/shared/PdfViewer";
import ProcessingPanel from "../components/pipeline/ProcessingPanel";
import {
  fetchPipelineDocuments, fetchDocumentHistory, fetchDocumentItems,
  PipelineDocument, PipelineStep, ExtractionItem,
} from "../services/pipelineApi";

// ── Tab definitions ─────────────────────────────────────────────

const ALL_TABS = [
  { id: "document", label: "Document" },
  { id: "content", label: "Content" },
  { id: "processing", label: "Processing" },
  { id: "review", label: "Review" },
  { id: "people", label: "People & Links" },
];

// ── Styles ──────────────────────────────────────────────────────

const S = {
  backLink: { fontSize: "0.84rem", color: "#2563eb", textDecoration: "none", fontWeight: 500 } as React.CSSProperties,
  pageTitle: { fontSize: "1.35rem", fontWeight: 700, color: "#0f172a", marginBottom: "0.15rem" } as React.CSSProperties,
  metaRow: {
    display: "flex", gap: "1.25rem", fontSize: "0.84rem", color: "#64748b",
    marginBottom: "1rem", alignItems: "center", flexWrap: "wrap",
  } as React.CSSProperties,
  tabBar: { display: "flex", gap: "0.15rem", borderBottom: "2px solid #e2e8f0", marginBottom: "1.25rem" } as React.CSSProperties,
  tabBase: {
    padding: "0.5rem 1rem", fontSize: "0.84rem", fontWeight: 500, color: "#64748b",
    cursor: "pointer", border: "none", background: "none", fontFamily: "inherit",
    borderBottom: "2px solid transparent", marginBottom: "-2px", transition: "color 0.15s ease",
  } as React.CSSProperties,
  tabActive: {
    padding: "0.5rem 1rem", fontSize: "0.84rem", fontWeight: 600, color: "#2563eb",
    cursor: "pointer", border: "none", background: "none", fontFamily: "inherit",
    borderBottom: "2px solid #2563eb", marginBottom: "-2px", transition: "color 0.15s ease",
  } as React.CSSProperties,
  empty: { padding: "3rem", textAlign: "center", color: "#94a3b8", fontSize: "0.9rem" } as React.CSSProperties,
  placeholder: {
    padding: "3rem", textAlign: "center", color: "#94a3b8", fontSize: "0.9rem",
    backgroundColor: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0",
  } as React.CSSProperties,
};

// ── Content tab helpers ──────────────────────────────────────────

const itemCardStyle: React.CSSProperties = {
  backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "8px",
  padding: "0.75rem 1rem", marginBottom: "0.5rem",
};
const typeBadge = (color: string): React.CSSProperties => ({
  display: "inline-block", padding: "0.1rem 0.45rem", borderRadius: "4px",
  fontSize: "0.68rem", fontWeight: 600, backgroundColor: color, color: "#fff",
});
const groundBadge = (ok: boolean): React.CSSProperties => ({
  display: "inline-block", padding: "0.1rem 0.4rem", borderRadius: "9999px",
  fontSize: "0.68rem", fontWeight: 600,
  backgroundColor: ok ? "#dcfce7" : "#fef9c3", color: ok ? "#166534" : "#854d0e",
});
const pdfBtnStyle: React.CSSProperties = {
  padding: "0.2rem 0.5rem", fontSize: "0.72rem", fontWeight: 500, border: "1px solid #e2e8f0",
  borderRadius: "4px", backgroundColor: "#f8fafc", color: "#2563eb", cursor: "pointer", fontFamily: "inherit",
};
const filterStyle: React.CSSProperties = {
  padding: "0.35rem 0.6rem", fontSize: "0.8rem", borderRadius: "6px", border: "1px solid #e2e8f0",
  fontFamily: "inherit", color: "#334155", backgroundColor: "#ffffff", marginBottom: "0.75rem",
};
const TYPE_COLORS: Record<string, string> = {
  Person: "#2563eb", Evidence: "#059669", Allegation: "#dc2626",
  Claim: "#7c3aed", Document: "#d97706", Event: "#0891b2",
};

// ── Component ───────────────────────────────────────────────────

const DocumentWorkspaceTabs: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const { user } = useAuth();
  const isAdmin = user?.permissions.is_admin ?? false;

  // Core state
  const [doc, setDoc] = useState<PipelineDocument | null>(null);
  const [history, setHistory] = useState<PipelineStep[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Tab state
  const [activeTab, setActiveTab] = useState("document");

  // PDF state (shared across tabs for cross-tab navigation)
  const [pdfPage, setPdfPage] = useState(1);

  // Content tab state (lazy-loaded)
  const [items, setItems] = useState<ExtractionItem[] | null>(null);
  const [itemsLoading, setItemsLoading] = useState(false);
  const [itemsError, setItemsError] = useState<string | null>(null);
  const [entityFilter, setEntityFilter] = useState("all");

  const docId = id ?? "";
  const isPublished = doc?.status === "PUBLISHED";
  const isAssignedReviewer = doc?.assigned_reviewer === user?.username;

  // Visible tabs based on role and document state
  const visibleTabs = useMemo(() => {
    return ALL_TABS.filter((tab) => {
      switch (tab.id) {
        case "document": return true;
        case "content": return isPublished || isAdmin;
        case "processing": return isAdmin;
        case "review": return isAdmin || isAssignedReviewer;
        case "people": return isPublished || isAdmin;
        default: return false;
      }
    });
  }, [isAdmin, isPublished, isAssignedReviewer]);

  // Load document + history
  const loadData = useCallback(async () => {
    if (!docId) return;
    try {
      const [docs, hist] = await Promise.all([
        fetchPipelineDocuments(),
        fetchDocumentHistory(docId).catch(() => ({ document_id: docId, steps: [] })),
      ]);
      const found = docs.find((d) => d.id === docId);
      if (!found) { setError(`Document '${docId}' not found`); return; }
      setDoc(found);
      setHistory(hist.steps);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load document");
    } finally {
      setLoading(false);
    }
  }, [docId]);

  useEffect(() => { loadData(); }, [loadData]);

  // Lazy-load extraction items when Content tab is first activated
  useEffect(() => {
    if (activeTab !== "content" || items !== null || itemsLoading || !docId) return;
    setItemsLoading(true);
    fetchDocumentItems(docId, { per_page: 500 })
      .then((res) => { setItems(res.items); setItemsError(null); })
      .catch((e) => { setItemsError(e instanceof Error ? e.message : "Failed to load items"); })
      .finally(() => setItemsLoading(false));
  }, [activeTab, items, itemsLoading, docId]);

  // Filtered items for Content tab
  const filteredItems = useMemo(() => {
    if (!items) return [];
    if (entityFilter === "all") return items;
    return items.filter((it) => it.entity_type === entityFilter);
  }, [items, entityFilter]);

  const entityTypes = useMemo(() => {
    if (!items) return [];
    const types = new Set(items.map((it) => it.entity_type));
    return Array.from(types).sort();
  }, [items]);

  // Cross-tab: view item in PDF
  const viewInPdf = (pageNum: number) => {
    setPdfPage(pageNum);
    setActiveTab("document");
  };

  // Early returns
  if (loading) return <div style={S.empty}>Loading...</div>;
  if (error) return <div style={{ ...S.empty, color: "#dc2626" }}>{error}</div>;
  if (!doc) return <div style={S.empty}>Document not found.</div>;

  // Non-admin users can only view published documents
  if (!isAdmin && !isPublished) {
    return (
      <div style={S.empty}>
        <p>This document is still being processed.</p>
        <Link to="/documents" style={S.backLink}>Back to Documents</Link>
      </div>
    );
  }

  const pdfUrl = `${API_BASE_URL}/api/documents/${encodeURIComponent(docId)}/file`;

  return (
    <div style={{ paddingTop: "1.5rem", paddingBottom: "2rem" }}>
      {/* Header */}
      <Link to="/documents" style={S.backLink}>&larr; Back to Documents</Link>
      <h1 style={{ ...S.pageTitle, marginTop: "0.75rem" }}>{doc.title}</h1>
      <div style={S.metaRow}>
        <DocumentStatusBadge status={doc.status} />
        <span>Type: {doc.document_type}</span>
        <span>ID: {doc.id}</span>
        <span>Updated: {new Date(doc.updated_at).toLocaleDateString()}</span>
      </div>

      {/* Tab bar */}
      <div style={S.tabBar}>
        {visibleTabs.map((tab) => (
          <button
            key={tab.id}
            style={activeTab === tab.id ? S.tabActive : S.tabBase}
            onClick={() => setActiveTab(tab.id)}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      {activeTab === "document" && (
        <div style={{ height: "calc(100vh - 280px)", minHeight: "400px" }}>
          <PdfViewer src={pdfUrl} page={pdfPage} onPageChange={setPdfPage} />
        </div>
      )}

      {activeTab === "content" && (
        <div>
          {itemsLoading && <div style={S.empty}>Loading extracted content...</div>}
          {itemsError && <div style={{ ...S.empty, color: "#dc2626" }}>{itemsError}</div>}
          {items && items.length === 0 && <div style={S.empty}>No extracted content yet.</div>}
          {items && items.length > 0 && (
            <>
              <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginBottom: "0.5rem" }}>
                <select style={filterStyle} value={entityFilter} onChange={(e) => setEntityFilter(e.target.value)}>
                  <option value="all">All types ({items.length})</option>
                  {entityTypes.map((t) => (
                    <option key={t} value={t}>{t} ({items.filter((i) => i.entity_type === t).length})</option>
                  ))}
                </select>
                <span style={{ fontSize: "0.76rem", color: "#64748b" }}>
                  {filteredItems.length} item{filteredItems.length !== 1 ? "s" : ""}
                </span>
              </div>
              <div style={{ maxHeight: "calc(100vh - 340px)", overflowY: "auto" }}>
                {filteredItems.map((item) => (
                  <div key={item.id} style={itemCardStyle}>
                    <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginBottom: "0.35rem" }}>
                      <span style={typeBadge(TYPE_COLORS[item.entity_type] || "#6b7280")}>{item.entity_type}</span>
                      <span style={{ fontSize: "0.88rem", fontWeight: 600, color: "#0f172a" }}>{item.label}</span>
                      {item.grounding_status && (
                        <span style={groundBadge(item.grounding_status === "grounded")}>
                          {item.grounding_status}
                        </span>
                      )}
                      {item.grounded_page && (
                        <button style={pdfBtnStyle} onClick={() => viewInPdf(item.grounded_page!)}>
                          View in PDF (p.{item.grounded_page})
                        </button>
                      )}
                    </div>
                    {item.verbatim_quote && (
                      <div style={{ fontSize: "0.78rem", color: "#64748b", fontStyle: "italic", lineHeight: 1.4 }}>
                        "{item.verbatim_quote.length > 150
                          ? item.verbatim_quote.slice(0, 150) + "..."
                          : item.verbatim_quote}"
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      )}

      {activeTab === "processing" && (
        <ProcessingPanel
          document={doc}
          history={history}
          onStepTriggered={loadData}
          onSwitchTab={setActiveTab}
        />
      )}

      {activeTab === "review" && (
        <div style={S.placeholder}>Review panel — coming in Phase C2</div>
      )}

      {activeTab === "people" && (
        <div style={S.placeholder}>People &amp; Links — coming in Phase C2</div>
      )}
    </div>
  );
};

export default DocumentWorkspaceTabs;
