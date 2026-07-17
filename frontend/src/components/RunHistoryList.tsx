// =============================================================================
// RunHistoryList.tsx — the scan-run history list for the Theme Scan panel.
// -----------------------------------------------------------------------------
// Renders one compact row per persisted run of a scenario (model + benchmark/real
// badge + counts + timestamp), newest first (the backend already orders them).
// Clicking a row selects it (single-select); the parent loads that run's full
// result (via the existing getScanRun) and renders it in the results display.
// Each row also carries a delete control (trash icon) that removes the run.
//
// Extracted into its own file (not grown into ThemeScanPanel) so the panel stays
// focused; styling is tokens.css only, matching the panel's own `S` object.
// =============================================================================

import React from "react";

import type { ScanRunHeader } from "../services/themeScan";
import { formatCost, formatElapsed, formatRunTimestamp } from "./themeScanFormat";

interface Props {
  runs: ScanRunHeader[];
  /** run_ids currently selected for display (single-select: 0 or 1). */
  selectedRunIds: string[];
  onToggle: (runId: string) => void;
  /** Delete a run. The parent owns the network call + error UI + post-delete
   *  state cleanup; this component only confirms and reports the user's intent. */
  onDelete: (runId: string) => void;
  /** Resolve a model id to its display name (owned by the panel's model catalog). */
  modelName: (id: string) => string;
}

/** The scenario's run history. Empty renders nothing (the parent decides the
 *  empty-state copy), so this component is purely the list when there ARE runs. */
const RunHistoryList: React.FC<Props> = ({
  runs,
  selectedRunIds,
  onToggle,
  onDelete,
  modelName,
}) => {
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
            onDelete={() => onDelete(run.run_id)}
            modelName={modelName(run.model_id)}
          />
        ))}
      </div>
    </div>
  );
};

/** ## A11y note: why the row is a `<div role="button">`, not a `<button>`
 *
 *  The row carries a nested delete `<button>`, and a `<button>` cannot legally
 *  contain another `<button>` (invalid HTML — React logs a DOM-nesting warning
 *  and click/focus behavior is undefined). So the row is a `<div>` with the
 *  button role restored by hand: `role="button"`, `tabIndex={0}`, `aria-pressed`,
 *  and an `onKeyDown` that maps Enter/Space to activation the way a native button
 *  does. The delete button is a real, separately-focusable `<button>` inside it. */
const RunHistoryRow: React.FC<{
  run: ScanRunHeader;
  selected: boolean;
  onToggle: () => void;
  onDelete: () => void;
  modelName: string;
}> = ({ run, selected, onToggle, onDelete, modelName }) => (
  <div
    role="button"
    tabIndex={0}
    onClick={onToggle}
    onKeyDown={(e) => {
      // Restore native-<button> keyboard activation lost by using a <div>: Enter
      // and Space select the row. preventDefault on Space stops the page scroll a
      // Space keypress would otherwise trigger.
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onToggle();
      }
    }}
    aria-pressed={selected}
    style={{ ...S.row, ...(selected ? S.rowSelected : {}) }}
  >
    <div style={S.rowMain}>
      <span style={S.model}>{modelName}</span>
      <span style={run.dry_run ? S.badgeBenchmark : S.badgeReal}>
        {run.dry_run ? "Benchmark" : "Real"}
      </span>
      <StatusBadge status={run.status} />
      <button
        type="button"
        aria-label="Delete run"
        title="Delete run"
        onClick={(e) => {
          // stopPropagation is load-bearing: without it this click ALSO bubbles to
          // the row's onClick and selects the run we are trying to delete. Confirm
          // is the last undo (delete is irreversible); the parent owns the rest.
          e.stopPropagation();
          if (window.confirm("Delete this scan run? This can't be undone.")) {
            onDelete();
          }
        }}
        style={S.deleteBtn}
      >
        <TrashIcon />
      </button>
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
  </div>
);

/** A trash-can glyph drawn as inline SVG (no icon dependency in this repo). Uses
 *  `currentColor` so it inherits the delete button's token color, and is marked
 *  `aria-hidden` because the button's `aria-label` already names the action. */
const TrashIcon: React.FC = () => (
  <svg
    width="15"
    height="15"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.4"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M2.5 4h11" />
    <path d="M6.5 4V2.8c0-.4.3-.8.8-.8h1.4c.5 0 .8.4.8.8V4" />
    <path d="M4 4l.6 8.6c0 .5.4.9.9.9h5c.5 0 .9-.4.9-.9L12 4" />
    <path d="M6.5 7v4M9.5 7v4" />
  </svg>
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
  deleteBtn: {
    // Sits after the status badge (which has marginLeft:auto), so it pins to the
    // far right of the row's first line. Icon-only, muted until the row is hovered
    // — no inline :hover here (the panel styles are plain style objects), so it
    // stays a quiet, always-visible affordance rather than a loud one.
    display: "inline-flex",
    alignItems: "center",
    justifyContent: "center",
    padding: "2px",
    marginLeft: "2px",
    background: "none",
    border: "none",
    borderRadius: "6px",
    cursor: "pointer",
    color: "var(--text-muted)",
    lineHeight: 0,
  },
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
