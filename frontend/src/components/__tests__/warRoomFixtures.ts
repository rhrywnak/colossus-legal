// =============================================================================
// warRoomFixtures.ts — the status card's test payloads
// =============================================================================
//
// The wording block is the MIGRATION's values
// (20260916130121_war_room_status_card_wording.sql), so a test asserting a
// rendered string is asserting what DEV will actually print.

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
  card_scan_template: "Scan: {model} · {date} · {relevant} relevant of {total}",
  card_scan_never: "Scan: never run",
  card_talking_points_label: "Talking points",
  card_watch_items_label: "Watch items",
  card_deck_label: "Deck",
  card_deck_template: "{count} questions · {date}",
  card_deck_none: "—",
  card_answered_count_template: "{answered} of {total}",
  card_answered_split_template: "answered · Chuck {chuck_answered}/{chuck_total} · defense {defense_answered}/{defense_total}",
  card_changed_template: "{count} new or changed for Marie",
  card_up_to_date: "Up to date",
  card_open_action: "Open scenario",
  card_practice_action: "Practice",
  card_timeline_action: "Timeline",
  card_delete_action: "Delete",
};

/** A scenario with nothing yet: every count zero, never scanned, no deck. */
export function bareProgress(): ScenarioProgress {
  return {
    facts_included: 0,
    candidates_to_rule: 0,
    matrix_linked: { linked: 0, total: 0 },
    last_scan: null,
    talking_points: 0,
    watch_items: 0,
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
  };
}

/** S-13 as the mockup draws it (PROD numbers, 16 Sep 2026). */
export function s13(overrides: Partial<ScenarioProgress> = {}): ScenarioSummary {
  return {
    id: "00000000-0000-0000-0000-000000000013",
    code: "S-13",
    attack: "The SSA form — no living children",
    status: "ready",
    baseless_repeat_count: null,
    theme_statement:
      "A fiduciary who swore to the federal government that his ward's children did not exist.",
    progress: {
      facts_included: 10,
      candidates_to_rule: 33,
      matrix_linked: { linked: 4, total: 99 },
      last_scan: { model_name: "Qwen3.8", when: "2026-09-11T15:00:00Z", relevant: 43, total: 273 },
      talking_points: 5,
      watch_items: 5,
      deck: { questions: 17, built_on: "2026-09-14T15:00:00Z" },
      answered: {
        total: 0,
        of: 17,
        chuck_answered: 0,
        chuck_total: 7,
        defense_answered: 0,
        defense_total: 10,
      },
      marie_changed: 3,
      ...overrides,
    },
  };
}
