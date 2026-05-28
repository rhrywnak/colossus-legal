/**
 * DocumentCard — renders a single document card in the documents list.
 *
 * Redesigned for the 5-status pipeline model (new, processing, completed,
 * failed, cancelled). Each status_group gets a distinct layout.
 */
import React, { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import DocumentStatusBadge from "../pipeline/DocumentStatusBadge";
import ReprocessDialog from "../pipeline/ReprocessDialog";
import { PipelineDocument, cancelProcessing } from "../../services/pipelineApi";

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
  backgroundColor: "var(--bg-surface)", border: "1px solid var(--border-default)", borderRadius: "8px",
  padding: "1rem 1.25rem", marginBottom: "0.75rem",
  transition: "box-shadow 0.15s ease",
};
const cardTitleLink: React.CSSProperties = {
  fontSize: "0.95rem", fontWeight: 600, color: "var(--text-primary)", textDecoration: "none",
};
const metaText: React.CSSProperties = {
  fontSize: "0.76rem", color: "var(--text-muted)",
};

/** Small action button factory. */
const smallBtn = (bg: string): React.CSSProperties => ({
  padding: "0.2rem 0.6rem", fontSize: "0.72rem", fontWeight: 600, border: "none",
  borderRadius: "4px", backgroundColor: bg, color: "var(--bg-surface)", cursor: "pointer",
  fontFamily: "inherit",
});

const badgeBase: React.CSSProperties = {
  display: "inline-block",
  padding: "0.1rem 0.45rem",
  fontSize: "0.68rem",
  fontWeight: 600,
  borderRadius: "4px",
  marginRight: "0.4rem",
};
const badgeAmber: React.CSSProperties = {
  ...badgeBase,
  backgroundColor: "var(--burden-warning-bg)",
  border: "1px solid var(--burden-warning-bg)",
  color: "var(--burden-warning-text)",
};
const badgeNeutral: React.CSSProperties = {
  ...badgeBase,
  backgroundColor: "var(--bg-page)",
  border: "1px solid var(--border-default)",
  color: "var(--text-muted)",
};
const badgePlain: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--text-muted)",
  marginRight: "0.4rem",
};

/**
 * Render the PDF-classification summary for a card row. Returns null when
 * no classification fields are present (row predates the classifier or the
 * upload-time classify() call failed).
 */
function renderContentInfo(doc: PipelineDocument): React.ReactNode {
  const ct = doc.content_type;
  if (!ct) return null;
  const pages = doc.page_count;
  switch (ct) {
    case "text_based":
      return (
        <span style={badgePlain}>
          {pages != null ? `${pages} page${pages === 1 ? "" : "s"}` : "Text-based"}
        </span>
      );
    case "scanned":
      return (
        <span style={badgeAmber}>
          Scanned{pages != null && ` · ${pages} pages`} · OCR required
        </span>
      );
    case "mixed":
      return (
        <span style={badgeAmber}>
          Mixed · {doc.text_pages ?? 0} text, {doc.scanned_pages ?? 0} scanned
        </span>
      );
    case "unknown":
      return <span style={badgeNeutral}>Unknown</span>;
    default:
      return null;
  }
}

// ── Component ───────────────────────────────────────────────────

const DocumentCard: React.FC<DocumentCardProps> = ({ doc, isAdmin, onRefresh }) => {
  const status = doc.status_group ?? "new";
  const [showReprocess, setShowReprocess] = useState(false);
  const navigate = useNavigate();

  // -- Action helpers (stop propagation so the Link wrapper isn't triggered) --

  /**
   * Navigate to the document's Process tab instead of kicking off
   * processing from the card. This lets the user review the Configuration
   * Panel (profile, model, overrides) before actually running extraction.
   */
  const handleProcess = (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    navigate(`/documents/${doc.id}?tab=processing`);
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
            <div style={{ marginBottom: "0.4rem" }}>{renderContentInfo(doc)}</div>
            {isAdmin && (
              <div style={{ display: "flex", gap: "0.5rem", alignItems: "center" }}>
                <button style={smallBtn("var(--accent-primary)")} onClick={handleProcess}>Configure</button>
                <Link
                  to={`/documents/${doc.id}`}
                  style={{ ...metaText, fontSize: "0.72rem", color: "var(--state-danger-strong)", textDecoration: "underline" }}
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
              <div style={{ height: "6px", backgroundColor: "var(--border-default)", borderRadius: "3px", flex: 1 }}>
                <div style={{
                  width: `${doc.percent_complete ?? 0}%`, height: "100%",
                  backgroundColor: "var(--accent-primary)", borderRadius: "3px",
                  transition: "width 0.3s ease",
                }} />
              </div>
              <span style={{ fontSize: "0.72rem", color: "var(--text-muted)" }}>
                {doc.percent_complete ?? 0}%
              </span>
              {isAdmin && (
                <button style={smallBtn("var(--state-warning-strong)")} onClick={handleCancel}>Cancel</button>
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
                <button style={smallBtn("var(--accent-primary)")} onClick={(e) => { e.preventDefault(); e.stopPropagation(); setShowReprocess(true); }}>Re-process</button>
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
                <button style={smallBtn("var(--accent-primary)")} onClick={(e) => { e.preventDefault(); e.stopPropagation(); setShowReprocess(true); }}>Re-process</button>
                <Link
                  to={`/documents/${doc.id}`}
                  style={{ ...metaText, fontSize: "0.72rem", color: "var(--state-danger-strong)", textDecoration: "underline" }}
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
    <>
      <Link
        to={`/documents/${doc.id}`}
        style={{
          ...cardStyle,
          textDecoration: "none",
          color: "inherit",
          display: "block",
          borderLeft: isFailed ? "3px solid var(--state-danger-strong)" : undefined,
        }}
        onMouseEnter={(e) => { e.currentTarget.style.boxShadow = "0 2px 8px rgba(0,0,0,0.08)"; }}
        onMouseLeave={(e) => { e.currentTarget.style.boxShadow = "none"; }}
      >
        {cardContent}
      </Link>
      {showReprocess && (
        <ReprocessDialog
          open={showReprocess}
          documentId={doc.id}
          onClose={() => setShowReprocess(false)}
          onSuccess={() => { setShowReprocess(false); onRefresh(); }}
        />
      )}
    </>
  );
};

export default DocumentCard;
