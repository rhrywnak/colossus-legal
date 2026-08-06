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
 * Where in the list a step lands, clamped into range.
 *
 * ## Why the position SENTENCE is no longer composed here (task 2.11 B2)
 *
 * `positionLabel` used to build "Scenario 2 of 5" from two numbers. That was
 * prose composed in a component, and the rehearsal page's language is a settings
 * row now — Roman edits it without a build. The backend fills the template once
 * per scenario and ships the finished sentences as `payload.positions`, so this
 * module keeps the ARITHMETIC (which is not language) and has given up the words.
 *
 * The off-by-one this file used to guard against is guarded on the other side:
 * the positions are generated `1..=total`, so a 0-based index can only ever be
 * used to look one up, never to build one.
 */
export function positionAt(index: number, positions: string[]): string | null {
  if (positions.length === 0) return null;
  const safe = Math.min(Math.max(index, 0), positions.length - 1);
  return positions[safe] ?? null;
}
