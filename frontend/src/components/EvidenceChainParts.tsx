import React from "react";
import { MotionClaimWithEvidence, EvidenceWithDocument } from "../services/evidenceChain";

// Professional color palette (shared with EvidenceExplorerPage)
export const COLORS = {
  bgPage: "#f8fafc",
  bgCard: "#ffffff",
  border: "#e2e8f0",
  textPrimary: "#1e293b",
  textSecondary: "#64748b",
  textMuted: "#94a3b8",
  accentExpanded: "#059669",
  accentMotionClaim: "#3b82f6",
  accentEvidence: "#8b5cf6",
  badgeIdBg: "#f1f5f9",
  badgeIdText: "#475569",
  badgeProvenBg: "#ecfdf5",
  badgeProvenText: "#059669",
  badgePartialBg: "#fffbeb",
  badgePartialText: "#d97706",
  badgeUnprovenBg: "#fef2f2",
  badgeUnprovenText: "#dc2626",
  badgeLegalBg: "#e0e7ff",
  badgeLegalText: "#4338ca",
  badgeParaBg: "#dbeafe",
  badgeParaText: "#1e40af",
};

export const STATUS_STYLES: Record<string, { bg: string; text: string }> = {
  PROVEN: { bg: COLORS.badgeProvenBg, text: COLORS.badgeProvenText },
  PARTIAL: { bg: COLORS.badgePartialBg, text: COLORS.badgePartialText },
  UNPROVEN: { bg: COLORS.badgeUnprovenBg, text: COLORS.badgeUnprovenText },
};

const DEFAULT_STATUS_STYLE = { bg: COLORS.badgeIdBg, text: COLORS.textSecondary };

export function getStatusStyle(status: string | undefined) {
  if (!status) return DEFAULT_STATUS_STYLE;
  return STATUS_STYLES[status.toUpperCase()] || DEFAULT_STATUS_STYLE;
}

// ─── DocumentLink ────────────────────────────────────────────────────────────

type DocumentLinkProps = {
  document: { id: string; title: string; page_number?: number };
};

export const DocumentLink: React.FC<DocumentLinkProps> = ({ document }) => (
  <div
    style={{
      marginLeft: "1.5rem",
      marginTop: "0.5rem",
      padding: "0.5rem 0.75rem",
      backgroundColor: COLORS.bgPage,
      border: `1px solid ${COLORS.border}`,
      borderRadius: "4px",
      fontSize: "0.85rem",
      color: COLORS.badgeIdText,
      display: "flex",
      alignItems: "center",
      gap: "0.5rem",
    }}
  >
    <span style={{ color: COLORS.textSecondary }}>Source:</span>
    <span style={{ fontWeight: 500 }}>{document.title}</span>
    {document.page_number !== undefined && (
      <span style={{ color: COLORS.textSecondary }}>(p. {document.page_number})</span>
    )}
  </div>
);

// ─── EvidenceItem ────────────────────────────────────────────────────────────

type EvidenceItemProps = {
  evidence: EvidenceWithDocument;
};

export const EvidenceItem: React.FC<EvidenceItemProps> = ({ evidence }) => (
  <div
    style={{
      marginLeft: "1rem",
      backgroundColor: COLORS.bgCard,
      border: `1px solid ${COLORS.border}`,
      borderLeft: `3px solid ${COLORS.accentEvidence}`,
      borderRadius: "6px",
      padding: "0.875rem",
    }}
  >
    <div style={{ fontWeight: 500, fontSize: "0.9rem", color: COLORS.textPrimary, marginBottom: "0.5rem" }}>
      {evidence.title}
    </div>

    {evidence.question && (
      <div style={{ fontSize: "0.85rem", color: COLORS.textSecondary, marginBottom: "0.35rem" }}>
        <span style={{ fontWeight: 600 }}>Q:</span>{" "}
        <span style={{ color: COLORS.badgeIdText }}>{evidence.question}</span>
      </div>
    )}

    {evidence.answer && (
      <div style={{ fontSize: "0.85rem", color: COLORS.textSecondary, marginBottom: "0.5rem" }}>
        <span style={{ fontWeight: 600 }}>A:</span>{" "}
        <span style={{ color: COLORS.textPrimary, fontStyle: "italic" }}>
          &ldquo;{evidence.answer}&rdquo;
        </span>
      </div>
    )}

    {evidence.document && <DocumentLink document={evidence.document} />}
  </div>
);

// ─── MotionClaimSection ──────────────────────────────────────────────────────

type MotionClaimSectionProps = {
  motionClaim: MotionClaimWithEvidence;
};

export const MotionClaimSection: React.FC<MotionClaimSectionProps> = ({ motionClaim }) => (
  <div
    style={{
      backgroundColor: COLORS.bgCard,
      border: `1px solid ${COLORS.border}`,
      borderLeft: `3px solid ${COLORS.accentMotionClaim}`,
      borderRadius: "6px",
      padding: "1rem",
    }}
  >
    <div style={{ fontWeight: 500, fontSize: "0.95rem", color: COLORS.textPrimary, marginBottom: "0.75rem" }}>
      {motionClaim.title}
    </div>

    {motionClaim.evidence.length === 0 ? (
      <div style={{ color: COLORS.textMuted, fontSize: "0.85rem", fontStyle: "italic" }}>
        No evidence linked
      </div>
    ) : (
      <div style={{ display: "flex", flexDirection: "column", gap: "0.5rem" }}>
        {motionClaim.evidence.map((ev) => (
          <EvidenceItem key={ev.id} evidence={ev} />
        ))}
      </div>
    )}
  </div>
);
