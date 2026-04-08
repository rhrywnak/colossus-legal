/**
 * AdminMetrics — Pipeline metrics dashboard tab.
 *
 * Shows summary cards (document counts, cost, grounding rate) and a
 * step performance table with proportional duration bars.
 */
import React, { useEffect, useState } from "react";
import { fetchMetrics, MetricsResponse } from "../../services/pipelineApi";
import ReviewerWorkloadSection from "./ReviewerWorkloadSection";

// ── Styles ──────────────────────────────────────────────────────

const cardRow: React.CSSProperties = {
  display: "flex", gap: "1rem", marginBottom: "1.5rem", flexWrap: "wrap",
};
const card: React.CSSProperties = {
  flex: "1 1 140px", padding: "0.75rem 1rem", backgroundColor: "#ffffff",
  borderRadius: "8px", border: "1px solid #e2e8f0",
};
const cardValue: React.CSSProperties = {
  fontSize: "1.5rem", fontWeight: 700, color: "#0f172a",
};
const cardLabel: React.CSSProperties = {
  fontSize: "0.76rem", color: "#64748b", marginTop: "0.1rem",
};
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
  padding: "3rem", textAlign: "center", color: "#94a3b8", fontSize: "0.9rem",
};

// ── Helpers ─────────────────────────────────────────────────────

function fmtDuration(secs: number): string {
  if (secs < 1) return `${(secs * 1000).toFixed(0)}ms`;
  if (secs < 60) return `${secs.toFixed(1)}s`;
  const m = Math.floor(secs / 60);
  const s = Math.round(secs % 60);
  return `${m}m ${s}s`;
}

function fmtTime(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  const h = Math.floor(secs / 3600);
  const m = Math.round((secs % 3600) / 60);
  return `${h}h ${m}m`;
}

// ── Component ───────────────────────────────────────────────────

const AdminMetrics: React.FC = () => {
  const [metrics, setMetrics] = useState<MetricsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchMetrics()
      .then((m) => { setMetrics(m); setError(null); })
      .catch((e) => setError(e instanceof Error ? e.message : "Failed to load metrics"))
      .finally(() => setLoading(false));
  }, []);

  if (loading) return <div style={emptyStyle}>Loading metrics...</div>;
  if (error) return <div style={{ ...emptyStyle, color: "#dc2626" }}>{error}</div>;
  if (!metrics) return <div style={emptyStyle}>No metrics available.</div>;

  const published = metrics.documents_by_status["PUBLISHED"] ?? 0;

  // Build ordered step rows from backend-provided label and order
  const stepRows = Object.entries(metrics.step_performance)
    .map(([key, perf]) => ({
      key,
      label: perf.label || key,
      order: perf.order ?? 99,
      ...perf,
    }))
    .sort((a, b) => a.order - b.order);

  const maxAvgDuration = Math.max(...stepRows.map((s) => s.avg_duration_secs), 0.001);

  return (
    <div>
      {/* Summary cards */}
      <div style={cardRow}>
        <div style={card}>
          <div style={cardValue}>{metrics.total_documents}</div>
          <div style={cardLabel}>Documents</div>
          <div style={{ fontSize: "0.72rem", color: "#94a3b8", marginTop: "0.15rem" }}>
            {Object.entries(metrics.documents_by_status)
              .map(([status, count]) => `${count} ${status.toLowerCase()}`)
              .join(", ")}
          </div>
        </div>
        <div style={card}>
          <div style={cardValue}>${metrics.total_cost_usd.toFixed(2)}</div>
          <div style={cardLabel}>Total Cost</div>
          <div style={{ fontSize: "0.72rem", color: "#94a3b8", marginTop: "0.15rem" }}>
            ${metrics.avg_cost_per_document.toFixed(2)} avg/doc
          </div>
        </div>
        <div style={card}>
          <div style={cardValue}>
            {metrics.avg_grounding_rate > 0 ? `${metrics.avg_grounding_rate.toFixed(1)}%` : "--"}
          </div>
          <div style={cardLabel}>Avg Grounding</div>
        </div>
        <div style={card}>
          <div style={cardValue}>{published} / {metrics.total_documents}</div>
          <div style={cardLabel}>Published</div>
        </div>
      </div>

      {/* Estimates */}
      {metrics.estimates.confidence !== "none" ? (
        <div style={{ ...cardRow, marginBottom: "1.5rem" }}>
          {metrics.estimates.estimated_remaining_cost_usd != null && (
            <div style={card}>
              <div style={cardValue}>~${metrics.estimates.estimated_remaining_cost_usd.toFixed(2)}</div>
              <div style={cardLabel}>Est. Remaining Cost</div>
            </div>
          )}
          {metrics.estimates.estimated_remaining_time_secs != null && (
            <div style={card}>
              <div style={cardValue}>~{fmtTime(metrics.estimates.estimated_remaining_time_secs)}</div>
              <div style={cardLabel}>Est. Remaining Time</div>
            </div>
          )}
          <div style={card}>
            <div style={cardValue}>{metrics.estimates.documents_remaining}</div>
            <div style={cardLabel}>Docs Remaining</div>
            <div style={{ fontSize: "0.72rem", color: "#94a3b8", marginTop: "0.15rem" }}>
              Confidence: {metrics.estimates.confidence}
            </div>
          </div>
        </div>
      ) : (
        <div style={{ fontSize: "0.8rem", color: "#94a3b8", marginBottom: "1.5rem" }}>
          Not enough data to estimate remaining cost and time.
        </div>
      )}

      {/* Step performance table */}
      <h2 style={{ fontSize: "1rem", fontWeight: 600, color: "#334155", marginBottom: "0.75rem" }}>
        Step Performance
        <span style={{ fontSize: "0.76rem", fontWeight: 400, color: "#94a3b8", marginLeft: "0.5rem" }}>
          {metrics.total_steps_executed} runs, {metrics.failed_steps} failed
        </span>
      </h2>

      {stepRows.length === 0 ? (
        <div style={emptyStyle}>No step execution data yet.</div>
      ) : (
        <div style={tableContainer}>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr>
                <th style={th}>Step</th>
                <th style={th}>Avg Duration</th>
                <th style={{ ...th, width: "30%" }}>Relative</th>
                <th style={th}>Runs</th>
                <th style={th}>Fails</th>
              </tr>
            </thead>
            <tbody>
              {stepRows.map((step) => {
                const pct = (step.avg_duration_secs / maxAvgDuration) * 100;
                const isMax = step.avg_duration_secs === maxAvgDuration;
                return (
                  <tr key={step.key}>
                    <td style={{ ...td, fontWeight: 500 }}>{step.label}</td>
                    <td style={td}>{fmtDuration(step.avg_duration_secs)}</td>
                    <td style={td}>
                      <div style={{
                        height: "8px", backgroundColor: "#e2e8f0", borderRadius: "4px",
                        overflow: "hidden",
                      }}>
                        <div style={{
                          width: `${pct}%`, height: "100%", borderRadius: "4px",
                          backgroundColor: isMax ? "#f59e0b" : "#2563eb",
                          transition: "width 0.3s ease",
                        }} />
                      </div>
                    </td>
                    <td style={td}>{step.count}</td>
                    <td style={{ ...td, color: step.failure_count > 0 ? "#dc2626" : "#64748b" }}>
                      {step.failure_count}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {/* Reviewer workload */}
      <div style={{ marginTop: "2rem" }}>
        <ReviewerWorkloadSection />
      </div>
    </div>
  );
};

export default AdminMetrics;
