// =============================================================================
// rehearsalNav.ts — the pure navigation rules for rehearsal mode (task 1.5)
// -----------------------------------------------------------------------------
// Extracted from the page so the rules a witness depends on are unit-testable
// without DOM test infrastructure (the house pattern: pure helpers + service
// tests; see CLAUDE.md rule 30).
// =============================================================================

/** The keys that move between scenarios, and what each one means. */
export type RehearsalStep = "next" | "previous" | null;

/**
 * Map a keydown to a step.
 *
 * Arrows and space/PageDown, because a rehearsal is run one-handed while reading
 * off the screen. `null` for anything else, so a stray keystroke does nothing —
 * the alternative is a witness losing their place mid-sentence.
 */
export function stepForKey(key: string): RehearsalStep {
  switch (key) {
    case "ArrowRight":
    case "ArrowDown":
    case "PageDown":
    case " ":
      return "next";
    case "ArrowLeft":
    case "ArrowUp":
    case "PageUp":
      return "previous";
    default:
      return null;
  }
}

/**
 * Where a step lands, given where we are and how many scenarios there are.
 *
 * ## Why it CLAMPS rather than wraps
 *
 * Wrapping from the last scenario to the first looks like a fresh start. In a
 * rehearsal that is genuinely confusing — Marie would work through the list
 * twice without noticing. Stopping at the end is the honest signal that the end
 * is the end.
 *
 * An out-of-range or negative `current` is clamped into range rather than
 * trusted, so a stale index (a scenario demoted while the page was open) can
 * never index past the list.
 */
export function stepTo(current: number, total: number, step: RehearsalStep): number {
  if (total <= 0) return 0;

  const safe = Math.min(Math.max(current, 0), total - 1);
  if (step === "next") return Math.min(safe + 1, total - 1);
  if (step === "previous") return Math.max(safe - 1, 0);
  return safe;
}

/**
 * "Scenario 2 of 5" — the position line.
 *
 * Composed here rather than inline so it stays one sentence in one place, and so
 * the 1-based count (what a human says) can never drift from the 0-based index
 * (what the array uses). Off-by-one in a position label is the kind of thing
 * nobody notices until it is on a screen in a courtroom.
 */
export function positionLabel(index: number, total: number): string {
  if (total <= 0) return "No scenarios are ready to rehearse";
  const safe = Math.min(Math.max(index, 0), total - 1);
  return `Scenario ${safe + 1} of ${total}`;
}
