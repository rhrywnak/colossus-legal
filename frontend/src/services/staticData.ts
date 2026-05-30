// =============================================================================
// staticData.ts — fetch + validate JSON files served from the frontend host
// -----------------------------------------------------------------------------
// The Home page reads two bundled, same-origin JSON files (case-summary.json and
// timeline.json) that live under `frontend/public/data/` and are served by the
// frontend host (Vite in dev, nginx in the container) at `/data/*`. These are
// NOT backend API calls, so they do NOT go through `authFetch` (no credentials,
// no API base URL). They DO still need a timeout and explicit error handling.
//
// This module centralizes the "fetch a static JSON file" mechanics so each
// consumer service only writes its own shape validation. Standing Rule 1 (no
// silent failures): every distinct failure — network/timeout, non-2xx, or
// unparseable body — throws a different, contextual error naming the resource.
// =============================================================================

// CONST: ceiling for loading one small bundled JSON file from the same origin.
// Not per-environment runtime config — it's a fixed client-side UX guardrail so
// a hung host doesn't leave a card spinning forever. The API client uses 30s for
// cross-origin backend calls (auth.ts); a local static file warrants a tighter
// bound. Promote to config only if a real deployment ever needs to flex it.
const STATIC_JSON_TIMEOUT_MS = 10000;

/**
 * Fetch a same-origin static JSON file and return its parsed body as `unknown`.
 *
 * The caller is responsible for validating the returned shape (we deliberately
 * return `unknown`, not `any`, so the caller cannot skip that step by accident).
 *
 * ## React/TS Learning: AbortController as a fetch timeout
 * `fetch` has no built-in timeout. We arm an `AbortController`, schedule
 * `controller.abort()` after the ceiling, and pass `controller.signal` to
 * `fetch`. If the timer fires first, `fetch` rejects with an `AbortError`,
 * which we translate into a human-readable message. The `finally` always
 * clears the timer so a fast response doesn't leave a dangling timeout.
 *
 * @param path absolute same-origin path, e.g. `/data/case-summary.json`
 * @param label human name of the resource for error messages, e.g. `case summary`
 * @returns the parsed JSON body, typed as `unknown` for the caller to validate
 * @throws Error on network failure/timeout, non-2xx status, or invalid JSON
 */
export async function fetchStaticJson(
  path: string,
  label: string,
): Promise<unknown> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), STATIC_JSON_TIMEOUT_MS);

  let response: Response;
  try {
    response = await fetch(path, { signal: controller.signal });
  } catch (err) {
    // Network error or the abort timer firing both land here. Surface which
    // resource failed and why, rather than collapsing to a blank card.
    const cause = err instanceof Error ? err.message : "network error";
    throw new Error(
      `Failed to load ${label} from ${path} (${cause}). Try reloading the page.`,
    );
  } finally {
    clearTimeout(timeoutId);
  }

  if (!response.ok) {
    throw new Error(
      `Failed to load ${label} from ${path} (HTTP ${response.status}). Try reloading the page.`,
    );
  }

  try {
    return await response.json();
  } catch {
    throw new Error(
      `${label} at ${path} was not valid JSON. Try reloading the page.`,
    );
  }
}
