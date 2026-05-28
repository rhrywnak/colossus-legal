// =============================================================================
// elementDetailService.ts — clients for the Element detail panel endpoints
// -----------------------------------------------------------------------------
// Endpoints (backend instruction E1):
//   GET   /api/cases/:slug/elements/:element_id/detail → fetchElementDetail
//   PATCH /api/cases/:slug/elements/:element_id/notes  → saveElementNotes
//
// Pattern mirrors causesOfAction.ts: types mirror the Rust DTO exactly,
// nullable fields are emitted as JSON `null` (so "absent" stays
// distinguishable from "empty"), and every failure path produces a distinct,
// observable error message (Standing Rule 1).
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

// ─── Types — mirror Rust DTOs in element_detail_repository.rs ───────────────

/**
 * One mapped Allegation as it appears in the detail panel's list. Backend
 * source: `AllegationSummary` in element_detail_repository.rs.
 *
 * `source_section` is one of "Common" | "Dedicated" | "Unknown". The panel
 * uses it to group the list under labeled section dividers.
 */
export type AllegationSummary = {
  allegation_id: string;
  paragraph_number: string;
  summary: string | null;
  title: string | null;
  verbatim_quote: string | null;
  source_section: string;
};

/**
 * Top-level payload for the Element detail panel. Backend source:
 * `ElementDetailResponse` in element_detail_repository.rs.
 *
 * `count_number`, `count_name`, and `order_in_count` are nullable: an
 * "orphan" Element (no parent LegalCount via HAS_ELEMENT) decodes them to
 * null rather than failing the request (Rule 1: distinguishable states).
 */
export type ElementDetailResponse = {
  element_id: string;
  element_name: string;
  what_plaintiff_must_prove: string;
  order_in_count: number | null;
  count_number: number | null;
  count_name: string | null;
  review_notes: string | null;
  allegations: AllegationSummary[];
  allegation_count: number;
  common_count: number;
  dedicated_count: number;
};

// ─── GET /detail ────────────────────────────────────────────────────────────

/**
 * Fetch the Element detail payload (Element + parent Count + mapped
 * Allegations + saved review_notes) for the panel.
 *
 * Validates the load-bearing fields and throws a contextual error at the
 * boundary rather than letting a malformed body crash the React tree later
 * — Standing Rule 1 (no silent failures). Each failure mode (404, bad JSON,
 * shape mismatch) produces a distinct, observable message so the panel can
 * surface a precise hint.
 *
 * @param slug      case slug (the caller passes `DEFAULT_CASE_SLUG` from
 *                  `caseHeader.ts` for the single-case deployment)
 * @param elementId stable Element id like `element-1-1`
 * @returns the typed Element detail payload
 * @throws Error on non-2xx, unparseable body, or missing `allegations` array
 */
export async function fetchElementDetail(
  slug: string,
  elementId: string,
): Promise<ElementDetailResponse> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/elements/${encodeURIComponent(elementId)}/detail`,
  );

  if (!response.ok) {
    // 404 here means the Element id is unknown to Neo4j — likely a stale link
    // or a case that hasn't been loaded yet.
    const reason =
      response.status === 404
        ? " — no Element with that id"
        : "";
    throw new Error(
      `Failed to load Element detail for "${elementId}" (HTTP ${response.status}${reason}). Try reloading the panel.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Element detail response for "${elementId}" was not valid JSON (the backend may be down). Try reloading the panel.`,
    );
  }

  const parsed = data as Partial<ElementDetailResponse>;
  if (!Array.isArray(parsed.allegations)) {
    throw new Error(
      `Element detail response for "${elementId}" is missing the "allegations" array — ` +
        `backend/frontend contract mismatch. If reloading does not help, report this to the site administrator.`,
    );
  }

  return parsed as ElementDetailResponse;
}

// ─── PATCH /notes ───────────────────────────────────────────────────────────

/**
 * Persist `review_notes` for an Element. Passing `null` clears the notes
 * column to SQL NULL; passing `""` writes an empty string — these are
 * intentionally distinguishable states on the wire (Rule 1).
 *
 * Throws on non-2xx so the caller can surface an explicit "Save failed"
 * indicator next to the textarea — this is NOT fire-and-forget like
 * `rateQAEntry`, because review notes are operator-authored work product
 * and a silent failure would mean lost edits.
 *
 * @param slug         case slug
 * @param elementId    Element id (e.g. `element-1-1`)
 * @param reviewNotes  the new notes content, or `null` to clear
 * @throws Error with HTTP status on non-2xx; the operator log carries the
 *               backend's bland 500 body, the toast/status surfaces the
 *               status code to the user
 */
export async function saveElementNotes(
  slug: string,
  elementId: string,
  reviewNotes: string | null,
): Promise<void> {
  // The body shape mirrors the backend's `UpdateNotesRequest` struct, which
  // has `#[serde(deny_unknown_fields)]` — any extra key here would 4xx.
  // We send the key explicitly (not omitted) so the backend's distinction
  // between "missing key → None" and "explicit null → None" is moot but our
  // intent is auditable on the wire.
  const body = JSON.stringify({ review_notes: reviewNotes });

  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/elements/${encodeURIComponent(elementId)}/notes`,
    {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body,
    },
  );

  if (!response.ok) {
    const reason =
      response.status === 404
        ? " — Element row not in authored_entities"
        : "";
    throw new Error(
      `Failed to save notes for "${elementId}" (HTTP ${response.status}${reason}).`,
    );
  }
}

