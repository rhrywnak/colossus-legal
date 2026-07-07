// =============================================================================
// CandidateFactsPanel — the scenario candidate workbench (Phase 1a.6).
// =============================================================================
//
// Cut over from the old bias-query finder to the gather workbench: it loads the
// full candidate pool (`GET …/facts/gather` — every Evidence node ABOUT the
// scenario's subject, each with its three-state status), narrows it with a
// client-side STATUS filter (default `undecided`), and lets a human rule on each
// candidate (include / drop / un-drop via `POST …/facts/:id/action`).
//
// One gather-driven, status-grouped list is the whole surface (the 1a.6 merge):
// the old separate "Curated facts" saved-list is now just the `included` filter.
// The orphan guarantee is preserved separately — see the `listScenarioFacts`
// call and `findOrphans` below.
//
// The subject is resolved SERVER-SIDE by the gather endpoint (definition.target
// → case default), so this panel no longer seeds filters from the definition and
// no case-specific name is hardcoded here (Standing Rule 2).

import React, { useEffect, useMemo, useState } from "react";

import {
  applyFactAction,
  gatherCandidates,
  type CandidateDto,
  type FactAction,
} from "../services/scenarioGather";
import { listScenarioFacts, type ScenarioFactDto } from "../services/scenarioFacts";
import EvidenceCard from "../pages/BiasExplorer/EvidenceCard";
import type { ScenarioDefinition } from "../pages/trialPrepData";
import {
  ACTION_LABEL,
  actionsForStatus,
  countByStatus,
  filterByStatus,
  findOrphans,
  orphansVisibleUnder,
  STATUS_FILTERS,
  STATUS_FILTER_LABEL,
  type StatusFilter,
} from "./candidateWorkbench";

interface Props {
  slug: string;
  scenarioId: string;
  /** @deprecated (1a.6) Vestigial. The gather endpoint resolves the subject
   *  server-side (definition.target → case default), so the panel no longer
   *  seeds filters from the definition. Kept on the signature only to preserve
   *  the mount contract; a follow-up tidy chunk removes it (and `candidateSeed`).
   */
  definition?: ScenarioDefinition;
}

const toggleStyle: React.CSSProperties = {
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  padding: "0.5rem 0.9rem",
  fontSize: "0.85rem",
  fontWeight: 600,
  color: "var(--text-primary)",
  cursor: "pointer",
};

const panelStyle: React.CSSProperties = {
  marginTop: "0.75rem",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  padding: "0.9rem",
  backgroundColor: "var(--bg-page)",
};

const controlRow: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  gap: "0.6rem",
  marginBottom: "0.75rem",
};

const selectStyle: React.CSSProperties = {
  padding: "0.35rem 0.5rem",
  fontSize: "0.82rem",
  border: "1px solid var(--border-default)",
  borderRadius: "5px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-primary)",
};

const messageStyle: React.CSSProperties = {
  fontSize: "0.84rem",
  color: "var(--text-muted)",
  padding: "0.5rem 0",
};

const errorStyle: React.CSSProperties = {
  margin: "0.5rem 0",
  padding: "0.6rem 0.8rem",
  backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
  fontSize: "0.82rem",
};

const staleCardStyle: React.CSSProperties = {
  backgroundColor: "var(--bg-surface)",
  border: "1px dashed var(--state-warning-strong)",
  borderRadius: "8px",
  padding: "0.85rem 1rem",
  fontSize: "0.82rem",
  color: "var(--text-secondary)",
};

// One button style per ruling. `include` reads as the affirmative accent; `drop`
// as the danger exclusion; `undrop` as a neutral recovery.
const actionBtnStyle: Record<FactAction, React.CSSProperties> = {
  include: {
    padding: "0.2rem 0.6rem",
    fontSize: "0.74rem",
    fontWeight: 600,
    border: "1px solid var(--accent-primary)",
    borderRadius: "5px",
    backgroundColor: "var(--accent-bg-soft)",
    color: "var(--accent-primary)",
    cursor: "pointer",
  },
  drop: {
    padding: "0.2rem 0.6rem",
    fontSize: "0.74rem",
    fontWeight: 600,
    border: "1px solid var(--state-danger-border)",
    borderRadius: "5px",
    backgroundColor: "var(--state-danger-bg-soft)",
    color: "var(--state-danger-strong)",
    cursor: "pointer",
  },
  undrop: {
    padding: "0.2rem 0.6rem",
    fontSize: "0.74rem",
    fontWeight: 600,
    border: "1px solid var(--border-default)",
    borderRadius: "5px",
    backgroundColor: "var(--bg-surface)",
    color: "var(--text-primary)",
    cursor: "pointer",
  },
};

// Status summary row — replicates the Document → Review screen's count line
// (ReviewPanel.tsx) by construction: same markup shape, same theme tokens, so
// the two read identically without extracting a shared component (rule of three:
// two consumers, one shipped, does not justify extraction). Token mapping:
// undecided → Review's neutral "pending", included → its green "approved",
// dropped → its red "rejected". No hex (Rule 11); tokens only (Rule 2).
const summaryRowStyle: React.CSSProperties = {
  display: "flex",
  gap: "0.75rem",
  alignItems: "center",
  marginBottom: "0.5rem",
  flexWrap: "wrap",
};

const undecidedCountStyle: React.CSSProperties = {
  fontSize: "0.76rem",
  color: "var(--text-secondary)",
  fontWeight: 600,
};

const includedCountStyle: React.CSSProperties = {
  fontSize: "0.76rem",
  color: "var(--status-active-text)",
};

const droppedCountStyle: React.CSSProperties = {
  fontSize: "0.76rem",
  color: "var(--status-dropped-text)",
};

// The "x of y" count beside the filter dropdown — matches ReviewPanel's
// `{filtered.length} / {items.length}` span (tokens + size), worded "of".
const xOfYStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  color: "var(--text-muted)",
  alignSelf: "center",
};

const CandidateFactsPanel: React.FC<Props> = ({ slug, scenarioId }) => {
  const [open, setOpen] = useState(false);

  const [candidates, setCandidates] = useState<CandidateDto[] | null>(null);
  const [orphans, setOrphans] = useState<ScenarioFactDto[]>([]);
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("undecided");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Per-row action error (keyed by node id) — a ruling failure is shown on the
  // row that failed, not as a page-level banner.
  const [actionError, setActionError] = useState<{ id: string; message: string } | null>(
    null,
  );

  // Bumped after a successful ruling to re-fetch the whole pool, so the UI is a
  // pure reflection of persisted state (no optimistic drift).
  const [refreshKey, setRefreshKey] = useState(0);

  // Load the whole pool + the saved list (for the orphan check) whenever the
  // panel is open. Both come from ONE fetch each; the status filter then works
  // in memory (see `filterByStatus`). `listScenarioFacts` is fetched alongside
  // gather ONLY to surface confirmed facts whose graph node has vanished (they
  // are absent from the pool-driven gather response — `findOrphans`).
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    Promise.all([
      gatherCandidates(slug, scenarioId),
      listScenarioFacts(slug, scenarioId),
    ])
      .then(([gather, saved]) => {
        if (cancelled) return;
        const combined = [...gather.pool, ...gather.dropped];
        const knownIds = new Set(combined.map((c) => c.content.evidence_id));
        setCandidates(combined);
        setOrphans(findOrphans(saved, knownIds));
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load candidate facts. Try again.",
        );
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, slug, scenarioId, refreshKey]);

  const visible = useMemo(
    () => filterByStatus(candidates ?? [], statusFilter),
    [candidates, statusFilter],
  );

  const handleAction = (graphNodeId: string, action: FactAction) => {
    setActionError(null);
    applyFactAction(slug, scenarioId, graphNodeId, action)
      .then(() => setRefreshKey((k) => k + 1))
      .catch((err: unknown) => {
        setActionError({
          id: graphNodeId,
          message:
            err instanceof Error ? err.message : "Failed to apply this ruling.",
        });
      });
  };

  if (!open) {
    return (
      <button type="button" style={toggleStyle} onClick={() => setOpen(true)}>
        + Candidate facts
      </button>
    );
  }

  const showOrphans = orphansVisibleUnder(statusFilter) && orphans.length > 0;
  // Exclude the error state: on a failed load `candidates` stays null, which
  // would otherwise render the "no matches" empty-state text UNDER the error
  // banner — making a load failure read as an empty filter result. The banner
  // is the observable; the empty-state message must yield to it (Standing Rule 1
  // — distinct states, distinct observables).
  const nothingToShow = !loading && !error && visible.length === 0 && !showOrphans;

  // Counts are derived from state already on screen (the gather pool + orphans),
  // re-derived each render — they cannot drift from the rendered list. Orphans
  // (statusless saved refs missing from the pool) are folded into `included`
  // HERE at the call site: a conservative over-approximation matching where 1a.6
  // renders them (under the included/all filters), so the count agrees with the
  // list. It never under-counts a confirmed fact (the ratified guarantee).
  const counts = countByStatus(candidates ?? []);
  const includedShown = counts.included + orphans.length;
  const totalShown = counts.total + orphans.length;
  // "x of y" numerator: rows visible under the current filter — orphans add in
  // only when the filter actually shows them (so it equals the rendered count).
  const shownCount = visible.length + (showOrphans ? orphans.length : 0);

  return (
    <div>
      <button type="button" style={toggleStyle} onClick={() => setOpen(false)}>
        − Hide candidate facts
      </button>
      <div style={panelStyle}>
        <div style={controlRow}>
          <label style={{ fontSize: "0.82rem", color: "var(--text-secondary)" }}>
            Status:{" "}
            <select
              style={selectStyle}
              value={statusFilter}
              onChange={(e) => setStatusFilter(e.target.value as StatusFilter)}
            >
              {STATUS_FILTERS.map((s) => (
                <option key={s} value={s}>
                  {STATUS_FILTER_LABEL[s]}
                </option>
              ))}
            </select>
          </label>
          {candidates !== null && (
            <span style={xOfYStyle}>
              {shownCount} of {totalShown}
            </span>
          )}
        </div>

        {/* Status summary — all three totals at once, so a ruling is watchable
            (drop one → undecided ticks down, dropped ticks up, in one glance).
            Shown once the pool has loaded; hidden during the first load / on an
            error so it never displays misleading zeros (Standing Rule 1). */}
        {candidates !== null && (
          <div style={summaryRowStyle}>
            <span style={undecidedCountStyle}>{counts.undecided} undecided</span>
            <span style={includedCountStyle}>{includedShown} included</span>
            <span style={droppedCountStyle}>{counts.dropped} dropped</span>
          </div>
        )}

        {error && <div style={errorStyle}>{error}</div>}

        {loading ? (
          <div style={messageStyle}>Loading candidate facts…</div>
        ) : nothingToShow ? (
          <div style={messageStyle}>No candidate facts match this filter.</div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem" }}>
            {visible.map((c) => (
              <div key={c.content.evidence_id}>
                <EvidenceCard
                  instance={c.content}
                  action={
                    <span style={{ display: "flex", gap: "0.4rem" }}>
                      {actionsForStatus(c.status).map((action) => (
                        <button
                          key={action}
                          type="button"
                          style={actionBtnStyle[action]}
                          onClick={() =>
                            handleAction(c.content.evidence_id, action)
                          }
                        >
                          {ACTION_LABEL[action]}
                        </button>
                      ))}
                    </span>
                  }
                />
                {actionError?.id === c.content.evidence_id && (
                  <div style={errorStyle}>{actionError.message}</div>
                )}
              </div>
            ))}

            {/* Orphan guarantee: saved facts whose graph node has vanished are
                absent from the pool-driven gather response, so they are surfaced
                here (never silently dropped — Standing Rule 1). Informational in
                1a.6; a ruling affordance on a stale card is out of scope. */}
            {showOrphans &&
              orphans.map((o) => (
                <div key={o.graph_node_id} style={staleCardStyle}>
                  Saved fact <code>{o.graph_node_id}</code> — content unavailable
                  (the source node may have been removed).
                </div>
              ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default CandidateFactsPanel;
