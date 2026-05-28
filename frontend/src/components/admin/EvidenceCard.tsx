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
      return { ...badgeBase, backgroundColor: "var(--state-success-bg-soft)", color: "var(--status-active-text)" };
    case "rejected":
      return { ...badgeBase, backgroundColor: "var(--state-danger-bg-soft)", color: "var(--status-dropped-text)" };
    default:
      return { ...badgeBase, backgroundColor: "var(--bg-page)", color: "var(--text-muted)" };
  }
};

const severityColor: Record<string, string> = {
  critical: "var(--state-danger-strong)", high: "var(--state-warning-strong)", medium: "var(--state-warning-strong)", low: "var(--state-success-strong)",
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
        border: isSelected ? "2px solid var(--accent-primary)" : "1px solid var(--border-default)",
        backgroundColor: isSelected ? "var(--accent-bg-soft)" : "var(--bg-surface)",
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
            color: "var(--bg-surface)", whiteSpace: "nowrap", flexShrink: 0, lineHeight: "1.4",
          }}>
            {nodeLabel}
          </span>
          <span style={{ fontSize: "0.84rem", fontWeight: 600, color: "var(--text-primary)" }}>
            {evidence.title || evidence.id}
          </span>
        </div>
        {pageLabel && (
          <span style={{
            ...badgeBase, backgroundColor: "var(--state-info-bg-soft)", color: "var(--bias-indigo-text)",
            whiteSpace: "nowrap", flexShrink: 0,
          }}>
            {pageLabel}
          </span>
        )}
      </div>

      {/* Speaker */}
      {evidence.speaker && (
        <div style={{ fontSize: "0.76rem", color: "var(--text-secondary)", marginBottom: "0.35rem" }}>
          — {evidence.speaker}
        </div>
      )}

      {/* Verbatim quote */}
      <div style={{
        fontSize: "0.78rem", color: "var(--text-secondary)", fontStyle: evidence.verbatim_quote ? "italic" : "normal",
        borderLeft: "3px solid var(--border-default)", paddingLeft: "0.6rem",
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
            backgroundColor: "var(--burden-warning-bg)",
            color: severityColor[evidence.flags[0].severity] || "var(--state-warning-strong)",
          }}>
            {evidence.flags.length} flag{evidence.flags.length > 1 ? "s" : ""}
          </span>
        )}
        {evidence.kind && (
          <span style={{ ...badgeBase, backgroundColor: "var(--bg-page)", color: "var(--text-muted)" }}>
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
              fontSize: "0.72rem", color: "var(--accent-primary)", fontFamily: "inherit", fontWeight: 500,
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
            border: "1px solid var(--state-success-bg-soft)", borderRadius: "5px",
            backgroundColor: "var(--state-success-bg-soft)", color: "var(--status-active-text)", cursor: "pointer",
            fontFamily: "inherit",
          }}
        >
          Verify
        </button>
        <button
          onClick={(e) => { e.stopPropagation(); onFlag(evidence); }}
          style={{
            padding: "0.25rem 0.6rem", fontSize: "0.74rem", fontWeight: 500,
            border: "1px solid var(--state-danger-border)", borderRadius: "5px",
            backgroundColor: "var(--state-danger-bg-soft)", color: "var(--state-danger-strong)", cursor: "pointer",
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
