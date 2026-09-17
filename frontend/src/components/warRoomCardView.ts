// =============================================================================
// warRoomCardView.ts — one scenario's status card, as strings and flags
// =============================================================================
//
// CC_TASK_WAR_ROOM_v1, reshaped by CC_TASK_REVIEW_LOOP_v1 to the ruled mockup v3.
// The card component renders what this returns and decides nothing: every
// sentence is a stored template from `WarRoomWording` filled with the payload's
// numbers, and every colour decision is a value computed here. That split is what
// lets the tests assert "a deck nobody answered says Not started" without a DOM
// (CLAUDE.md rule 30).
//
// ## The card's one colour rule
//
// Amber = something is owed by someone; gray = nothing has started; green =
// started and nothing owed. Candidates to rule above zero, a scan that never ran,
// answers the viewer has not reviewed, and questions new or changed for Marie
// are owed work. Everything else is ink.

import { fill } from "../services/caseTimeline";
import { pickByCount } from "../utils/countWording";
import type { ScenarioSummary, WarRoomWording } from "../pages/trialPrepData";

/** One label/value row in a pane. */
export interface CardRow {
  label: string;
  value: string;
  /** True when the value is owed work and renders in the warning colour. */
  warning: boolean;
}

/** The prep pane's headline, its muted line, and the bar under them. */
export interface AnsweredLine {
  /** The bold half — "42 of 42". */
  count: string;
  /** The regular word after it — "answered". */
  word: string;
  /** ONE muted line — "Chuck 12/12 · defense 30/30 · deck 42 q · Sep 15". */
  meta: string;
  /** 0‥1, the bar's fill. */
  fraction: number;
}

/**
 * One pill. `viewer` and `marie` are amber (owed work); `not_started` is gray;
 * `up_to_date` is green.
 */
export type CardBadge =
  | { kind: "viewer"; text: string }
  | { kind: "marie"; text: string }
  | { kind: "not_started"; text: string }
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
  /** null when the deck has no visible questions — the headline is `deckNone`. */
  answered: AnsweredLine | null;
  /** The headline for a deck with no visible questions. */
  deckNone: string;
  /** Never empty: the amber pills that apply, or exactly one gray/green pill. */
  badges: CardBadge[];
  actions: {
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
      { label: wording.card_facts_included_label, value: String(p.facts_included), warning: false },
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
    // Date only (task §5): the model and relevant counts live on the scenario page.
    scanLine:
      p.last_scan === null
        ? wording.card_scan_never
        : fill(wording.card_scan_template, { date: formatCardDay(p.last_scan.when) }),
    scanWarning: p.last_scan === null,
    prepHeading: wording.card_prep_heading,
    answered: answeredLine(scenario, wording),
    deckNone: wording.card_deck_none,
    badges: cardBadges(scenario, wording),
    actions: {
      practice: wording.card_practice_action,
      timeline: hasTimeline ? wording.card_timeline_action : null,
      delete: wording.card_delete_action,
    },
  };
}

/**
 * The pills, in the order the mockup stacks them.
 *
 * ## Why "Not started" is decided first
 *
 * "Up to date" on a deck nobody has answered claimed a state the deck never
 * reached — the defect this replaces. With nothing answered, neither amber count
 * can be above zero (both are counts OF answers), so gray is the whole story.
 * Green needs BOTH: started, and nothing pending for either badge.
 */
export function cardBadges(scenario: ScenarioSummary, wording: WarRoomWording): CardBadge[] {
  const p = scenario.progress;
  if (p.answered.total === 0) return [{ kind: "not_started", text: wording.card_not_started }];

  const owed: CardBadge[] = [];
  if (p.new_answers_for_viewer > 0) {
    owed.push({
      kind: "viewer",
      text: fill(
        pickByCount(p.new_answers_for_viewer, wording.card_viewer_new_one, wording.card_viewer_new_template),
        { count: p.new_answers_for_viewer },
      ),
    });
  }
  if (p.marie_changed > 0) {
    owed.push({
      kind: "marie",
      text: fill(
        pickByCount(p.marie_changed, wording.card_changed_one, wording.card_changed_template),
        { count: p.marie_changed },
      ),
    });
  }
  return owed.length > 0 ? owed : [{ kind: "up_to_date", text: wording.card_up_to_date }];
}

/** The prep headline and its one muted line, or null for a deck with no questions. */
function answeredLine(scenario: ScenarioSummary, wording: WarRoomWording): AnsweredLine | null {
  const { answered: a, deck } = scenario.progress;
  if (a.of === 0 || deck.built_on === null) return null;
  return {
    count: fill(wording.card_answered_count_template, { answered: a.total, total: a.of }),
    word: wording.card_answered_word,
    meta: fill(wording.card_prep_meta_template, {
      ...a,
      count: deck.questions,
      date: formatCardDay(deck.built_on),
    }),
    // Clamped: a count above its denominator would be a backend defect, and a
    // bar drawn past its track would hide that behind a layout glitch.
    fraction: Math.min(1, Math.max(0, a.total / a.of)),
  };
}
