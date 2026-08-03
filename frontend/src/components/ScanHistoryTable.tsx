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
import type { ScanRunHeader } from "../services/themeScan";

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

interface Props {
  runs: ScanRunHeader[];
  /** run_ids currently selected for display (single-select: 0 or 1). */
  selectedRunIds: string[];
  onToggle: (runId: string) => void;
  /** The parent owns the network call, the error UI, and post-delete cleanup. */
  onDelete: (runId: string) => void;
  /** Resolves a model id to its display name (the panel's model catalogue). */
  modelName: (id: string) => string;
}

const ScanHistoryTable: React.FC<Props> = ({
  runs,
  selectedRunIds,
  onToggle,
  onDelete,
  modelName,
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
            <th style={headStyle}>Status</th>
            {/* Not in §2.3's sketch — see the module header on why delete stays.
                The header cell is deliberately EMPTY rather than labelled: the
                column holds one icon button per row and each of those carries its
                own `aria-label` naming the run it removes, which is what a screen
                reader announces. A visible "Remove" heading would add a word to
                every reading of the table for no gain. `scope="col"` keeps the
                empty cell a legitimate header rather than a stray blank. */}
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
                <td style={cellStyle}>{row.status}</td>
                <td style={{ ...cellStyle, textAlign: "right" }}>
                  <button
                    type="button"
                    aria-label={`Remove the scan run from ${row.when}`}
                    title="Remove this run"
                    onClick={(event) => {
                      // The row's own click selects; this must not also do that.
                      event.stopPropagation();
                      onDelete(row.runId);
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
