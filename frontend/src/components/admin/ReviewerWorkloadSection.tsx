/**
 * ReviewerWorkloadSection — shows per-reviewer assignment and review progress.
 *
 * Fetches workload data independently. Displays a table with progress bars.
 */
import React, { useEffect, useState } from "react";
import { fetchWorkload, ReviewerWorkload } from "../../services/pipelineApi";

// ── Styles ──────────────────────────────────────────────────────

const tableContainer: React.CSSProperties = {
  backgroundColor: "#ffffff", borderRadius: "8px", border: "1px solid #e2e8f0",
  overflow: "hidden",
};
const th: React.CSSProperties = {
  padding: "0.6rem 1rem", textAlign: "left", fontSize: "0.76rem",
  fontWeight: 600, color: "#64748b", borderBottom: "1px solid #e2e8f0",
  backgroundColor: "#f8fafc",
};
const td: React.CSSProperties = {
  padding: "0.6rem 1rem", fontSize: "0.84rem", color: "#334155",
  borderBottom: "1px solid #f1f5f9",
};
const emptyStyle: React.CSSProperties = {
  padding: "2rem", textAlign: "center", color: "#94a3b8", fontSize: "0.84rem",
};

// ── Component ───────────────────────────────────────────────────

const ReviewerWorkloadSection: React.FC = () => {
  const [reviewers, setReviewers] = useState<ReviewerWorkload[]>([]);
  const [unassigned, setUnassigned] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchWorkload()
      .then((w) => { setReviewers(w.reviewers); setUnassigned(w.unassigned_documents); })
      .catch((e) => setError(e instanceof Error ? e.message : "Failed to load workload"))
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <div style={emptyStyle}>Loading workload...</div>;
  if (error) return <div style={{ ...emptyStyle, color: "#dc2626" }}>{error}</div>;
  if (reviewers.length === 0) return <div style={emptyStyle}>No reviewers assigned yet.</div>;

  return (
    <div>
      <h2 style={{ fontSize: "1rem", fontWeight: 600, color: "#334155", marginBottom: "0.75rem" }}>
        Reviewer Workload
        {unassigned > 0 && (
          <span style={{ fontSize: "0.76rem", fontWeight: 400, color: "#f59e0b", marginLeft: "0.5rem" }}>
            {unassigned} unassigned
          </span>
        )}
      </h2>

      <div style={tableContainer}>
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={th}>Reviewer</th>
              <th style={th}>Documents</th>
              <th style={{ ...th, width: "25%" }}>Progress</th>
              <th style={th}>Approved</th>
              <th style={th}>Pending</th>
              <th style={th}>Rejected</th>
            </tr>
          </thead>
          <tbody>
            {reviewers.map((r) => {
              const docPct = r.assigned_documents > 0
                ? Math.round((r.reviewed_documents / r.assigned_documents) * 100)
                : 0;
              return (
                <tr key={r.username}>
                  <td style={{ ...td, fontWeight: 500 }}>
                    {r.display_name || r.username}
                  </td>
                  <td style={td}>
                    {r.reviewed_documents} / {r.assigned_documents}
                  </td>
                  <td style={td}>
                    <div style={{
                      height: "8px", backgroundColor: "#e2e8f0", borderRadius: "4px",
                      overflow: "hidden",
                    }}>
                      <div style={{
                        width: `${docPct}%`, height: "100%", borderRadius: "4px",
                        backgroundColor: docPct === 100 ? "#16a34a" : "#2563eb",
                        transition: "width 0.3s ease",
                      }} />
                    </div>
                  </td>
                  <td style={{ ...td, color: "#16a34a" }}>{r.approved_items}</td>
                  <td style={{ ...td, color: r.pending_items > 0 ? "#f59e0b" : "#64748b" }}>
                    {r.pending_items}
                  </td>
                  <td style={{ ...td, color: r.rejected_items > 0 ? "#dc2626" : "#64748b" }}>
                    {r.rejected_items}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
};

export default ReviewerWorkloadSection;
