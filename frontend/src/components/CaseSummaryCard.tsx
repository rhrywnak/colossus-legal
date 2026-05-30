// =============================================================================
// CaseSummaryCard.tsx — Home page "Case Summary" card
// -----------------------------------------------------------------------------
// A plain-language case paragraph and a null-safe "View Complaint →" link. The
// prose comes from the static /data/case-summary.json file (services/
// caseSummaryDoc); the complaint link target is resolved DYNAMICALLY from the
// case-header response's `complaint_document_id` (passed in as a prop) so no
// document id is hardcoded (Standing Rule 2).
//
// (A venue/filed/status line used to render here but was removed — it exactly
// duplicated the CaseHeader metadata strip shown directly above this card.)
// =============================================================================

import React, { useEffect, useState } from "react";
import { API_BASE_URL } from "../services/api";
import { CaseSummaryDoc, getCaseSummaryDoc } from "../services/caseSummaryDoc";

// ─── Pure helpers (exported for unit testing — no DOM, no React) ─────────────

/**
 * Build the complaint PDF URL the document file route accepts, or `null` when
 * there is no complaint id to link to.
 *
 * Null-safety is the point: on DEV the case-header field is null until the
 * operator registers the complaint, and we must render NO link rather than a
 * broken one. The route is the Neo4j-backed `GET /api/documents/:id/file`
 * (the same path the Timeline page's document links use).
 *
 * @param complaintDocumentId the id from the case-header response, or null
 * @returns an absolute file URL, or null when the id is absent/blank
 */
export function complaintFileHref(
  complaintDocumentId: string | null,
): string | null {
  if (!complaintDocumentId || complaintDocumentId.trim() === "") return null;
  return `${API_BASE_URL}/api/documents/${encodeURIComponent(complaintDocumentId)}/file`;
}

// ─── Styles (inline + tokens; no new hex) ────────────────────────────────────

const CARD_STYLE: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  padding: "24px",
};

// Sans body paragraph (decision C: serif stays reserved for the H1). `.proof-text`
// is the design system's body token (14px / 400 / --text-primary / --font-sans);
// we only add paragraph line-height inline for comfortable reading.
const SUMMARY_PARAGRAPH_STYLE: React.CSSProperties = {
  marginTop: "8px",
  lineHeight: 1.6,
};

const VIEW_COMPLAINT_LINK_STYLE: React.CSSProperties = {
  display: "inline-block",
  marginTop: "12px",
  fontSize: "14px",
  fontWeight: 500,
  color: "var(--accent-primary)",
  textDecoration: "none",
};

// ─── Component ────────────────────────────────────────────────────────────────

/**
 * Props: only the complaint document id from the already-fetched case header.
 * The card owns its own fetch of the static summary doc.
 */
export interface CaseSummaryCardProps {
  /** From `getCaseHeader().complaint_document_id`; null when not yet registered. */
  complaintDocumentId: string | null;
}

/**
 * Case Summary card.
 *
 * ## React Learning: fetch-on-mount with a cancel flag (matches Home/panel)
 * The effect guards every setState with `cancelled` and the cleanup sets it
 * true, so navigating away mid-request never setStates an unmounted component.
 */
const CaseSummaryCard: React.FC<CaseSummaryCardProps> = ({
  complaintDocumentId,
}) => {
  const [doc, setDoc] = useState<CaseSummaryDoc | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    getCaseSummaryDoc()
      .then((data) => {
        if (!cancelled) setDoc(data);
      })
      .catch((err: unknown) => {
        // No silent failure (Rule 1): surface the message in the card.
        if (!cancelled) {
          setError(
            err instanceof Error ? err.message : "Failed to load the case summary.",
          );
        }
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (error) {
    return (
      <div style={CARD_STYLE}>
        <div className="h2-section-header">Case Summary</div>
        <div style={{ marginTop: "8px", color: "var(--status-dropped-text)", fontSize: "14px" }}>
          {error}
        </div>
      </div>
    );
  }

  if (!doc) {
    return (
      <div style={CARD_STYLE}>
        <div className="h2-section-header">Case Summary</div>
        <div style={{ marginTop: "8px", color: "var(--text-muted)", fontSize: "14px" }}>
          Loading case summary...
        </div>
      </div>
    );
  }

  const complaintHref = complaintFileHref(complaintDocumentId);

  return (
    <div style={CARD_STYLE}>
      <div className="h2-section-header">Case Summary</div>
      <p className="proof-text" style={SUMMARY_PARAGRAPH_STYLE}>
        {doc.summary}
      </p>
      {/* Null-safe: render the link only when a complaint id is present. */}
      {complaintHref && (
        <a
          href={complaintHref}
          target="_blank"
          rel="noopener noreferrer"
          style={VIEW_COMPLAINT_LINK_STYLE}
        >
          View Complaint →
        </a>
      )}
    </div>
  );
};

export default CaseSummaryCard;
