// =============================================================================
// evidenceLocator.ts — where a piece of evidence came from, as a link and a label
// =============================================================================
//
// Two pure helpers, moved here from `ElementAllegationList` by PROOF_MATRIX_v2
// when the evidence row became its own component. They have three callers now —
// the matrix row, the trial-prep views, and their own tests — and a shared module
// is what keeps the row from having to import the list it lives inside.
//
// Nothing about the behaviour changed in the move; the tests that covered them
// still do, from `__tests__/evidenceLocator.test.ts`.

import { API_BASE_URL } from "../services/api";
import type { AllegationEvidence } from "../services/elementDetailService";

/**
 * Build the existing document-file URL with an optional `#page=N` fragment —
 * the same pattern AnswerDisplay/GraphPage use. We do not invent a viewer.
 */
export function pdfHref(documentId: string, page: number | null): string {
  const fragment = page !== null ? `#page=${page}` : "";
  return `${API_BASE_URL}/api/documents/${encodeURIComponent(documentId)}/file${fragment}`;
}

/**
 * Human locator text: "{title} · p. {n}" (title alone when no page).
 *
 * ## The "Source document" fallback, and why the Word export does NOT share it
 *
 * An Evidence node with no `CONTAINED_IN` edge has no title to print. On SCREEN
 * this falls back to a generic word so the page number survives — a muted,
 * unlinked "Source document · p. 5" is visibly a gap, and it still tells a reader
 * where to look. In the Word EXPORT the same item prints no citation at all
 * (`services::matrix_export::source_line`), because a placeholder in a filed
 * document reads as a citation to a source with that name.
 *
 * The literal below predates PROOF_MATRIX_v2 and is carried unchanged by the
 * move. It is the one user-facing word on this surface that is not a stored row,
 * and it is named in the task report as a wording row still owed.
 */
export function locatorLabel(ev: AllegationEvidence): string {
  const title = ev.source_document_title ?? "Source document";
  return ev.page_number !== null ? `${title} · p. ${ev.page_number}` : title;
}
