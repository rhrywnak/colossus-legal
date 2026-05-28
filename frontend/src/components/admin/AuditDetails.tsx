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
  critical: { backgroundColor: "var(--state-danger-bg-soft)", color: "var(--state-danger-strong)" },
  high:     { backgroundColor: "var(--burden-warning-bg)", color: "var(--state-warning-strong)" },
  medium:   { backgroundColor: "var(--burden-warning-bg)", color: "var(--burden-warning-text)" },
  low:      { backgroundColor: "var(--bg-page)", color: "var(--text-muted)" },
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
      fontSize: "0.74rem", color: "var(--text-secondary)", lineHeight: "1.5",
      borderTop: "1px solid var(--border-default)", paddingTop: "0.4rem", marginTop: "0.15rem",
    }}>
      {/* Verification details */}
      {ver && ver.status !== "pending" && (
        <div style={{ marginBottom: evidence.flags.length > 0 ? "0.4rem" : 0 }}>
          <span style={{ fontWeight: 600, color: "var(--text-secondary)" }}>
            {ver.status === "verified" ? "Verified" : "Rejected"}
          </span>
          {" by "}
          <span style={{ fontWeight: 500 }}>{ver.verified_by}</span>
          {" — "}{formatDate(ver.verified_at)}
          {ver.notes && (
            <div style={{
              marginTop: "0.2rem", fontStyle: "italic", color: "var(--text-muted)",
              paddingLeft: "0.5rem", borderLeft: "2px solid var(--border-default)",
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
          paddingLeft: "0.5rem", borderLeft: "2px solid var(--state-danger-border)",
        }}>
          <span style={{ ...sevBadge, ...(severityStyles[flag.severity] || severityStyles.low) }}>
            {flag.severity}
          </span>
          {flag.description && (
            <span style={{ marginLeft: "0.35rem" }}>{flag.description}</span>
          )}
          <div style={{ fontSize: "0.68rem", color: "var(--text-disabled)" }}>
            {flag.flagged_by} — {formatDate(flag.flagged_at)}
          </div>
        </div>
      ))}
    </div>
  );
};

export default AuditDetails;
