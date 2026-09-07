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
 * One piece of Evidence corroborating an Allegation. Backend source:
 * `EvidenceRef` in element_detail_repository.rs (Part 2). Named
 * `AllegationEvidence` here — NOT `EvidenceRef` — to stay distinct from the
 * differently-shaped matrix-column `EvidenceRef` in `proofMatrix.ts`; two
 * different shapes must not share a name.
 *
 * `source_document_id` is nullable: when an Evidence node has no `CONTAINED_IN`
 * Document (a data gap the backend warn-logs), the locator renders as text with
 * no PDF link rather than a dead link (Rule 1: distinguishable states).
 */
export type AllegationEvidence = {
  id: string;
  verbatim_quote: string | null;
  /** PDF page number — the click-through locator. */
  page_number: number | null;
  /** Interrogatory / request id, e.g. "Q74". */
  paragraph: string | null;
  /** Page range when the Q&A spans pages, e.g. "pages 10-11". */
  page_note: string | null;
  source_document_id: string | null;
  source_document_title: string | null;
  /** `Evidence.statement_type` — half the tier key the backend ranks on. */
  statement_type: string | null;
  /** `Evidence.evidence_strength` — the other half. */
  evidence_strength: string | null;
  /** Who said it, via STATED_BY. `null` for evidence with no speaker edge. */
  speaker: string | null;
  /** The interrogatory this answers, or `null` for documentary evidence. */
  question: string | null;
  /** The answer half of a Q&A card. Feeds the backend-composed `rfa_line`. */
  answer: string | null;

  // ── What the linking pass wrote on the edge (PROOF_MATRIX_v2 §1) ───────────
  //
  // Raw tokens, never labels: every word a reader sees comes from the served
  // `MatrixWording`. `null` on all of them is a real state — 286 edges predate
  // the pass and carry nothing.

  /** The pass's position within a stance, 1..N. */
  rank: number | null;
  /** `direct_proof` / `their_own_words` / `context` / `duplicate_of` /
   *  `does_not_belong`, or `null` when absent or unreadable by the backend. */
  role: string | null;
  /** `high` / `medium` / `low`, or `null` for an older edge. */
  confidence: string | null;
  /** The pass's one line about why this item ranks here. */
  rank_reason: string | null;
  /** The older pass's reason, shown when there is no `rank_reason`. */
  why: string | null;
  /** The item both supports and disputes this Allegation — 28 edges do. */
  conflict: boolean;
  /** The item this one restates, when the pass called it a duplicate. */
  duplicate_of_card_id: string | null;
  /** The SOURCE DOCUMENT's date, `YYYY-MM-DD`, or `null`/`""` when it has none. */
  document_date: string | null;

  // ── What a human said, and what the backend worked out (§2, §3) ────────────

  /** `keep` / `remove`, or `null` when nobody has ruled on this item. */
  ruling: string | null;
  /** Who ruled. `null` exactly when `ruling` is `null`. */
  ruled_by: string | null;
  /**
   * Why this item is not in the default list: `does_not_belong` / `removed`, or
   * `null` for a visible one.
   *
   * Domain note: SERVED, not derived here. The page and the Word export apply
   * one set of hide rules, and that set lives in the backend.
   */
  hidden_reason: string | null;
  /**
   * The finished one-line rendering of a Q&A card, composed by the backend from
   * the stored templates. `null` for anything that is not one — the row then
   * renders `verbatim_quote` as usual.
   */
  rfa_line: string | null;
  /**
   * How many items this row stands for: itself, plus every `duplicate_of` the
   * backend folded into it — the "×N". `1` means nothing folded, which is most
   * rows.
   */
  occurrences: number;
};

/**
 * One mapped Allegation as it appears in the detail panel's list. Backend
 * source: `AllegationSummary` in element_detail_repository.rs.
 *
 * `source_section` is one of "Common" | "Dedicated" | "Unknown". The panel
 * uses it to group the list under labeled section dividers.
 *
 * `supporting_evidence` is the Evidence corroborating this Allegation and
 * `disputing_evidence` the Evidence rebutting it. For both, an **empty array is
 * the visible gap**, rendered explicitly and never omitted (Rule 1) — but they
 * are different gaps: nothing corroborates this Allegation, versus nothing
 * disputes it.
 */
export type AllegationSummary = {
  allegation_id: string;
  paragraph_number: string;
  summary: string | null;
  title: string | null;
  verbatim_quote: string | null;
  source_section: string;
  supporting_evidence: AllegationEvidence[];
  /**
   * Evidence REBUTTING this Allegation — the items behind the Proof Matrix's
   * Disputes column. Same shape as `supporting_evidence`; the two are rendered
   * as a matched pair.
   */
  disputing_evidence: AllegationEvidence[];
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
  /**
   * How many items each paragraph shows before "N more" — the stored
   * `matrix_visible_items`, served rather than compiled in (Rule 2). The Word
   * export prints the same number per list, so the page and the document a
   * reader takes away from it cannot disagree.
   */
  visible_items: number;
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

  // Both evidence legs must be arrays on every allegation. The panel reads
  // `.length` on each to decide between the item list and the empty state, so a
  // missing key would throw a TypeError mid-render — and this app has NO React
  // error boundary, so that surfaces as a BLANK PAGE, not an error message.
  // Failing here instead turns a white screen into a sentence naming the
  // problem. Checked at the boundary, once, rather than guarded at every
  // dereference (Standing Rule 1 — a contract mismatch is loud, and it is not
  // the component's job to tolerate one).
  const malformed = parsed.allegations.find(
    (a) =>
      !Array.isArray(a?.supporting_evidence) ||
      !Array.isArray(a?.disputing_evidence),
  );
  if (malformed) {
    throw new Error(
      `Element detail response for "${elementId}" has an allegation ` +
        `(${malformed.allegation_id ?? "id unknown"}) missing its ` +
        `"supporting_evidence" or "disputing_evidence" array — backend/frontend ` +
        `contract mismatch. If reloading does not help, report this to the site administrator.`,
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

