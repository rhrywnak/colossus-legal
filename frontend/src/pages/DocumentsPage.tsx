import React, { useEffect, useMemo, useState } from "react";
import { useAuth } from "../context/AuthContext";
import UploadDialog from "../components/pipeline/UploadDialog";
import BatchProgressHeader from "../components/documents/BatchProgressHeader";
import DocumentCard from "../components/documents/DocumentCard";
import {
  fetchPipelineDocuments, fetchMetrics, fetchErrors,
  processDocument, PipelineDocument, EstimatesData,
} from "../services/pipelineApi";

// ── Helpers ────────────────────────────────────────────────────────

function titleizeType(slug: string): string {
  return slug.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}


// ── Styles ─────────────────────────────────────────────────────────

const pageStyle: React.CSSProperties = {
  paddingTop: "1.5rem", paddingBottom: "2rem",
};
const headerRow: React.CSSProperties = {
  display: "flex", justifyContent: "space-between", alignItems: "center",
  marginBottom: "0.25rem",
};
const pageTitle: React.CSSProperties = {
  fontSize: "1.35rem", fontWeight: 700, color: "#0f172a", margin: 0,
};
const subtitle: React.CSSProperties = {
  fontSize: "0.84rem", color: "#64748b", marginBottom: "1.25rem",
};
const filtersRow: React.CSSProperties = {
  display: "flex", gap: "0.75rem", marginBottom: "1.25rem", flexWrap: "wrap",
  alignItems: "center",
};
const filterSelect: React.CSSProperties = {
  padding: "0.4rem 0.6rem", fontSize: "0.8rem", borderRadius: "6px",
  border: "1px solid #e2e8f0", fontFamily: "inherit", color: "#334155",
  backgroundColor: "#ffffff",
};
const searchInput: React.CSSProperties = {
  padding: "0.4rem 0.6rem", fontSize: "0.8rem", borderRadius: "6px",
  border: "1px solid #e2e8f0", fontFamily: "inherit", color: "#334155",
  minWidth: "180px",
};
const uploadBtn: React.CSSProperties = {
  padding: "0.45rem 1rem", fontSize: "0.84rem", fontWeight: 600, border: "none",
  borderRadius: "6px", backgroundColor: "#2563eb", color: "#ffffff",
  cursor: "pointer", fontFamily: "inherit",
};
const footerStyle: React.CSSProperties = {
  fontSize: "0.8rem", color: "#64748b", marginTop: "1rem",
  paddingTop: "0.75rem", borderTop: "1px solid #e2e8f0",
};
const emptyState: React.CSSProperties = {
  padding: "3rem", textAlign: "center", color: "#94a3b8", fontSize: "0.9rem",
};
const errorBox: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "#fef2f2", border: "1px solid #fecaca",
  borderRadius: "6px", color: "#991b1b", fontSize: "0.84rem",
};
const errorBanner: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "#fffbeb", border: "1px solid #fde68a",
  borderRadius: "6px", color: "#92400e", fontSize: "0.84rem",
  marginBottom: "1rem", cursor: "pointer",
};

// ── Component ──────────────────────────────────────────────────────

const DocumentsPage: React.FC = () => {
  const { user } = useAuth();
  const isAdmin = user?.permissions.is_admin ?? false;

  const [documents, setDocuments] = useState<PipelineDocument[]>([]);
  const [complaintExists, setComplaintExists] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [uploadOpen, setUploadOpen] = useState(false);
  const [estimates, setEstimates] = useState<EstimatesData | null>(null);
  const [errorCount, setErrorCount] = useState(0);

  // Filters
  const [statusFilter, setStatusFilter] = useState("all");
  const [typeFilter, setTypeFilter] = useState("all");
  const [sortBy, setSortBy] = useState("recent");
  const [search, setSearch] = useState("");

  const loadData = async () => {
    try {
      const result = await fetchPipelineDocuments();
      setDocuments(result.documents);
      setComplaintExists(result.complaint_exists);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load documents");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
    // Fetch estimates and errors in background (non-blocking)
    fetchMetrics()
      .then((m) => setEstimates(m.estimates))
      .catch(() => { /* metrics are optional */ });
    fetchErrors()
      .then((e) => setErrorCount(e.total_errors))
      .catch(() => { /* errors are optional */ });
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

  // Poll when documents are processing
  useEffect(() => {
    const hasProcessing = documents.some(d => d.status_group === "processing");
    if (!hasProcessing) return;
    const interval = setInterval(() => { loadData(); }, 3000);
    return () => clearInterval(interval);
  }, [documents]);  // eslint-disable-line react-hooks/exhaustive-deps

  const uniqueTypes = useMemo(() => {
    const types = new Set(documents.map((d) => d.document_type));
    return Array.from(types).sort();
  }, [documents]);

  const filtered = useMemo(() => {
    let result = documents;
    if (statusFilter !== "all") result = result.filter(d => d.status_group === statusFilter);
    if (typeFilter !== "all") result = result.filter(d => d.document_type === typeFilter);
    if (search.trim()) {
      const q = search.toLowerCase();
      result = result.filter(d => d.title.toLowerCase().includes(q));
    }
    // Sort
    const copy = [...result];
    switch (sortBy) {
      case "recent": return copy.sort((a, b) => new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime());
      case "oldest": return copy.sort((a, b) => new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime());
      case "name": return copy.sort((a, b) => a.title.localeCompare(b.title));
      case "failed_first": {
        const order: Record<string, number> = { failed: 0, processing: 1, new: 2, cancelled: 3, completed: 4 };
        return copy.sort((a, b) => (order[a.status_group ?? ""] ?? 5) - (order[b.status_group ?? ""] ?? 5));
      }
      default: return copy;
    }
  }, [documents, statusFilter, typeFilter, search, sortBy]);

  const counts = useMemo(() => ({
    total: documents.length,
    completed: documents.filter(d => d.status_group === "completed").length,
    failed: documents.filter(d => d.status_group === "failed").length,
    processing: documents.filter(d => d.status_group === "processing").length,
    new: documents.filter(d => d.status_group === "new").length,
    cancelled: documents.filter(d => d.status_group === "cancelled").length,
  }), [documents]);

  const handleProcessAllNew = async () => {
    const newDocs = documents.filter(d => d.status === "NEW");
    for (const doc of newDocs) {
      try { await processDocument(doc.id); } catch (e) { console.error(`Failed to start processing ${doc.id}:`, e); }
    }
    loadData();
  };

  if (loading) return <div style={emptyState}>Loading documents...</div>;
  if (error) return <div style={errorBox}>{error}</div>;

  return (
    <div style={pageStyle}>
      {/* Header */}
      <div style={headerRow}>
        <h1 style={pageTitle}>Documents</h1>
        <div style={{ display: "flex", alignItems: "center" }}>
          {isAdmin && counts.new > 0 && (
            <button style={{ ...uploadBtn, backgroundColor: "#16a34a", marginRight: "0.5rem" }} onClick={handleProcessAllNew}>
              Process All New ({counts.new})
            </button>
          )}
          {isAdmin && (
            <button style={uploadBtn} onClick={() => setUploadOpen(true)}>
              + Upload
            </button>
          )}
        </div>
      </div>
      <p style={subtitle}>
        Case documents
      </p>

      {isAdmin && (
        <UploadDialog
          open={uploadOpen}
          onClose={() => setUploadOpen(false)}
          onSuccess={() => { setUploadOpen(false); loadData(); }}
          complaintExists={complaintExists}
        />
      )}

      {/* Error alert banner */}
      {errorCount > 0 && (
        <div style={errorBanner} onClick={() => setStatusFilter("failed")}>
          {errorCount} document{errorCount !== 1 ? "s" : ""} need attention — click to filter
        </div>
      )}

      {/* Complaint-first warning */}
      {documents.length === 0 && (
        <div style={{ padding: "1rem", backgroundColor: "#fffbeb", border: "1px solid #fde68a", borderRadius: "8px", color: "#92400e", fontSize: "0.84rem", marginBottom: "1rem" }}>
          <strong>A Complaint must be uploaded and processed first.</strong>
          <p style={{ margin: "0.25rem 0 0", fontSize: "0.8rem" }}>
            The Complaint establishes the parties, claims, and legal context that all other documents reference.
          </p>
        </div>
      )}

      {/* Batch progress */}
      <BatchProgressHeader
        documents={documents}
        estimates={estimates}
        onStatusFilter={setStatusFilter}
      />

      {/* Filters */}
      <div style={filtersRow}>
        <select style={filterSelect} value={statusFilter} onChange={(e) => setStatusFilter(e.target.value)}>
          <option value="all">All Statuses</option>
          <option value="new">New</option>
          <option value="processing">Processing</option>
          <option value="completed">Completed</option>
          <option value="failed">Failed</option>
          <option value="cancelled">Cancelled</option>
        </select>

        <select style={filterSelect} value={typeFilter} onChange={(e) => setTypeFilter(e.target.value)}>
          <option value="all">All Types</option>
          {uniqueTypes.map((t) => (
            <option key={t} value={t}>{titleizeType(t)}</option>
          ))}
        </select>

        <select style={filterSelect} value={sortBy} onChange={(e) => setSortBy(e.target.value)}>
          <option value="recent">Most Recent</option>
          <option value="oldest">Oldest</option>
          <option value="name">Name A-Z</option>
          <option value="failed_first">Failed First</option>
        </select>

        <input
          style={searchInput}
          type="text"
          placeholder="Search by title..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </div>

      {/* Document cards */}
      {filtered.length === 0 ? (
        <div style={emptyState}>
          {documents.length === 0
            ? "No documents yet. Upload your first document to get started."
            : "No documents match the current filters."}
        </div>
      ) : (
        filtered.map((doc) => (
          <DocumentCard
            key={doc.id}
            doc={doc}
            isAdmin={isAdmin}
            onRefresh={loadData}
          />
        ))
      )}

      {/* Summary footer */}
      {documents.length > 0 && (
        <div style={footerStyle}>
          {counts.total} document{counts.total !== 1 ? "s" : ""}
          {" | "}{counts.completed} completed
          {counts.failed > 0 && <>{" | "}<span style={{ color: "#dc2626" }}>{counts.failed} failed</span></>}
          {counts.processing > 0 && <>{" | "}{counts.processing} processing</>}
          {counts.new > 0 && <>{" | "}{counts.new} new</>}
        </div>
      )}
    </div>
  );
};

export default DocumentsPage;
