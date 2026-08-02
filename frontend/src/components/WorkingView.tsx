// =============================================================================
// WorkingView — the included evidence, Casefleet Facts-table style (task 1.4)
// =============================================================================
//
// What a human has PUT IN the scenario, one row each: quote · accusation chips ·
// pinpoint chip → viewer · ruling state · C-code. Search top-left, create
// top-right (study §1.4/§3).
//
// The card queue (1.3) is where items are ruled ON; this is where the result is
// read. Two surfaces, two jobs — the queue is a working position, this is the
// record of what the position produced.
//
// ## Visual language (§2c)
//
// White surface, hairline borders, regular weight, one accent. Born compliant;
// the app's tinted `--bg-page` is not this task's business.
//
// ## Renders only
//
// Which rows exist and which are visible is `factsTable.ts`, pure and tested.
// Every string shown is a payload string.

import React, { useMemo, useState } from "react";

import { filterRows, includedRows, type WorkingRow } from "./factsTable";
import type { ScenarioCard } from "../services/scenarioCards";

const SURFACE = "var(--bg-surface)";
const HAIRLINE = "1px solid var(--border-default)";

const rowStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: "0.4rem",
  padding: "0.85rem 1rem",
  borderBottom: HAIRLINE,
  fontWeight: 400,
};

const chipStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "999px",
  padding: "0.1rem 0.55rem",
  fontSize: "0.75rem",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

const searchStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "6px",
  padding: "0.35rem 0.6rem",
  fontWeight: 400,
  minWidth: "16rem",
};

interface Props {
  cards: ScenarioCard[];
  /** Opens the augmentation panel — the one create action on this surface. */
  onAdd: () => void;
}

const WorkingView: React.FC<Props> = ({ cards, onAdd }) => {
  const [term, setTerm] = useState("");

  const rows = useMemo(() => includedRows(cards), [cards]);
  const visible = useMemo(() => filterRows(rows, term), [rows, term]);

  return (
    <div style={{ background: SURFACE, border: HAIRLINE, borderRadius: "8px" }}>
      {/* Search top-left, one create button top-right — the study's list-screen
          header, unchanged. */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "1rem",
          padding: "0.75rem 1rem",
          borderBottom: HAIRLINE,
        }}
      >
        <input
          value={term}
          onChange={(e) => setTerm(e.target.value)}
          placeholder="Search these facts"
          aria-label="Search the scenario's facts"
          style={searchStyle}
        />
        <button
          type="button"
          onClick={onAdd}
          style={{
            ...chipStyle,
            cursor: "pointer",
            background: SURFACE,
            color: "var(--accent-primary)",
            borderColor: "var(--accent-primary)",
          }}
        >
          Add human fact
        </button>
      </div>

      {rows.length === 0 ? (
        <div style={{ padding: "1rem", color: "var(--text-muted)", fontSize: "0.85rem" }}>
          Nothing included yet. Rule on candidates in the queue below and they appear here.
        </div>
      ) : visible.length === 0 ? (
        // A filter that matches nothing is a DIFFERENT state from an empty
        // scenario, and says so rather than looking like the latter.
        <div style={{ padding: "1rem", color: "var(--text-muted)", fontSize: "0.85rem" }}>
          No fact here matches “{term}”.
        </div>
      ) : (
        visible.map((row) => <Row key={row.graphNodeId} row={row} />)
      )}

      <div style={{ padding: "0.6rem 1rem", fontSize: "0.78rem", color: "var(--text-muted)" }}>
        {visible.length} of {rows.length} included
      </div>
    </div>
  );
};

/** One Facts-table row. */
const Row: React.FC<{ row: WorkingRow }> = ({ row }) => (
  <div style={rowStyle}>
    <div style={{ display: "flex", gap: "0.5rem", alignItems: "baseline" }}>
      <span style={{ ...chipStyle, color: "var(--text-primary)" }}>{row.code ?? "—"}</span>
      <span style={{ fontSize: "0.95rem", lineHeight: 1.6, color: "var(--text-primary)" }}>
        {row.text}
      </span>
    </div>
    <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", alignItems: "center" }}>
      {row.bearsOn.map((accusation) => (
        <span key={accusation} style={chipStyle}>
          {accusation}
        </span>
      ))}
      <a
        href={row.pinpointHref}
        target="_blank"
        rel="noreferrer"
        style={{ ...chipStyle, color: "var(--accent-primary)", textDecoration: "none" }}
      >
        {row.pinpointLabel} ↗
      </a>
      <span style={{ ...chipStyle, marginLeft: "auto" }}>{row.statusLabel}</span>
    </div>
  </div>
);

export default WorkingView;
