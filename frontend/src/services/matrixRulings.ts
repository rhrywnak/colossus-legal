// =============================================================================
// matrixRulings.ts — the two write calls behind the Keep / Remove / Undo buttons
// -----------------------------------------------------------------------------
// PROOF_MATRIX_v2 §2.
//
//   PUT    /api/cases/:slug/proof-matrix/rulings  → saveRuling
//   DELETE /api/cases/:slug/proof-matrix/rulings  → withdrawRuling
//
// Both carry the pair in the BODY, on both verbs. An Evidence id is a content
// hash of the shape `doc-…:evidence:01d3e125`; two of those in a path would make
// every click depend on the browser and the router agreeing about
// percent-encoding, on a control a reader presses dozens of times a sitting.
//
// ## Why a failure here is never swallowed
//
// The row updates OPTIMISTICALLY — it moves the moment the button is pressed,
// because a proof surface that pauses on every click is a surface nobody reads
// through. That makes a silent failure the worst possible outcome: the screen
// would show a decision the database does not hold, and nothing would say so.
// Every call below throws with a message the caller renders through the stored
// `matrix_ruling_failed_template` (Standing Rule 1).
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

/**
 * The two verdicts. Mirrors `domain::matrix_ruling::MatrixRuling` — a closed
 * vocabulary, so a token this build does not define is a compile error here
 * rather than a 4xx at the boundary.
 */
export type MatrixRulingToken = "keep" | "remove";

/** What the write did, as the backend reports it. */
export type RulingResult = {
  evidence_id: string;
  allegation_id: string;
  /** `rule` / `rerule` / `withdraw` / `none`. */
  action: string;
  /**
   * Whether anything actually changed. `false` from a withdrawal means the item
   * was not ruled after all — the view was stale, and only that outcome means
   * the page should reload rather than quietly agree with itself.
   */
  changed: boolean;
};

/** The endpoint both verbs address. */
function rulingsUrl(slug: string): string {
  return `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/proof-matrix/rulings`;
}

/**
 * Keep or remove one item.
 *
 * @param slug         case slug
 * @param evidenceId   the Evidence node id
 * @param allegationId the Allegation the item is ruled UNDER — a ruling is about
 *                     a pair, because the same statement commonly bears on
 *                     several accusations and a verdict under one says nothing
 *                     about the others
 * @param ruling       `"keep"` or `"remove"`
 * @throws Error naming the status, for the caller's error line
 */
export async function saveRuling(
  slug: string,
  evidenceId: string,
  allegationId: string,
  ruling: MatrixRulingToken,
): Promise<RulingResult> {
  return writeRuling(slug, "PUT", {
    evidence_id: evidenceId,
    allegation_id: allegationId,
    ruling,
  });
}

/**
 * Take a verdict back — the Undo beside a removed item.
 *
 * The body omits `ruling` deliberately: a withdrawal asserts nothing, and the
 * backend's ledger stores NULL there for the same reason.
 */
export async function withdrawRuling(
  slug: string,
  evidenceId: string,
  allegationId: string,
): Promise<RulingResult> {
  return writeRuling(slug, "DELETE", {
    evidence_id: evidenceId,
    allegation_id: allegationId,
  });
}

/**
 * The shared write: one verb, one body, one set of failure messages.
 *
 * ## Why the two verbs share this and do not each spell it out
 *
 * They differ by one word and one optional field. Two copies would be two places
 * for the error handling to drift, and the error handling is the part that
 * matters — see the module header on optimistic updates.
 */
async function writeRuling(
  slug: string,
  method: "PUT" | "DELETE",
  body: Record<string, string>,
): Promise<RulingResult> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(rulingsUrl(slug), {
    method,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    // Each status means something different to a reader, so each says something
    // different. A 403 in particular is not a fault to report — it is a
    // permission the person does not have.
    const reason =
      response.status === 403
        ? "you do not have permission to rule on evidence"
        : response.status === 400
          ? "the server refused the request as malformed"
          : `the server answered HTTP ${response.status}`;
    throw new Error(`${reason}.`);
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error("the server's reply was not valid JSON.");
  }

  const parsed = data as Partial<RulingResult>;
  if (typeof parsed.changed !== "boolean" || typeof parsed.action !== "string") {
    // A body this build cannot read means the write MAY have happened. Saying so
    // is the only honest answer: the alternative is a row that stays where the
    // click put it on the strength of a reply nobody parsed.
    throw new Error(
      "the server's reply did not say what it did — reload to see what is stored.",
    );
  }
  return parsed as RulingResult;
}

/**
 * The URL behind the "Export this count (Word)" button.
 *
 * A plain link rather than a fetch: the response is a file with a
 * `Content-Disposition`, and letting the browser handle the download is what
 * makes it land in the downloads folder with its filename intact. A fetch would
 * mean holding the bytes in memory and synthesising an anchor, for nothing.
 */
export function countExportUrl(slug: string, countNumber: number): string {
  return `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/proof-matrix/export.docx?count=${encodeURIComponent(String(countNumber))}`;
}
