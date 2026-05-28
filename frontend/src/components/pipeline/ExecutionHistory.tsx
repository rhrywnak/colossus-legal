import React, { useState } from "react";
import type { PipelineStep } from "../../services/pipelineApi";

interface Props {
  steps: PipelineStep[];
}

const headerStyle: React.CSSProperties = {
  display: "flex", alignItems: "center", gap: "0.5rem", cursor: "pointer",
  padding: "0.6rem 0", fontSize: "0.84rem", fontWeight: 600, color: "var(--text-secondary)",
  userSelect: "none",
};

const rowStyle: React.CSSProperties = {
  display: "flex", gap: "1rem", padding: "0.4rem 0.85rem",
  fontSize: "0.76rem", borderBottom: "1px solid var(--bg-page)",
};

const headerRowStyle: React.CSSProperties = {
  ...rowStyle,
  backgroundColor: "var(--bg-page)",
  borderBottom: "1px solid var(--border-default)",
  fontWeight: 600,
  color: "var(--text-secondary)",
  textTransform: "uppercase",
  fontSize: "0.68rem",
  letterSpacing: "0.03em",
};

const cellStyle: React.CSSProperties = { color: "var(--text-muted)" };

const ExecutionHistory: React.FC<Props> = ({ steps }) => {
  const [expanded, setExpanded] = useState(false);

  return (
    <div style={{ marginTop: "1.25rem" }}>
      <div style={headerStyle} onClick={() => setExpanded(!expanded)}>
        <span>{expanded ? "\u25BC" : "\u25B6"}</span>
        Execution History ({steps.length} entries)
      </div>
      {expanded && (
        <div style={{
          backgroundColor: "var(--bg-surface)", borderRadius: "8px", border: "1px solid var(--border-default)",
          overflow: "hidden",
        }}>
          {steps.length === 0 ? (
            <div style={{ padding: "1rem", color: "var(--text-disabled)", fontSize: "0.84rem", textAlign: "center" }}>
              No execution history yet.
            </div>
          ) : (
            <>
              <div style={headerRowStyle}>
                <span style={{ minWidth: "130px" }}>Date</span>
                <span style={{ minWidth: "100px" }}>Step</span>
                <span style={{ minWidth: "70px" }}>Status</span>
                <span style={{ minWidth: "60px" }}>Duration</span>
                <span>Triggered By</span>
              </div>
              {steps.map((s) => (
              <div key={s.id} style={rowStyle}>
                <span style={{ ...cellStyle, minWidth: "130px" }}>
                  {new Date(s.started_at).toLocaleString()}
                </span>
                <span style={{ ...cellStyle, minWidth: "100px", fontWeight: 500 }}>{s.step_label ?? s.step_name}</span>
                <span style={{
                  ...cellStyle, minWidth: "70px",
                  color: s.status === "completed" ? "var(--state-success-strong)" : s.status === "failed" ? "var(--state-danger-strong)" : "var(--accent-primary)",
                }}>
                  {s.status}
                </span>
                <span style={{ ...cellStyle, minWidth: "60px" }}>
                  {s.duration_secs != null ? `${s.duration_secs.toFixed(1)}s` : "--"}
                </span>
                <span style={cellStyle}>{s.triggered_by || ""}</span>
                {s.status === "failed" && s.error_message && (
                  <span style={{ color: "var(--state-danger-strong)", fontSize: "0.72rem" }}>{s.error_message}</span>
                )}
              </div>
              ))}
            </>
          )}
        </div>
      )}
    </div>
  );
};

export default ExecutionHistory;
