// =============================================================================
// candidateWorkbench.ts — pure logic for the gather workbench (Phase 1a.6).
// -----------------------------------------------------------------------------
// The panel that consumes this (`CandidateFactsPanel`) has no component-test
// infra (Rule 30), so every risk-bearing decision lives here as a pure function
// and is unit-tested in `__tests__/candidateWorkbench.test.ts`. The component is
// then thin wiring over these helpers.
// =============================================================================

import type {
  CandidateDto,
  FactAction,
  FactStatus,
} from "../services/scenarioGather";
import type { ScenarioFactDto } from "../services/scenarioFacts";

/** The status filter the workbench offers. `"all"` is the union of the three
 *  real states; the three others narrow to one `FactStatus`. */
export type StatusFilter = FactStatus | "all";

/** The ordered filter options rendered in the dropdown. `undecided` is first
 *  because it is the default working view (candidates awaiting a ruling). */
export const STATUS_FILTERS: StatusFilter[] = [
  "undecided",
  "included",
  "dropped",
  "all",
];

/** Human labels for the filter dropdown (never a hardcoded case value — these
 *  are the generic state vocabulary, safe to compile in). */
export const STATUS_FILTER_LABEL: Record<StatusFilter, string> = {
  undecided: "Undecided",
  included: "Included",
  dropped: "Dropped",
  all: "All",
};

/**
 * Select the candidates a status filter shows, in memory.
 *
 * TS-learning: this is a CLIENT-SIDE predicate because `gatherCandidates`
 * returns the whole bounded pool (~94 nodes) in ONE response — so changing the
 * filter never refetches. Contrast the old bias query, which POSTed a new
 * request on every filter change. Rule of thumb: a bounded set fetched in one
 * call → filter in memory; an unbounded / server-paged set → refetch per filter.
 */
export function filterByStatus(
  candidates: CandidateDto[],
  filter: StatusFilter,
): CandidateDto[] {
  if (filter === "all") return candidates;
  return candidates.filter((c) => c.status === filter);
}

/**
 * Count candidates per status in a single pass.
 *
 * TS-learning: this is a pure single-pass fold — the returned counts are a
 * DERIVATION of the candidate list, re-derived on every render from the same
 * state the list renders from, never a separately-stored number that could drift
 * out of sync with the list. That is precisely why a status summary built on
 * this can never disagree with the rendered rows: both read this one source.
 *
 * Folds the LIVE (status-known) pool ONLY. Orphaned facts (statusless saved refs
 * missing from the pool) are NOT handled here — that is a call-site policy (the
 * component folds `orphans.length` into `included`), kept out of this fold so the
 * helper stays honest and independently testable.
 */
export function countByStatus(candidates: CandidateDto[]): {
  undecided: number;
  included: number;
  dropped: number;
  total: number;
} {
  return candidates.reduce(
    (acc, c) => {
      acc[c.status] += 1;
      acc.total += 1;
      return acc;
    },
    { undecided: 0, included: 0, dropped: 0, total: 0 },
  );
}

/**
 * The ruling buttons a candidate offers, given its current status.
 *
 * - `undecided` → Include or Drop (rule on a fresh candidate).
 * - `included`  → Drop (exclude a confirmed fact from this scenario).
 * - `dropped`   → Un-drop (recover it to the pool as undecided).
 *
 * TS-learning: the `default` branch assigns `status` to a `never` binding — the
 * TypeScript twin of Rust's exhaustive `match` with no `_` arm. If a fourth
 * `FactStatus` is ever added, `status` is no longer assignable to `never` and
 * THIS function fails to compile until its case is written. The compiler becomes
 * the checklist; a new state cannot silently fall through with no actions.
 */
export function actionsForStatus(status: FactStatus): FactAction[] {
  switch (status) {
    case "undecided":
      return ["include", "drop"];
    case "included":
      return ["drop"];
    case "dropped":
      return ["undrop"];
    default: {
      const _exhaustive: never = status;
      return _exhaustive;
    }
  }
}

/** Display label for each ruling button. */
export const ACTION_LABEL: Record<FactAction, string> = {
  include: "Include",
  drop: "Drop",
  undrop: "Un-drop",
};

/**
 * Find saved facts the gather pool does NOT know about — the orphan guarantee.
 *
 * Gather is pool-driven: it returns every LIVE Evidence node ABOUT the subject,
 * so a saved ref whose graph node has vanished (deleted / re-ingested under a new
 * id) is simply ABSENT from `pool` and `dropped`. Left unhandled, an orphaned
 * *confirmed* fact would silently disappear from the UI (violates Standing
 * Rule 1 / the ratified orphan policy). This surfaces those refs so the caller
 * can render the "content unavailable" stale card.
 *
 * ## Limitation, deliberately conservative
 *
 * The old `GET …/facts` (`ScenarioFactDto`) carries NO status — so we cannot tell
 * an orphaned *included* ref from an orphaned *dropped* one. We therefore surface
 * EVERY saved ref missing from the gather set. This never hides a confirmed fact
 * (the guarantee), at the cost of also surfacing the rare orphaned-dropped ref —
 * a harmless over-approximation. `knownIds` is the union of the pool's and
 * dropped list's node ids.
 */
export function findOrphans(
  saved: ScenarioFactDto[],
  knownIds: Set<string>,
): ScenarioFactDto[] {
  return saved.filter((f) => !knownIds.has(f.graph_node_id));
}

/** Whether orphan stale cards belong in the current view. Shown only where a
 *  confirmed fact would be expected — the `included` and `all` filters — since
 *  an orphan's true status is unknown (see `findOrphans`) but the guarantee is
 *  about not losing *confirmed* facts. */
export function orphansVisibleUnder(filter: StatusFilter): boolean {
  return filter === "included" || filter === "all";
}
