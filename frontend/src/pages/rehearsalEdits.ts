// =============================================================================
// rehearsalEdits.ts — what the rehearsal page WRITES, and where each write goes
// =============================================================================
//
// Task 2.11 C, ruling C3/C4. Roman's editing ruling turned this surface into a
// caller of the guarded write routes, and this module is the whole map of which
// act reaches which one.
//
// ## Reuse, never fork — asserted here rather than promised
//
// Every function below calls a client this repo already had, aimed at a route
// the scenario working page already calls. There is no rehearsal-only write
// path, and this file is where that is checkable in one screen:
//
//   the accusation sentence  → PUT …/scenarios/:id/accusation
//   "What this is"           → PUT  /cases/:slug/scenarios/:id  (partial body)
//   edit one talking point   → PUT …/scenarios/:id/talking-points/:position
//   add a talking point      → PUT …/scenarios/:id/talking-points  (the list)
//   edit one watch item      → PUT …/scenarios/:id/human-facts/:fact_id
//   add a watch item         → POST …/scenarios/:id/human-facts
//
// ## Extracted from the page so it can be read AND tested
//
// Which route an act reaches is the kind of thing that is invisible when wrong:
// "What this is" pointed at the accusation route would save the theme into the
// accusation column and look fine until somebody read the rehearsal page. A pure
// factory can be handed fakes and asked what it called.

import type { RehearsalEdits } from "../components/RehearsalScenarioBlocks";
import type { RehearsalScenario } from "../services/rehearsal";
import { setAccusationText } from "../services/scenarioAccusation";
import {
  addHumanFact,
  editTalkingPoint,
  editWatchItem,
  setTalkingPoints,
} from "../services/scenarioAugmentation";
import { updateScenario } from "../services/scenarioCrud";

/**
 * Runs one write, reports any failure in the page banner, and re-reads.
 *
 * RESOLVES either way, with the failure as its value (`null` on success). It
 * does not throw — see `RehearsalPage.runWrite` for why, and see
 * {@link rethrowing} for how the four list writes get the rejection their row
 * editors need.
 */
export type RunWrite = (write: () => Promise<void>) => Promise<unknown>;

/**
 * Turn a reported failure back into a rejection, for the callers that need one.
 *
 * `AuthoredLineEditor` keeps a human's draft on screen when its `onSave`
 * rejects, and closes the box when it resolves. Resolving on a save that never
 * landed would throw away words somebody just typed — so the four list writes
 * pass through here, and the two sentence writes, which have no such box, do
 * not.
 *
 * Nothing is swallowed on either path: `run` has already put the failure in the
 * page banner before this is reached.
 */
async function rethrowing(run: RunWrite, write: () => Promise<void>): Promise<void> {
  const failure = await run(write);
  // Loose `!=` on purpose: it covers `null` AND `undefined` in one comparison.
  // The contract says `null` for success, and a runner that returned `undefined`
  // instead would otherwise make this throw a value with no message at all —
  // which `AuthoredLineEditor` could only report as its generic notice, hiding
  // the specific failure the page banner is already showing.
  if (failure != null) {
    throw failure;
  }
}

/**
 * Build the write bundle for one scenario on screen.
 *
 * ## Two shapes, because two editors need two things
 *
 * `SentenceEditor` has no failure surface of its own — it closes on save, and
 * the page banner is the whole report. `AuthoredLineEditor` DOES have one: it
 * keeps the draft on screen and names the failure at the row it came from. So
 * the four list writes go through {@link rethrowing} and the two sentence
 * writes do not. Neither path discards anything — the banner is set before
 * either returns.
 */
export function rehearsalEdits(
  slug: string,
  scenario: RehearsalScenario,
  run: RunWrite,
  busy: boolean,
): RehearsalEdits {
  const id = scenario.scenario_id;

  return {
    onSaveWhat: (text) => {
      // A partial body carrying ONLY the theme. The identity modal edits four
      // fields at once and puts `motivation` on screen, which §10 keeps off a
      // witness surface — so this page reuses the ROUTE and not that component
      // (ruling C4c). The route was measured to accept a partial body; nothing
      // new was added for it.
      //
      // `void` on a promise that cannot reject: `run` reports the failure in the
      // page banner and resolves with it. There is nothing here to catch.
      void run(async () => {
        await updateScenario(slug, id, { theme_statement: text ?? "" });
      });
    },

    onSaveAccusation: (text) => {
      void run(async () => {
        await setAccusationText(slug, id, text);
      });
    },

    onEditPoint: (position, text) =>
      rethrowing(run, () => editTalkingPoint(slug, id, position, text)),

    // The list plus one. The ordering is server-owned and the LIST write is what
    // owns it; an append route would invent an ordering protocol this endpoint
    // does not have. Editing one point is the per-row route, because that
    // changes no ordering and must not re-stamp anybody's authorship.
    onAddPoint: (text) =>
      rethrowing(run, () =>
        setTalkingPoints(slug, id, [...scenario.points.map((p) => p.text), text]),
      ),

    onEditWatchItem: (itemId, text) =>
      rethrowing(run, () => editWatchItem(slug, id, itemId, text)),

    onAddWatchItem: (text) =>
      rethrowing(run, async () => {
        // `kind` is what separates a watch item from a human fact in one table.
        // Forgetting it would file a thing-to-expect as a fact about the case.
        await addHumanFact(slug, id, { text, kind: "watch_list" });
      }),

    busy,
  };
}
