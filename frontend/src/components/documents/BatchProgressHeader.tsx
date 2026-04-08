/**
 * BatchProgressHeader — shows batch processing progress above the document list.
 *
 * Displays a progress bar, status bucket counts, and cost/time estimates.
 * Only renders when there are unpublished documents.
 */
import React from "react";
import { PipelineDocument, EstimatesData } from "../../services/pipelineApi";

interface BatchProgressHeaderProps {
  documents: PipelineDocument[];
  estimates: EstimatesData | null;
  onStatusFilter: (status: string) => void;
}

// ── Helpers ─────────────────────────────────────────────────────

function fmtCost(usd: number): string {
  return `$${usd.toFixed(2)}`;
}

function fmtTime(secs: number): string {
  if (secs < 60) return `${Math.round(secs)}s`;
  if (secs < 3600) return `${Math.round(secs / 60)}m`;
  const h = Math.floor(secs / 3600);
  const m = Math.round((secs % 3600) / 60);
  return `${h}h ${m}m`;
}


// ── Styles ──────────────────────────────────────────────────────

const container: React.CSSProperties = {
  backgroundColor: "#f8fafc", border: "1px solid #e2e8f0", borderRadius: "8px",
  padding: "1rem 1.25rem", marginBottom: "1.25rem",
};
const progressBarOuter: React.CSSProperties = {
  height: "8px", backgroundColor: "#e2e8f0", borderRadius: "4px",
  overflow: "hidden", marginBottom: "0.75rem",
};
const bucketsRow: React.CSSProperties = {
  display: "flex", gap: "0.5rem", flexWrap: "wrap", marginBottom: "0.75rem",
};
const bucketBtn: React.CSSProperties = {
  padding: "0.3rem 0.6rem", fontSize: "0.76rem", borderRadius: "4px",
  border: "1px solid #e2e8f0", background: "#ffffff", cursor: "pointer",
  fontFamily: "inherit", color: "#334155",
};
const metaRow: React.CSSProperties = {
  display: "flex", gap: "1.25rem", fontSize: "0.76rem", color: "#64748b",
  flexWrap: "wrap",
};

// ── Component ───────────────────────────────────────────────────

const BatchProgressHeader: React.FC<BatchProgressHeaderProps> = ({
  documents, estimates, onStatusFilter,
}) => {
  if (documents.length === 0) return null;

  const total = documents.length;
  const buckets = { published: 0, uploaded: 0, in_review: 0, processing: 0 };
  let totalCost = 0;
  for (const d of documents) {
    buckets[(d.status_group ?? "processing") as keyof typeof buckets]++;
    if (d.total_cost_usd != null) totalCost += d.total_cost_usd;
  }
  const published = buckets.published;
  const pct = Math.round((published / total) * 100);

  if (published === total) {
    return (
      <div style={{ ...container, textAlign: "center", fontSize: "0.9rem", color: "#16a34a" }}>
        All {total} documents published
      </div>
    );
  }

  const bucketLabels: { key: string; label: string; filter: string }[] = [
    { key: "uploaded", label: "Uploaded", filter: "uploaded" },
    { key: "processing", label: "Processing", filter: "processing" },
    { key: "in_review", label: "In Review", filter: "in_review" },
    { key: "published", label: "Published", filter: "published" },
  ];

  return (
    <div style={container}>
      {/* Progress bar */}
      <div style={{ fontSize: "0.8rem", color: "#334155", marginBottom: "0.35rem", fontWeight: 500 }}>
        {published} / {total} published ({pct}%)
      </div>
      <div style={progressBarOuter}>
        <div style={{
          width: `${pct}%`, height: "100%", backgroundColor: "#2563eb",
          borderRadius: "4px", transition: "width 0.3s ease",
        }} />
      </div>

      {/* Status buckets */}
      <div style={bucketsRow}>
        {bucketLabels.map((b) => {
          const count = buckets[b.key as keyof typeof buckets];
          if (count === 0) return null;
          return (
            <button key={b.key} style={bucketBtn} onClick={() => onStatusFilter(b.filter)}>
              {count} {b.label}
            </button>
          );
        })}
      </div>

      {/* Cost / estimates */}
      <div style={metaRow}>
        {totalCost > 0 && <span>Total cost: {fmtCost(totalCost)}</span>}
        {estimates && estimates.confidence !== "none" && (
          <>
            {estimates.estimated_remaining_cost_usd != null && (
              <span>Est. remaining: ~{fmtCost(estimates.estimated_remaining_cost_usd)}</span>
            )}
            {estimates.estimated_remaining_time_secs != null && (
              <span>Est. time: ~{fmtTime(estimates.estimated_remaining_time_secs)}</span>
            )}
          </>
        )}
      </div>
    </div>
  );
};

export default BatchProgressHeader;
