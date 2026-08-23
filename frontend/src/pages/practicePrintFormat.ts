// practicePrintFormat.ts — the two dates on Chuck's sheets.
//
// PURE, and in a module of its own rather than exported from the page, for the
// reason CLAUDE.md rule 30 gives: this project tests pure helpers and services,
// not components. A decision exported from a `.tsx` is a decision that will not
// be tested, and both of these make one — see `asSheetDate`'s guard.
//
// ## Why the browser's locale and not a stored template
//
// These are DATES, not sentences. There is no wording to get wrong, and a stored
// strftime-like template would be a second format nobody could preview before it
// reached paper. Everything a human READS on the sheets is a settings row; how a
// date is spelled is not.

/**
 * The deck's own date, as the sheet header shows it — `19 Aug 2026`.
 *
 * ## The guard, and why it is the whole point of this function
 *
 * Returns `null` for an absent OR unparseable value. `new Date("nonsense")` is a
 * `Date` whose `toLocaleDateString` is the string `"Invalid Date"` — which would
 * print, in the header, on a sheet going to a meeting, and read as though the
 * deck itself were broken. `null` withdraws the line instead, which is the honest
 * rendering of "we do not know when this deck last changed".
 */
export function asSheetDate(iso: string | null): string | null {
  if (iso === null) return null;
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return null;
  return at.toLocaleDateString(undefined, {
    day: "numeric",
    month: "short",
    year: "numeric",
  });
}

/**
 * When this copy came off the printer — date AND time.
 *
 * Distinct from the deck's date, and both are on the sheet: paper outlives the
 * deck it was taken from, and a sheet carrying only one of the two cannot tell a
 * reader which of them it is.
 */
export function asPrintedAt(at: Date): string {
  return at.toLocaleString(undefined, {
    day: "numeric",
    month: "short",
    year: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}
