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

import React, { useEffect, useMemo, useRef, useState } from "react";

import {
  arrivedIds,
  filterRows,
  humanFactRows,
  includedRows,
  type WorkingRow,
} from "./factsTable";
import { ghostButtonStyle } from "./scenarioSectionStyles";
import { openViewerWindow } from "./viewerWindow";
import type { ScenarioCard } from "../services/scenarioCards";
import type { HumanFactDto } from "../services/scenarioAugmentation";
import type { LinkPanelWording } from "../services/evidenceLinks";
import RemoveControl from "./FactRemoveControl";

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
 * The row that just landed (task 1.7F Part A).
 *
 * A ruling made in the queue above adds a row to a list that can be long, and a
 * human who cannot see WHICH row appeared has to go looking for their own work.
 * The tint fades out on its own; nothing about the row's meaning depends on it.
 */
const arrivedRowStyle: React.CSSProperties = {
  ...rowStyle,
  background: "var(--state-warning-bg-soft)",
  transition: "background 600ms ease-out",
};

// CONST: how long a newly-added row stays tinted, in milliseconds.
//
// Presentational, and therefore NOT a stored setting (ruling R5 draws exactly
// this line): it changes how long a colour fades, never what the list contains,
// which rows exist, or what any of them mean. A tunable is something that changes
// BEHAVIOUR; this changes an animation.
const ARRIVAL_HIGHLIGHT_MS = 2400;

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

/**
 * An accusation chip, which unlike the others is a SENTENCE (item F).
 *
 * ¶41's label runs to about two hundred characters. `nowrap` on a chip that long
 * pushes it past the container's right edge, where `overflow: hidden` cuts it —
 * so the end of the accusation, which is the part that distinguishes it from its
 * neighbours, was the part you could not read.
 *
 * The short chips (pinpoint, status) keep `nowrap`: they are labels, they fit,
 * and letting "CFS responses at 26" break across two lines would look broken.
 */
const accusationChipStyle: React.CSSProperties = {
  ...chipStyle,
  whiteSpace: "normal",
  overflowWrap: "anywhere",
  textAlign: "left",
};

/**
 * The rows' scroll region (item E).
 *
 * `60vh` rather than the queue's `70vh`: this section sits BELOW the candidate
 * queue on the same page, and two 70vh regions stacked would mean neither is
 * fully visible at once on a laptop. The scrollbar is deliberately not hidden —
 * it is the only thing that says there are more facts below the fold.
 */
const factsScrollRegionStyle: React.CSSProperties = {
  maxHeight: "60vh",
  overflowY: "auto",
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
  /** Remove one human fact — deleted outright; it exists nowhere else. */
  onRemoveHumanFact: (factId: string) => void;
  /** Take one EVIDENCE fact back out of the scenario (task 2.12, item G). */
  onRemoveFact: (graphNodeId: string) => void;
  /** The stored words. `null` until they load — no Remove control until then. */
  wording: LinkPanelWording | null;
}

const WorkingView: React.FC<Props> = ({
  cards,
  humanFacts,
  onAdd,
  onRemoveHumanFact,
  onRemoveFact,
  wording,
}) => {
  const [term, setTerm] = useState("");

  // Evidence first, then the human facts — the order the mockup shows, and the
  // order that reads as "what the record says, then what we know".
  const rows = useMemo(
    () => [...includedRows(cards), ...humanFactRows(humanFacts)],
    [cards, humanFacts],
  );
  const visible = useMemo(() => filterRows(rows, term), [rows, term]);

  // Which rows are NEW since the last render of this list (task 1.7F Part A).
  //
  // Derived by comparing ids against the previous set rather than being told by
  // the ruling path: the facts list is drawn from what the SERVER returned
  // (ruling R3 — no optimistic rows), so the honest definition of "just arrived"
  // is "present now, absent from the last payload". A row that arrives because
  // somebody else ruled it, or because a merge added it, highlights too — which
  // is correct, since the point is to show what changed under the reader.
  const seenIds = useRef<Set<string> | null>(null);
  const [arrived, setArrived] = useState<Set<string>>(new Set());

  useEffect(() => {
    const ids = rows.map((r) => r.graphNodeId);
    const fresh = arrivedIds(seenIds.current, ids);
    seenIds.current = new Set(ids);
    if (fresh.length === 0) return;

    setArrived(new Set(fresh));
    const timer = setTimeout(() => setArrived(new Set()), ARRIVAL_HIGHLIGHT_MS);
    // Cleared on unmount and before the next arrival, so a fast second ruling
    // cannot leave a stale timer to blank the newer highlight early.
    return () => clearTimeout(timer);
  }, [rows]);

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

      {/* Item E: forty-six facts made the page enormous. The rows scroll in their
          own box; the search field, the create button and the "N of M shown"
          count stay OUTSIDE it, so nothing that acts on the list can be pushed
          away by the list — the same treatment, and the same reasoning, as the
          candidate queue's scroll region.

          Presentational geometry, not a Rule-13 tunable: this serves every row
          and only chooses how much glass they are seen through (the standing
          ruling from `CandidateList.scrollRegionStyle`). */}
      <div style={factsScrollRegionStyle}>
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
            justArrived={arrived.has(row.graphNodeId)}
            // Item G: EVERY row can be taken out from where it is now. A human
            // fact is deleted outright (it exists nowhere else); an evidence
            // fact returns to the queue as not ruled. Two different acts, which
            // is why the confirmation names what will happen.
            //
            // The evidence control is WITHHELD until the wording has loaded —
            // `undefined`, not a control with an invented label. R4 leaves no
            // literal to fall back to, and an unlabelled control that removes a
            // ruling is worse than no control for the seconds it takes to load.
            onRemove={
              row.isHuman
                ? () => onRemoveHumanFact(row.graphNodeId.replace(/^human:/, ""))
                : wording
                  ? () => onRemoveFact(row.graphNodeId)
                  : undefined
            }
            confirm={row.isHuman ? null : wording}
          />
        ))
      )}

      </div>

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
  confirm?: LinkPanelWording | null;
}> = ({ row, onBlocked, onRemove, confirm = null }) => (
  <div style={{ display: "flex", gap: "0.4rem", flexWrap: "wrap", alignItems: "center" }}>
    {row.bearsOn.map((accusation) => (
      <span key={accusation} style={accusationChipStyle}>
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
    {onRemove && <RemoveControl row={row} onRemove={onRemove} confirm={confirm} />}
  </div>
);

/** One Facts-table row. */
const Row: React.FC<{
  row: WorkingRow;
  /** Tinted briefly because this row was not in the previous payload. */
  justArrived?: boolean;
  onRemove?: () => void;
  /**
   * The stored wording for the removal confirmation, or `null` to skip it.
   *
   * `null` for a human fact — it is the author's own text and deleting it needs
   * no explanation of where it goes, because it goes nowhere. Non-null for an
   * evidence fact, which returns to the queue and where the human must be told
   * that before they commit (item G).
   */
  confirm?: LinkPanelWording | null;
}> = ({ row, justArrived = false, onRemove, confirm = null }) => {
  // A refused popup has to be SAID, not swallowed (Standing Rule 1). Local to the
  // row so the message appears where the human just clicked.
  const [blocked, setBlocked] = useState<string | null>(null);

  return (
    <div style={justArrived ? arrivedRowStyle : rowStyle}>
      {/* Item 6: the coloured left-edge cue. */}
      <span style={stripeStyle(row.isHuman)} aria-hidden="true" />
      <div style={{ display: "flex", flexDirection: "column", gap: "0.4rem", flex: 1 }}>
        {/* Item F: `minWidth: 0` on the text below is what makes it WRAP.
            A flex item defaults to `min-width: auto`, so it refuses to shrink
            below its content; the card container sets `overflow: hidden`, so the
            overflow was clipped and the end of every long quote was unreadable.
            The flex row itself wraps too, so a very long code + quote pair moves
            to two lines rather than squeezing the code. */}
        <div style={{ display: "flex", gap: "0.5rem", alignItems: "baseline", flexWrap: "wrap" }}>
          {/* A human fact has no candidate number, and the mockup's human row has no
              code cell. An em-dash placeholder implied a missing value where there
              is no value to miss. */}
          {row.code && (
            <span style={{ ...chipStyle, color: "var(--text-primary)" }}>{row.code}</span>
          )}
          <span
            style={{
              fontSize: "0.95rem",
              lineHeight: 1.6,
              color: "var(--text-primary)",
              minWidth: 0,
              flex: 1,
              // Belt to that braces: a single unbroken token (a URL in a quote)
              // has no space to wrap at, and would clip exactly as before.
              overflowWrap: "anywhere",
            }}
          >
            {row.text}
          </span>
        </div>

        <RowMeta row={row} onBlocked={setBlocked} onRemove={onRemove} confirm={confirm} />

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
