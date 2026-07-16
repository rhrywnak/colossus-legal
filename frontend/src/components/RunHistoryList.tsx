// =============================================================================
// RunHistoryList.tsx — the scan-run history list for the Theme Scan panel.
// -----------------------------------------------------------------------------
// Renders one compact row per persisted run of a scenario (model + benchmark/real
// badge + counts + timestamp), newest first (the backend already orders them).
// Clicking a row toggles its selection; the parent loads that run's full result
// (via the existing getScanRun) and renders it in the existing results display.
// Selecting a SECOND row drives the existing two-run comparison hero.
//
// Extracted into its own file (not grown into ThemeScanPanel) so the panel stays
// focused; styling is tokens.css only, matching the panel's own `S` object.
// =============================================================================

import React from "react";

import type { ScanRunHeader } from "../services/themeScan";
import { formatCost, formatElapsed, formatRunTimestamp } from "./themeScanFormat";

interface Props {
  runs: ScanRunHeader[];
  /** run_ids currently selected for display/comparison (0, 1, or 2). */
  selectedRunIds: string[];
  onToggle: (runId: string) => void;
  /** Resolve a model id to its display name (owned by the panel's model catalog). */
  modelName: (id: string) => string;
}

/** The scenario's run history. Empty renders nothing (the parent decides the
 *  empty-state copy), so this component is purely the list when there ARE runs. */
const RunHistoryList: React.FC<Props> = ({ runs, selectedRunIds, onToggle, modelName }) => {
  if (runs.length === 0) return null;

  return (
    <div style={S.wrap}>
      <div style={S.head}>Run history</div>
      <div style={S.list}>
        {runs.map((run) => (
          <RunHistoryRow
            key={run.run_id}
            run={run}
            selected={selectedRunIds.includes(run.run_id)}
            onToggle={() => onToggle(run.run_id)}
            modelName={modelName(run.model_id)}
          />
        ))}
      </div>
    </div>
  );
};

const RunHistoryRow: React.FC<{
  run: ScanRunHeader;
  selected: boolean;
  onToggle: () => void;
  modelName: string;
}> = ({ run, selected, onToggle, modelName }) => (
  <button
    type="button"
    onClick={onToggle}
    aria-pressed={selected}
    style={{ ...S.row, ...(selected ? S.rowSelected : {}) }}
  >
    <div style={S.rowMain}>
      <span style={S.model}>{modelName}</span>
      <span style={run.dry_run ? S.badgeBenchmark : S.badgeReal}>
        {run.dry_run ? "Benchmark" : "Real"}
      </span>
      <StatusBadge status={run.status} />
    </div>
    <div style={S.rowMeta}>
      {/* Counts: the outcome partition (relevant / not-relevant / failed). */}
      <span style={S.count}>
        <b style={S.countRelevant}>{run.relevant_count}</b> relevant
      </span>
      <span style={S.dot}>·</span>
      <span style={S.count}>{run.irrelevant_count} not</span>
      {run.failed_count > 0 && (
        <>
          <span style={S.dot}>·</span>
          <span style={S.countFailed}>{run.failed_count} failed</span>
        </>
      )}
      <span style={S.dot}>·</span>
      <span style={S.muted}>{formatCost(run.computed_cost)}</span>
      <span style={S.dot}>·</span>
      <span style={S.muted}>{formatElapsed(run.duration_ms)}</span>
      <span style={S.dot}>·</span>
      <span style={S.muted}>{formatRunTimestamp(run.started_at)}</span>
    </div>
  </button>
);

/** A run's terminal/progress state, colored distinctly (Standing Rule 1 — the
 *  three states are visually distinguishable, not collapsed). */
const StatusBadge: React.FC<{ status: ScanRunHeader["status"] }> = ({ status }) => {
  const style =
    status === "completed"
      ? S.statusCompleted
      : status === "failed"
        ? S.statusFailed
        : S.statusRunning;
  return <span style={style}>{status}</span>;
};

// ─── styling (tokens.css only) ────────────────────────────────────────────────

const S: Record<string, React.CSSProperties> = {
  wrap: { marginTop: "18px" },
  head: {
    fontSize: "0.72rem",
    fontWeight: 600,
    textTransform: "uppercase",
    letterSpacing: "0.04em",
    color: "var(--text-muted)",
    marginBottom: "8px",
  },
  list: { display: "flex", flexDirection: "column", gap: "6px" },
  row: {
    display: "flex",
    flexDirection: "column",
    gap: "4px",
    padding: "10px 12px",
    background: "var(--bg-page)",
    border: "1px solid var(--border-default)",
    borderRadius: "10px",
    cursor: "pointer",
    textAlign: "left",
    fontFamily: "var(--font-sans)",
    width: "100%",
  },
  rowSelected: {
    borderColor: "var(--accent-primary)",
    boxShadow: "inset 0 0 0 1px var(--accent-primary)",
  },
  rowMain: { display: "flex", alignItems: "center", gap: "8px" },
  model: { fontSize: "0.9rem", fontWeight: 600, color: "var(--text-primary)" },
  rowMeta: {
    display: "flex",
    alignItems: "center",
    flexWrap: "wrap",
    gap: "6px",
    fontSize: "0.8rem",
    color: "var(--text-secondary)",
  },
  count: { color: "var(--text-secondary)" },
  countRelevant: { color: "var(--state-success-strong)", fontWeight: 700 },
  countFailed: { color: "var(--state-danger-strong)" },
  dot: { color: "var(--text-muted)" },
  muted: { color: "var(--text-muted)" },
  badgeBenchmark: {
    fontSize: "0.68rem",
    fontWeight: 600,
    color: "var(--text-muted)",
    border: "1px solid var(--border-default)",
    borderRadius: "6px",
    padding: "1px 6px",
  },
  badgeReal: {
    fontSize: "0.68rem",
    fontWeight: 600,
    color: "var(--accent-primary)",
    border: "1px solid var(--accent-primary)",
    borderRadius: "6px",
    padding: "1px 6px",
  },
  statusCompleted: {
    fontSize: "0.68rem",
    fontWeight: 600,
    color: "var(--state-success-strong)",
    marginLeft: "auto",
  },
  statusFailed: {
    fontSize: "0.68rem",
    fontWeight: 600,
    color: "var(--state-danger-strong)",
    marginLeft: "auto",
  },
  statusRunning: {
    fontSize: "0.68rem",
    fontWeight: 600,
    color: "var(--text-muted)",
    marginLeft: "auto",
  },
};

export default RunHistoryList;
