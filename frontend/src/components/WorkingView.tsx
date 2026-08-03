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

import { filterRows, humanFactRows, includedRows, type WorkingRow } from "./factsTable";
import { ghostButtonStyle } from "./scenarioSectionStyles";
import { openViewerWindow } from "./viewerWindow";
import type { ScenarioCard } from "../services/scenarioCards";
import type { HumanFactDto } from "../services/scenarioAugmentation";

const SURFACE = "var(--bg-surface)";
const HAIRLINE = "1px solid var(--border-default)";

/**
 * One fact row. Mockup `.facts td`: 12px 16px 12px 12px, divided below.
 *
 * `display: flex` with the stripe as the first child — the stripe must be a
 * SIBLING of the content, not a border on the row, because it has to stretch to the
 * row's full height and stay rounded at both ends. R6 kept the flex rows rather than
 * converting to a table for CSS convenience.
 */
const rowStyle: React.CSSProperties = {
  display: "flex",
  gap: "12px",
  padding: "12px 16px 12px 12px",
  borderBottom: HAIRLINE,
  fontWeight: 400,
  alignItems: "stretch",
};

/**
 * The coloured left-edge cue. Mockup `.acc i`: 4px wide, radius 2, full height.
 *
 * Green = evidence a human ruled in. Blue = a fact a human wrote. It is a cue, not
 * the only signal — the row's own provenance line still says which it is in words,
 * because a colour alone fails a colourblind reader and a greyscale print.
 */
const stripeStyle = (isHuman: boolean): React.CSSProperties => ({
  width: "4px",
  borderRadius: "2px",
  flexShrink: 0,
  alignSelf: "stretch",
  minHeight: "44px",
  background: isHuman ? "var(--accent-primary)" : "var(--state-success-strong)",
});

const chipStyle: React.CSSProperties = {
  border: HAIRLINE,
  borderRadius: "999px",
  padding: "0.1rem 0.55rem",
  fontSize: "0.75rem",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
};

// Mockup `input[type=search]`: borderless on the chrome fill, radius 8.
const searchStyle: React.CSSProperties = {
  border: "none",
  borderRadius: "8px",
  background: "var(--v3-chrome)",
  padding: "8px 12px",
  fontFamily: "inherit",
  fontSize: "13.5px",
  fontWeight: 400,
  minWidth: "16rem",
  flex: 1,
  maxWidth: "340px",
};

interface Props {
  cards: ScenarioCard[];
  /**
   * C4 — facts a human wrote. They join the SAME table as the evidence (task 1.7D
   * item 6), distinguished by the row's coloured stripe and its provenance line.
   */
  humanFacts: HumanFactDto[];
  /** Opens the add-fact form — the one create action on this surface. */
  onAdd: () => void;
  /** Remove one human fact. Evidence rows are un-ruled in the queue, not here. */
  onRemoveHumanFact: (factId: string) => void;
}

const WorkingView: React.FC<Props> = ({ cards, humanFacts, onAdd, onRemoveHumanFact }) => {
  const [term, setTerm] = useState("");

  // Evidence first, then the human facts — the order the mockup shows, and the
  // order that reads as "what the record says, then what we know".
  const rows = useMemo(
    () => [...includedRows(cards), ...humanFactRows(humanFacts)],
    [cards, humanFacts],
  );
  const visible = useMemo(() => filterRows(rows, term), [rows, term]);

  return (
    <div style={{ background: SURFACE, borderRadius: "var(--radius-card)", boxShadow: "var(--shadow-card)", overflow: "hidden" }}>
      {/* Search top-left, one create button top-right — the study's list-screen
          header, unchanged. */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "1rem",
          padding: "12px 16px",
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
        <button type="button" onClick={onAdd} style={ghostButtonStyle}>
          + Add human fact
        </button>
      </div>

      {rows.length === 0 ? (
        <div style={{ padding: "1rem", color: "var(--text-muted)", fontSize: "0.85rem" }}>
Nothing here yet. ✓ Include a candidate above, or add a fact of your own.
        </div>
      ) : visible.length === 0 ? (
        // A filter that matches nothing is a DIFFERENT state from an empty
        // scenario, and says so rather than looking like the latter.
        <div style={{ padding: "1rem", color: "var(--text-muted)", fontSize: "0.85rem" }}>
          No fact here matches “{term}”.
        </div>
      ) : (
        visible.map((row) => (
          <Row
            key={row.graphNodeId}
            row={row}
            onRemove={
              row.isHuman
                ? () => onRemoveHumanFact(row.graphNodeId.replace(/^human:/, ""))
                : undefined
            }
          />
        ))
      )}

      <div style={{ padding: "0.6rem 1rem", fontSize: "0.78rem", color: "var(--text-muted)" }}>
        {visible.length} of {rows.length} shown
      </div>
    </div>
  );
};

/**
 * The pinpoint chip, or nothing.
 *
 * Extracted from `Row` for the function-size limit (Rule 18). A HUMAN fact has no
 * pinpoint by design (§8), so the chip is absent rather than rendered empty or as a
 * dead link — which is also why this returns `null` rather than a placeholder.
 */
const PinpointChip: React.FC<{ row: WorkingRow; onBlocked: (m: string | null) => void }> = ({
  row,
  onBlocked,
}) => {
  if (!row.pinpointHref) return null;
  return (
    // §2.7 / defect D5: a pinpoint opens the dedicated viewer in a real sized
    // WINDOW, from the facts rows exactly as from the queue card — reading a fact
    // against its own page is the same job on both surfaces. Kept as an anchor with
    // an href so it still middle-clicks like a link.
    <a
      href={row.pinpointHref}
      onClick={(event) => {
        event.preventDefault();
        const result = openViewerWindow(row.pinpointHref);
        onBlocked(result.opened ? null : result.message);
      }}
      style={{ ...chipStyle, color: "var(--accent-primary)", textDecoration: "none" }}
    >
      {row.pinpointLabel} ↗
    </a>
  );
};

/** The row's second line: what it bears on, where it is, and its provenance. */
const RowMeta: React.FC<{
  row: WorkingRow;
  onBlocked: (m: string | null) => void;
  onRemove?: () => void;
}> = ({ row, onBlocked, onRemove }) => (
  <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", alignItems: "center" }}>
    {row.bearsOn.map((accusation) => (
      <span key={accusation} style={chipStyle}>
        {accusation}
      </span>
    ))}
    <PinpointChip row={row} onBlocked={onBlocked} />
    {/* For evidence this is the ruling state; for a human fact it is the composed
        "Added by Roman · Around Mar 2010". Either way it is the row's AUTHORITY,
        and it is the words that back up the coloured stripe on the left. */}
    <span style={{ ...chipStyle, marginLeft: "auto" }}>{row.statusLabel}</span>
    {/* Only a human fact can be removed from here — evidence is un-ruled in the
        queue, which is where its ruling was made. */}
    {onRemove && (
      <button
        type="button"
        onClick={onRemove}
        style={{
          border: "none",
          background: "none",
          color: "var(--text-muted)",
          cursor: "pointer",
          fontFamily: "inherit",
          fontSize: "12px",
        }}
      >
        Remove
      </button>
    )}
  </div>
);

/** One Facts-table row. */
const Row: React.FC<{ row: WorkingRow; onRemove?: () => void }> = ({ row, onRemove }) => {
  // A refused popup has to be SAID, not swallowed (Standing Rule 1). Local to the
  // row so the message appears where the human just clicked.
  const [blocked, setBlocked] = useState<string | null>(null);

  return (
    <div style={rowStyle}>
      {/* Item 6: the coloured left-edge cue. */}
      <span style={stripeStyle(row.isHuman)} aria-hidden="true" />
      <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem", flex: 1 }}>
        <div style={{ display: "flex", gap: "0.5rem", alignItems: "baseline" }}>
          {/* A human fact has no candidate number, and the mockup's human row has no
              code cell. An em-dash placeholder implied a missing value where there
              is no value to miss. */}
          {row.code && (
            <span style={{ ...chipStyle, color: "var(--text-primary)" }}>{row.code}</span>
          )}
          <span style={{ fontSize: "0.95rem", lineHeight: 1.6, color: "var(--text-primary)" }}>
            {row.text}
          </span>
        </div>

        <RowMeta row={row} onBlocked={setBlocked} onRemove={onRemove} />

        {blocked && (
          <div role="alert" style={{ fontSize: "0.78rem", color: "var(--state-danger-strong)" }}>
            {blocked}
          </div>
        )}
      </div>
    </div>
  );
};

export default WorkingView;
