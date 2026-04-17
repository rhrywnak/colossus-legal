/**
 * DocumentWorkspaceTabs — Tabbed workspace for a single pipeline document.
 *
 * Tabs: Document (PDF), Content (extracted items), Processing (admin),
 * Review (side-by-side review), People & Links (people/org summary).
 *
 * Loaded at /documents/:id. Tab content is rendered by dedicated panel
 * components to keep this file under 300 lines.
 */
import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Link, useNavigate, useParams, useSearchParams } from "react-router-dom";
import { useAuth } from "../context/AuthContext";
import { API_BASE_URL } from "../services/api";
import DocumentStatusBadge from "../components/pipeline/DocumentStatusBadge";
import PdfViewer from "../components/shared/PdfViewer";
import ProcessingPanel from "../components/pipeline/ProcessingPanel";
import ContentPanel from "../components/pipeline/ContentPanel";
import ReviewPanel from "../components/pipeline/ReviewPanel";
import PeopleLinksPanel from "../components/pipeline/PeopleLinksPanel";
import DeleteConfirmDialog from "../components/pipeline/DeleteConfirmDialog";
import {
  fetchPipelineDocuments, fetchDocumentHistory, fetchDocumentItems,
  fetchDocumentActions, deleteDocument,
  PipelineDocument, PipelineStep, ExtractionItem, DocumentActions,
} from "../services/pipelineApi";

// ── Tab definitions ─────────────────────────────────────────────

const ALL_TABS = [
  { id: "document", label: "Document" },
  { id: "content", label: "Content" },
  { id: "processing", label: "Process" },
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
};

// ── Component ───────────────────────────────────────────────────

const DocumentWorkspaceTabs: React.FC = () => {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { user } = useAuth();
  const isAdmin = user?.permissions.is_admin ?? false;
  const [deleting, setDeleting] = useState(false);
  const [showDeleteDialog, setShowDeleteDialog] = useState(false);

  // Core state
  const [doc, setDoc] = useState<PipelineDocument | null>(null);
  const [history, setHistory] = useState<PipelineStep[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Tab state — persisted in URL search params so refresh keeps the tab
  const [searchParams, setSearchParams] = useSearchParams();
  const [activeTab, setActiveTabRaw] = useState(searchParams.get("tab") || "document");
  const handleTabChange = (tabId: string) => {
    setActiveTabRaw(tabId);
    setSearchParams({ tab: tabId }, { replace: true });
  };

  // PDF state (shared across tabs for cross-tab navigation)
  const [pdfPage, setPdfPage] = useState(1);

  // Actions state (from backend state machine)
  const [actionsData, setActionsData] = useState<DocumentActions | null>(null);

  // Items state (shared between Content and People tabs)
  const [items, setItems] = useState<ExtractionItem[] | null>(null);
  const [itemsLoading, setItemsLoading] = useState(false);
  const [itemsError, setItemsError] = useState<string | null>(null);

  const docId = id ?? "";

  // Visible tabs computed by backend — no client-side role/status checks
  const visibleTabs = useMemo(() => {
    const allowed = doc?.visible_tabs ?? ["document"];
    return ALL_TABS.filter((tab) => allowed.includes(tab.id));
  }, [doc?.visible_tabs]);

  // Load document + history + actions
  const loadData = useCallback(async () => {
    if (!docId) return;
    try {
      const [listResponse, hist] = await Promise.all([
        fetchPipelineDocuments(),
        fetchDocumentHistory(docId).catch(() => ({ document_id: docId, steps: [] })),
      ]);
      const found = listResponse.documents.find((d) => d.id === docId);
      if (!found) { setError(`Document '${docId}' not found`); return; }
      setDoc(found);
      setHistory(hist.steps);
      setError(null);
      // Load actions (non-blocking — used for delete confirmation level)
      fetchDocumentActions(docId).then(setActionsData).catch(() => {});
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load document");
    } finally {
      setLoading(false);
    }
  }, [docId]);

  useEffect(() => { loadData(); }, [loadData]);

  // Poll every 3s while document is PROCESSING so header badge + panels update
  useEffect(() => {
    if (doc?.status_group !== "processing") return;
    const interval = setInterval(() => { loadData(); }, 3000);
    return () => clearInterval(interval);
  }, [doc?.status_group, loadData]);

  // Load extraction items (called by Content and People tabs)
  const loadItems = useCallback(() => {
    if (items !== null || itemsLoading || !docId) return;
    setItemsLoading(true);
    fetchDocumentItems(docId, { per_page: 500 })
      .then((res) => { setItems(res.items); setItemsError(null); })
      .catch((e) => { setItemsError(e instanceof Error ? e.message : "Failed to load items"); })
      .finally(() => setItemsLoading(false));
  }, [items, itemsLoading, docId]);

  // Lazy-load items when Content tab is first activated
  useEffect(() => {
    if (activeTab === "content") loadItems();
  }, [activeTab, loadItems]);

  // Cross-tab: view item in PDF
  const viewInPdf = (pageNum: number) => {
    setPdfPage(pageNum);
    handleTabChange("document");
  };

  // Early returns
  if (loading) return <div style={S.empty}>Loading...</div>;
  if (error) return <div style={{ ...S.empty, color: "#dc2626" }}>{error}</div>;
  if (!doc) return <div style={S.empty}>Document not found.</div>;

  // Access gate — backend computes can_view based on role + status
  if (doc.can_view === false) {
    return (
      <div style={S.empty}>
        <p>This document is still being processed.</p>
        <Link to="/documents" style={S.backLink}>Back to Documents</Link>
      </div>
    );
  }

  const pdfUrl = `${API_BASE_URL}/api/admin/pipeline/documents/${encodeURIComponent(docId)}/file`;

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
        {doc.total_cost_usd != null && (
          <span>Cost: ${doc.total_cost_usd.toFixed(2)}</span>
        )}
        {isAdmin && (
          <button
            disabled={deleting}
            style={{
              marginLeft: "auto",
              padding: "0.3rem 0.75rem",
              fontSize: "0.78rem",
              fontWeight: 600,
              color: "#fff",
              backgroundColor: deleting ? "#94a3b8" : "#dc2626",
              border: "none",
              borderRadius: "4px",
              cursor: deleting ? "not-allowed" : "pointer",
              fontFamily: "inherit",
            }}
            onClick={() => setShowDeleteDialog(true)}
          >
            {deleting ? "Deleting..." : "Delete"}
          </button>
        )}
      </div>

      {/* Tab bar */}
      <div style={S.tabBar}>
        {visibleTabs.map((tab) => (
          <button
            key={tab.id}
            style={activeTab === tab.id ? S.tabActive : S.tabBase}
            onClick={() => handleTabChange(tab.id)}
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
        <ContentPanel items={items} loading={itemsLoading} error={itemsError} onViewInPdf={viewInPdf} />
      )}

      {activeTab === "processing" && (
        <ProcessingPanel
          document={doc}
          onStepTriggered={loadData}
          onSwitchTab={handleTabChange}
          history={history}
        />
      )}

      {activeTab === "review" && (
        <ReviewPanel documentId={docId} pdfUrl={pdfUrl} />
      )}

      {activeTab === "people" && (
        <PeopleLinksPanel documentId={docId} items={items} onLoadItems={loadItems} />
      )}

      {/* Delete confirmation dialog */}
      {showDeleteDialog && (
        <DeleteConfirmDialog
          documentTitle={doc.title}
          confirmationLevel={actionsData?.delete_confirmation_level ?? "simple"}
          itemCount={items?.length ?? 0}
          onCancel={() => setShowDeleteDialog(false)}
          onConfirm={async (reason) => {
            setShowDeleteDialog(false);
            setDeleting(true);
            try {
              await deleteDocument(doc.id, reason);
              navigate("/documents");
            } catch (e) {
              setError(e instanceof Error ? e.message : "Delete failed");
              setDeleting(false);
            }
          }}
        />
      )}
    </div>
  );
};

export default DocumentWorkspaceTabs;
