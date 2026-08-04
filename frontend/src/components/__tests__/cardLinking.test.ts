// =============================================================================
// cardLinking.test.ts — the link panel's rules (task 2.10)
// =============================================================================
//
// The draft, the refusals, and the filter. The per-card fence lives in
// `cardTriage.test.ts` beside 1.7G's, because that is where the reducer is.
//
// `nextStuckAfter` and `stillStuck` and their tests went with task 2.12 item A:
// saving no longer advances, so the helpers that chose where to advance TO have
// no caller. Cut rather than kept switched off — an unused export lies to the
// next reader about what this module does.

import { describe, expect, it } from "vitest";

import {
  canSave,
  EMPTY_DRAFT,
  needsLinking,
  refusalFor,
  toggleAllegation,
  visibleOptions,
} from "../cardLinking";
import type { AllegationOption, LinkPanelWording } from "../../services/evidenceLinks";
import type { ScenarioCard } from "../../services/scenarioCards";

/** The stored words, as the backend serves them. Nothing here is a default. */
const wording: LinkPanelWording = {
  intro: "This statement isn't linked to anything they've accused you of.",
  scope_notice: "An accusation link belongs to the statement, not to this scenario.",
  allegations_heading: "What it bears on",
  show_all_label: "Show all 120",
  filter_placeholder: "Type to filter",
  no_match_notice: "No accusation matches that.",
  empty_options_notice: "No accusations are stored for this case yet.",
  cut_heading: "How it cuts",
  cut_supports_label: "It supports us",
  cut_against_label: "They'll use it against us",
  save_label: "Save",
  cancel_label: "Cancel",
  unlink_label: "Unlink",
  missing_cut_refusal: "Say which way this cuts before saving.",
  missing_allegation_refusal: "Tick at least one accusation before saving.",
  question_revert_label: "Restore the system's wording",
  unlink_found_nothing: "That link was already gone — your view was out of date.",
  save_failed_template: "That link did not save: {detail}",
  save_blocks_ruling: "Save the link first.",
  fact_remove_label: "Remove",
  fact_remove_confirm_template: "Remove {code} from this scenario? It goes back to the queue as not ruled.",
  fact_remove_confirm_yes: "Remove it",
  fact_remove_confirm_cancel: "Keep it",
  fact_remove_failed_template: "That fact could not be removed: {detail}",
};

function option(id: string, label: string, count: string | null = null): AllegationOption {
  return {
    allegation_id: id,
    label,
    count_label: count,
    filter_text: `${label} ${count ?? ""}`.toLowerCase(),
  };
}

/** A card the extraction never linked — the stuck class. */
function stuckCard(id: string, overrides: Partial<ScenarioCard> = {}): ScenarioCard {
  return {
    code: id.toUpperCase(),
    graph_node_id: id,
    quote: {
      text: "I do not recall that meeting.",
      context_before: "",
      context_after: "",
      context_before_complete: true,
      context_after_complete: true,
      context_before_notice: null,
      context_after_notice: null,
      question: null,
    },
    pinpoint: {
      document_id: "doc-7",
      document_title: "CFS responses",
      label: "CFS responses at 14",
      page: 14,
      viewer_href: "/documents/doc-7?page=14&tab=document",
    },
    speaker: { name: null, attribution: "extracted" },
    statement_kind: null,
    stance: null,
    bears_on: [],
    grounding: null,
    confidence: { band: "unscored", label: "Not scored" },
    status: "undecided",
    status_label: "Not yet decided",
    defer_required: true,
    defer_required_reason: "This item is not linked to any accusation yet.",
    defer_reason: null,
    human_links: [],
    human_link_summary: null,
    ...overrides,
  };
}

describe("the draft", () => {
  it("ticks and unticks one accusation at a time", () => {
    const one = toggleAllegation(EMPTY_DRAFT, "a-41");
    expect(one.allegationIds).toEqual(["a-41"]);

    const two = toggleAllegation(one, "a-92");
    expect(two.allegationIds).toEqual(["a-41", "a-92"]);

    expect(toggleAllegation(two, "a-41").allegationIds).toEqual(["a-92"]);
  });

  it("keeps the order the human ticked in", () => {
    // That order reaches the card's sentence, so the draft is a list, not a set.
    let draft = toggleAllegation(EMPTY_DRAFT, "a-92");
    draft = toggleAllegation(draft, "a-41");
    expect(draft.allegationIds).toEqual(["a-92", "a-41"]);
  });
});

describe("what may be saved", () => {
  it("refuses an empty selection, in the STORED words", () => {
    // R4: the browser's pre-check and the backend's 400 use one sentence, so a
    // human cannot be told two different things about one mistake.
    expect(refusalFor(EMPTY_DRAFT, wording)).toBe(wording.missing_allegation_refusal);
    expect(canSave(EMPTY_DRAFT)).toBe(false);
  });

  it("REFUSES A LINK WITH NO CUT — the validation rule the design turns on", () => {
    // An unlabelled link would feed a readiness verdict that cannot tell
    // ammunition from hazard. This is the one rule the panel exists to enforce.
    const ticked = toggleAllegation(EMPTY_DRAFT, "a-41");
    expect(ticked.cut).toBeNull();
    expect(refusalFor(ticked, wording)).toBe(wording.missing_cut_refusal);
    expect(canSave(ticked)).toBe(false);
  });

  it("accepts a draft with both halves", () => {
    const ready = { ...toggleAllegation(EMPTY_DRAFT, "a-41"), cut: "against" as const };
    expect(refusalFor(ready, wording)).toBeNull();
    expect(canSave(ready)).toBe(true);
  });

  it("names the step the human is ON when both halves are missing", () => {
    // Told to tick something first, not to choose a cut for nothing.
    expect(refusalFor(EMPTY_DRAFT, wording)).toBe(wording.missing_allegation_refusal);
  });
});

describe("which accusations are offered", () => {
  const serving = [option("a-41", "¶41 — refused to divide the property")];
  const others = [
    option("a-92", "¶92 — failed to account for estate funds"),
    option("a-47", "¶47 — obstructed the sale", "Count 2 — Breach"),
  ];

  it("shows the short list until Show all is pressed", () => {
    expect(visibleOptions(EMPTY_DRAFT, serving, others)).toEqual(serving);
    expect(visibleOptions({ ...EMPTY_DRAFT, showAll: true }, serving, others)).toHaveLength(3);
  });

  it("filters on the backend's haystack, and matches the count too", () => {
    const draft = { ...EMPTY_DRAFT, showAll: true, filter: "breach" };
    const shown = visibleOptions(draft, serving, others);
    expect(shown.map((o) => o.allegation_id)).toEqual(["a-47"]);
  });

  it("ignores case and surrounding space in what was typed", () => {
    const draft = { ...EMPTY_DRAFT, showAll: true, filter: "  ESTATE  " };
    expect(visibleOptions(draft, serving, others).map((o) => o.allegation_id)).toEqual(["a-92"]);
  });

  it("never filters away something already ticked", () => {
    // Otherwise a human saves a link to an accusation they can no longer see, and
    // then wonders where the extra chip came from.
    const draft = {
      ...EMPTY_DRAFT,
      showAll: true,
      filter: "estate",
      allegationIds: ["a-41"],
    };
    const shown = visibleOptions(draft, serving, others).map((o) => o.allegation_id);
    expect(shown).toContain("a-41");
    expect(shown).toContain("a-92");
  });

  it("returns nothing when the filter matches nothing — a state with its own sentence", () => {
    const draft = { ...EMPTY_DRAFT, showAll: true, filter: "zzz" };
    expect(visibleOptions(draft, serving, others)).toEqual([]);
  });
});

describe("which cards need the control", () => {
  it("a card the extraction never linked needs it", () => {
    expect(needsLinking(stuckCard("ev-1"))).toBe(true);
  });

  it("a card the machine DID link does not", () => {
    const linked = stuckCard("ev-2", {
      stance: { verb: "supports", object: "¶41", summary: "This supports ¶41" },
      defer_required: false,
      defer_required_reason: null,
    });
    expect(needsLinking(linked)).toBe(false);
  });

  it("a card with no QUOTE does not — linking cannot give a statement words", () => {
    // The two unrulable classes are independent. Offering the panel here would be
    // a control that cannot do anything, beside a refusal saying so.
    const quoteless = stuckCard("ev-3", {
      quote: { ...stuckCard("ev-3").quote, text: "   " },
    });
    expect(needsLinking(quoteless)).toBe(false);
  });

  it("a card a human has already linked is no longer stuck", () => {
    const done = stuckCard("ev-4", {
      human_links: [
        { allegation_id: "a-41", label: "¶41 — …", cut: "against", cut_label: "Against" },
      ],
      defer_required: false,
      defer_required_reason: null,
    });
    // Still a card the extraction never linked — but no longer waiting for a
    // human, which is what stops the panel being re-offered under the answer.
    expect(needsLinking(done)).toBe(true);
    expect(done.human_links.length).toBe(1);
  });
});
