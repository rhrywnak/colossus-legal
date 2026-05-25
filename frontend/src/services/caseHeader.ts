// =============================================================================
// caseHeader.ts — client for GET /api/cases/:slug (the redesigned Home header)
// -----------------------------------------------------------------------------
// Endpoint: GET /api/cases/:slug  → backend handler `api::case_header`.
// Read-only. Returns the case caption, court strip, parties, and counsel.
//
// IMPORTANT — the wire shape is NESTED, not flat. These interfaces mirror the
// Rust DTO `dto::case_header::CaseHeaderResponse` exactly: court fields live
// under `court`, party arrays under `parties`. (An earlier draft of the design
// doc showed a flattened shape — the backend is the source of truth.)
//
// Null vs empty: the backend deliberately does NOT omit absent fields — every
// optional is emitted as JSON `null` so the UI can distinguish "absent" from
// "explicitly empty". Notably the builder collapses both a NULL and an empty
// `case_number` to `null`, so the [pending] indicator keys off null-or-empty.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

// CONST: single-case deployment. Not env-configurable in a useful sense — the
// app serves exactly one case, so the slug is a named constant rather than a
// route param or runtime config. The value matches the `case_slug` seeded in
// Postgres for Awad v. CFS. Going multi-case later means parameterizing the
// callers of `getCaseHeader`, not hunting for a literal sprinkled across files.
export const DEFAULT_CASE_SLUG = "awad_v_catholic_family_service";

/** A plaintiff or active defendant. (`entity_type`/`notes` may be null.) */
export type HeaderParty = {
  party_id: string;
  name: string;
  entity_type: string | null;
  notes: string | null;
};

/**
 * A dropped / dismissed / settled defendant. Carries the extra dismissal detail
 * the header surfaces under the "DROPPED" subheader. `status` is the specific
 * non-active lifecycle state ("dropped" | "dismissed" | "settled").
 */
export type DroppedDefendant = {
  party_id: string;
  name: string;
  entity_type: string | null;
  status: string;
  dismissal_date: string | null;
  dismissal_basis: string | null;
  notes: string | null;
};

/** Parties grouped for the two-column header layout (already pre-sorted by the backend). */
export type PartiesGroups = {
  plaintiffs: HeaderParty[];
  active_defendants: HeaderParty[];
  dropped_defendants: DroppedDefendant[];
};

/**
 * The court / case-metadata strip beneath the title. All fields are nullable:
 * `case_number` is null when not yet assigned (→ "[pending]"); `transferred_from`
 * is null when the case was filed in the current court.
 */
export type CourtInfo = {
  name: string | null;
  jurisdiction: string | null;
  case_number: string | null;
  filed_date: string | null; // ISO "YYYY-MM-DD"
  transferred_from: string | null;
  transfer_date: string | null; // ISO "YYYY-MM-DD"
};

/** One counsel-of-record row, rendered as a single self-labeled line. */
export type CounselContact = {
  counsel_id: string;
  represents_role: string;
  firm_name: string | null;
  attorney_name: string;
  bar_number: string | null;
  address: string | null;
  phone: string | null;
  email: string | null;
};

/** The full case-header payload (mirrors Rust `CaseHeaderResponse`). */
export type CaseHeaderResponse = {
  case_id: string;
  case_slug: string;
  display_title: string;
  display_title_full: string | null;
  court: CourtInfo;
  status: string;
  complaint_document_id: string | null;
  parties: PartiesGroups;
  counsel: CounselContact[];
};

/**
 * Fetch the case header for `slug` (defaults to the single seeded case).
 *
 * ## React/TS Learning: why validate the shape instead of trusting `as`
 * `response.json()` returns `any` — TypeScript can't guarantee the server sent
 * what we expect. A bare `return data as CaseHeaderResponse` would let a
 * malformed body (e.g. an HTML error page or a partial object) flow into the
 * component and blow up later with a confusing `undefined` access. Instead we
 * assert the load-bearing fields are present and throw a clear error here, at
 * the boundary — Standing Rule 1 (no silent failures): the caller's `.catch`
 * gets a message it can show, not a mystery crash three components deep.
 *
 * @param slug case slug; defaults to {@link DEFAULT_CASE_SLUG}
 * @returns the typed case-header payload
 * @throws Error on non-2xx, unparseable body, or a body missing required fields
 */
export async function getCaseHeader(
  slug: string = DEFAULT_CASE_SLUG,
): Promise<CaseHeaderResponse> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}`,
  );

  if (!response.ok) {
    // Name the slug and translate the common statuses so the message says what
    // failed, on which case, and what to do — not just a bare number.
    const reason =
      response.status === 404
        ? " — case not found"
        : response.status === 403
          ? " — access denied"
          : "";
    throw new Error(
      `Failed to load case "${slug}" (HTTP ${response.status}${reason}). Try reloading the page.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Case header response for "${slug}" was not valid JSON (the backend may be down). Try reloading the page.`,
    );
  }

  const parsed = data as Partial<CaseHeaderResponse>;
  if (
    typeof parsed.display_title !== "string" ||
    typeof parsed.court !== "object" ||
    parsed.court === null ||
    typeof parsed.parties !== "object" ||
    parsed.parties === null
  ) {
    throw new Error(
      `Case header response for "${slug}" is missing required fields ` +
        `(expected display_title, court, parties) — backend/frontend contract mismatch.`,
    );
  }

  return parsed as CaseHeaderResponse;
}
