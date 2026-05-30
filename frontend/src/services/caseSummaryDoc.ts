// =============================================================================
// caseSummaryDoc.ts — client for the static /data/case-summary.json file
// -----------------------------------------------------------------------------
// The Home page's Case Summary card reads its plain-language prose and venue
// facts from a bundled static file (NOT the backend). This is intentionally a
// static file for Stage 1: the text is editorial/case-specific and does not
// change per request, so it ships with the frontend rather than adding a backend
// endpoint. (The complaint *document link* still resolves dynamically from the
// case-header API — see CaseSummaryCard — so no document id is hardcoded here.)
//
// Naming note: this is distinct from `caseSummary.ts`, which is the API client
// for case-level rollup STATS. This module is only the editorial summary DOC.
// =============================================================================

import { fetchStaticJson } from "./staticData";

/**
 * The shape of `/data/case-summary.json`. All four fields are required prose
 * strings; the card composes them into a paragraph + a venue/filed/status line.
 */
export type CaseSummaryDoc = {
  summary: string;
  venue: string;
  filed: string;
  status: string;
};

/** Resource path + label kept next to the loader so both stay in sync. */
const CASE_SUMMARY_PATH = "/data/case-summary.json";
const CASE_SUMMARY_LABEL = "case summary";

/**
 * Load and validate the case-summary document.
 *
 * Mirrors the API-client pattern (e.g. {@link getCaseHeader}): fetch, then
 * assert the load-bearing fields are present and correctly typed, throwing a
 * contextual error at the boundary rather than letting a malformed file flow
 * into the card and surface as a blank paragraph later — Standing Rule 1.
 *
 * @returns the validated {@link CaseSummaryDoc}
 * @throws Error on fetch/timeout/non-2xx/invalid-JSON (via {@link fetchStaticJson})
 *   or when any required string field is missing
 */
export async function getCaseSummaryDoc(): Promise<CaseSummaryDoc> {
  const data = await fetchStaticJson(CASE_SUMMARY_PATH, CASE_SUMMARY_LABEL);

  const parsed = data as Partial<CaseSummaryDoc>;
  if (
    typeof parsed.summary !== "string" ||
    typeof parsed.venue !== "string" ||
    typeof parsed.filed !== "string" ||
    typeof parsed.status !== "string"
  ) {
    throw new Error(
      `${CASE_SUMMARY_LABEL} at ${CASE_SUMMARY_PATH} is missing required fields ` +
        `(expected summary, venue, filed, status as strings). ` +
        `Fix ${CASE_SUMMARY_PATH} and redeploy the frontend ` +
        `(reloading the page will not help — the file itself is malformed).`,
    );
  }

  return parsed as CaseSummaryDoc;
}
