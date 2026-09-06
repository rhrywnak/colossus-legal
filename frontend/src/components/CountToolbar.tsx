// =============================================================================
// CountToolbar.tsx — the line under the Count buttons: the legend, and the export
// -----------------------------------------------------------------------------
// PROOF_MATRIX_v2 §3 and §4. Split out of ProofMatrixPage, which was already the
// page's largest file before this line existed.
//
// ## Why the legend is on the page and not in a help panel
//
// Chuck reads the Matrix as a reference, in minutes, and nothing on it may
// require him to click anything. A colour vocabulary he has to infer is one he
// will get wrong once — and the one he would get wrong is red, which means two
// different things on this page depending on where it appears: the complaint's
// own words down the left of a paragraph, and a rebuttal beside a quote.
//
// ## Why the export is an anchor and not a button with a fetch
//
// The response is a file with a `Content-Disposition`. Letting the browser follow
// a link is what puts it in the downloads folder under the name the backend gave
// it; a fetch would mean holding the bytes in memory and synthesising an anchor
// to do the same thing worse.
// =============================================================================

import React from "react";
import type { MatrixWording } from "../services/causesOfAction";
import { countExportUrl } from "../services/matrixRulings";

export interface CountToolbarProps {
  caseSlug: string;
  /** The Count currently selected — the one the export link points at. */
  countNumber: number;
  /** The Matrix's served words. Both strings here are stored rows. */
  matrixWording: MatrixWording;
}

const CountToolbar: React.FC<CountToolbarProps> = ({
  caseSlug,
  countNumber,
  matrixWording,
}) => (
  <div style={TOOLBAR_STYLE}>
    <span style={LEGEND_STYLE}>{matrixWording.legend_line}</span>
    <a
      href={countExportUrl(caseSlug, countNumber)}
      style={EXPORT_LINK_STYLE}
      // The backend names the file (`proof-matrix-count-N.docx`); a bare
      // `download` attribute keeps that name rather than overriding it.
      download
    >
      {matrixWording.export_button_label}
    </a>
  </div>
);

export default CountToolbar;

// ─── Styles (tokens only) ────────────────────────────────────────────────────

const TOOLBAR_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  justifyContent: "space-between",
  flexWrap: "wrap",
  gap: "12px",
  marginTop: "12px",
};

const LEGEND_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  color: "var(--text-secondary)",
};

// An outlined link, not a filled button: the export is one of two things on this
// line and neither is the page's primary action — that is the Count buttons above.
const EXPORT_LINK_STYLE: React.CSSProperties = {
  padding: "4px 12px",
  border: "1px solid var(--accent-primary)",
  borderRadius: "6px",
  color: "var(--accent-primary)",
  textDecoration: "none",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 600,
  whiteSpace: "nowrap",
};
