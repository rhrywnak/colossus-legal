// =============================================================================
// documentPhase.ts — which phase of the case a document belongs to
// -----------------------------------------------------------------------------
// The sibling of `documentDate`: two callers (the upload dialog at intake, the
// document page afterwards), one endpoint, so the vocabulary is validated once
// in the backend and cannot drift between the two screens.
//
// The SLUG travels; the label never does. Labels come from `/data/timeline.json`
// via `casePhases` — see that module for the ruling.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

const PIPELINE_BASE = `${API_BASE_URL}/api/admin/pipeline`;

/** A document's stored phase. `phase` is absent when none is recorded. */
export interface DocumentPhase {
  document_id: string;
  phase?: string;
}

/** Surface the backend's own refusal, which names the four valid slugs. */
async function messageFor(res: Response, fallback: string): Promise<string> {
  try {
    const body = await res.json();
    if (typeof body?.message === "string" && body.message.length > 0) return body.message;
    if (typeof body?.error === "string" && body.error.length > 0) return body.error;
  } catch {
    // A non-JSON error body is not itself informative; fall through rather than
    // swallowing the failure.
  }
  return `${fallback} (${res.status})`;
}

/**
 * Record, change or clear a document's phase.
 *
 * Pass `""` to clear it — the backend treats empty as "no phase", which is a
 * legitimate answer because the field is never required. There is no separate
 * clear call for the same reason there is no separate set call.
 */
export async function setDocumentPhase(
  documentId: string,
  phase: string,
): Promise<DocumentPhase> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const res = await authFetch(
    `${PIPELINE_BASE}/documents/${encodeURIComponent(documentId)}/phase`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ phase: phase === "" ? null : phase }),
    },
  );
  if (!res.ok) {
    throw new Error(await messageFor(res, "Failed to save the phase"));
  }
  return res.json();
}
