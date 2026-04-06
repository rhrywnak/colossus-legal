/**
 * AuditDetails — Collapsible verification + flag details for an EvidenceCard.
 *
 * Shows verification info (who, when, notes) and individual flag entries
 * (severity badge, description, who flagged and when).
 *
 * TODO: B-4 — v1 dead code. Part of the manual evidence import workflow
 * superseded by the v2 pipeline. Remove or rewrite when v1 is deprecated.
 */
import React from "react";
import { DocumentEvidence } from "../../services/documentEvidence";

const severityStyles: Record<string, React.CSSProperties> = {
  critical: { backgroundColor: "#fee2e2", color: "#dc2626" },
  high:     { backgroundColor: "#ffedd5", color: "#ea580c" },
  medium:   { backgroundColor: "#fef9c3", color: "#a16207" },
  low:      { backgroundColor: "#f1f5f9", color: "#64748b" },
};

const sevBadge: React.CSSProperties = {
  display: "inline-block", padding: "0.1rem 0.4rem", borderRadius: "9999px",
  fontSize: "0.65rem", fontWeight: 600, letterSpacing: "0.02em",
};

function formatDate(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
}

interface AuditDetailsProps {
  evidence: DocumentEvidence;
}

const AuditDetails: React.FC<AuditDetailsProps> = ({ evidence }) => {
  const ver = evidence.verification;
  const hasDetails = (ver && ver.status !== "pending") || evidence.flags.length > 0;

  if (!hasDetails) return null;

  return (
    <div style={{
      fontSize: "0.74rem", color: "#475569", lineHeight: "1.5",
      borderTop: "1px solid #e2e8f0", paddingTop: "0.4rem", marginTop: "0.15rem",
    }}>
      {/* Verification details */}
      {ver && ver.status !== "pending" && (
        <div style={{ marginBottom: evidence.flags.length > 0 ? "0.4rem" : 0 }}>
          <span style={{ fontWeight: 600, color: "#334155" }}>
            {ver.status === "verified" ? "Verified" : "Rejected"}
          </span>
          {" by "}
          <span style={{ fontWeight: 500 }}>{ver.verified_by}</span>
          {" — "}{formatDate(ver.verified_at)}
          {ver.notes && (
            <div style={{
              marginTop: "0.2rem", fontStyle: "italic", color: "#64748b",
              paddingLeft: "0.5rem", borderLeft: "2px solid #e2e8f0",
            }}>
              {ver.notes}
            </div>
          )}
        </div>
      )}

      {/* Flag details */}
      {evidence.flags.map((flag, i) => (
        <div key={i} style={{
          marginBottom: i < evidence.flags.length - 1 ? "0.35rem" : 0,
          paddingLeft: "0.5rem", borderLeft: "2px solid #fecaca",
        }}>
          <span style={{ ...sevBadge, ...(severityStyles[flag.severity] || severityStyles.low) }}>
            {flag.severity}
          </span>
          {flag.description && (
            <span style={{ marginLeft: "0.35rem" }}>{flag.description}</span>
          )}
          <div style={{ fontSize: "0.68rem", color: "#94a3b8" }}>
            {flag.flagged_by} — {formatDate(flag.flagged_at)}
          </div>
        </div>
      ))}
    </div>
  );
};

export default AuditDetails;
