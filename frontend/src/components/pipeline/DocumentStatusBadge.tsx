import React from "react";

const STATUS_COLORS: Record<string, { bg: string; text: string }> = {
  NEW: { bg: "#eff6ff", text: "#2563eb" },
  PROCESSING: { bg: "#fffbeb", text: "#d97706" },
  COMPLETED: { bg: "#f0fdf4", text: "#16a34a" },
  FAILED: { bg: "#fef2f2", text: "#dc2626" },
  CANCELLED: { bg: "#f8fafc", text: "#64748b" },
};

const DEFAULT_COLOR = { bg: "#f1f5f9", text: "#64748b" };

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
