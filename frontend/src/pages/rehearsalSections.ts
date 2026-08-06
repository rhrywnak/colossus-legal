// =============================================================================
// rehearsalSections.ts — the pure rules the collapse layer needs (task 2.11 B2)
// =============================================================================
//
// Extracted from the page so the rules a witness depends on are unit-testable
// without DOM test infrastructure — the house pattern (pure helpers + service
// tests, CLAUDE.md rule 30), and the same reason `rehearsalNav` exists.
//
// Both functions are deliberately tiny, and both sit where a plausible mistake
// would be invisible on screen: a default-state map wired to the wrong field
// opens the wrong section, and a placeholder left unfilled prints "{code}" to a
// witness.

import type { RehearsalCollapse } from "../services/rehearsal";

/** Which sections are open right now. Per-visit state, never stored. */
export type OpenSections = {
  /** Task 2.11 C: "What this is" folds like the others per the signed mockup. */
  what: boolean;
  accusation: boolean;
  timeline: boolean;
  points: boolean;
  watchFor: boolean;
};

/**
 * The opening state each section starts in.
 *
 * ## Why this is a mapping and not a spread
 *
 * The payload's field names are snake_case wire names and this type's are
 * camelCase; `{...collapse}` would compile to an object whose keys the component
 * never reads, so every section would default to `undefined` — which renders as
 * closed. Every section silently shut, with nothing in the log, on a page whose
 * whole design is that the accusation block is open when you arrive.
 *
 * The values are the SERVER's decision, parsed from the store at boot where an
 * unreadable token is a named refusal. Nothing is decided here.
 */
export function openSectionsFrom(collapse: RehearsalCollapse): OpenSections {
  return {
    what: collapse.what_open,
    accusation: collapse.accusation_open,
    timeline: collapse.timeline_open,
    points: collapse.points_open,
    watchFor: collapse.watch_for_open,
  };
}

/**
 * Put a scenario's handle into the stored not-ready sentence.
 *
 * The one substitution this page performs, and it is not composition: the
 * sentence is the human's, and `{code}` is the slot it leaves for the handle the
 * reader typed into their own address bar — the one thing this page may say about
 * a scenario it is not showing.
 *
 * A template edited to drop the placeholder is refused by the settings write path
 * (`REQUIRED_PLACEHOLDERS`), so an unfilled `{code}` reaching a screen means the
 * store was edited around the API. It then shows verbatim, visibly wrong, which
 * is how a reader finds out.
 */
export function fillCode(template: string, code: string | undefined): string {
  return template.replace("{code}", code ?? "");
}
