import React, { useEffect, useMemo, useState } from "react";
import { useAuth } from "../context/AuthContext";
import UploadDialog from "../components/pipeline/UploadDialog";
import BatchProgressHeader from "../components/documents/BatchProgressHeader";
import DocumentCard from "../components/documents/DocumentCard";
import {
  fetchPipelineDocuments, fetchUsers, fetchMetrics, fetchErrors,
  assignReviewer, PipelineDocument, KnownUser, EstimatesData,
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
  const [users, setUsers] = useState<KnownUser[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [uploadOpen, setUploadOpen] = useState(false);
  const [estimates, setEstimates] = useState<EstimatesData | null>(null);
  const [errorCount, setErrorCount] = useState(0);

  // Filters
  const [statusFilter, setStatusFilter] = useState("all");
  const [typeFilter, setTypeFilter] = useState("all");
  const [reviewerFilter, setReviewerFilter] = useState("all");
  const [search, setSearch] = useState("");

  const loadData = async () => {
    try {
      const [docs, knownUsers] = await Promise.all([
        fetchPipelineDocuments(),
        isAdmin ? fetchUsers() : Promise.resolve([]),
      ]);
      setDocuments(docs);
      setUsers(knownUsers);
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

  const uniqueTypes = useMemo(() => {
    const types = new Set(documents.map((d) => d.document_type));
    return Array.from(types).sort();
  }, [documents]);

  const uniqueReviewers = useMemo(() => {
    const reviewers = new Set(
      documents.map((d) => d.assigned_reviewer).filter(Boolean) as string[]
    );
    return Array.from(reviewers).sort();
  }, [documents]);

  const filtered = useMemo(() => {
    return documents.filter((doc) => {
      if (statusFilter !== "all" && (doc.status_group ?? "processing") !== statusFilter) return false;
      if (typeFilter !== "all" && doc.document_type !== typeFilter) return false;
      if (reviewerFilter === "unassigned" && doc.assigned_reviewer) return false;
      if (reviewerFilter === "assigned_to_me" && doc.assigned_reviewer !== user?.username) return false;
      if (reviewerFilter !== "all" && reviewerFilter !== "unassigned" && reviewerFilter !== "assigned_to_me"
          && doc.assigned_reviewer !== reviewerFilter) return false;
      if (search && !doc.title.toLowerCase().includes(search.toLowerCase())) return false;
      return true;
    });
  }, [documents, statusFilter, typeFilter, reviewerFilter, search, user?.username]);

  const counts = useMemo(() => {
    const total = documents.length;
    let published = 0, inReview = 0, processing = 0;
    for (const d of documents) {
      const bucket = (d.status_group ?? "processing");
      if (bucket === "published") published++;
      else if (bucket === "in_review") inReview++;
      else if (bucket === "processing" || bucket === "uploaded") processing++;
    }
    return { total, published, inReview, processing };
  }, [documents]);

  const handleAssign = async (docId: string, reviewer: string | null) => {
    try {
      const result = await assignReviewer(docId, reviewer);
      setDocuments((prev) =>
        prev.map((d) =>
          d.id === docId
            ? { ...d, assigned_reviewer: result.assigned_reviewer, assigned_at: result.assigned_at }
            : d
        )
      );
    } catch (e) {
      console.error("Assign reviewer failed:", e);
    }
  };

  if (loading) return <div style={emptyState}>Loading documents...</div>;
  if (error) return <div style={errorBox}>{error}</div>;

  return (
    <div style={pageStyle}>
      {/* Header */}
      <div style={headerRow}>
        <h1 style={pageTitle}>Documents</h1>
        {isAdmin && (
          <button style={uploadBtn} onClick={() => setUploadOpen(true)}>
            + Upload
          </button>
        )}
      </div>
      <p style={subtitle}>
        Case documents — extraction pipeline status and review.
      </p>

      {isAdmin && (
        <UploadDialog
          open={uploadOpen}
          onClose={() => setUploadOpen(false)}
          onSuccess={() => { setUploadOpen(false); loadData(); }}
        />
      )}

      {/* Error alert banner */}
      {errorCount > 0 && (
        <div style={errorBanner} onClick={() => setStatusFilter("processing")}>
          {errorCount} document{errorCount !== 1 ? "s" : ""} need attention — click to filter
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
          <option value="published">Published</option>
          <option value="in_review">In Review</option>
          <option value="processing">Processing</option>
          <option value="uploaded">Uploaded</option>
        </select>

        <select style={filterSelect} value={typeFilter} onChange={(e) => setTypeFilter(e.target.value)}>
          <option value="all">All Types</option>
          {uniqueTypes.map((t) => (
            <option key={t} value={t}>{titleizeType(t)}</option>
          ))}
        </select>

        <select style={filterSelect} value={reviewerFilter} onChange={(e) => setReviewerFilter(e.target.value)}>
          <option value="all">All Reviewers</option>
          <option value="assigned_to_me">Assigned to Me</option>
          <option value="unassigned">Unassigned</option>
          {uniqueReviewers.map((r) => (
            <option key={r} value={r}>{r}</option>
          ))}
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
            users={users}
            onAssign={handleAssign}
          />
        ))
      )}

      {/* Summary footer */}
      {documents.length > 0 && (
        <div style={footerStyle}>
          {counts.total} document{counts.total !== 1 ? "s" : ""}
          {" \u2502 "}{counts.published} published
          {" \u2502 "}{counts.inReview} in review
          {" \u2502 "}{counts.processing} awaiting processing
        </div>
      )}
    </div>
  );
};

export default DocumentsPage;
