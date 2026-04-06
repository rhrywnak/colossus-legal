/**
 * StepCard — Display-only card for a completed or in-progress pipeline step.
 *
 * Shows icon, label, duration, and metric summary. No business logic —
 * just presentation based on step data from the backend.
 */
import React from "react";

interface StepData {
  step_name: string;
  label: string;
  status: string;
  duration_secs: number | null;
  result_summary: Record<string, unknown> | null;
  error_message: string | null;
}

interface Props {
  step: StepData;
  index: number;
}

const cardStyle: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: "0.75rem",
  padding: "0.6rem 0.85rem", borderBottom: "1px solid #f1f5f9",
};

const iconStyle = (status: string): React.CSSProperties => ({
  width: "24px", textAlign: "center" as const, fontSize: "0.9rem",
  color: status === "completed" ? "#22c55e" : status === "failed" ? "#ef4444"
    : status === "running" ? "#2563eb" : "#cbd5e1",
});

function statusIcon(status: string): string {
  switch (status) {
    case "completed": return "\u2705";
    case "running": return "\uD83D\uDD04";
    case "failed": return "\u274C";
    default: return "\u2B1C";
  }
}

function formatDuration(secs: number | null): string {
  if (secs == null) return "--";
  if (secs < 60) return `${secs.toFixed(1)}s`;
  const mins = Math.floor(secs / 60);
  const s = Math.round(secs % 60);
  return `${mins}m ${s}s`;
}

function formatMetric(step: StepData): string {
  if (step.status !== "completed" || !step.result_summary) return "";
  const r = step.result_summary;
  switch (step.step_name) {
    case "extract_text": return `${r.page_count ?? ""} pages, ${r.total_chars ?? ""} chars`;
    case "extract": return `${r.entity_count ?? ""} items`;
    case "verify": return r.grounding_rate != null ? `${r.grounding_rate}% grounded` : "";
    case "ingest": return `${r.nodes_created ?? ""} nodes, ${r.relationships_created ?? ""} rels`;
    case "index": return `${r.nodes_embedded ?? ""} embedded`;
    case "completeness": {
      const p = (r.checks_passed as number) ?? 0;
      const f = (r.checks_failed as number) ?? 0;
      return `${p}/${p + f} checks passed`;
    }
    default: return "";
  }
}

const StepCard: React.FC<Props> = ({ step, index }) => {
  const metric = formatMetric(step);

  return (
    <div style={cardStyle}>
      <span style={iconStyle(step.status)}>{statusIcon(step.status)}</span>
      <span style={{ fontWeight: 600, fontSize: "0.84rem", color: "#0f172a", minWidth: "110px" }}>
        {index + 1}. {step.label}
      </span>
      <span style={{ fontSize: "0.76rem", color: "#64748b", minWidth: "60px" }}>
        {formatDuration(step.duration_secs)}
      </span>
      <span style={{ flex: 1, fontSize: "0.76rem", color: "#64748b" }}>{metric}</span>
      {step.status === "failed" && step.error_message && (
        <span style={{ fontSize: "0.72rem", color: "#ef4444", maxWidth: "200px", overflow: "hidden", textOverflow: "ellipsis" }}>
          {step.error_message}
        </span>
      )}
    </div>
  );
};

export default StepCard;
