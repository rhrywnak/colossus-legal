// =============================================================================
// trialPrepData.ts — Trial Prep ("War Room") payload CONTRACT (types only)
// -----------------------------------------------------------------------------
// The eventual backend-payload shape. Every field is present even when empty;
// optional display fields are `T | null` (present-as-null, not omitted) — the
// same convention the Proof Review service uses, so Stage 2 fills this identical
// structure without changing a component. The Stage-1 placeholder *values* live
// in `trialPrepPlaceholder.ts`; view-shaping lives in `trialPrepHelpers.ts`.
// =============================================================================

// ─── Contract types ──────────────────────────────────────────────────────────

/** Scenario lifecycle. Drives the status dot and labels. */
export type ScenarioStatus = "draft" | "needs_evidence" | "ready";

/** One dashboard scenario card. */
export interface ScenarioSummary {
  id: string;
  attack: string;
  status: ScenarioStatus;
  instance_count: number;
  response_count: number;
  speakers: string[];
  /** null = pattern analysis pending; 0 = analysed, no baseless repeat. */
  baseless_repeat_count: number | null;
}

/** The dashboard payload: metrics band + alerts strip + scenario cards. */
export interface TrialPrepDashboard {
  metrics: {
    scenarios: number;
    ready: number;
    drafted_or_review: number;
    instances: number;
    /** The Count IV signal — accusations repeated after a proven rebuttal. */
    baseless_repeat_patterns: number;
    no_response_yet: number;
  };
  /** Living-binder notices ("N new instances …"). Empty array = no alerts. */
  alerts: { message: string }[];
  scenarios: ScenarioSummary[];
}

/** A turn in a scenario's exchange timeline. */
export type ExchangeTurnKind =
  | "accusation"
  | "accusation_repeat"
  | "rebuttal"
  | "defense_counter";

export interface ExchangeTurn {
  kind: ExchangeTurnKind;
  /** true = from the record (has a citation); false = anticipated (no cite). */
  grounded: boolean;
  speaker: string | null;
  /** ISO date; used for chronological ordering. null sorts last (projected). */
  date: string | null;
  text: string;
  /** 'characterizes' | 'rebuts' | 'contradicts' | null. */
  relationship_type: string | null;
  // Citation — present on grounded turns, null on anticipated ones.
  source_document: string | null;
  page_number: number | null;
  paragraph: string | null;
  /** true on accusation_repeat turns that postdate a proven rebuttal. */
  repeated_after_rebuttal: boolean;
}

/** Provenance of a rehearsable response. */
export type ResponseProvenance = "system_draft" | "marie";

export interface MarieResponse {
  id: string;
  /** 'primary' or a targeted-angle label. */
  label: string;
  text: string;
  authored_by: ResponseProvenance;
}

/** The full scenario exchange shown on the detail page. */
export interface ScenarioDetail {
  id: string;
  attack: string;
  status: ScenarioStatus;
  /** e.g. "repeated 3× after rebuttal"; null when no pattern. */
  pattern_summary: string | null;
  timeline: ExchangeTurn[];
  responses: MarieResponse[];
  notes: string | null;
}
