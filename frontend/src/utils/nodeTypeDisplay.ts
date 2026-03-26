/**
 * Display helpers for multi-type content nodes and document source types.
 *
 * Used by EvidenceCard (node type badge) and DocumentWorkspace (source type badge).
 * Extracted here to keep both components under the 300-line limit.
 */

// ── Node type display ────────────────────────────────────────────

export interface NodeTypeDisplay {
  label: string;
  color: string;
}

export function getNodeTypeDisplay(nodeType: string): NodeTypeDisplay {
  switch (nodeType) {
    case "ComplaintAllegation":
      return { label: "Allegation", color: "#9C27B0" };
    case "MotionClaim":
      return { label: "Motion Claim", color: "#4CAF50" };
    case "LegalCount":
      return { label: "Legal Count", color: "#FF9800" };
    case "Harm":
      return { label: "Harm", color: "#F44336" };
    case "Evidence":
    default:
      return { label: "Evidence", color: "#2196F3" };
  }
}

// ── Source type display ──────────────────────────────────────────

export interface SourceTypeDisplay {
  label: string;
  color: string;
  tooltip: string;
}

export function getSourceTypeDisplay(
  sourceType: string | null | undefined
): SourceTypeDisplay | null {
  switch (sourceType) {
    case "native_pdf":
      return { label: "Native PDF", color: "#4CAF50", tooltip: "Text layer available — highlighting supported" };
    case "docx_converted":
      return { label: "DOCX Converted", color: "#4CAF50", tooltip: "Text layer available — highlighting supported" };
    case "ocr_processed":
      return { label: "OCR Processed", color: "#FF9800", tooltip: "OCR text layer — highlighting may have gaps" };
    case "scanned":
      return { label: "Scanned", color: "#9E9E9E", tooltip: "No text layer — highlighting unavailable" };
    default:
      return null;
  }
}

// ── Page label helper ────────────────────────────────────────────

/**
 * Returns a display string for the page/paragraph reference.
 * ComplaintAllegation uses paragraph numbers (shown as "¶12"),
 * all other types use page numbers (shown as "p. 5").
 */
export function getPageLabel(
  nodeType: string | undefined,
  pageNumber: string | null | undefined
): string | null {
  if (pageNumber == null) return null;
  if (nodeType === "ComplaintAllegation") return `¶${pageNumber}`;
  return `p. ${pageNumber}`;
}
