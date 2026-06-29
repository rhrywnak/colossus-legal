// =============================================================================
// trialPrepPlaceholder.ts — Stage 1 placeholder data for the Trial Prep pages
// -----------------------------------------------------------------------------
// UI-first (Charter §8): this module holds illustrative data shaped EXACTLY like
// the eventual backend payload (the contract types live in `trialPrepData.ts`).
// Stage 2 replaces these constants with a backend fetch returning the identical
// shape — no component changes. The scenarios are the approved mockup attacks
// (Too many attorneys · $50,000 · Marie is obstructive · Selective sanctions ·
// Bias — who gained). Document ids are placeholders; Stage 2 supplies real ones.
// =============================================================================

import type {
  ScenarioDetail,
  TrialPrepDashboard,
} from "./trialPrepData";

const DOC_PHILLIPS_DISCOVERY = "doc-george-phillips-response-to-discovery";
const DOC_CFS_BRIEF = "doc-cfs-coa-brief";
const DOC_COMPLAINT = "doc-awad-complaint";

const DASHBOARD: TrialPrepDashboard = {
  metrics: {
    scenarios: 5,
    ready: 1,
    drafted_or_review: 3,
    instances: 16,
    baseless_repeat_patterns: 1,
    no_response_yet: 1,
  },
  alerts: [
    { message: "6 new instances of “Marie is obstructive” since last review" },
    { message: "Pattern analysis pending for “Selective sanctions”" },
  ],
  scenarios: [
    {
      id: "too-many-attorneys",
      attack: "Marie hired too many attorneys",
      status: "draft",
      instance_count: 4,
      response_count: 2,
      speakers: ["George Phillips", "CFS"],
      baseless_repeat_count: 0,
    },
    {
      id: "fifty-thousand",
      attack: "The $50,000 was a gift",
      status: "ready",
      instance_count: 3,
      response_count: 2,
      speakers: ["George Phillips"],
      baseless_repeat_count: 0,
    },
    {
      id: "marie-obstructive",
      attack: "Marie is obstructive and uncooperative",
      status: "draft",
      instance_count: 6,
      response_count: 1,
      speakers: ["CFS", "George Phillips"],
      baseless_repeat_count: 3,
    },
    {
      id: "selective-sanctions",
      attack: "Sanctions were never selectively pursued",
      status: "draft",
      instance_count: 2,
      response_count: 1,
      speakers: ["CFS"],
      baseless_repeat_count: null,
    },
    {
      id: "bias-who-gained",
      attack: "Bias — who gained from the decisions?",
      status: "needs_evidence",
      instance_count: 1,
      response_count: 0,
      speakers: ["George Phillips"],
      baseless_repeat_count: 0,
    },
  ],
};

const SCENARIO_DETAILS: Record<string, ScenarioDetail> = {
  "too-many-attorneys": {
    id: "too-many-attorneys",
    attack: "Marie hired too many attorneys",
    status: "draft",
    pattern_summary: null,
    timeline: [
      {
        kind: "accusation",
        grounded: true,
        speaker: "George Phillips",
        date: "2025-03-12",
        text: "Ms. Awad retained no fewer than four separate attorneys, driving up cost.",
        relationship_type: "characterizes",
        source_document: DOC_CFS_BRIEF,
        page_number: 8,
        paragraph: "¶22",
        repeated_after_rebuttal: false,
      },
      {
        kind: "rebuttal",
        grounded: true,
        speaker: "Marie Awad",
        date: "2025-04-02",
        text: "Each counsel was retained sequentially after prior counsel withdrew for cause.",
        relationship_type: "rebuts",
        source_document: DOC_PHILLIPS_DISCOVERY,
        page_number: 14,
        paragraph: "Q31",
        repeated_after_rebuttal: false,
      },
    ],
    responses: [
      {
        id: "tma-primary",
        label: "primary",
        text: "I changed counsel only when a prior attorney withdrew — each change is documented in the court record.",
        authored_by: "system_draft",
      },
      {
        id: "tma-cost",
        label: "if pressed on cost",
        text: "The cost of changing counsel was a direct result of the conduct that forced each withdrawal.",
        authored_by: "system_draft",
      },
    ],
    notes: null,
  },
  "fifty-thousand": {
    id: "fifty-thousand",
    attack: "The $50,000 was a gift",
    status: "ready",
    pattern_summary: null,
    timeline: [
      {
        kind: "accusation",
        grounded: true,
        speaker: "George Phillips",
        date: "2025-02-20",
        text: "The $50,000 transferred to Ms. Awad was an unconditional gift.",
        relationship_type: "characterizes",
        source_document: DOC_CFS_BRIEF,
        page_number: 5,
        paragraph: "¶11",
        repeated_after_rebuttal: false,
      },
      {
        kind: "rebuttal",
        grounded: true,
        speaker: "George Phillips",
        date: "2025-04-02",
        text: "Admits under oath that no document records the transfer as a gift.",
        relationship_type: "contradicts",
        source_document: DOC_PHILLIPS_DISCOVERY,
        page_number: 19,
        paragraph: "Q73",
        repeated_after_rebuttal: false,
      },
    ],
    responses: [
      {
        id: "fk-primary",
        label: "primary",
        text: "There is no gift letter, no tax filing, and Mr. Phillips conceded under oath that nothing in writing calls it a gift.",
        authored_by: "marie",
      },
      {
        id: "fk-repayment",
        label: "if asked about repayment",
        text: "The repayment schedule we agreed to is reflected in the contemporaneous emails already in the record.",
        authored_by: "system_draft",
      },
    ],
    notes: "Strongest scenario — the sworn discovery answer directly contradicts the brief.",
  },
  "marie-obstructive": {
    id: "marie-obstructive",
    attack: "Marie is obstructive and uncooperative",
    status: "draft",
    pattern_summary: "repeated 3× after rebuttal",
    timeline: [
      {
        kind: "accusation",
        grounded: true,
        speaker: "CFS",
        date: "2025-01-15",
        text: "Ms. Awad has been uniformly uncooperative with every reasonable request.",
        relationship_type: "characterizes",
        source_document: DOC_COMPLAINT,
        page_number: 3,
        paragraph: "¶9",
        repeated_after_rebuttal: false,
      },
      {
        kind: "rebuttal",
        grounded: true,
        speaker: "Marie Awad",
        date: "2025-04-02",
        text: "Produced every requested document on schedule; the log shows zero missed deadlines.",
        relationship_type: "rebuts",
        source_document: DOC_PHILLIPS_DISCOVERY,
        page_number: 22,
        paragraph: "Q48",
        repeated_after_rebuttal: false,
      },
      {
        kind: "accusation_repeat",
        grounded: true,
        speaker: "CFS",
        date: "2025-05-19",
        text: "Repeats the obstruction characterization after the production log was produced.",
        relationship_type: "characterizes",
        source_document: DOC_CFS_BRIEF,
        page_number: 11,
        paragraph: "¶34",
        repeated_after_rebuttal: true,
      },
      {
        kind: "defense_counter",
        grounded: false,
        speaker: "CFS",
        date: null,
        text: "Anticipated: CFS will argue the production was “selective” rather than complete — prepare the full index as a counter.",
        relationship_type: null,
        source_document: null,
        page_number: null,
        paragraph: null,
        repeated_after_rebuttal: false,
      },
    ],
    responses: [
      {
        id: "mo-primary",
        label: "primary",
        text: "The production log is in the record — every document was delivered on time. The accusation was repeated after that log was produced.",
        authored_by: "system_draft",
      },
    ],
    notes: "Count IV signal: the characterization is repeated after it was rebutted on the record.",
  },
  "selective-sanctions": {
    id: "selective-sanctions",
    attack: "Sanctions were never selectively pursued",
    status: "draft",
    pattern_summary: null,
    timeline: [
      {
        kind: "accusation",
        grounded: true,
        speaker: "CFS",
        date: "2025-03-01",
        text: "To our knowledge, sanctions were never pursued with regard to Nadia Awad.",
        relationship_type: null,
        source_document: DOC_PHILLIPS_DISCOVERY,
        page_number: 9,
        paragraph: "Q11",
        repeated_after_rebuttal: false,
      },
      {
        kind: "defense_counter",
        grounded: false,
        speaker: null,
        date: null,
        text: "Anticipated rebuttal: cross-reference the board minutes that show sanctions raised against others but not Nadia.",
        relationship_type: null,
        source_document: null,
        page_number: null,
        paragraph: null,
        repeated_after_rebuttal: false,
      },
    ],
    responses: [
      {
        id: "ss-primary",
        label: "primary",
        text: "The board minutes show sanctions were raised against other members in identical circumstances — pattern analysis is still being assembled.",
        authored_by: "system_draft",
      },
    ],
    notes: "Pattern analysis pending — baseless_repeat_count is null until the cross-document pass runs.",
  },
  "bias-who-gained": {
    id: "bias-who-gained",
    attack: "Bias — who gained from the decisions?",
    status: "needs_evidence",
    pattern_summary: null,
    timeline: [
      {
        kind: "accusation",
        grounded: true,
        speaker: "George Phillips",
        date: "2025-02-28",
        text: "Every contested decision was made in the organization's neutral interest.",
        relationship_type: "characterizes",
        source_document: DOC_CFS_BRIEF,
        page_number: 6,
        paragraph: "¶16",
        repeated_after_rebuttal: false,
      },
    ],
    responses: [],
    notes: "No response drafted yet — awaiting the financial-beneficiary analysis.",
  },
};

/** Return the placeholder dashboard payload (Stage 2: a backend fetch). */
export function getTrialPrepDashboard(): TrialPrepDashboard {
  return DASHBOARD;
}

/** Return the placeholder scenario detail for `id`, or null if unknown. */
export function getScenarioDetail(id: string): ScenarioDetail | null {
  return SCENARIO_DETAILS[id] ?? null;
}
