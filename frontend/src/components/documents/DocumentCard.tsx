/**
 * DocumentCard — renders a single document card in the documents list.
 *
 * Redesigned for the 5-status pipeline model (new, processing, completed,
 * failed, cancelled). Each status_group gets a distinct layout.
 */
import React from "react";
import { Link } from "react-router-dom";
import DocumentStatusBadge from "../pipeline/DocumentStatusBadge";
import { PipelineDocument, processDocument, cancelProcessing } from "../../services/pipelineApi";

interface DocumentCardProps {
  doc: PipelineDocument;
  isAdmin: boolean;
  onRefresh: () => void;
}

// ── Helpers ─────────────────────────────────────────────────────

function titleizeType(slug: string): string {
  return slug.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString();
}

/** Truncate long strings to roughly `max` chars with an ellipsis. */
function truncate(text: string, max = 60): string {
  return text.length > max ? text.slice(0, max - 1) + "\u2026" : text;
}

// ── Styles ──────────────────────────────────────────────────────

const cardStyle: React.CSSProperties = {
  backgroundColor: "#ffffff", border: "1px solid #e2e8f0", borderRadius: "8px",
  padding: "1rem 1.25rem", marginBottom: "0.75rem",
  transition: "box-shadow 0.15s ease",
};
const cardTitleLink: React.CSSProperties = {
  fontSize: "0.95rem", fontWeight: 600, color: "#0f172a", textDecoration: "none",
};
const metaText: React.CSSProperties = {
  fontSize: "0.76rem", color: "#64748b",
};

/** Small action button factory. */
const smallBtn = (bg: string): React.CSSProperties => ({
  padding: "0.2rem 0.6rem", fontSize: "0.72rem", fontWeight: 600, border: "none",
  borderRadius: "4px", backgroundColor: bg, color: "#ffffff", cursor: "pointer",
  fontFamily: "inherit",
});

// ── Component ───────────────────────────────────────────────────

const DocumentCard: React.FC<DocumentCardProps> = ({ doc, isAdmin, onRefresh }) => {
  const status = doc.status_group ?? "new";

  // -- Action helpers (stop propagation so the Link wrapper isn't triggered) --

  const handleProcess = async (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    await processDocument(doc.id);
    onRefresh();
  };

  const handleCancel = async (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    await cancelProcessing(doc.id);
    onRefresh();
  };

  // -- Status-specific body rows --

  const renderBody = (): React.ReactNode => {
    switch (status) {
      // ---- NEW ----
      case "new":
        return (
          <>
            <div style={{ ...metaText, marginBottom: "0.4rem" }}>
              {titleizeType(doc.document_type)} | Created {formatDate(doc.created_at)}
            </div>
            {isAdmin && (
              <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
                <button style={smallBtn("#2563eb")} onClick={handleProcess}>Process</button>
                <Link
                  to={`/documents/${doc.id}`}
                  style={{ ...metaText, fontSize: "0.72rem", color: "#dc2626", textDecoration: "underline" }}
                  onClick={(e) => e.stopPropagation()}
                >
                  Delete
                </Link>
              </div>
            )}
          </>
        );

      // ---- PROCESSING ----
      case "processing":
        return (
          <>
            <div style={{ ...metaText, marginBottom: "0.4rem" }}>
              {titleizeType(doc.document_type)} | {doc.processing_step_label ?? "Processing\u2026"}
            </div>
            <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
              {/* inline progress bar */}
              <div style={{ height: "6px", backgroundColor: "#e2e8f0", borderRadius: "3px", flex: 1 }}>
                <div style={{
                  width: `${doc.percent_complete ?? 0}%`, height: "100%",
                  backgroundColor: "#2563eb", borderRadius: "3px",
                  transition: "width 0.3s ease",
                }} />
              </div>
              <span style={{ fontSize: "0.72rem", color: "#64748b" }}>
                {doc.percent_complete ?? 0}%
              </span>
              {isAdmin && (
                <button style={smallBtn("#d97706")} onClick={handleCancel}>Cancel</button>
              )}
            </div>
          </>
        );

      // ---- COMPLETED ----
      case "completed":
        return (
          <>
            <div style={{ ...metaText, marginBottom: "0.25rem" }}>
              {titleizeType(doc.document_type)} | Processed {formatDate(doc.updated_at)}
              {doc.total_cost_usd != null && ` | $${doc.total_cost_usd.toFixed(2)}`}
            </div>
            <div style={metaText}>
              {doc.entities_written ?? 0} entities | {doc.relationships_written ?? 0} relationships
              {(doc.entities_flagged ?? 0) > 0 && ` | ${doc.entities_flagged} flagged`}
            </div>
          </>
        );

      // ---- FAILED ----
      case "failed":
        return (
          <>
            <div style={{ ...metaText, marginBottom: "0.25rem" }}>
              {titleizeType(doc.document_type)} | {truncate(doc.error_message ?? "Unknown error")}
            </div>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ ...metaText, fontStyle: "italic" }}>
                {doc.error_suggestion ? `Suggestion: ${doc.error_suggestion}` : ""}
              </span>
              {isAdmin && (
                <button style={smallBtn("#2563eb")} onClick={handleProcess}>Re-process</button>
              )}
            </div>
          </>
        );

      // ---- CANCELLED ----
      case "cancelled":
        return (
          <>
            <div style={{ ...metaText, marginBottom: "0.4rem" }}>
              {titleizeType(doc.document_type)} | Cancelled
            </div>
            {isAdmin && (
              <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
                <button style={smallBtn("#2563eb")} onClick={handleProcess}>Re-process</button>
                <Link
                  to={`/documents/${doc.id}`}
                  style={{ ...metaText, fontSize: "0.72rem", color: "#dc2626", textDecoration: "underline" }}
                  onClick={(e) => e.stopPropagation()}
                >
                  Delete
                </Link>
              </div>
            )}
          </>
        );

      default:
        return <div style={metaText}>{titleizeType(doc.document_type)}</div>;
    }
  };

  // -- Card shell --

  const isFailed = status === "failed";

  const cardContent = (
    <>
      {/* Row 1: Title + Status badge */}
      <div style={{
        display: "flex", justifyContent: "space-between",
        alignItems: "flex-start", marginBottom: "0.4rem",
      }}>
        <span style={{ ...cardTitleLink, marginRight: "1rem", flex: 1, minWidth: 0 }}>
          {doc.title}
        </span>
        <div style={{ flexShrink: 0 }}>
          <DocumentStatusBadge status={doc.status} />
        </div>
      </div>

      {/* Rows 2+: status-specific content */}
      {renderBody()}
    </>
  );

  return (
    <Link
      to={`/documents/${doc.id}`}
      style={{
        ...cardStyle,
        textDecoration: "none",
        color: "inherit",
        display: "block",
        borderLeft: isFailed ? "3px solid #dc2626" : undefined,
      }}
      onMouseEnter={(e) => { e.currentTarget.style.boxShadow = "0 2px 8px rgba(0,0,0,0.08)"; }}
      onMouseLeave={(e) => { e.currentTarget.style.boxShadow = "none"; }}
    >
      {cardContent}
    </Link>
  );
};

export default DocumentCard;
