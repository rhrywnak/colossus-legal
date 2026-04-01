import React from "react";

const STATUS_COLORS: Record<string, { bg: string; text: string }> = {
  UPLOADED: { bg: "#f1f5f9", text: "#64748b" },
  TEXT_EXTRACTED: { bg: "#dbeafe", text: "#1e40af" },
  EXTRACTED: { bg: "#dbeafe", text: "#1e40af" },
  VERIFIED: { bg: "#fef9c3", text: "#854d0e" },
  INGESTED: { bg: "#fed7aa", text: "#9a3412" },
  INDEXED: { bg: "#e9d5ff", text: "#6b21a8" },
  PUBLISHED: { bg: "#dcfce7", text: "#166534" },
  FAILED: { bg: "#fee2e2", text: "#991b1b" },
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
