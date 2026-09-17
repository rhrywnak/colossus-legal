// =============================================================================
// warRoomStripView.ts — the War Room's four queue tiles, as values
// =============================================================================
//
// CC_TASK_REVIEW_LOOP_v1 §5, ruled mockup v3. Pure (CLAUDE.md rule 30): the tile
// component renders what this returns.
//
// ## Domain note: four queues, each naming WHO owes the move
//
//   Questions answered   — the rehearsal's progress, all scenarios
//   Waiting for Marie    — visible questions with no answer, BOTH sides
//                          (GO v1 ruling 5: 185 − 87 = 98 on 17 Sep data)
//   New answers for you  — the signed-in viewer's unreviewed answers
//   Candidates for Roman — scan proposals nobody has ruled
//
// ## Why summed here and not sent as a fifth payload field
//
// Every number is already on the cards the page renders. Summing the cards
// makes the strip and the cards one fact by construction: a strip that read its
// own backend total could disagree with the cards beneath it, and both would
// render.

import { fill } from "../services/caseTimeline";
import type { ScenarioSummary, WarRoomWording } from "../pages/trialPrepData";

/** One tile: the big number (already composed) and its label. */
export interface StripTile {
  /** A stable handle for the markup and the tests. */
  key: "answered" | "waiting" | "new_for_you" | "candidates";
  value: string;
  label: string;
  /** True when the number is owed work and renders in the warning colour. */
  warning: boolean;
}

/** The four tiles, in the mockup's order. */
export function warRoomStripView(
  scenarios: ScenarioSummary[],
  wording: WarRoomWording,
): StripTile[] {
  const sum = (pick: (s: ScenarioSummary) => number) =>
    scenarios.reduce((total, s) => total + pick(s), 0);

  const answered = sum((s) => s.progress.answered.total);
  const visible = sum((s) => s.progress.answered.of);
  const waiting = visible - answered;
  const newForYou = sum((s) => s.progress.new_answers_for_viewer);
  const candidates = sum((s) => s.progress.candidates_to_rule);

  return [
    {
      key: "answered",
      value: fill(wording.strip_answered_template, { answered, total: visible }),
      label: wording.strip_answered_label,
      warning: false,
    },
    { key: "waiting", value: String(waiting), label: wording.strip_waiting_label, warning: waiting > 0 },
    {
      key: "new_for_you",
      value: String(newForYou),
      label: wording.strip_new_for_you_label,
      warning: newForYou > 0,
    },
    {
      key: "candidates",
      value: String(candidates),
      label: wording.strip_candidates_label,
      // The mockup draws Roman's queue in the link colour, not amber: it is
      // a count of proposals, and the cards below already flag each owed one.
      warning: false,
    },
  ];
}
