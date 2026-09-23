// =============================================================================
// warRoomFixtures.ts — the status card's test payloads
// =============================================================================
//
// The wording block is the MIGRATIONS' values — 20260916130121 as corrected and
// extended by 20260917080207_review_loop_cursor_and_wording.sql and
// 20260917104454_simple_counts_reviewer_and_summary_wording.sql — so a test
// asserting a rendered string is asserting what DEV will actually print.

import type { ScenarioProgress, ScenarioSummary, WarRoomWording } from "../../pages/trialPrepData";

export const warRoomWording: WarRoomWording = {
  subtitle:
    "The attacks and what we answer them with — built by you, gathered by the system, rehearsed by Marie.",
  metric_scenarios_label: "Scenarios",
  metric_ready_label: "Ready",
  metric_draft_label: "Draft",
  card_evidence_heading: "Evidence",
  card_prep_heading: "Prep & rehearsal",
  card_facts_included_label: "Facts included",
  card_candidates_label: "Candidates to rule",
  card_matrix_linked_label: "Matrix linked",
  card_matrix_linked_template: "{linked} of {total}",
  card_matrix_linked_none: "—",
  card_scan_template: "Last scan {date}",
  card_scan_never: "Never scanned",
  card_deck_none: "—",
  card_answered_count_template: "{answered} of {total}",
  card_answered_word: "answered",
  card_prep_meta_template:
    "Chuck {chuck_answered}/{chuck_total} · defense {defense_answered}/{defense_total} · deck {count} q · {date}",
  card_changed_template: "{count} new or changed for Marie",
  card_review_template: "{count} answers awaiting your review",
  card_review_one: "{count} answer awaiting your review",
  card_changed_one: "{count} new or changed for Marie",
  card_not_started: "Not started",
  card_up_to_date: "Up to date",
  card_practice_action: "Practice →",
  card_timeline_action: "Timeline",
  card_delete_action: "Delete",
  summary_answered_label: "Questions answered",
  summary_answered_rest_template: "of {total} · {pct}%",
  summary_unanswered_label: "Unanswered questions",
  summary_review_label: "Answers awaiting your review",
  summary_review_chip: "YOU",
  summary_candidates_label: "Candidates to rule",
  owner_marie: "Marie",
  owner_roman: "Roman",
  summary_unanswered_context_template: "across {n} scenarios · {codes} untouched",
  summary_unanswered_context_one: "across {n} scenario · {codes} untouched",
  summary_unanswered_context_none_untouched: "across {n} scenarios",
  summary_unanswered_context_none_untouched_one: "across {n} scenario",
  summary_review_context_template: "oldest waiting since {date} · {code} has {n}",
  summary_candidates_pile_template: "{codes} {n}",
  summary_list_joiner: "·",
  summary_tie_joiner: "&",
  summary_code_joiner: ",",
  summary_unanswered_zero: "every question answered",
  summary_review_zero: "nothing waiting",
  summary_candidates_zero: "nothing to rule",
  summary_unanswered_changed_clause: "{n} new or changed",
  reviewer_display_name: "Chuck",
};

/** A scenario with nothing yet: every count zero, never scanned, no deck. */
export function bareProgress(): ScenarioProgress {
  return {
    facts_included: 0,
    candidates_to_rule: 0,
    matrix_linked: { linked: 0, total: 0 },
    last_scan: null,
    deck: { questions: 0, built_on: null },
    answered: {
      total: 0,
      of: 0,
      chuck_answered: 0,
      chuck_total: 0,
      defense_answered: 0,
      defense_total: 0,
    },
    marie_changed: 0,
    awaiting_review: 0,
    oldest_awaiting_review: null,
  };
}

/** S-1 as the ruled mockup v3 draws it (DEV numbers, 17 Sep 2026), viewed by Chuck. */
export function s1(overrides: Partial<ScenarioProgress> = {}): ScenarioSummary {
  return {
    id: "00000000-0000-0000-0000-000000000001",
    code: "S-1",
    attack: "Marie is obstructive and uncooperative",
    status: "ready",
    baseless_repeat_count: null,
    theme_statement: "Every act they call obstruction is a right the law gives an heir.",
    progress: {
      facts_included: 19,
      candidates_to_rule: 0,
      matrix_linked: { linked: 16, total: 142 },
      last_scan: null,
      deck: { questions: 42, built_on: "2026-09-15T15:00:00Z" },
      answered: {
        total: 42,
        of: 42,
        chuck_answered: 12,
        chuck_total: 12,
        defense_answered: 30,
        defense_total: 30,
      },
      marie_changed: 0,
      awaiting_review: 42,
      oldest_awaiting_review: "2026-09-15T15:00:00Z",
      ...overrides,
    },
  };
}

/** S-11 as the mockup draws it: scanned, a deck, nothing answered. */
export function s11(overrides: Partial<ScenarioProgress> = {}): ScenarioSummary {
  return {
    ...s1(),
    id: "00000000-0000-0000-0000-000000000011",
    code: "S-11",
    attack: "The $50,000",
    progress: {
      facts_included: 10,
      candidates_to_rule: 34,
      matrix_linked: { linked: 4, total: 99 },
      last_scan: { when: "2026-08-29T15:00:00Z" },
      deck: { questions: 12, built_on: "2026-08-30T15:00:00Z" },
      answered: {
        total: 0,
        of: 12,
        chuck_answered: 0,
        chuck_total: 7,
        defense_answered: 0,
        defense_total: 5,
      },
      marie_changed: 0,
      awaiting_review: 0,
      oldest_awaiting_review: null,
      ...overrides,
    },
  };
}
