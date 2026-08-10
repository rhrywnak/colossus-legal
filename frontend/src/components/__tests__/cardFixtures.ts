/**
 * Shared fixtures for the one-card tests.
 *
 * ## Why the fixtures live in one file
 *
 * Three test files now build a `ScenarioCard` and an `AllegationOptions`, and the
 * whole claim under test is that TWO WRAPPERS RENDER ONE PAYLOAD. Three
 * hand-typed payloads would let the candidate's fixture and the fact's fixture
 * drift apart — and a "same fields" test whose two sides were built from
 * different literals would pass while proving nothing.
 *
 * Nothing here is production code and nothing here is a default: `evidenceLinks`
 * deliberately has no fallback wording anywhere in the frontend, and these values
 * exist only so a test can render something.
 */

import type { AllegationOptions } from "../../services/evidenceLinks";
import type { CardBearsOn, ScenarioCard } from "../../services/scenarioCards";

/** What a caller may vary about the card under test. */
export type CardOverrides = {
  code?: string | null;
  graphNodeId?: string;
  quote?: string;
  question?: string | null;
  speaker?: string | null;
  statementKind?: string | null;
  bearsOn?: CardBearsOn[];
  scanReason?: string;
  status?: ScenarioCard["status"];
  contextBefore?: string;
  contextAfter?: string;
};

/** One complete card, with everything a §7 payload can carry. */
export function cardFixture(over: CardOverrides = {}): ScenarioCard {
  const question = over.question === undefined ? null : over.question;
  return {
    code: over.code === undefined ? "C-1" : over.code,
    graph_node_id: over.graphNodeId ?? "ev-1",
    quote: {
      text: over.quote ?? "Yes.",
      context_before: over.contextBefore ?? "",
      context_after: over.contextAfter ?? "",
      context_before_complete: true,
      context_after_complete: true,
      context_before_notice: null,
      context_after_notice: null,
      question,
      question_authorship: question ? { source: "system", label: "System" } : undefined,
    },
    pinpoint: {
      document_id: "doc-7",
      document_title: "CFS responses",
      label: "CFS responses at 26",
      page: 26,
      viewer_href: "/documents/doc-7?page=26",
    },
    speaker: {
      name: over.speaker === undefined ? "George Phillips" : over.speaker,
      attribution: "extracted",
    },
    statement_kind:
      over.statementKind === undefined ? "sworn discovery answer" : over.statementKind,
    stance: null,
    bears_on: over.bearsOn ?? [],
    grounding: { state: "exact", label: "Grounded — found on the page" },
    confidence: { band: "high", label: "Model confidence: high" },
    status: over.status ?? "undecided",
    status_label: "Not yet decided",
    defer_required: false,
    defer_required_reason: null,
    defer_reason: null,
    human_links: [],
    human_link_summary: null,
    ...(over.scanReason ? { scan_reason: over.scanReason } : {}),
  };
}

/**
 * The stored words, as the store seeds them.
 *
 * Values match the migration's seed so a test reads the way the product behaves.
 * Only the keys the card and the fact wrapper actually render are filled; the
 * rest of `LinkPanelWording` is irrelevant here and is cast past rather than
 * invented, so nothing in this file can be mistaken for a fallback.
 */
export function optionsFixture(): AllegationOptions {
  return {
    wording: {
      fact_tier_carries_label: "Carries the scenario",
      fact_tier_backup_label: "Backup",
      fact_tier_background_label: "Background",
      fact_tier_prompt: "How much does this fact carry?",
      fact_order_drag_hint: "Drag to reorder",
      fact_remove_label: "Remove",
      fact_remove_confirm_template: "Remove {code}?",
      fact_remove_confirm_yes: "Remove it",
      fact_remove_confirm_cancel: "Keep it",
      cut_heading: "Which way does it cut?",
      cut_supports_label: "It supports us",
      cut_against_label: "They'll use it against us",
      missing_allegation_refusal: "Pick an accusation first.",
      save_failed_template: "That could not be saved: {detail}",
      save_blocks_ruling: "Save your link choices first.",
      card_locked_condition_label: "Include and Exclude are closed on this card:",
    } as unknown as AllegationOptions["wording"],
    card_grammar: {
      filter_proposed_label: "Proposed",
      filter_deferred_label: "Deferred",
      filter_included_label: "Included",
      filter_excluded_label: "Excluded",
      filter_full_pool_label: "Full pool",
      full_pool_explainer:
        "Everything the system ever gathered for this scenario, across all scans.",
      filter_progress_template: "{ruled} of {total} addressed",
      question_expand_label: "show full question",
      question_collapse_label: "hide full question",
      question_machine_authorship_label: "Question as transcribed from the document",
      speaker_extracted_label: "extracted",
      speaker_absent_label: "speaker not extracted",
      elements_more_template: "+{count} more",
      elements_fewer_label: "show fewer",
      context_show_label: "Show context",
      context_hide_label: "Hide context",
      scan_reason_label: "Scan:",
      link_typeahead_placeholder: "Type A-41, or a word from any allegation…",
      link_typeahead_intro: "This statement is not linked to anything.",
      link_typeahead_no_match: "No allegation matches what you typed.",
      link_woke_ruling_template: "Linked. {code} can be ruled now.",
      weight_picker_label: "Weight",
      weight_changed_template: "Weight set: {code} now reads {tier}.",
      weight_undo_label: "undo",
      reset_order_label: "Reset order",
      reset_order_confirm: "Forget where you have placed every fact?",
      reset_order_confirm_yes: "Reset the order",
      reset_order_confirm_cancel: "Keep my order",
      reset_order_done_template: "Order reset — {count} facts returned.",
      reset_order_failed_template: "The order could not be reset: {reason}",
      chip_filter_hint_template: "Show only: {value}",
      chip_filter_clear_template: "Showing only {value} — show everything",
    },
    serving: [],
    others: [],
    total: 0,
    card_question_truncate_chars: 110,
    card_element_chips_visible_k: 2,
  };
}
