// envBanner.ts — the test-system bar's words, from the store.
//
// One GET, read once by the shell. The bar is already on screen when this is
// called: the warning paints from the compiled fallback and these words replace
// it. So a failure here is a degraded bar, never an absent one.
//
// ## ⚑ RULED CARVE-OUT (Roman, 2026-09-22) — this fetch, and no other
//
// CLAUDE.md Rule 1 requires an explicit error UI on every `fetch`/`authFetch`,
// and the rules-enforcer flagged this file for failing it. Ruled: the carve-out
// stands HERE because `FALLBACK_TEXT` is pinned word-for-word to the stored
// `env_banner_text` (a test reads the migration off disk), so a failed read
// changes nothing on screen — there is no degraded state for a surface to
// describe. An error line would put "the wording could not be read" on a
// witness's warning bar, which the fatal-flaw ruling forbids, and would shout
// loudest on the machine that matters least.
//
// The grant is this function only. Every other data read in this app keeps the
// full requirement: an explicit `.catch()` AND an error surface. A failed data
// read is never best-effort.
//
// What makes the failure observable instead: every arm below logs with its
// cause, and `services/__tests__/envBanner.test.ts` pins each one.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

/**
 * The bar's four strings.
 *
 * ## ⚑ Checked BY EYE against `backend/src/dto/env_banner.rs::EnvBannerDto`
 *
 *   pub text: String        → text: string
 *   pub link_label: String  → link_label: string
 *   pub print_line: String  → print_line: string
 *   pub real_url: String    → real_url: string
 */
export type EnvBannerWords = {
  text: string;
  link_label: string;
  print_line: string;
  real_url: string;
};

/** How long to wait. The same 30s every ordinary read in this app uses. */
const TIMEOUT_MS = 30000;

/**
 * Fetch the bar's words, or `null` when they cannot be had.
 *
 * `null` is a legitimate answer and the caller draws the fallback sentence for
 * it. Every refusal is logged with its cause — a warning bar quietly showing
 * the compiled words when the store says something else is exactly the drift
 * the pinning test exists to prevent, and the log is how anybody finds out.
 */
export async function fetchEnvBannerWords(): Promise<EnvBannerWords | null> {
  try {
    const response = await authFetch(`${API_BASE_URL}/api/env-banner`, {
      timeoutMs: TIMEOUT_MS,
    });
    if (!response.ok) {
      // eslint-disable-next-line no-console
      console.warn(`env banner: the stored words could not be read (HTTP ${response.status}).`);
      return null;
    }
    const parsed = (await response.json()) as Partial<EnvBannerWords>;
    if (
      typeof parsed.text !== "string" ||
      typeof parsed.link_label !== "string" ||
      typeof parsed.print_line !== "string" ||
      typeof parsed.real_url !== "string"
    ) {
      // eslint-disable-next-line no-console
      console.warn("env banner: the response is missing a field — backend/frontend contract mismatch.");
      return null;
    }
    return {
      text: parsed.text,
      link_label: parsed.link_label,
      print_line: parsed.print_line,
      real_url: parsed.real_url,
    };
  } catch (cause: unknown) {
    // eslint-disable-next-line no-console
    console.warn("env banner: the stored words could not be read", cause);
    return null;
  }
}
