// =============================================================================
// fetchUtils.ts — shared helpers for the scenario service clients.
// =============================================================================
//
// Small utilities common to the `authFetch`-based service modules, kept in ONE
// place so a change to how the backend shapes an error body is a single edit
// rather than N drifting copies (Rule 8 — no duplication / no tech debt).

/**
 * Best-effort: pull the backend's human-readable `message` out of an error
 * response body so a validation/failure reason reaches the user.
 *
 * The Rust API layer returns `{ error, message, details }` on a 4xx (see
 * `backend/src/error.rs`). This reads only `message`, and only when it is a
 * string, so a body-absent / non-JSON / wrong-shape body degrades to an empty
 * suffix rather than throwing — the caller always throws its own contextual
 * error regardless; this only ENRICHES it. Returns `" — <message>"` (ready to
 * append) or `""`.
 */
export async function readErrorMessage(response: Response): Promise<string> {
  try {
    const body: unknown = await response.json();
    if (
      body !== null &&
      typeof body === "object" &&
      typeof (body as { message?: unknown }).message === "string"
    ) {
      return ` — ${(body as { message: string }).message}`;
    }
  } catch {
    // Body absent or not JSON — no extra detail to surface.
  }
  return "";
}
