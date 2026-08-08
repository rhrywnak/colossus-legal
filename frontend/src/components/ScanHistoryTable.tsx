// =============================================================================
// ScanHistoryTable — the collapsed run history (§2.3 item 2, ruling R2)
// =============================================================================
//
// A `<details>` disclosure holding a table: When / Model / Candidates / New /
// Status. Replaces `RunHistoryList`'s always-visible card list, which grew without
// bound down the panel and had no room for the two columns §2.3 asks for.
//
// ## What was kept from the list it replaces
//
// Row SELECT (clicking a row loads that run's full result into the results display)
// and row DELETE. Both are existing function, and §2.3's five-column sketch does
// not mention either — dropping them to match a sketch would remove capability the
// human has today, so they survive as the row's click target and a sixth cell.
// That divergence from the design is stated rather than silently taken.
//
// ## Renders only
//
// Every cell string comes from `scanHistoryRows` (pure, tested), including the two
// honesty rules: `candidates_read == 0` renders as an em dash rather than "0", and
// a null delta renders as an em dash rather than "+0".

import React from "react";

import { scanHistoryRows, scanHistorySummary } from "./scanHistoryRows";
import type { ScanRunHeader, ScanWording } from "../services/themeScan";

// v3: the divider token, resolved inside the scoped layer to #eef0f3.
const HAIRLINE = "1px solid var(--border-default)";

const headStyle: React.CSSProperties = {
  fontSize: "0.66rem",
  textTransform: "uppercase",
  letterSpacing: "0.07em",
  color: "var(--text-muted)",
  fontWeight: 600,
  textAlign: "left",
  padding: "0.35rem 0.6rem",
  borderBottom: HAIRLINE,
  whiteSpace: "nowrap",
};

const cellStyle: React.CSSProperties = {
  padding: "0.4rem 0.6rem",
  borderBottom: HAIRLINE,
  color: "var(--text-secondary)",
  fontSize: "0.8rem",
  verticalAlign: "top",
};

const summaryStyle: React.CSSProperties = {
  cursor: "pointer",
  listStyle: "none",
  color: "var(--accent-primary)",
  fontSize: "0.82rem",
};

/**
 * The view control, quiet when closed and marked when open.
 *
 * A style function rather than two constants: the only difference is which state
 * it is in, and two near-identical objects invite one of them to drift.
 */
function viewButtonStyle(selected: boolean): React.CSSProperties {
  return {
    border: "none",
    background: selected ? "var(--v3-chrome-strong)" : "var(--v3-chrome)",
    borderRadius: "6px",
    padding: "0.2rem 0.5rem",
    cursor: "pointer",
    color: selected ? "var(--text-primary)" : "var(--accent-primary)",
    fontFamily: "inherit",
    fontSize: "0.78rem",
  };
}

/**
 * Fill `{run}` in the delete confirmation.
 *
 * The row's when-label is formatted in the reader's locale (see
 * `formatRunTimestamp`), so it is the one part of this sentence the server cannot
 * compose — which is why this template, alone among the scan's strings, is filled
 * in the browser. Same single-`replace` shape as `fillCode` beside it.
 */
function fillRun(template: string, run: string): string {
  return template.replace("{run}", run);
}

interface Props {
  runs: ScanRunHeader[];
  /**
   * The words this table's own controls speak, served with the list.
   *
   * `null` while the payload is still loading, or if it predates them. A control
   * with no label is NOT rendered — a compiled-in fallback would be a literal on
   * a surface the configuration law covers, and a blank button is worse than an
   * absent one.
   */
  wording: ScanWording | null;
  /** run_ids currently selected for display (single-select: 0 or 1). */
  selectedRunIds: string[];
  onToggle: (runId: string) => void;
  /** The parent owns the network call, the error UI, and post-delete cleanup. */
  onDelete: (runId: string) => void;
  /** Resolves a model id to its display name (the panel's model catalogue). */
  modelName: (id: string) => string;
  /**
   * The run whose verdicts are currently PROPOSING candidates, or `null`.
   *
   * Only the latest completed run projects (R-b), so at most one row may carry a
   * proposed count. Every other row shows an em dash — architect ruling R6: a
   * run's frozen relevant count under a "Proposed" heading would be a false claim
   * about every row but one.
   */
  proposingRunId: string | null;
  /** How many candidates that run is proposing right now. The LIVE number. */
  proposedCount: number | null;
}

const ScanHistoryTable: React.FC<Props> = ({
  runs,
  wording,
  selectedRunIds,
  onToggle,
  onDelete,
  modelName,
  proposingRunId,
  proposedCount,
}) => {
  const summary = scanHistorySummary(runs.length);
  // No runs → no disclosure at all. A `<details>` that opens onto an empty table
  // is a worse answer than nothing, and the section's own copy covers a
  // never-scanned scenario.
  if (!summary) return null;

  const rows = scanHistoryRows(runs, modelName);

  return (
    <details style={{ fontSize: "0.82rem" }}>
      <summary style={summaryStyle} className="no-marker">{summary} ▾</summary>
      <table style={{ width: "100%", borderCollapse: "collapse", marginTop: "0.5rem" }}>
        <thead>
          <tr>
            <th style={headStyle}>When</th>
            <th style={headStyle}>Model</th>
            <th style={headStyle}>Candidates</th>
            <th style={headStyle}>New</th>
            <th style={headStyle}>Proposed</th>
            <th style={headStyle}>Status</th>
            {/* Not in §2.3's sketch — see the module header on why delete stays.
                The header cell is deliberately EMPTY rather than labelled: the
                column holds one icon button per row and each of those carries its
                own `aria-label` naming the run it removes, which is what a screen
                reader announces. A visible "Remove" heading would add a word to
                every reading of the table for no gain. `scope="col"` keeps the
                empty cell a legitimate header rather than a stray blank. */}
            {/* Two unlabelled control columns: view, then remove. Empty for the
                same reason — each button carries its own accessible name. */}
            <th style={headStyle} scope="col" aria-label="Open a run's results" />
            <th style={headStyle} scope="col" aria-label="Remove a run" />
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => {
            const selected = selectedRunIds.includes(row.runId);
            return (
              <tr
                key={row.runId}
                onClick={() => onToggle(row.runId)}
                style={{
                  cursor: "pointer",
                  background: selected ? "var(--accent-bg-soft)" : undefined,
                }}
              >
                <td style={{ ...cellStyle, color: "var(--text-primary)", whiteSpace: "nowrap" }}>
                  {row.when}
                </td>
                <td style={cellStyle}>
                  {row.model}
                  {/* Measured: 3 of the 4 runs stored on DEV are dry runs. A table
                      that rendered them identically to real ones would tell the
                      reader something false about what has been done to the pool. */}
                  {row.dryRun && (
                    <span style={{ color: "var(--text-muted)" }}> (Dry run)</span>
                  )}
                </td>
                <td style={cellStyle}>{row.candidates}</td>
                <td style={cellStyle}>{row.newCount}</td>
                {/* Live, and only on the run that is actually projecting (R6).
                    The em dash is the house treatment for "not measured here" —
                    an older completed run proposes nothing, and printing its
                    frozen relevant count under this heading would say it does. */}
                <td style={cellStyle}>
                  {row.runId === proposingRunId && proposedCount !== null
                    ? proposedCount
                    : "—"}
                </td>
                <td style={cellStyle}>{row.status}</td>
                {/* The VIEW control (task 2.15, piece 4a).

                    The whole row has been clickable since 1.7C and nothing said
                    so, which is why a completed run's findings read as
                    unreachable after a refresh — the human had to guess that a
                    table row was a button. The row stays clickable; this names
                    the action. Rendered only for a run that HAS a stored result:
                    offering "View results" on a failed run promises a panel that
                    would open onto an error. */}
                <td style={{ ...cellStyle, textAlign: "right", whiteSpace: "nowrap" }}>
                  {wording && row.status === "completed" && (
                    <button
                      type="button"
                      style={viewButtonStyle(selected)}
                      onClick={(event) => {
                        // The row's own click already toggles; without this the
                        // two would fire and cancel each other out.
                        event.stopPropagation();
                        onToggle(row.runId);
                      }}
                      aria-expanded={selected}
                    >
                      {wording.view_label}
                    </button>
                  )}
                </td>
                <td style={{ ...cellStyle, textAlign: "right" }}>
                  {/* No words, no ✕ (task 2.15, piece 4b).

                      The confirmation IS the mis-click guard, and its sentence is
                      a stored row. A ✕ rendered without it would either delete
                      unguarded — the defect — or refuse silently, which is worse:
                      a control that does nothing teaches the human it is broken.
                      So the destructive control does not exist until the words
                      that guard it have arrived. */}
                  {wording && (
                    <button
                      type="button"
                      aria-label={`Remove the scan run from ${row.when}`}
                      title="Remove this run"
                      onClick={(event) => {
                        // The row's own click selects; this must not also do that.
                        event.stopPropagation();
                        // Until now one stray click here destroyed the run AND
                        // every verdict in it — the only support the rulings it
                        // produced have. Same shape as the merge confirm.
                        if (window.confirm(fillRun(wording.delete_confirm_template, row.when))) {
                          onDelete(row.runId);
                        }
                      }}
                      style={{
                        border: "none",
                        background: "none",
                        cursor: "pointer",
                        color: "var(--text-muted)",
                        fontFamily: "inherit",
                        fontSize: "0.8rem",
                      }}
                    >
                      ✕
                    </button>
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </details>
  );
};

export default ScanHistoryTable;
