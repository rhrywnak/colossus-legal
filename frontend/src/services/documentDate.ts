// =============================================================================
// documentDate.ts — the document's own date (task P4b, B2 §3)
// =============================================================================
//
// Two callers, one endpoint: the upload dialog records a document's date right
// after the file lands, and the document page corrects it afterwards. Both hit
// `PUT /documents/:id/date`, so the mandatory-with-override rule is validated
// once, in the backend, and cannot drift between the two screens.
//
// The precision vocabulary is FETCHED, never hardcoded here (Standing Rule 12:
// no business logic in the frontend). Which precisions exist, and which of them
// require a date, is a backend decision — a TypeScript copy would drift from the
// Rust lookup the first time one changed.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

const PIPELINE_BASE = `${API_BASE_URL}/api/admin/pipeline`;

/** One precision the intake control can offer. Shape mirrors the backend's
 *  `DatePrecisionOption`. */
export interface DatePrecisionOption {
  value: string;
  label: string;
  /** When false, this precision is the override and must be sent with no date. */
  requires_date: boolean;
}

/** A document's stored date. `document_date` is absent when the precision is
 *  `unknown` — the key is omitted, not null. */
export interface DocumentDate {
  document_id: string;
  document_date?: string;
  date_precision: string;
  date_precision_label: string;
}

/**
 * Read the error body a failed write returns, so the user sees the backend's
 * own message.
 *
 * The refusal for a blank date names the override ("send precision 'unknown'"),
 * which is precisely the sentence a user needs — surfacing a bare status code
 * instead would leave them fighting the form.
 */
async function messageFor(res: Response, fallback: string): Promise<string> {
  try {
    const body = await res.json();
    if (typeof body?.message === "string" && body.message.length > 0) {
      return body.message;
    }
    if (typeof body?.error === "string" && body.error.length > 0) {
      return body.error;
    }
  } catch {
    // A non-JSON error body is not itself informative; fall through to the
    // status-based message rather than swallowing the failure.
  }
  return `${fallback} (${res.status})`;
}

/**
 * A document's date as stored, including the state where nobody has answered.
 *
 * Every field but the id is optional on purpose: an absent `date_precision`
 * means the question has not been asked yet, which the edit control must show as
 * unanswered rather than as "unknown".
 */
export interface StoredDocumentDate {
  document_id: string;
  document_date?: string;
  date_precision?: string;
  date_precision_label?: string;
}

/** Read what is recorded for one document. */
export async function fetchDocumentDate(
  documentId: string,
): Promise<StoredDocumentDate> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const res = await authFetch(
    `${PIPELINE_BASE}/documents/${encodeURIComponent(documentId)}/date`,
  );
  if (!res.ok) {
    throw new Error(await messageFor(res, "Failed to load the document date"));
  }
  return res.json();
}

/** The precisions the intake control and the document page offer. */
export async function fetchDatePrecisions(): Promise<DatePrecisionOption[]> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const res = await authFetch(`${PIPELINE_BASE}/documents/date-precisions`);
  if (!res.ok) {
    throw new Error(await messageFor(res, "Failed to load date precisions"));
  }
  return res.json();
}

/**
 * Record or correct a document's own date.
 *
 * `documentDate` must be omitted when `datePrecision` is the override, and
 * present otherwise. The backend enforces that and returns a 400 naming the
 * problem; this function does not second-guess it, so there is exactly one
 * place the rule lives.
 */
export async function setDocumentDate(
  documentId: string,
  documentDate: string | null,
  datePrecision: string,
): Promise<DocumentDate> {
  const res = await authFetch(
    `${PIPELINE_BASE}/documents/${encodeURIComponent(documentId)}/date`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        document_date: documentDate,
        date_precision: datePrecision,
      }),
    },
  );
  if (!res.ok) {
    throw new Error(await messageFor(res, "Failed to save the document date"));
  }
  return res.json();
}
