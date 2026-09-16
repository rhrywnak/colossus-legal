// =============================================================================
// warRoomCardView.ts — one scenario's status card, as strings and flags
// =============================================================================
//
// CC_TASK_WAR_ROOM_v1. The card component renders what this returns and decides
// nothing: every sentence is a stored template from `WarRoomWording` filled with
// the payload's numbers, and every colour decision is a boolean computed here.
// That split is what lets the tests assert "never scanned renders the never
// line in the warning colour" without a DOM (CLAUDE.md rule 30).
//
// ## The card's one colour rule (CC_GO_WAR_ROOM_v3)
//
// Amber = something is owed by someone; green = nothing is owed. Candidates to
// rule above zero, a scan that never ran, and questions new or changed for
// Marie are owed work. Everything else is ink.

import { fill } from "../services/caseTimeline";
import type { ScenarioSummary, WarRoomWording } from "../pages/trialPrepData";

/** One label/value row in a pane. */
export interface CardRow {
  label: string;
  value: string;
  /** True when the value is owed work and renders in the warning colour. */
  warning: boolean;
}

/** The answered line and the bar under it. */
export interface AnsweredLine {
  /** The bold half — "0 of 17". */
  count: string;
  /** The rest — "answered · Chuck 0/7 · defense 0/10". */
  split: string;
  /** 0‥1, the bar's fill. */
  fraction: number;
}

/** The badge row: exactly one pill, always. */
export type CardBadge =
  | { kind: "changed"; text: string }
  | { kind: "up_to_date"; text: string };

/** Everything the card renders. */
export interface WarRoomCardView {
  code: string;
  title: string;
  /** null → the theme line is not rendered at all (ruling Q3). */
  theme: string | null;
  evidenceHeading: string;
  evidenceRows: CardRow[];
  scanLine: string;
  scanWarning: boolean;
  prepHeading: string;
  prepRows: CardRow[];
  /** null when the deck has no visible questions — "0 of 0" says nothing. */
  answered: AnsweredLine | null;
  badge: CardBadge;
  actions: {
    open: string;
    practice: string;
    /** null → no Timeline link (the scenario carries no subset). */
    timeline: string | null;
    delete: string;
  };
}

/**
 * A day as the reader's locale says it — "Sep 14".
 *
 * The sentence is the store's; the date format is the reader's, the same split
 * the scan header makes. An unparseable timestamp yields the raw string rather
 * than "Invalid Date", so a bad value is visible rather than disguised.
 */
export function formatCardDay(iso: string): string {
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return iso;
  return at.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

/**
 * Shape one scenario into its card.
 *
 * @param scenario   the dashboard payload's card, with `progress`
 * @param wording    the page's stored words
 * @param hasTimeline whether a timeline subset carries this scenario
 */
export function warRoomCardView(
  scenario: ScenarioSummary,
  wording: WarRoomWording,
  hasTimeline: boolean,
): WarRoomCardView {
  const p = scenario.progress;
  const theme = scenario.theme_statement?.trim() ? scenario.theme_statement.trim() : null;

  return {
    code: scenario.code,
    title: scenario.attack,
    theme,
    evidenceHeading: wording.card_evidence_heading,
    evidenceRows: [
      row(wording.card_facts_included_label, p.facts_included),
      {
        label: wording.card_candidates_label,
        value: String(p.candidates_to_rule),
        warning: p.candidates_to_rule > 0,
      },
      {
        label: wording.card_matrix_linked_label,
        // Nothing stuck is an em dash, never "0 of 0" (ruling Q4).
        value:
          p.matrix_linked.total === 0
            ? wording.card_matrix_linked_none
            : fill(wording.card_matrix_linked_template, p.matrix_linked),
        warning: false,
      },
    ],
    scanLine:
      p.last_scan === null
        ? wording.card_scan_never
        : fill(wording.card_scan_template, {
            model: p.last_scan.model_name,
            date: formatCardDay(p.last_scan.when),
            relevant: p.last_scan.relevant,
            total: p.last_scan.total,
          }),
    scanWarning: p.last_scan === null,
    prepHeading: wording.card_prep_heading,
    prepRows: [
      row(wording.card_talking_points_label, p.talking_points),
      row(wording.card_watch_items_label, p.watch_items),
      {
        label: wording.card_deck_label,
        value:
          p.deck.questions === 0 || p.deck.built_on === null
            ? wording.card_deck_none
            : fill(wording.card_deck_template, {
                count: p.deck.questions,
                date: formatCardDay(p.deck.built_on),
              }),
        warning: false,
      },
    ],
    answered: answeredLine(scenario, wording),
    badge:
      p.marie_changed > 0
        ? { kind: "changed", text: fill(wording.card_changed_template, { count: p.marie_changed }) }
        : { kind: "up_to_date", text: wording.card_up_to_date },
    actions: {
      open: wording.card_open_action,
      practice: wording.card_practice_action,
      timeline: hasTimeline ? wording.card_timeline_action : null,
      delete: wording.card_delete_action,
    },
  };
}

/** A plain count row — never owed work. */
function row(label: string, count: number): CardRow {
  return { label, value: String(count), warning: false };
}

/** The answered line, or null for a deck with no visible questions. */
function answeredLine(scenario: ScenarioSummary, wording: WarRoomWording): AnsweredLine | null {
  const a = scenario.progress.answered;
  if (a.of === 0) return null;
  return {
    count: fill(wording.card_answered_count_template, { answered: a.total, total: a.of }),
    split: fill(wording.card_answered_split_template, a),
    // Clamped: a count above its denominator would be a backend defect, and a
    // bar drawn past its track would hide that behind a layout glitch.
    fraction: Math.min(1, Math.max(0, a.total / a.of)),
  };
}
