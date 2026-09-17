// =============================================================================
// warRoomSummaryView.ts — the War Room's ONE summary card, as strings and flags
// =============================================================================
//
// CC_TASK_SIMPLE_COUNTS_v1, ruled on WAR_ROOM_SUMMARY_CARD_RULED_2026-09-17. Pure
// (CLAUDE.md rule 30): `WarRoomSummaryCard` renders what this returns.
//
// ## Domain note: three owned queues, one truth
//
//   Unanswered questions      — Marie's:      visible questions with no answer
//   Answers requiring review  — the reviewer's: answers newer than the REVIEWER's
//                               Done reviewing (the server binds the reviewer)
//   Candidates to rule        — Roman's:      scan proposals nobody has ruled
//
// Every number is the same for every viewer. There is no "for you" here any more.
//
// ## Why summed from the cards, not a second payload
//
// Every count is already on the scenario cards this page renders. Summing them
// makes the summary and the cards one fact by construction: a summary read on its
// own could disagree with the cards beneath it, and both would render.

import { fill } from "../services/caseTimeline";
import { pickByCount } from "../utils/countWording";
import { formatCardDay } from "./warRoomCardView";
import type { ScenarioSummary, TrialPrepDashboard, WarRoomWording } from "../pages/trialPrepData";

/** STRUCTURAL: the ruled mockup lists up to three untouched scenarios. */
const UNTOUCHED_SHOWN = 3;
/** STRUCTURAL: the ruled mockup lists the four largest candidate piles. */
const PILES_SHOWN = 4;

/** One quiet metric in the top row. */
export interface SummaryMetric {
  value: number;
  label: string;
}

/** Who owns a queue — the cell's stripe and chip colour. */
export type QueueOwner = "marie" | "reviewer" | "roman";

/** One cell of the bottom row. */
export interface SummaryCell {
  owner: QueueOwner;
  label: string;
  /** The chip's words — a stored name, never a username. */
  chip: string;
  count: number;
  /** True when the count renders in the warning colour. */
  warning: boolean;
  /** The one muted line under the count; every brace already filled. */
  context: string;
}

/** Everything the summary card renders. */
export interface WarRoomSummaryView {
  metrics: SummaryMetric[];
  answeredLabel: string;
  answered: number;
  /** "of 185 · 58%" — the rest of the answered figure. */
  answeredRest: string;
  /** 0‥1, the bar's fill. */
  fraction: number;
  cells: SummaryCell[];
}

/**
 * Shape the dashboard into the summary card.
 *
 * @param dashboard the payload: metrics and every scenario card with `progress`
 * @param wording   the page's stored words (and the reviewer's display name)
 */
export function warRoomSummaryView(
  dashboard: Pick<TrialPrepDashboard, "metrics" | "scenarios">,
  wording: WarRoomWording,
): WarRoomSummaryView {
  const scenarios = dashboard.scenarios;
  const sum = (pick: (s: ScenarioSummary) => number) =>
    scenarios.reduce((total, s) => total + pick(s), 0);

  const answered = sum((s) => s.progress.answered.total);
  const visible = sum((s) => s.progress.answered.of);
  const pct = visible === 0 ? 0 : Math.round((answered * 100) / visible);

  return {
    metrics: [
      { value: dashboard.metrics.scenarios, label: wording.metric_scenarios_label },
      { value: dashboard.metrics.ready, label: wording.metric_ready_label },
      { value: dashboard.metrics.drafted_or_review, label: wording.metric_draft_label },
    ],
    answeredLabel: wording.summary_answered_label,
    answered,
    answeredRest: fill(wording.summary_answered_rest_template, { total: visible, pct }),
    fraction: visible === 0 ? 0 : Math.min(1, answered / visible),
    cells: [unansweredCell(scenarios, wording), reviewCell(scenarios, wording), candidatesCell(scenarios, wording)],
  };
}

/** Marie's cell: unanswered questions, and which scenarios nobody has started. */
export function unansweredCell(scenarios: ScenarioSummary[], wording: WarRoomWording): SummaryCell {
  const unanswered = (s: ScenarioSummary) => s.progress.answered.of - s.progress.answered.total;
  const count = scenarios.reduce((total, s) => total + unanswered(s), 0);
  const across = scenarios.filter((s) => unanswered(s) > 0).length;
  // Untouched = visible questions and zero answers (GO ruling 6). No deck = absent.
  const untouched = scenarios
    .filter((s) => s.progress.answered.of > 0 && s.progress.answered.total === 0)
    .slice(0, UNTOUCHED_SHOWN)
    .map((s) => s.code);

  let context: string;
  if (count === 0) {
    context = wording.summary_unanswered_zero;
  } else if (untouched.length === 0) {
    context = fill(
      pickByCount(across, wording.summary_unanswered_context_none_untouched_one, wording.summary_unanswered_context_none_untouched),
      { n: across },
    );
  } else {
    context = fill(
      pickByCount(across, wording.summary_unanswered_context_one, wording.summary_unanswered_context_template),
      { n: across, codes: untouched.join(`${wording.summary_code_joiner} `) },
    );
  }
  // CC_GO_QUESTION_CHAT_v1 (STOP 2): the SUM of the cards' own new-or-changed
  // pill — one concept, one counting rule. Suppressed at 0.
  const changed = scenarios.reduce((total, s) => total + s.progress.marie_changed, 0);
  if (changed > 0) {
    context += ` ${wording.summary_list_joiner} ${fill(wording.summary_unanswered_changed_clause, { n: changed })}`;
  }
  return {
    owner: "marie",
    label: wording.summary_unanswered_label,
    chip: wording.owner_marie,
    count,
    // Marie's queue stays ink: unanswered questions are rehearsal ahead, not a fault.
    warning: false,
    context,
  };
}

/** The reviewer's cell: answers awaiting review, since when, and the biggest pile. */
export function reviewCell(scenarios: ScenarioSummary[], wording: WarRoomWording): SummaryCell {
  const count = scenarios.reduce((total, s) => total + s.progress.awaiting_review, 0);
  let context = wording.summary_review_zero;
  if (count > 0) {
    // ISO timestamps sort as strings, so the smallest is the oldest.
    const oldest = scenarios
      .map((s) => s.progress.oldest_awaiting_review)
      .filter((at): at is string => at !== null)
      .sort()[0];
    // The first scenario with the largest pile, in the dashboard's order.
    const largest = scenarios.reduce((best, s) =>
      s.progress.awaiting_review > best.progress.awaiting_review ? s : best,
    );
    context = fill(wording.summary_review_context_template, {
      date: oldest === undefined ? "" : formatCardDay(oldest),
      code: largest.code,
      n: largest.progress.awaiting_review,
    });
  }
  return {
    owner: "reviewer",
    label: wording.summary_review_label,
    chip: wording.reviewer_display_name,
    count,
    warning: count > 0,
    context,
  };
}

/** Roman's cell: candidates to rule, and the four largest piles (ties share one). */
export function candidatesCell(scenarios: ScenarioSummary[], wording: WarRoomWording): SummaryCell {
  const count = scenarios.reduce((total, s) => total + s.progress.candidates_to_rule, 0);
  let context = wording.summary_candidates_zero;
  if (count > 0) {
    const sizes = [...new Set(scenarios.map((s) => s.progress.candidates_to_rule).filter((n) => n > 0))]
      .sort((a, b) => b - a)
      .slice(0, PILES_SHOWN);
    context = sizes
      .map((n) =>
        fill(wording.summary_candidates_pile_template, {
          codes: scenarios
            .filter((s) => s.progress.candidates_to_rule === n)
            .map((s) => s.code)
            .join(` ${wording.summary_tie_joiner} `),
          n,
        }),
      )
      .join(` ${wording.summary_list_joiner} `);
  }
  return {
    owner: "roman",
    label: wording.summary_candidates_label,
    chip: wording.owner_roman,
    count,
    warning: count > 0,
    context,
  };
}
