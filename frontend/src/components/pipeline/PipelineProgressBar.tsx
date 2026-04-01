import React from "react";

const STEPS_ORDER = [
  "UPLOADED",
  "TEXT_EXTRACTED",
  "EXTRACTED",
  "VERIFIED",
  "INGESTED",
  "INDEXED",
  "PUBLISHED",
];

interface Props {
  status: string;
}

const PipelineProgressBar: React.FC<Props> = ({ status }) => {
  const idx = STEPS_ORDER.indexOf(status);
  const completed = idx >= 0 ? idx + 1 : 0;
  const total = STEPS_ORDER.length;
  const pct = Math.round((completed / total) * 100);

  return (
    <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
      <div style={{
        flex: 1, height: "6px", backgroundColor: "#e2e8f0", borderRadius: "3px", overflow: "hidden",
      }}>
        <div style={{
          width: `${pct}%`, height: "100%", borderRadius: "3px",
          backgroundColor: completed === total ? "#22c55e" : "#2563eb",
          transition: "width 0.3s ease",
        }} />
      </div>
      <span style={{ fontSize: "0.72rem", color: "#64748b", whiteSpace: "nowrap" }}>
        {completed}/{total}
      </span>
    </div>
  );
};

export default PipelineProgressBar;
