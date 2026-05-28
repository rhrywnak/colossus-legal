import React from "react";

const statusBadgeColors: Record<string, { bg: string; text: string; border: string }> = {
  PUBLISHED:  { bg: "var(--state-success-bg-soft)", text: "var(--status-active-text)", border: "var(--state-success-bg-soft)" },
  UPLOADED:   { bg: "var(--accent-bg-soft)", text: "var(--accent-primary)", border: "var(--accent-bg-soft)" },
  IN_REVIEW:  { bg: "var(--burden-warning-bg)", text: "var(--burden-warning-text)", border: "var(--burden-warning-bg)" },
  EXTRACTED:  { bg: "var(--state-success-bg-soft)", text: "var(--status-active-text)", border: "var(--state-success-bg-soft)" },
};

const StatusBadge: React.FC<{ status: string }> = ({ status }) => {
  const c = statusBadgeColors[status] || { bg: "var(--bg-page)", text: "var(--text-secondary)", border: "var(--border-default)" };
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
