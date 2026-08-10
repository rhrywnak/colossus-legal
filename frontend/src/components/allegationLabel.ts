// =============================================================================
// allegationLabel.ts — one composer for an allegation chip's words (task 1.7C)
// =============================================================================
//
// `"A-41 — refused amicable division"`. The identity MODAL has needed this since
// 1.7B (its chip picker), and 1.7C's identity block (defect D8) needs the same
// string for the read-only "Bears on" chips.
//
// Extracted from `ScenarioIdentityModal` by ruling R11 for one reason: two copies
// of a label rule drift. The modal would keep saying `"A-41 — …"` while the block
// said something else, and the human would be looking at the same allegation
// rendered two ways on one screen.
//
// ## Why the frontend composes this at all
//
// Grudgingly, and narrowly. The language law puts display decisions server-side,
// and this is the exception the 1.7B modal already established: the allegation
// endpoint (`GET /api/allegations`) is a generic, case-wide read shared with the
// Allegations page, and it serves the complaint's own fields — `paragraph`,
// `allegation`, `title` — not a scenario-chip label. So the JOIN of paragraph and
// summary happens here, in ONE tested function, over strings the backend supplied
// verbatim.
//
// It composes no vocabulary of its own: every word in the output came from the
// complaint. The only thing this adds is the A-code prefix and a dash.
//
// ## The prefix is `A-`, not `¶` (task R2 / 10b)
//
// The backend has an allegation-code composer — `domain::scenario_code::
// allegation_code`, which renders paragraph 41 as `A-41` — and it already labels
// the card link chips and the link-options catalogue. This module did not go
// through it and emitted `¶41`, so the identity block's "Bears on" chips called
// the same allegation something different from the chips one section down.
//
// One name, every surface. The `¶` is retired here rather than the code being
// served from `GET /api/allegations`: that endpoint is the case-wide read the
// Allegations page shares, and widening it to carry a scenario-surface code would
// change a screen this task is not touching. Filed as the purer fix.

import type { AllegationDto } from "../services/allegations";

/**
 * The chip label for one allegation.
 *
 * Prefers the complaint's own sentence (`allegation`) over the shorter `title`,
 * because the chip is read to decide whether a scenario really bears on this
 * paragraph, and a title rarely says enough to answer that.
 *
 * Falls back through three levels rather than ever rendering an empty chip:
 *
 * | available | renders |
 * |---|---|
 * | paragraph + summary | `A-41 — refused amicable division` |
 * | paragraph only | `A-41` |
 * | neither | the allegation's id |
 *
 * The id is a poor label, but it is an HONEST one: it names the thing that exists
 * and can be looked up. A blank chip would be the page showing a link to nothing.
 */
export function allegationLabel(allegation: AllegationDto): string {
  const paragraph = allegation.paragraph ? `A-${allegation.paragraph}` : allegation.id;
  const summary = allegation.allegation ?? allegation.title;
  return summary ? `${paragraph} — ${summary}` : paragraph;
}

/**
 * The chip label for an allegation id, using whatever the catalogue knows.
 *
 * The identity block holds `anchor_allegation_ids` — ids, not allegations — and
 * the catalogue arrives from a separate read that may not contain every id (an
 * allegation can be re-keyed, or the read can be narrower than the scenario's
 * anchors).
 *
 * An id the catalogue does not know renders AS THE ID rather than being dropped.
 * Dropping it would make the "Bears on" row quietly shorter than the scenario's
 * actual anchors — the chip count would disagree with the data and nothing on
 * screen would say why.
 */
export function labelForAllegationId(
  allegationId: string,
  catalogue: AllegationDto[],
): string {
  const found = catalogue.find((a) => a.id === allegationId);
  return found ? allegationLabel(found) : allegationId;
}
