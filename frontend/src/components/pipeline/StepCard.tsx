import React from "react";
import type { PipelineStep } from "../../services/pipelineApi";

export interface StepDef {
  name: string;
  label: string;
  statusRequired: string | null;
  isManual?: boolean;
}

interface Props {
  step: StepDef;
  index: number;
  historyEntry: PipelineStep | undefined;
  documentStatus: string;
  isNextAction: boolean;
  running: boolean;
  onTrigger: (stepName: string) => void;
  onNavigate?: (stepName: string) => void;
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

const btnStyle = (enabled: boolean): React.CSSProperties => ({
  padding: "0.3rem 0.7rem", fontSize: "0.76rem", fontWeight: 600,
  border: "1px solid #2563eb", borderRadius: "6px", cursor: enabled ? "pointer" : "default",
  backgroundColor: enabled ? "#2563eb" : "#e2e8f0",
  color: enabled ? "#ffffff" : "#94a3b8",
  fontFamily: "inherit", opacity: enabled ? 1 : 0.6,
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

function formatMetric(entry: PipelineStep | undefined): string {
  if (!entry || entry.status !== "completed") return "";
  const r = entry.result_summary as Record<string, unknown>;
  switch (entry.step_name) {
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

function stepStatus(entry: PipelineStep | undefined): string {
  if (!entry) return "pending";
  return entry.status;
}

const StepCard: React.FC<Props> = ({
  step, index, historyEntry, isNextAction, running, onTrigger, onNavigate,
}) => {
  const status = stepStatus(historyEntry);
  const metric = formatMetric(historyEntry);
  const enabled = isNextAction && !running;

  const handleClick = () => {
    if (!enabled) return;
    if (step.isManual && onNavigate) {
      onNavigate(step.name);
    } else {
      onTrigger(step.name);
    }
  };

  const buttonLabel = step.isManual ? "Review Items \u2192" : `Run ${step.label}`;

  return (
    <div style={cardStyle}>
      <span style={iconStyle(status)}>{statusIcon(status)}</span>
      <span style={{ fontWeight: 600, fontSize: "0.84rem", color: "#0f172a", minWidth: "110px" }}>
        {index + 1}. {step.label}
      </span>
      <span style={{ fontSize: "0.76rem", color: "#64748b", minWidth: "60px" }}>
        {formatDuration(historyEntry?.duration_secs ?? null)}
      </span>
      <span style={{ flex: 1, fontSize: "0.76rem", color: "#64748b" }}>{metric}</span>
      {isNextAction && (
        <button style={btnStyle(enabled)} onClick={handleClick} disabled={!enabled}>
          {running ? "Running..." : buttonLabel}
        </button>
      )}
      {status === "failed" && historyEntry?.error_message && (
        <span style={{ fontSize: "0.72rem", color: "#ef4444", maxWidth: "200px", overflow: "hidden", textOverflow: "ellipsis" }}>
          {historyEntry.error_message}
        </span>
      )}
    </div>
  );
};

export default StepCard;
