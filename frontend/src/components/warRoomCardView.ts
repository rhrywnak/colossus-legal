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
// answers awaiting the reviewer, and questions new or changed for Marie
// are owed work. Everything else is ink.

import { fill } from "../services/caseTimeline";
import { pickByCount } from "../utils/countWording";
import { forYouPath } from "../utils/routePaths";
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
 * One pill. `review` and `marie` are amber (owed work); `not_started` is gray;
 * `up_to_date` is green. `review` is the reviewer's queue on this scenario — the
 * same for every viewer (CC_TASK_SIMPLE_COUNTS_v1).
 */
/**
 * One pill on a card.
 *
 * ## `href` — the count stops being a dead end (CC_TASK_FOR_YOU_v1 L2)
 *
 * The two OWED pills are counts of things waiting for a person, and until this
 * layer they were numbers with nowhere to go: a reader saw "2 notes for Marie"
 * and had to find the deck, open it, and read every question to find the two.
 * They now open that person's list, filtered to this deck — the same predicate
 * that produced the number, so the count and the rows cannot disagree.
 *
 * The two STATE pills ("Not started", "Up to date") carry no href: there is
 * nothing waiting to go and look at, and a link to an empty list is a promise
 * the page cannot keep.
 */
export type CardBadge =
  | { kind: "review"; text: string; href: string }
  | { kind: "marie"; text: string; href: string }
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
  /** The case, for the owed pills' links. */
  slug: string,
  /** May the reader review? Decides whether the review pill is drawn at all. */
  mayReview: boolean,
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
    badges: cardBadges(scenario, wording, slug, mayReview),
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
export function cardBadges(
  scenario: ScenarioSummary,
  wording: WarRoomWording,
  slug: string,
  /** May the reader review? The review pill is drawn only when they may. */
  mayReview: boolean,
): CardBadge[] {
  const p = scenario.progress;
  if (p.answered.total === 0) return [{ kind: "not_started", text: wording.card_not_started }];

  // Where an owed count goes when it is clicked: the reader's own list, this
  // deck only. Composed through the builder, so the route-side guard covers it.
  const href = forYouPath(slug, scenario.id);
  const owed: CardBadge[] = [];
  // ## Domain note: drawn only for somebody who may review (ruled 2026-09-22)
  //
  // Since L2 the number is the READER's own backlog, and a reader with no
  // review duty has none. Suppressed rather than zeroed: a "0 answers awaiting
  // your review" still claims the duty is theirs. `mayReview` is the server's
  // answer (`may_review`) — the page compares no username (rule 12).
  if (mayReview && p.awaiting_review > 0) {
    owed.push({
      kind: "review",
      href,
      text: fill(pickByCount(p.awaiting_review, wording.card_review_one, wording.card_review_template), {
        count: p.awaiting_review,
      }),
    });
  }
  if (p.marie_changed > 0) {
    owed.push({
      kind: "marie",
      href,
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
