// =============================================================================
// notFoundHelpers.ts — pure view-shaping for the not-found page
// -----------------------------------------------------------------------------
// The 404 page shows the URL that did not match, because "no such page" without
// the path is a dead end: the usual causes are a typo, a stale bookmark, and a
// link to a retired route, and all three are diagnosed by SEEING the path.
//
// Showing a URL the user (or a link) controls means shaping it first, which is
// what these pure functions do. No DOM, no fetch, no React — so vitest exercises
// them without jsdom/RTL, the established frontend test pattern (CLAUDE.md §30).
//
// NONE of this is business logic: it is presentation of the browser's own
// location, computed nowhere else and decided by nothing on the server.
// =============================================================================

// CONST: a layout property of this page, not a deployment knob. It answers "how
// much text fits on one line in this box" — a question decided by the page's own
// typography and max-width, both of which live in the component beside it, and
// which are identical on DEV and PROD. Making it configurable would let an
// operator break the layout by restart, and would create a value no environment
// has any reason to set differently. Contrast a genuine deployment value (an
// endpoint, a timeout, a model id): change the box, change this — one commit,
// both. Standing Rule 2's test ("could Roman change it by editing YAML?") does
// not apply, because there is nothing an operator would ever want to change here.
/**
 * How much of an unmatched path to show before truncating.
 *
 * A pathname has no practical length limit — browsers accept URLs into the
 * thousands of characters — and the page renders it as a single line, so an
 * unbounded string would blow out the layout on a mistyped or generated URL.
 * 120 keeps every realistic route in this app fully visible (the longest,
 * `/cases/:slug/trial-prep/:scenarioId`, is well under it with real values)
 * while capping the pathological case.
 */
export const MAX_DISPLAYED_PATH_LENGTH = 120;

/**
 * The unmatched path, shaped for display.
 *
 * - An empty or whitespace-only pathname renders as `/`. React Router always
 *   supplies at least `/`, so this is defensive rather than expected — but the
 *   alternative is an empty gap where the page promises a path, which reads as a
 *   rendering bug rather than as "the root".
 * - A path longer than `MAX_DISPLAYED_PATH_LENGTH` is cut and marked with `…`,
 *   so the reader can tell truncation from a path that genuinely ends there.
 *   The ellipsis replaces the last character rather than being appended, so the
 *   result never exceeds the limit it is enforcing.
 *
 * Escaping is NOT this function's job: React escapes text children, so a path
 * containing markup renders as literal text.
 */
export function formatUnknownPath(pathname: string): string {
  const trimmed = pathname.trim();
  if (trimmed === "") return "/";
  if (trimmed.length <= MAX_DISPLAYED_PATH_LENGTH) return trimmed;
  return `${trimmed.slice(0, MAX_DISPLAYED_PATH_LENGTH - 1)}…`;
}
