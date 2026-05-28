/**
 * BatchProgressHeader — shows batch processing progress above the document list.
 *
 * Displays a progress bar, status bucket counts, and time estimates.
 * Updated for 5-status model: new, processing, completed, failed, cancelled.
 */
import React from "react";
import { PipelineDocument, EstimatesData } from "../../services/pipelineApi";

interface BatchProgressHeaderProps {
  documents: PipelineDocument[];
  estimates: EstimatesData | null;
  onStatusFilter: (status: string) => void;
}

// ── Helpers ─────────────────────────────────────────────────────

function fmtTime(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  const h = Math.floor(secs / 3600);
  const m = Math.round((secs % 3600) / 60);
  return `${h}h ${m}m`;
}

// ── Styles ──────────────────────────────────────────────────────

const container: React.CSSProperties = {
  backgroundColor: "var(--bg-page)", border: "1px solid var(--border-default)", borderRadius: "8px",
  padding: "1rem 1.25rem", marginBottom: "1.25rem",
};
const progressBarOuter: React.CSSProperties = {
  height: "8px", backgroundColor: "var(--border-default)", borderRadius: "4px",
  overflow: "hidden", marginBottom: "0.75rem",
};
const bucketsRow: React.CSSProperties = {
  display: "flex", gap: "0.5rem", flexWrap: "wrap", marginBottom: "0.75rem",
};
const bucketBtn = (color: string): React.CSSProperties => ({
  padding: "0.3rem 0.6rem", fontSize: "0.76rem", borderRadius: "4px",
  border: `1px solid ${color}20`, background: `${color}10`, cursor: "pointer",
  fontFamily: "inherit", color,
});
const metaRow: React.CSSProperties = {
  display: "flex", gap: "1.25rem", fontSize: "0.76rem", color: "var(--text-muted)",
  flexWrap: "wrap",
};

// ── Component ───────────────────────────────────────────────────

const BatchProgressHeader: React.FC<BatchProgressHeaderProps> = ({
  documents, estimates, onStatusFilter,
}) => {
  if (documents.length === 0) return null;

  const total = documents.length;
  const buckets = { new: 0, processing: 0, completed: 0, failed: 0, cancelled: 0 };
  for (const d of documents) {
    const group = (d.status_group ?? "new") as keyof typeof buckets;
    if (group in buckets) buckets[group]++;
  }
  const completed = buckets.completed;
  const pct = total > 0 ? Math.round((completed / total) * 100) : 0;

  if (completed === total) {
    return (
      <div style={{ ...container, textAlign: "center", fontSize: "0.9rem", color: "var(--state-success-strong)" }}>
        All {total} documents completed
      </div>
    );
  }

  const bucketLabels: { key: keyof typeof buckets; label: string; color: string }[] = [
    { key: "failed", label: "Failed", color: "var(--state-danger-strong)" },
    { key: "processing", label: "Processing", color: "var(--state-warning-strong)" },
    { key: "new", label: "New", color: "var(--accent-primary)" },
    { key: "cancelled", label: "Cancelled", color: "var(--text-muted)" },
    { key: "completed", label: "Completed", color: "var(--state-success-strong)" },
  ];

  return (
    <div style={container}>
      {/* Progress bar */}
      <div style={{ fontSize: "0.8rem", color: "var(--text-secondary)", marginBottom: "0.35rem", fontWeight: 500 }}>
        {completed} / {total} completed ({pct}%)
      </div>
      <div style={progressBarOuter}>
        <div style={{
          width: `${pct}%`, height: "100%", backgroundColor: "var(--state-success-strong)",
          borderRadius: "4px", transition: "width 0.3s ease",
        }} />
      </div>

      {/* Status buckets */}
      <div style={bucketsRow}>
        {bucketLabels.map((b) => {
          const count = buckets[b.key];
          if (count === 0) return null;
          return (
            <button key={b.key} style={bucketBtn(b.color)} onClick={() => onStatusFilter(b.key)}>
              {count} {b.label}
            </button>
          );
        })}
      </div>

      {/* Estimates */}
      <div style={metaRow}>
        {estimates && estimates.confidence !== "none" && estimates.estimated_remaining_time_secs != null && (
          <span>Est. time: ~{fmtTime(estimates.estimated_remaining_time_secs)}</span>
        )}
      </div>
    </div>
  );
};

export default BatchProgressHeader;
