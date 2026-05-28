import React from "react";
import { ContradictionDto } from "../services/contradictions";

// ─── Topic label humanization ────────────────────────────────────────────────

const TOPIC_LABELS: Record<string, string> = {
  marie_character: "Marie's Character",
  selective_enforcement: "Selective Enforcement",
  property_division_obstruction: "Property Division",
  frivolous_claims: "Frivolous Claims Accusation",
  estate_necessity: "Estate Necessity",
};

function humanizeTopic(topic: string): string {
  return TOPIC_LABELS[topic] || topic.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

// ─── Severity badge colors ───────────────────────────────────────────────────

function severityColor(value: string | undefined): string {
  if (!value) return "var(--text-muted)";
  if (value === "HIGH") return "var(--state-danger-strong)";
  if (value === "MEDIUM") return "var(--state-warning-strong)";
  return "var(--text-muted)";
}

// ─── Card component ──────────────────────────────────────────────────────────

const ImpeachmentCard: React.FC<{ contradiction: ContradictionDto }> = ({ contradiction }) => (
  <div style={{ border: "1px solid var(--border-default)", borderRadius: "8px", overflow: "hidden", marginBottom: "1.25rem" }}>
    {/* Header */}
    <div style={{
      padding: "0.75rem 1rem", backgroundColor: "var(--bg-page)", borderBottom: "1px solid var(--border-default)",
      fontWeight: 600, color: "var(--text-secondary)", display: "flex", alignItems: "center", gap: "0.75rem",
    }}>
      <span>{contradiction.topic ? humanizeTopic(contradiction.topic) : "Impeachment Evidence"}</span>
      {contradiction.impeachment_value && (
        <span style={{
          padding: "0.25rem 0.5rem", borderRadius: "4px", fontSize: "0.75rem",
          fontWeight: 600, color: "var(--bg-surface)", backgroundColor: severityColor(contradiction.impeachment_value),
        }}>
          {contradiction.impeachment_value}
        </span>
      )}
      {contradiction.description && (
        <span style={{ fontWeight: "normal", color: "var(--text-muted)" }}>— {contradiction.description}</span>
      )}
    </div>

    {/* Side-by-side comparison */}
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr" }}>
      {/* Left — Claimed */}
      <div style={{ padding: "1rem", backgroundColor: "var(--state-danger-bg-soft)", borderRight: "1px solid var(--border-default)" }}>
        {contradiction.earlier_claim && (
          <>
            <div style={{ fontSize: "0.75rem", fontWeight: 600, color: "var(--status-dropped-text)", marginBottom: "0.5rem", textTransform: "uppercase" }}>
              Claimed:
            </div>
            <blockquote style={{
              margin: "0 0 1rem 0", padding: "0.5rem 0.75rem", borderLeft: "3px solid var(--state-danger-border)",
              backgroundColor: "var(--bg-surface)", color: "var(--text-secondary)", fontSize: "0.9rem", fontStyle: "italic", lineHeight: 1.5,
            }}>
              &ldquo;{contradiction.earlier_claim}&rdquo;
            </blockquote>
          </>
        )}
        <div style={{ fontSize: "0.75rem", fontWeight: 600, color: "var(--status-dropped-text)", marginBottom: "0.5rem", textTransform: "uppercase" }}>
          Claim A
        </div>
        {contradiction.evidence_a.title && (
          <div style={{ fontWeight: 600, marginBottom: "0.5rem", color: "var(--text-primary)" }}>{contradiction.evidence_a.title}</div>
        )}
        {contradiction.evidence_a.answer && (
          <blockquote style={{
            margin: "0.5rem 0", padding: "0.5rem 0.75rem", borderLeft: "3px solid var(--state-danger-border)",
            backgroundColor: "var(--bg-surface)", color: "var(--text-secondary)", fontSize: "0.9rem", fontStyle: "italic", lineHeight: 1.5,
          }}>
            &ldquo;{contradiction.evidence_a.answer}&rdquo;
          </blockquote>
        )}
        {/* TODO: Link to /documents/:id when document_id is added to ContradictionEvidence API type */}
        {contradiction.evidence_a.document_title && (
          <div style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "0.5rem" }}>
            Source: {contradiction.evidence_a.document_title}
          </div>
        )}
      </div>

      {/* Right — Actually admitted */}
      <div style={{ padding: "1rem", backgroundColor: "var(--state-success-bg-soft)" }}>
        {contradiction.later_admission && (
          <>
            <div style={{ fontSize: "0.75rem", fontWeight: 600, color: "var(--status-active-text)", marginBottom: "0.5rem", textTransform: "uppercase" }}>
              Actually admitted:
            </div>
            <blockquote style={{
              margin: "0 0 1rem 0", padding: "0.5rem 0.75rem", borderLeft: "3px solid var(--state-success-bg-soft)",
              backgroundColor: "var(--bg-surface)", color: "var(--text-secondary)", fontSize: "0.9rem", fontStyle: "italic", lineHeight: 1.5,
            }}>
              &ldquo;{contradiction.later_admission}&rdquo;
            </blockquote>
          </>
        )}
        <div style={{ fontSize: "0.75rem", fontWeight: 600, color: "var(--status-active-text)", marginBottom: "0.5rem", textTransform: "uppercase" }}>
          Contradicted By
        </div>
        {contradiction.evidence_b.title && (
          <div style={{ fontWeight: 600, marginBottom: "0.5rem", color: "var(--text-primary)" }}>{contradiction.evidence_b.title}</div>
        )}
        {contradiction.evidence_b.answer && (
          <blockquote style={{
            margin: "0.5rem 0", padding: "0.5rem 0.75rem", borderLeft: "3px solid var(--state-success-bg-soft)",
            backgroundColor: "var(--bg-surface)", color: "var(--text-secondary)", fontSize: "0.9rem", fontStyle: "italic", lineHeight: 1.5,
          }}>
            &ldquo;{contradiction.evidence_b.answer}&rdquo;
          </blockquote>
        )}
        {/* TODO: Link to /documents/:id when document_id is added to ContradictionEvidence API type */}
        {contradiction.evidence_b.document_title && (
          <div style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "0.5rem" }}>
            Source: {contradiction.evidence_b.document_title}
          </div>
        )}
      </div>
    </div>
  </div>
);

export default ImpeachmentCard;
