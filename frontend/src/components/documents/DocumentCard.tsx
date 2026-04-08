/**
 * DocumentCard — renders a single document card in the documents list.
 *
 * Extracted from DocumentsPage to keep that file under 300 lines.
 */
import React from "react";
import { Link } from "react-router-dom";
import DocumentStatusBadge from "../pipeline/DocumentStatusBadge";
import PipelineProgressBar from "../pipeline/PipelineProgressBar";
import { PipelineDocument, KnownUser } from "../../services/pipelineApi";

interface DocumentCardProps {
  doc: PipelineDocument;
  isAdmin: boolean;
  users: KnownUser[];
  onAssign: (docId: string, reviewer: string | null) => void;
}

// ── Helpers ─────────────────────────────────────────────────────

function titleizeType(slug: string): string {
  return slug.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString();
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
const reviewerText: React.CSSProperties = {
  fontSize: "0.76rem", color: "#64748b", fontStyle: "italic",
};
const assignSelect: React.CSSProperties = {
  padding: "0.25rem 0.4rem", fontSize: "0.72rem", borderRadius: "4px",
  border: "1px solid #e2e8f0", fontFamily: "inherit", color: "#334155",
  backgroundColor: "#f8fafc", cursor: "pointer",
};
const errorIndicator: React.CSSProperties = {
  fontSize: "0.72rem", color: "#dc2626", fontWeight: 500,
};

// ── Component ───────────────────────────────────────────────────

const DocumentCard: React.FC<DocumentCardProps> = ({ doc, isAdmin, users, onAssign }) => {
  const canInteract = doc.can_view ?? true;

  return (
    <div
      style={{
        ...cardStyle,
        opacity: canInteract ? 1 : 0.5,
        pointerEvents: canInteract ? "auto" : "none",
        borderLeft: doc.has_failed_steps ? "3px solid #dc2626" : undefined,
      }}
      onMouseEnter={(e) => {
        if (canInteract) e.currentTarget.style.boxShadow = "0 2px 8px rgba(0,0,0,0.08)";
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.boxShadow = "none";
      }}
    >
      {/* Row 1: Title + Status */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "0.5rem" }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          {canInteract ? (
            <Link to={`/documents/${doc.id}`} style={cardTitleLink}>
              {doc.title}
            </Link>
          ) : (
            <span style={{ ...cardTitleLink, color: "#94a3b8" }}>{doc.title}</span>
          )}
        </div>
        <div style={{ marginLeft: "1rem", flexShrink: 0, display: "flex", alignItems: "center", gap: "0.5rem" }}>
          {doc.has_failed_steps && <span style={errorIndicator}>Needs attention</span>}
          <DocumentStatusBadge status={doc.status} />
        </div>
      </div>

      {/* Row 2: Metadata */}
      <div style={{ display: "flex", gap: "1rem", alignItems: "center", flexWrap: "wrap", marginBottom: "0.5rem" }}>
        <span style={metaText}>{titleizeType(doc.document_type)}</span>
        <span style={metaText}>Updated {formatDate(doc.updated_at)}</span>
        <span style={metaText}>Created {formatDate(doc.created_at)}</span>
        {doc.total_cost_usd != null && (
          <span style={metaText}>${doc.total_cost_usd.toFixed(2)}</span>
        )}
      </div>

      {/* Row 3: Progress bar (non-published) */}
      {doc.status_group !== "published" && (
        <div style={{ maxWidth: "240px", marginBottom: "0.5rem" }}>
          <PipelineProgressBar status={doc.status} />
        </div>
      )}

      {/* Row 4: Reviewer assignment */}
      <div style={{ display: "flex", alignItems: "center", gap: "0.75rem", marginTop: "0.25rem" }}>
        <span style={reviewerText}>
          Reviewer: {doc.assigned_reviewer || "Not assigned"}
        </span>
        {isAdmin && (
          <select
            style={assignSelect}
            value={doc.assigned_reviewer || ""}
            onChange={(e) => {
              const val = e.target.value || null;
              onAssign(doc.id, val);
            }}
          >
            <option value="">Unassigned</option>
            {users.map((u) => (
              <option key={u.username} value={u.username}>
                {u.display_name || u.username}
              </option>
            ))}
          </select>
        )}
      </div>
    </div>
  );
};

export default DocumentCard;
