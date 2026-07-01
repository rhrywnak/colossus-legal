// =============================================================================
// trialPrepData.ts — Trial Prep ("War Room") payload CONTRACT (types only)
// -----------------------------------------------------------------------------
// The backend-payload shape. Every field is present even when empty; optional
// display fields are `T | null` (present-as-null, not omitted) — the same
// convention the Proof Review service uses. The live backend now fills this
// (via `services/trialPrep.ts`); view-shaping lives in `trialPrepHelpers.ts`.
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
  | "defense_counter"
  // Graph-evidence turn (Chunk 4): a fact anchored to the scenario's allegation.
  // Its REBUTS/CORROBORATES polarity is carried in `relationship_type`, NOT the
  // kind, so no accusation/rebuttal litigation narrative is fabricated.
  | "evidence";

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

/**
 * A scenario's authored definition body — the typed mirror of the backend
 * `ScenarioDefinition` (`backend/src/dto/scenario_crud.rs`). Must stay
 * field-for-field with that struct.
 *
 * Required-vs-optional follows the backend serde attrs exactly:
 * - `attack_text` and `schema_v` are the REQUIRED pair (no serde default/skip on
 *   the backend — B1's parse contract rejects a definition missing either).
 * - `attack_meaning` / `target` / `notes` are `Option` + `skip_serializing_if` →
 *   optional (omitted when absent).
 * - `wielders` / `seed_phrases` / `anti_seed_phrases` are `#[serde(default)]` on
 *   the backend, so a read ALWAYS sees them as `[]` (never absent); they are
 *   non-optional here for reads, though an authoring form may start them empty.
 */
export interface ScenarioDefinition {
  attack_text: string;
  attack_meaning?: string;
  wielders: string[];
  target?: string;
  seed_phrases: string[];
  anti_seed_phrases: string[];
  notes?: string;
  schema_v: number;
}

/**
 * The definition schema version this frontend build authors under. MIRRORS
 * `backend/src/dto/scenario_crud.rs::CURRENT_SCHEMA_V` — the backend const is not
 * frontend-reachable (it is neither re-exported nor shipped on any endpoint), so
 * we carry our own copy. The two MUST move together on any schema bump: raising
 * one without the other means the frontend authors a version the backend reader
 * does not recognize (or vice versa).
 */
// CONST: mirrors backend CURRENT_SCHEMA_V — a build-time coupling invariant, NOT
// a deployment knob (the backend const is not frontend-reachable; the two must
// move together on any schema bump). Cannot live in env/config.
export const CURRENT_SCHEMA_V = 1;

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
  /**
   * The authored definition, opaque on the wire (backend sends the raw jsonb).
   * OPTIONAL: an un-authored scenario legitimately lacks one (the backend sends
   * `{}`, which does NOT satisfy this typed shape — treat a missing/`{}`
   * definition as "not yet defined"). Present only once a scenario is authored.
   */
  definition?: ScenarioDefinition;
}
