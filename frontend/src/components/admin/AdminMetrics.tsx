/**
 * AdminMetrics — Pipeline metrics dashboard tab.
 *
 * Shows summary cards (document counts, grounding rate, completion) and a
 * step performance table with proportional duration bars.
 */
import React, { useEffect, useState } from "react";
import { fetchMetrics, MetricsResponse } from "../../services/pipelineApi";
// ReviewerWorkloadSection removed — will be repurposed for audit workload later

// ── Styles ──────────────────────────────────────────────────────

const cardRow: React.CSSProperties = {
  display: "flex", gap: "1rem", marginBottom: "1.5rem", flexWrap: "wrap",
};
const card: React.CSSProperties = {
  flex: "1 1 140px", padding: "0.75rem 1rem", backgroundColor: "var(--bg-surface)",
  borderRadius: "8px", border: "1px solid var(--border-default)",
};
const cardValue: React.CSSProperties = {
  fontSize: "1.5rem", fontWeight: 700, color: "var(--text-primary)",
};
const cardLabel: React.CSSProperties = {
  fontSize: "0.76rem", color: "var(--text-muted)", marginTop: "0.1rem",
};
const tableContainer: React.CSSProperties = {
  backgroundColor: "var(--bg-surface)", borderRadius: "8px", border: "1px solid var(--border-default)",
  overflow: "hidden",
};
const th: React.CSSProperties = {
  padding: "0.6rem 1rem", textAlign: "left", fontSize: "0.76rem",
  fontWeight: 600, color: "var(--text-muted)", borderBottom: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-page)",
};
const td: React.CSSProperties = {
  padding: "0.6rem 1rem", fontSize: "0.84rem", color: "var(--text-secondary)",
  borderBottom: "1px solid var(--bg-page)",
};
const emptyStyle: React.CSSProperties = {
  padding: "3rem", textAlign: "center", color: "var(--text-disabled)", fontSize: "0.9rem",
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
  if (error) return <div style={{ ...emptyStyle, color: "var(--state-danger-strong)" }}>{error}</div>;
  if (!metrics) return <div style={emptyStyle}>No metrics available.</div>;

  const completed = metrics.documents_by_status["COMPLETED"] ?? 0;

  // Build ordered step rows from backend-provided label and order.
  // `StepMetrics` (pipelineApi.ts:234) declares both `label: string`
  // and `order: number` as required, so the spread alone supplies
  // them. The earlier `label: perf.label || key` and
  // `order: perf.order ?? 99` lines were shadowed dead code (the
  // spread on the next line overwrote them); removed to clear the
  // TS2783 duplicate-key errors surfaced by `npm run typecheck`.
  const stepRows = Object.entries(metrics.step_performance)
    .map(([key, perf]) => ({
      key,
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
          <div style={{ fontSize: "0.72rem", color: "var(--text-disabled)", marginTop: "0.15rem" }}>
            {Object.entries(metrics.documents_by_status)
              .map(([status, count]) => `${count} ${status.toLowerCase()}`)
              .join(", ")}
          </div>
        </div>
        <div style={card}>
          <div style={cardValue}>
            {metrics.avg_grounding_rate > 0 ? `${metrics.avg_grounding_rate.toFixed(1)}%` : "--"}
          </div>
          <div style={cardLabel}>Avg Grounding</div>
        </div>
        <div style={card}>
          <div style={cardValue}>{completed} / {metrics.total_documents}</div>
          <div style={cardLabel}>Completed</div>
        </div>
      </div>

      {/* Estimates */}
      {metrics.estimates.confidence !== "none" ? (
        <div style={{ ...cardRow, marginBottom: "1.5rem" }}>
          {metrics.estimates.estimated_remaining_time_secs != null && (
            <div style={card}>
              <div style={cardValue}>~{fmtTime(metrics.estimates.estimated_remaining_time_secs)}</div>
              <div style={cardLabel}>Est. Remaining Time</div>
            </div>
          )}
          <div style={card}>
            <div style={cardValue}>{metrics.estimates.documents_remaining}</div>
            <div style={cardLabel}>Docs Remaining</div>
            <div style={{ fontSize: "0.72rem", color: "var(--text-disabled)", marginTop: "0.15rem" }}>
              Confidence: {metrics.estimates.confidence}
            </div>
          </div>
        </div>
      ) : (
        <div style={{ fontSize: "0.8rem", color: "var(--text-disabled)", marginBottom: "1.5rem" }}>
          Not enough data to estimate remaining time.
        </div>
      )}

      {/* Step performance table */}
      <h2 style={{ fontSize: "1rem", fontWeight: 600, color: "var(--text-secondary)", marginBottom: "0.75rem" }}>
        Step Performance
        <span style={{ fontSize: "0.76rem", fontWeight: 400, color: "var(--text-disabled)", marginLeft: "0.5rem" }}>
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
                        height: "8px", backgroundColor: "var(--border-default)", borderRadius: "4px",
                        overflow: "hidden",
                      }}>
                        <div style={{
                          width: `${pct}%`, height: "100%", borderRadius: "4px",
                          backgroundColor: isMax ? "var(--state-warning-strong)" : "var(--accent-primary)",
                          transition: "width 0.3s ease",
                        }} />
                      </div>
                    </td>
                    <td style={td}>{step.count}</td>
                    <td style={{ ...td, color: step.failure_count > 0 ? "var(--state-danger-strong)" : "var(--text-muted)" }}>
                      {step.failure_count}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {/* Reviewer workload removed — audit workload coming in a future session */}
    </div>
  );
};

export default AdminMetrics;
