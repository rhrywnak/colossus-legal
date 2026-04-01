import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useAuth } from "../context/AuthContext";
import DocumentStatusBadge from "../components/pipeline/DocumentStatusBadge";
import PipelineProgressBar from "../components/pipeline/PipelineProgressBar";
import { fetchPipelineDocuments, fetchMetrics, PipelineDocument, MetricsResponse } from "../services/pipelineApi";

// ── Styles ──────────────────────────────────────────────────────

const pageTitle: React.CSSProperties = {
  fontSize: "1.35rem", fontWeight: 700, color: "#0f172a", marginBottom: "0.25rem",
};

const statsRow: React.CSSProperties = {
  display: "flex", gap: "1rem", marginBottom: "1.5rem", flexWrap: "wrap",
};

const statCard: React.CSSProperties = {
  flex: "1 1 140px", padding: "0.75rem 1rem", backgroundColor: "#ffffff",
  borderRadius: "8px", border: "1px solid #e2e8f0",
};

const statValue: React.CSSProperties = {
  fontSize: "1.5rem", fontWeight: 700, color: "#0f172a",
};

const statLabel: React.CSSProperties = {
  fontSize: "0.76rem", color: "#64748b", marginTop: "0.1rem",
};

const tableContainer: React.CSSProperties = {
  backgroundColor: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0",
  overflow: "hidden",
};

const thStyle: React.CSSProperties = {
  padding: "0.6rem 1rem", textAlign: "left" as const, fontSize: "0.76rem",
  fontWeight: 600, color: "#64748b", borderBottom: "1px solid #e2e8f0",
  backgroundColor: "#f8fafc",
};

const tdStyle: React.CSSProperties = {
  padding: "0.6rem 1rem", fontSize: "0.84rem", color: "#334155",
  borderBottom: "1px solid #f1f5f9",
};

const emptyState: React.CSSProperties = {
  padding: "3rem", textAlign: "center", color: "#94a3b8", fontSize: "0.9rem",
};

const linkStyle: React.CSSProperties = {
  color: "#2563eb", textDecoration: "none", fontWeight: 500, fontSize: "0.84rem",
};

// ── Component ───────────────────────────────────────────────────

const PipelineDashboard: React.FC = () => {
  const { user } = useAuth();
  const [documents, setDocuments] = useState<PipelineDocument[]>([]);
  const [metrics, setMetrics] = useState<MetricsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function load() {
      try {
        const [docs, m] = await Promise.all([fetchPipelineDocuments(), fetchMetrics()]);
        setDocuments(docs);
        setMetrics(m);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to load pipeline data");
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  if (!user?.permissions.is_admin) {
    return <div style={emptyState}>Admin access required.</div>;
  }

  if (loading) {
    return <div style={emptyState}>Loading pipeline data...</div>;
  }

  if (error) {
    return <div style={{ ...emptyState, color: "#dc2626" }}>{error}</div>;
  }

  return (
    <div style={{ paddingTop: "1.5rem", paddingBottom: "2rem" }}>
      <h1 style={pageTitle}>Pipeline Dashboard</h1>
      <p style={{ fontSize: "0.84rem", color: "#64748b", marginBottom: "1.25rem" }}>
        Document extraction pipeline status and controls.
      </p>

      {/* Stats row */}
      {metrics && (
        <div style={statsRow}>
          <div style={statCard}>
            <div style={statValue}>{metrics.total_documents}</div>
            <div style={statLabel}>Documents</div>
          </div>
          <div style={statCard}>
            <div style={statValue}>{metrics.total_steps_executed}</div>
            <div style={statLabel}>Steps Executed</div>
          </div>
          <div style={statCard}>
            <div style={statValue}>${metrics.total_cost_usd.toFixed(2)}</div>
            <div style={statLabel}>Total Cost</div>
          </div>
          <div style={statCard}>
            <div style={statValue}>
              {metrics.avg_grounding_rate > 0 ? `${metrics.avg_grounding_rate.toFixed(0)}%` : "--"}
            </div>
            <div style={statLabel}>Avg Grounding</div>
          </div>
          <div style={statCard}>
            <div style={statValue}>{metrics.failed_steps}</div>
            <div style={statLabel}>Failed Steps</div>
          </div>
        </div>
      )}

      {/* Documents table */}
      {documents.length === 0 ? (
        <div style={emptyState}>
          No pipeline documents yet. Upload your first document to get started.
        </div>
      ) : (
        <div style={tableContainer}>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr>
                <th style={thStyle}>Title</th>
                <th style={thStyle}>Type</th>
                <th style={thStyle}>Status</th>
                <th style={thStyle}>Progress</th>
                <th style={thStyle}>Updated</th>
                <th style={thStyle}></th>
              </tr>
            </thead>
            <tbody>
              {documents.map((doc) => (
                <tr key={doc.id}>
                  <td style={tdStyle}>
                    <div style={{ fontWeight: 500 }}>{doc.title}</div>
                    <div style={{ fontSize: "0.72rem", color: "#94a3b8" }}>{doc.id}</div>
                  </td>
                  <td style={tdStyle}>{doc.document_type}</td>
                  <td style={tdStyle}><DocumentStatusBadge status={doc.status} /></td>
                  <td style={{ ...tdStyle, minWidth: "120px" }}>
                    <PipelineProgressBar status={doc.status} />
                  </td>
                  <td style={tdStyle}>
                    <span style={{ fontSize: "0.76rem", color: "#64748b" }}>
                      {new Date(doc.updated_at).toLocaleDateString()}
                    </span>
                  </td>
                  <td style={tdStyle}>
                    <Link to={`/pipeline/${doc.id}`} style={linkStyle}>
                      View
                    </Link>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default PipelineDashboard;
