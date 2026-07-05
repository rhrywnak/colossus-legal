// =============================================================================
// scenarioFormLabels.ts — plain-language UI copy for the scenario define form.
// =============================================================================
//
// Every visible label, help line, and placeholder the define form shows comes
// from HERE, not from inline JSX strings. Two reasons:
//
//  1. Zero schema jargon reaches the user. The form's underlying fields are
//     `attack_text`, `wielders`, `actor_role`, `target` — none of which a
//     non-technical author (Marie, Roman) should ever see. This module is the one
//     place those map to plain questions.
//
//  2. Reusability (Standing Rule 2). The copy here is DELIBERATELY generic — no
//     case names, no party names, no Awad-specific words. Another Michigan civil
//     case reuses this file unchanged; the case-specific data (the actual party
//     names in the pickers, the allegation text) comes from the graph/DB, never
//     from code. If a future case wants different wording, this stays the single
//     edit point. (A per-case, no-rebuild label config served by the backend is a
//     deliberate week-2+ want — Chuck's vocabulary review — and is NOT built here.)
//
// Guidance note (Amendment 2): the attack-text guidance steers the author toward
// ONE specific accusation per scenario, because a broad theme is a pattern, not a
// scenario, and mis-authoring that is the failure this rebuild exists to prevent.

import type { ActorRole } from "../pages/trialPrepData";

/** One selectable actor-role option: the wire code + its plain-language display. */
export interface ActorRoleOption {
  /** The wire token — MUST be a member of the backend `ActorRole` vocabulary. */
  code: ActorRole;
  /** What the user reads in the role dropdown. */
  label: string;
}

/**
 * The role choices offered per wielder, in display order. The `code`s are the
 * single frontend list of valid roles — the schema-v2 `ActorRole` union — and the
 * backend enum re-validates them on save, so a drift here fails loudly at the PUT
 * rather than silently storing a junk role.
 */
export const ACTOR_ROLE_OPTIONS: ActorRoleOption[] = [
  { code: "originated", label: "originated it" },
  { code: "repeated", label: "repeated it" },
  { code: "adopted", label: "adopted it" },
];

/** The default role a freshly-added wielder row starts on. */
export const DEFAULT_ACTOR_ROLE: ActorRole = "originated";

/** The full label set the define form renders. */
export interface ScenarioFormLabels {
  header: string;
  attackText: { label: string; placeholder: string; guidance: string };
  attackMeaning: { label: string; placeholder: string };
  target: { label: string; placeholder: string; empty: string };
  wielders: {
    label: string;
    help: string;
    partyPlaceholder: string;
    addButton: string;
    removeButton: string;
    empty: string;
  };
  anchorAllegations: { label: string; help: string; empty: string };
  save: string;
  saving: string;
  vocabError: string;
}

/**
 * The default (generic) label set. Wording follows the D1 instruction's plain-
 * language defaults; the one case-specific token in the source instruction ("they
 * attack Marie") is reworded generically here — the client's real name surfaces
 * through the `target` picker's data, not this copy.
 */
export const SCENARIO_FORM_LABELS: ScenarioFormLabels = {
  header: "Define this scenario",
  attackText: {
    label: "What is the specific accusation? Quote it if you can.",
    placeholder: "e.g. “She refused to divide the property amicably.”",
    guidance:
      "One scenario = one specific accusation. Broad themes (e.g. “the other side is obstructive”) are patterns, not scenarios.",
  },
  attackMeaning: {
    label:
      "What are they really saying? Describe the meaning in your own words — the system uses this to find matching evidence.",
    placeholder: "Plain-language description of what the accusation actually asserts",
  },
  target: {
    label: "Who is the accusation about?",
    placeholder: "Choose a party…",
    empty: "No parties available yet.",
  },
  wielders: {
    label: "Who makes or repeats this accusation?",
    help: "Add each party and how they are involved.",
    partyPlaceholder: "Choose a party…",
    addButton: "+ Add a party",
    removeButton: "Remove",
    empty: "No parties added yet.",
  },
  anchorAllegations: {
    label: "Which complaint paragraphs does this touch? (optional)",
    help: "Tick any complaint paragraphs this scenario relates to.",
    empty: "No complaint paragraphs available.",
  },
  save: "Save definition",
  saving: "Saving…",
  vocabError:
    "Couldn't load the party / allegation lists — close and reopen this page to retry. You can still save the accusation text.",
};
