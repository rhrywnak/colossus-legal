// =============================================================================
// scenarioHeader.ts — what the lean one-line header shows (task 1.7B)
// =============================================================================
//
// The Casefleet issue-header pattern (study §1.7): an issue's header is LEAN —
// all the richness lives on the facts below it. One line: code · name ·
// direction chip · status chip · edit · rehearsal link.
//
// What died to get here: a permanent full-width definition form with giant
// always-visible inputs and a scroll-box allegation picker, sitting between the
// reader and the evidence. Editing moved into one modal; the header states what
// this scenario IS and gets out of the way.
//
// Pure and tested here so the page renders a descriptor rather than deciding.

import type { ScenarioStatus } from "../pages/trialPrepData";
import { statusMeta } from "../pages/trialPrepHelpers";

/** One chip in the header line. */
export interface HeaderChip {
  label: string;
  /** A CSS custom property, resolved by the caller's style. */
  color: string;
  /** Shown on hover — why this chip reads the way it does, when that is not
   *  self-evident. `null` when the label speaks for itself. */
  title: string | null;
}

export interface HeaderDescriptor {
  /** "S-2" — the speakable handle (§2a). Never editable. */
  code: string;
  name: string;
  direction: HeaderChip;
  status: HeaderChip;
  /**
   * The readiness verdict's slot (§9), which task 2.4 fills.
   *
   * `null` until then, and the header renders NOTHING for it — not "Unknown",
   * not a grey placeholder chip. A verdict is a claim about whether this
   * scenario can be taken into a courtroom; inventing a neutral-looking one
   * before the code that computes it exists would be the page saying something
   * it does not know.
   */
  readiness: HeaderChip | null;
}

/**
 * The direction chip.
 *
 * ## Domain note: why this is read-only
 *
 * A scenario's offense/defense stance is its IDENTITY, not an attribute —
 * flipping it would make it a different scenario. The backend refuses it on the
 * update route (`ScenarioUpdateRequest` has no `direction` field and is
 * `deny_unknown_fields`), so the chip states the fact and the modal offers no
 * control for it. A wrongly-created direction is cured by archive-and-recreate
 * (task 3.6), not by mutation — and the `title` says so, because a human who
 * wants to change it deserves to be told how rather than left clicking a chip
 * that does nothing.
 */
export function directionChip(direction: string): HeaderChip {
  const known: Record<string, string> = {
    offense: "Offensive",
    defense: "Defensive",
  };
  const label = known[direction];
  return label
    ? {
        label,
        color: "var(--text-secondary)",
        title:
          "A scenario's direction is set when it is created and cannot be " +
          "changed — flipping it would make this a different scenario.",
      }
    : {
        // An unknown token is shown VERBATIM rather than mapped to a default.
        // "Defensive" on a scenario the database calls something else would be
        // the page inventing a posture.
        label: direction,
        color: "var(--state-warning-strong)",
        title: `This build does not recognise the direction '${direction}'.`,
      };
}

/** The whole header line, as data. */
export function headerDescriptor(input: {
  code: string;
  name: string;
  direction: string;
  status: ScenarioStatus;
}): HeaderDescriptor {
  const meta = statusMeta(input.status);
  return {
    code: input.code,
    name: input.name,
    direction: directionChip(input.direction),
    status: { label: meta.label, color: meta.color, title: null },
    readiness: null,
  };
}
