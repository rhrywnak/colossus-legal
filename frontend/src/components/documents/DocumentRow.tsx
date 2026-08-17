/**
 * DocumentRow — one document as a table row.
 *
 * Replaces DocumentCard (deleted). Roman: the Documents page is one compact
 * table, and it must show the phase — which a stack of cards could not do
 * legibly, because a column you can scan down is the whole point of asking
 * "which of these are COA documents?".
 *
 * ## What survived the card → row change
 *
 * Every per-status affordance, moved into the Detail and Actions cells:
 * Configure and Review for a new document, the progress bar and Cancel while
 * processing, the entity/relationship counts when complete, the error message
 * and suggestion when failed, and Re-process on both failed and cancelled rows.
 *
 * The first draft of this file kept every ACTION and quietly lost one piece of
 * INFORMATION — the PDF classification badge ("Scanned · 5 pages · OCR
 * required") the card showed on new documents. The architecture gate caught it;
 * `contentInfo` below restores it. Worth remembering that "no action was
 * dropped" and "nothing was dropped" are different claims.
 * The card's whole-card link becomes a link on the title, so the action buttons
 * are no longer nested inside a link (which they were, and which is why they all
 * needed `preventDefault`).
 */
import React, { useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import DocumentStatusBadge from "../pipeline/DocumentStatusBadge";
import ReprocessDialog from "../pipeline/ReprocessDialog";
import { PipelineDocument, cancelProcessing } from "../../services/pipelineApi";
import { phaseLabel, PhaseOption } from "../../services/casePhases";

interface Props {
  doc: PipelineDocument;
  isAdmin: boolean;
  /** Loaded once by the page, passed down so every row does not re-resolve it. */
  phases: PhaseOption[];
  onRefresh: () => void;
}

function titleizeType(slug: string): string {
  return slug.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}
function truncate(text: string, max = 70): string {
  return text.length > max ? text.slice(0, max - 1) + "…" : text;
}

/**
 * The PDF's classification, for a document that has not been processed yet.
 *
 * Carried over from DocumentCard rather than dropped: it is how Roman scans the
 * list and sees at a glance which new documents will need OCR. The architecture
 * gate caught its absence from the first draft of this row — the actions had all
 * survived the card→row change, and this information had not.
 */
function contentInfo(doc: PipelineDocument): React.ReactNode {
  const pages = doc.page_count;
  switch (doc.content_type) {
    case "text_based":
      return pages != null ? `${pages} page${pages === 1 ? "" : "s"}` : "Text-based";
    case "scanned":
      return `Scanned${pages != null ? ` · ${pages} pages` : ""} · OCR required`;
    case "mixed":
      return `Mixed · ${doc.text_pages ?? 0} text, ${doc.scanned_pages ?? 0} scanned`;
    case "unknown":
      return "Unknown format";
    default:
      return "Not processed";
  }
}

const td: React.CSSProperties = {
  padding: "0.5rem 0.6rem",
  borderBottom: "1px solid var(--border-default)",
  fontSize: "0.8rem",
  color: "var(--text-secondary)",
  verticalAlign: "middle",
};
const titleLink: React.CSSProperties = {
  color: "var(--text-primary)",
  fontWeight: 600,
  textDecoration: "none",
};
const smallBtn = (color: string): React.CSSProperties => ({
  padding: "0.2rem 0.5rem",
  fontSize: "0.72rem",
  fontWeight: 600,
  color: "var(--bg-surface)",
  backgroundColor: color,
  border: "none",
  borderRadius: "4px",
  cursor: "pointer",
  fontFamily: "inherit",
});
const mutedLink: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--accent-primary)",
  textDecoration: "none",
  fontWeight: 600,
};

const DocumentRow: React.FC<Props> = ({ doc, isAdmin, phases, onRefresh }) => {
  const navigate = useNavigate();
  const [showReprocess, setShowReprocess] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const status = doc.status_group ?? "new";

  // A failed cancel must not vanish: `void handleCancel()` would discard the
  // rejection, the row would still say "processing", and the user would be left
  // believing they had stopped it (Standing Rule 1).
  const handleCancel = async () => {
    setError(null);
    try {
      await cancelProcessing(doc.id);
      onRefresh();
    } catch (e) {
      setError(
        `Could not cancel “${doc.title}”: ${
          e instanceof Error ? e.message : "unknown error"
        } — it is still processing. Retry, or Re-process once it finishes.`,
      );
    }
  };

  // Status-specific detail, kept to one line so the table stays scannable.
  const detail = () => {
    switch (status) {
      case "processing":
        return (
          <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
            <div style={{ height: "5px", width: "70px", backgroundColor: "var(--border-default)", borderRadius: "3px" }}>
              <div style={{
                width: `${doc.percent_complete ?? 0}%`, height: "100%",
                backgroundColor: "var(--accent-primary)", borderRadius: "3px",
              }} />
            </div>
            <span style={{ fontSize: "0.72rem" }}>
              {doc.processing_step_label ?? "Processing…"} {doc.percent_complete ?? 0}%
            </span>
          </div>
        );
      case "completed":
        return (
          <span style={{ fontSize: "0.72rem" }}>
            {doc.entities_written ?? 0} entities · {doc.relationships_written ?? 0} relationships
            {(doc.entities_flagged ?? 0) > 0 && ` · ${doc.entities_flagged} flagged`}
          </span>
        );
      case "failed":
        return (
          <span style={{ fontSize: "0.72rem", color: "var(--state-danger-strong)" }}>
            {truncate(doc.error_message ?? "Unknown error")}
            {doc.error_suggestion ? ` — ${truncate(doc.error_suggestion, 40)}` : ""}
          </span>
        );
      case "cancelled":
        return <span style={{ fontSize: "0.72rem" }}>Cancelled</span>;
      default:
        // A new document: show what the PDF classification found, because
        // "needs OCR" is the thing worth knowing before processing it.
        return (
          <span
            style={{
              fontSize: "0.72rem",
              color:
                doc.content_type === "scanned" || doc.content_type === "mixed"
                  ? "var(--state-warning-strong)"
                  : "var(--text-muted)",
            }}
          >
            {contentInfo(doc)}
          </span>
        );
    }
  };

  const actions = () => {
    switch (status) {
      case "new":
        return (
          <>
            <button
              style={smallBtn("var(--accent-primary)")}
              onClick={() => navigate(`/documents/${doc.id}?tab=processing`)}
            >
              Configure
            </button>
            <Link to={`/documents/${doc.id}`} style={mutedLink}>Open</Link>
          </>
        );
      case "processing":
        return isAdmin ? (
          <button style={smallBtn("var(--state-warning-strong)")} onClick={() => { void handleCancel(); }}>
            Cancel
          </button>
        ) : null;
      case "failed":
      case "cancelled":
        return (
          <>
            {isAdmin && (
              <button style={smallBtn("var(--accent-primary)")} onClick={() => setShowReprocess(true)}>
                Re-process
              </button>
            )}
            <Link to={`/documents/${doc.id}`} style={mutedLink}>Open</Link>
          </>
        );
      default:
        return <Link to={`/documents/${doc.id}`} style={mutedLink}>Review</Link>;
    }
  };

  // An em dash rather than a blank: "no phase recorded" and "this cell failed to
  // render" must not look the same.
  const label = phaseLabel(phases, doc.phase);

  return (
    <>
      <tr style={status === "failed" ? { borderLeft: "3px solid var(--state-danger-strong)" } : undefined}>
        <td style={{ ...td, maxWidth: "26rem" }}>
          <Link to={`/documents/${doc.id}`} style={titleLink}>{doc.title}</Link>
        </td>
        <td style={td}>{titleizeType(doc.document_type)}</td>
        <td style={td}>{label === "" ? "—" : label}</td>
        <td style={td}><DocumentStatusBadge status={doc.status} /></td>
        <td style={td}>
          {detail()}
          {error !== null && (
            <div style={{ fontSize: "0.7rem", color: "var(--status-dropped-text)", marginTop: "0.2rem" }}>
              {error}
            </div>
          )}
        </td>
        <td style={{ ...td, whiteSpace: "nowrap" }}>
          <div style={{ display: "flex", gap: "0.4rem", alignItems: "center" }}>{actions()}</div>
        </td>
      </tr>
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

export default DocumentRow;
