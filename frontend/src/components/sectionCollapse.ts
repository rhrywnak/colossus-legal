// =============================================================================
// sectionCollapse.ts — remembered fold state for the scenario page's sections
// =============================================================================
//
// The scenario detail page has two long sections — the candidates queue and the
// scenario facts — and both now arrive COLLAPSED, with whatever the human last
// chose remembered per scenario.
//
// ## What changed, and what it supersedes (ruled 2026-08-28)
//
// Ruling R7 declined to persist this, on the reasoning that "a queue that
// remembers 'collapsed' over 145 unruled candidates is a silent failure wearing
// a preference's clothes". That objection was about a section that hides its
// contents. It no longer applies, because the thing R7 was protecting is now
// guaranteed by construction: a folded section keeps its heading AND its count
// line on screen, so a closed candidates queue still reads "Included — 21 · 3 of
// 148 ruled" and a closed facts list still reads "21 included · 4 added by hand".
// The work is never invisible; only the rows are.
//
// What the ruling did not weigh is the cost of the other default. Both sections
// are long, they sit one above the other, and a scenario page that opens with
// both expanded starts several screens from the top — so every visit began with
// the same two clicks. Remembering the answer is the whole fix.
//
// ## Why the key carries the scenario id
//
// A preference stored per SECTION alone would make expanding S-10's candidates
// expand S-11's too, which is not a preference — it is one scenario's state
// leaking onto another's page. Each scenario remembers its own answer.
//
// ## Why this is a module and not two `useState`s
//
// Two sections, two components, one behaviour. Written by hand in each place
// they would drift on the key format, on what an absent value means, and on
// which failures are survivable — and the failure mode of drift here is a
// preference that silently stops being read.

import { useState } from "react";

/**
 * Which of the scenario page's foldable sections a key refers to.
 *
 * ## TS learning: a string-literal union as a closed vocabulary
 *
 * `"candidates" | "facts"` is not a loose `string`: the compiler rejects a third
 * value at the call site, so a typo cannot mint a key that nothing else ever
 * reads. That matters more than usual here — a mistyped key does not fail, it
 * silently stores a preference in a slot no one looks at, and the section just
 * appears to forget.
 */
export type ScenarioSection = "candidates" | "facts";

/**
 * The `localStorage` key for one section of one scenario.
 *
 * Shape: `scenario:<scenario_id>:<section>:collapsed`.
 *
 * The key says `collapsed` rather than `open` because that is what the stored
 * value means, and a key whose name is the opposite of its value is how a later
 * reader inverts the logic. See [`readSectionOpen`] for the encoding.
 */
export function sectionCollapseKey(scenarioId: string, section: ScenarioSection): string {
  return `scenario:${scenarioId}:${section}:collapsed`;
}

/** The stored value meaning "this section was left open". */
const NOT_COLLAPSED = "false";

/**
 * Is this section open?
 *
 * **Absent, unreadable, or unrecognised all mean COLLAPSED**, which is the new
 * default and also the safe direction: the failure of this preference is a
 * section that needs one click, not a section that hides work the human never
 * asked to hide.
 *
 * Only the exact string [`NOT_COLLAPSED`] opens a section. A garbage value left
 * by a hand-edit or a future format change therefore falls back to the default
 * rather than being coerced into a boolean by accident.
 */
export function readSectionOpen(scenarioId: string, section: ScenarioSection): boolean {
  const key = sectionCollapseKey(scenarioId, section);
  try {
    return localStorage.getItem(key) === NOT_COLLAPSED;
  } catch (e: unknown) {
    // best-effort: this is a COSMETIC fold preference, so a private window, a
    // browser with site data blocked, or a server-render with no `localStorage`
    // at all degrades to the documented default instead of raising a banner —
    // the Standing Rule 1 carve-out for browser-storage preferences. It stays
    // observable in the console, and it is NOT a data read: nothing about the
    // scenario itself is fetched, stored, or lost here.
    console.warn(`Could not read the fold preference ${key}; defaulting to collapsed.`, e);
    return false;
  }
}

/**
 * Remember whether this section is open.
 *
 * Writes on every toggle rather than on unload: a page that is closed, reloaded,
 * or navigated away from mid-thought must remember the same thing as one that
 * was closed politely.
 */
export function writeSectionOpen(
  scenarioId: string,
  section: ScenarioSection,
  open: boolean,
): void {
  const key = sectionCollapseKey(scenarioId, section);
  try {
    localStorage.setItem(key, open ? NOT_COLLAPSED : "true");
  } catch (e: unknown) {
    // best-effort: same carve-out as the read above. A storage quota error or a
    // blocked store means the fold works for this visit and is forgotten by the
    // next one, which is a cosmetic loss and does not warrant interrupting the
    // human mid-triage with an error banner.
    console.warn(`Could not save the fold preference ${key}; it will not persist.`, e);
  }
}

/**
 * The remembered fold state for one section, and a toggle that saves it.
 *
 * ## Why the key is re-read when the scenario changes
 *
 * `useState`'s initialiser runs ONCE per mount, and the router keeps this page
 * mounted while `:id` changes — so navigating S-10 → S-11 would otherwise carry
 * S-10's answer onto S-11's page, which is exactly the leak the per-scenario key
 * exists to prevent. Comparing the key against the one the current state was
 * built from, and re-reading when they differ, closes it.
 *
 * ## TS/React learning: adjusting state during render
 *
 * Assigning state inside the render body looks wrong and is in fact React's
 * documented pattern for deriving state from changed props (the alternative, a
 * `useEffect`, renders once with the WRONG section open and then corrects it,
 * which the human sees as a flicker). The inequality guard is what makes it
 * safe: it runs on the render where the key changed and never again.
 * `ScanSection` already used this shape for its own default tracking.
 */
export function useSectionOpen(
  scenarioId: string,
  section: ScenarioSection,
): [boolean, () => void] {
  const key = sectionCollapseKey(scenarioId, section);
  const [open, setOpen] = useState(() => readSectionOpen(scenarioId, section));
  const [lastKey, setLastKey] = useState(key);

  if (lastKey !== key) {
    setLastKey(key);
    setOpen(readSectionOpen(scenarioId, section));
  }

  const toggle = () => {
    const next = !open;
    setOpen(next);
    writeSectionOpen(scenarioId, section, next);
  };

  return [open, toggle];
}
