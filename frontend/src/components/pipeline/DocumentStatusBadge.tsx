import React from "react";

const STATUS_COLORS: Record<string, { bg: string; text: string }> = {
  NEW: { bg: "var(--accent-bg-soft)", text: "var(--accent-primary)" },
  PROCESSING: { bg: "var(--burden-warning-bg)", text: "var(--state-warning-strong)" },
  COMPLETED: { bg: "var(--state-success-bg-soft)", text: "var(--state-success-strong)" },
  FAILED: { bg: "var(--state-danger-bg-soft)", text: "var(--state-danger-strong)" },
  CANCELLED: { bg: "var(--bg-page)", text: "var(--text-muted)" },
};

const DEFAULT_COLOR = { bg: "var(--bg-page)", text: "var(--text-muted)" };

interface Props {
  status: string;
}

const badgeStyle = (colors: { bg: string; text: string }): React.CSSProperties => ({
  display: "inline-block",
  padding: "0.15rem 0.55rem",
  borderRadius: "9999px",
  fontSize: "0.72rem",
  fontWeight: 600,
  backgroundColor: colors.bg,
  color: colors.text,
  whiteSpace: "nowrap",
});

const DocumentStatusBadge: React.FC<Props> = ({ status }) => {
  const colors = STATUS_COLORS[status] || DEFAULT_COLOR;
  return <span style={badgeStyle(colors)}>{status}</span>;
};

export default DocumentStatusBadge;
