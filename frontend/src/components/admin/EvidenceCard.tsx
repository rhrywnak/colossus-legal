/**
 * EvidenceCard — Displays a single evidence item with verify/flag actions.
 *
 * Used in the Document Workspace right panel. Shows the evidence title,
 * speaker, verbatim quote, page number, and audit status badges.
 * Clicking the card navigates the PDF viewer to the cited page.
 *
 * TODO: B-4 — v1 dead code. This component is part of the manual evidence
 * import workflow (DocumentEvidence service) superseded by the v2 pipeline.
 * No Evidence nodes exist in v2. Remove or rewrite when v1 is fully deprecated.
 */
import React, { useState } from "react";
import { DocumentEvidence } from "../../services/documentEvidence";
import { getNodeTypeDisplay, getPageLabel } from "../../utils/nodeTypeDisplay";
import AuditDetails from "./AuditDetails";

interface EvidenceCardProps {
  evidence: DocumentEvidence;
  isSelected: boolean;
  onSelect: (evidence: DocumentEvidence) => void;
  onVerify: (evidence: DocumentEvidence) => void;
  onFlag: (evidence: DocumentEvidence) => void;
}

// ── Status badge styles ──────────────────────────────────────────

const badgeBase: React.CSSProperties = {
  display: "inline-block", padding: "0.15rem 0.5rem", borderRadius: "9999px",
  fontSize: "0.7rem", fontWeight: 600, letterSpacing: "0.02em",
};

const statusBadge = (status: string): React.CSSProperties => {
  switch (status) {
    case "verified":
      return { ...badgeBase, backgroundColor: "#dcfce7", color: "#166534" };
    case "rejected":
      return { ...badgeBase, backgroundColor: "#fee2e2", color: "#991b1b" };
    default:
      return { ...badgeBase, backgroundColor: "#f1f5f9", color: "#64748b" };
  }
};

const severityColor: Record<string, string> = {
  critical: "#dc2626", high: "#ea580c", medium: "#d97706", low: "#65a30d",
};

// ── Component ────────────────────────────────────────────────────

const EvidenceCard: React.FC<EvidenceCardProps> = ({
  evidence, isSelected, onSelect, onVerify, onFlag,
}) => {
  const verStatus = evidence.verification?.status;
  const [expanded, setExpanded] = useState(false);
  const { label: nodeLabel, color: nodeColor } = getNodeTypeDisplay(evidence.node_type);
  const pageLabel = getPageLabel(evidence.node_type, evidence.page_number);

  // Show toggle when there's verification or flag data to display
  const hasDetails =
    (evidence.verification && evidence.verification.status !== "pending") ||
    evidence.flags.length > 0;

  return (
    <div
      onClick={() => onSelect(evidence)}
      style={{
        padding: "0.75rem 1rem",
        borderRadius: "8px",
        border: isSelected ? "2px solid #2563eb" : "1px solid #e2e8f0",
        backgroundColor: isSelected ? "#eff6ff" : "#fff",
        cursor: "pointer",
        transition: "border-color 0.15s, background-color 0.15s",
        marginBottom: "0.5rem",
      }}
    >
      {/* Header: node type badge + title + page badge */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "0.35rem" }}>
        <div style={{ display: "flex", alignItems: "center", gap: "0.4rem", flex: 1, marginRight: "0.5rem" }}>
          <span style={{
            display: "inline-block", padding: "0.1rem 0.45rem", borderRadius: "4px",
            fontSize: "0.68rem", fontWeight: 600, backgroundColor: nodeColor,
            color: "#fff", whiteSpace: "nowrap", flexShrink: 0, lineHeight: "1.4",
          }}>
            {nodeLabel}
          </span>
          <span style={{ fontSize: "0.84rem", fontWeight: 600, color: "#0f172a" }}>
            {evidence.title || evidence.id}
          </span>
        </div>
        {pageLabel && (
          <span style={{
            ...badgeBase, backgroundColor: "#e0e7ff", color: "#3730a3",
            whiteSpace: "nowrap", flexShrink: 0,
          }}>
            {pageLabel}
          </span>
        )}
      </div>

      {/* Speaker */}
      {evidence.speaker && (
        <div style={{ fontSize: "0.76rem", color: "#475569", marginBottom: "0.35rem" }}>
          — {evidence.speaker}
        </div>
      )}

      {/* Verbatim quote */}
      <div style={{
        fontSize: "0.78rem", color: "#334155", fontStyle: evidence.verbatim_quote ? "italic" : "normal",
        borderLeft: "3px solid #e2e8f0", paddingLeft: "0.6rem",
        marginBottom: "0.5rem", lineHeight: "1.45",
        maxHeight: "4.5em", overflow: "hidden",
      }}>
        {evidence.verbatim_quote
          ? (evidence.verbatim_quote.length > 200
              ? evidence.verbatim_quote.slice(0, 200) + "..."
              : evidence.verbatim_quote)
          : "No quote recorded"}
      </div>

      {/* Status badges row */}
      <div style={{ display: "flex", alignItems: "center", gap: "0.4rem", marginBottom: "0.5rem", flexWrap: "wrap" }}>
        {verStatus && (
          <span style={statusBadge(verStatus)}>
            {verStatus.charAt(0).toUpperCase() + verStatus.slice(1)}
          </span>
        )}
        {evidence.flags.length > 0 && (
          <span style={{
            ...badgeBase,
            backgroundColor: "#fef3c7",
            color: severityColor[evidence.flags[0].severity] || "#d97706",
          }}>
            {evidence.flags.length} flag{evidence.flags.length > 1 ? "s" : ""}
          </span>
        )}
        {evidence.kind && (
          <span style={{ ...badgeBase, backgroundColor: "#f1f5f9", color: "#64748b" }}>
            {evidence.kind}
          </span>
        )}
      </div>

      {/* Collapsible audit details */}
      {hasDetails && (
        <div style={{ marginBottom: "0.4rem" }}>
          <button
            onClick={(e) => { e.stopPropagation(); setExpanded(!expanded); }}
            style={{
              background: "none", border: "none", padding: 0, cursor: "pointer",
              fontSize: "0.72rem", color: "#2563eb", fontFamily: "inherit", fontWeight: 500,
            }}
          >
            {expanded ? "Hide details" : "Show details"}
          </button>
          {expanded && <AuditDetails evidence={evidence} />}
        </div>
      )}

      {/* Action buttons */}
      <div style={{ display: "flex", gap: "0.4rem" }}>
        <button
          onClick={(e) => { e.stopPropagation(); onVerify(evidence); }}
          style={{
            padding: "0.25rem 0.6rem", fontSize: "0.74rem", fontWeight: 500,
            border: "1px solid #a7f3d0", borderRadius: "5px",
            backgroundColor: "#ecfdf5", color: "#047857", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          Verify
        </button>
        <button
          onClick={(e) => { e.stopPropagation(); onFlag(evidence); }}
          style={{
            padding: "0.25rem 0.6rem", fontSize: "0.74rem", fontWeight: 500,
            border: "1px solid #fecaca", borderRadius: "5px",
            backgroundColor: "#fef2f2", color: "#dc2626", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          Flag
        </button>
      </div>
    </div>
  );
};

export default EvidenceCard;
