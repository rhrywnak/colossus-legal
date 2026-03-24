import React from "react";

const statusBadgeColors: Record<string, { bg: string; text: string; border: string }> = {
  PUBLISHED:  { bg: "#ecfdf5", text: "#047857", border: "#a7f3d0" },
  UPLOADED:   { bg: "#eff6ff", text: "#1d4ed8", border: "#bfdbfe" },
  IN_REVIEW:  { bg: "#fef3c7", text: "#92400e", border: "#fcd34d" },
  EXTRACTED:  { bg: "#f0fdfa", text: "#0f766e", border: "#99f6e4" },
};

const StatusBadge: React.FC<{ status: string }> = ({ status }) => {
  const c = statusBadgeColors[status] || { bg: "#f1f5f9", text: "#475569", border: "#e2e8f0" };
  return (
    <span style={{
      display: "inline-block", padding: "0.1rem 0.45rem", fontSize: "0.7rem",
      fontWeight: 600, borderRadius: "4px", backgroundColor: c.bg,
      color: c.text, border: `1px solid ${c.border}`, textTransform: "uppercase",
      letterSpacing: "0.03em",
    }}>
      {status}
    </span>
  );
};

export default StatusBadge;
