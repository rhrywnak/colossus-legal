import React, { useEffect, useMemo, useState } from "react";
import { useAuth } from "../context/AuthContext";
import UploadDialog from "../components/pipeline/UploadDialog";
import BatchProgressHeader from "../components/documents/BatchProgressHeader";
import DocumentRow from "../components/documents/DocumentRow";
import { getPhaseOptions, PhaseOption } from "../services/casePhases";
import { attentionBannerText } from "./documentsPageHelpers";
import {
  fetchPipelineDocuments, fetchMetrics, fetchErrors,
  PipelineDocument, EstimatesData,
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
  fontSize: "1.35rem", fontWeight: 700, color: "var(--text-primary)", margin: 0,
};
const subtitle: React.CSSProperties = {
  fontSize: "0.84rem", color: "var(--text-muted)", marginBottom: "1.25rem",
};
const chipRow: React.CSSProperties = {
  display: "flex", gap: "0.4rem", alignItems: "center", flexWrap: "wrap",
  marginBottom: "0.75rem",
};
const chipBase: React.CSSProperties = {
  padding: "0.25rem 0.7rem", fontSize: "0.76rem", fontWeight: 600,
  border: "1px solid var(--border-default)", borderRadius: "999px",
  backgroundColor: "var(--bg-surface)", color: "var(--text-muted)",
  cursor: "pointer", fontFamily: "inherit",
};
const chipActive: React.CSSProperties = {
  ...chipBase,
  backgroundColor: "var(--accent-primary)", color: "var(--bg-surface)",
  borderColor: "var(--accent-primary)",
};
const tableStyle: React.CSSProperties = {
  width: "100%", borderCollapse: "collapse", backgroundColor: "var(--bg-surface)",
  border: "1px solid var(--border-default)", borderRadius: "8px", overflow: "hidden",
};
const th: React.CSSProperties = {
  textAlign: "left", padding: "0.5rem 0.6rem", fontSize: "0.72rem",
  fontWeight: 700, textTransform: "uppercase", letterSpacing: "0.03em",
  color: "var(--text-muted)", borderBottom: "2px solid var(--border-default)",
  whiteSpace: "nowrap",
};
const filtersRow: React.CSSProperties = {
  display: "flex", gap: "0.75rem", marginBottom: "1.25rem", flexWrap: "wrap",
  alignItems: "center",
};
const filterSelect: React.CSSProperties = {
  padding: "0.4rem 0.6rem", fontSize: "0.8rem", borderRadius: "6px",
  border: "1px solid var(--border-default)", fontFamily: "inherit", color: "var(--text-secondary)",
  backgroundColor: "var(--bg-surface)",
};
const searchInput: React.CSSProperties = {
  padding: "0.4rem 0.6rem", fontSize: "0.8rem", borderRadius: "6px",
  border: "1px solid var(--border-default)", fontFamily: "inherit", color: "var(--text-secondary)",
  minWidth: "180px",
};
const uploadBtn: React.CSSProperties = {
  padding: "0.45rem 1rem", fontSize: "0.84rem", fontWeight: 600, border: "none",
  borderRadius: "6px", backgroundColor: "var(--accent-primary)", color: "var(--bg-surface)",
  cursor: "pointer", fontFamily: "inherit",
};
const footerStyle: React.CSSProperties = {
  fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "1rem",
  paddingTop: "0.75rem", borderTop: "1px solid var(--border-default)",
};
const emptyState: React.CSSProperties = {
  padding: "3rem", textAlign: "center", color: "var(--text-disabled)", fontSize: "0.9rem",
};
const errorBox: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "var(--state-danger-bg-soft)", border: "1px solid var(--state-danger-border)",
  borderRadius: "6px", color: "var(--status-dropped-text)", fontSize: "0.84rem",
};
const errorBanner: React.CSSProperties = {
  padding: "0.6rem 1rem", backgroundColor: "var(--burden-warning-bg)", border: "1px solid var(--burden-warning-bg)",
  borderRadius: "6px", color: "var(--burden-warning-text)", fontSize: "0.84rem",
  marginBottom: "1rem", cursor: "pointer",
};

// ── Component ──────────────────────────────────────────────────────

const DocumentsPage: React.FC = () => {
  const { user } = useAuth();
  const isAdmin = user?.permissions.is_admin ?? false;

  const [documents, setDocuments] = useState<PipelineDocument[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [uploadOpen, setUploadOpen] = useState(false);
  const [estimates, setEstimates] = useState<EstimatesData | null>(null);
  const [errorCount, setErrorCount] = useState(0);

  // Filters
  const [statusFilter, setStatusFilter] = useState("all");
  // DOCUMENT_PHASE: "all" plus the four slugs, driven by the chip row.
  const [phaseFilter, setPhaseFilter] = useState("all");
  const [phases, setPhases] = useState<PhaseOption[]>([]);
  const [phaseError, setPhaseError] = useState<string | null>(null);
  const [typeFilter, setTypeFilter] = useState("all");
  const [sortBy, setSortBy] = useState("recent");
  const [search, setSearch] = useState("");

  const loadData = async () => {
    try {
      const result = await fetchPipelineDocuments();
      setDocuments(result.documents);
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
      .catch((e: unknown) => {
        // Pre-existing silent catch, flagged by the rules gate. These two are
        // genuinely optional decorations on this page — the document list
        // renders fully without either — but "optional" is not "invisible":
        // a failing endpoint must leave a trace someone can find.
        console.warn("Documents page: cost estimates unavailable", e);
      });
    fetchErrors()
      // `needs_attention`, not `total_errors`: the latter counts every step that
      // has ever failed, so a retried-then-completed pass-2 kept the banner up
      // forever on a PUBLISHED document.
      .then((e) => setErrorCount(e.needs_attention))
      .catch((e: unknown) => {
        console.warn("Documents page: error count unavailable", e);
      });
  }, []);  // eslint-disable-line react-hooks/exhaustive-deps

  // DOCUMENT_PHASE: the chip row and every row's Phase cell read their labels
  // from /data/timeline.json. Loaded once here and passed down rather than
  // fetched per row.
  useEffect(() => {
    let cancelled = false;
    getPhaseOptions()
      .then((opts) => { if (!cancelled) setPhases(opts); })
      .catch((e: unknown) => {
        if (cancelled) return;
        setPhaseError(e instanceof Error ? e.message : "Could not load the case phases");
      });
    return () => { cancelled = true; };
  }, []);

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
    if (phaseFilter !== "all") result = result.filter(d => (d.phase ?? "") === phaseFilter);
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
  }, [documents, statusFilter, typeFilter, phaseFilter, search, sortBy]);

  const counts = useMemo(() => ({
    total: documents.length,
    completed: documents.filter(d => d.status_group === "completed").length,
    failed: documents.filter(d => d.status_group === "failed").length,
    processing: documents.filter(d => d.status_group === "processing").length,
    new: documents.filter(d => d.status_group === "new").length,
    cancelled: documents.filter(d => d.status_group === "cancelled").length,
  }), [documents]);

  if (loading) return <div style={emptyState}>Loading documents...</div>;
  if (error) return <div style={errorBox}>{error}</div>;

  return (
    <div style={pageStyle}>
      {/* Header */}
      <div style={headerRow}>
        <h1 style={pageTitle}>Documents</h1>
        <div style={{ display: "flex", alignItems: "center" }}>
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
        />
      )}

      {/* Error alert banner */}
      {attentionBannerText(errorCount) !== null && (
        <div style={errorBanner} onClick={() => setStatusFilter("failed")}>
          {attentionBannerText(errorCount)}
        </div>
      )}

      {/* Complaint-first warning */}
      {documents.length === 0 && (
        <div style={{ padding: "1rem", backgroundColor: "var(--burden-warning-bg)", border: "1px solid var(--burden-warning-bg)", borderRadius: "8px", color: "var(--burden-warning-text)", fontSize: "0.84rem", marginBottom: "1rem" }}>
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

      {/* Phase filter chips — All plus the four, in the case's order. */}
      <div style={chipRow}>
        <button
          style={phaseFilter === "all" ? chipActive : chipBase}
          onClick={() => setPhaseFilter("all")}
        >
          All
        </button>
        {phases.map((p) => (
          <button
            key={p.slug}
            style={phaseFilter === p.slug ? chipActive : chipBase}
            onClick={() => setPhaseFilter(p.slug)}
          >
            {p.label}
          </button>
        ))}
        {phaseError !== null && (
          <span style={{ fontSize: "0.72rem", color: "var(--status-dropped-text)" }}>
            Phases unavailable ({phaseError})
          </span>
        )}
      </div>

      {/* One compact table */}
      {filtered.length === 0 ? (
        <div style={emptyState}>
          {documents.length === 0
            ? "No documents yet. Upload your first document to get started."
            : "No documents match the current filters."}
        </div>
      ) : (
        <table style={tableStyle}>
          <thead>
            <tr>
              <th style={th}>Document</th>
              <th style={th}>Type</th>
              <th style={th}>Phase</th>
              <th style={th}>Status</th>
              <th style={th}>Detail</th>
              <th style={th}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((doc) => (
              <DocumentRow
                key={doc.id}
                doc={doc}
                isAdmin={isAdmin}
                phases={phases}
                onRefresh={loadData}
              />
            ))}
          </tbody>
        </table>
      )}

      {/* Summary footer */}
      {documents.length > 0 && (
        <div style={footerStyle}>
          {counts.total} document{counts.total !== 1 ? "s" : ""}
          {" | "}{counts.completed} completed
          {counts.failed > 0 && <>{" | "}<span style={{ color: "var(--state-danger-strong)" }}>{counts.failed} failed</span></>}
          {counts.processing > 0 && <>{" | "}{counts.processing} processing</>}
          {counts.new > 0 && <>{" | "}{counts.new} new</>}
        </div>
      )}
    </div>
  );
};

export default DocumentsPage;
