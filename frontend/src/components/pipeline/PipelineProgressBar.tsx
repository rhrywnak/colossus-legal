/**
 * PipelineProgressBar — Visual progress indicator for document processing.
 *
 * After the pipeline simplification (beta.61), documents use 5 statuses:
 * NEW → PROCESSING → COMPLETED (success path)
 *                 → FAILED     (error path)
 *                 → CANCELLED  (user cancelled)
 *
 * The old 7-step progress bar (UPLOADED → TEXT_EXTRACTED → ... → PUBLISHED)
 * was based on the pre-simplification 8-step manual pipeline and is no
 * longer meaningful. This replacement shows a simple state indicator.
 */
import React from "react";

interface Props {
  status: string;
  // During PROCESSING, show chunk-level progress if available
  percentComplete?: number;
}

const PipelineProgressBar: React.FC<Props> = ({ status, percentComplete }) => {
  // During PROCESSING, show the actual percent_complete from the backend.
  // This reflects real chunk progress, not a derived step position.
  if (status === "PROCESSING") {
    const pct = percentComplete ?? 0;
    return (
      <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
        <div style={{
          flex: 1, height: "6px", backgroundColor: "#e2e8f0",
          borderRadius: "3px", overflow: "hidden",
        }}>
          <div style={{
            width: `${pct}%`, height: "100%", borderRadius: "3px",
            backgroundColor: "#2563eb",
            transition: "width 0.3s ease",
          }} />
        </div>
        <span style={{ fontSize: "0.72rem", color: "#64748b", whiteSpace: "nowrap" }}>
          {pct}%
        </span>
      </div>
    );
  }

  // For terminal statuses, show a colored indicator bar.
  // Full green for COMPLETED, full red for FAILED, grey for CANCELLED/NEW.
  const color = status === "COMPLETED" ? "#22c55e"
    : status === "FAILED" ? "#ef4444"
    : status === "CANCELLED" ? "#94a3b8"
    : "#e2e8f0"; // NEW — empty bar

  const pct = status === "COMPLETED" || status === "FAILED" || status === "CANCELLED" ? 100 : 0;

  return (
    <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
      <div style={{
        flex: 1, height: "6px", backgroundColor: "#e2e8f0",
        borderRadius: "3px", overflow: "hidden",
      }}>
        <div style={{
          width: `${pct}%`, height: "100%", borderRadius: "3px",
          backgroundColor: color,
        }} />
      </div>
    </div>
  );
};

export default PipelineProgressBar;
