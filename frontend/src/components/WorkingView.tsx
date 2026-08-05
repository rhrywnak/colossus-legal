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
  neighboursForDrop,
  orderedRows,
  splitBackground,
  type WorkingRow,
} from "./factsTable";
import { ghostButtonStyle } from "./scenarioSectionStyles";
import type { FactTier, ScenarioCard } from "../services/scenarioCards";
import type { HumanFactDto } from "../services/scenarioAugmentation";
import { fillCount, type LinkPanelWording } from "../services/evidenceLinks";
import FactRow from "./FactRow";

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
  /** Record a fact's weight (task 2.13). */
  onSetTier: (graphNodeId: string, tier: FactTier) => void;
  /** Record where a dragged fact landed, named by its two new neighbours. */
  onMoveFact: (
    graphNodeId: string,
    after: string | null,
    before: string | null,
  ) => void;
}

const WorkingView: React.FC<Props> = ({
  cards,
  humanFacts,
  onAdd,
  onRemoveHumanFact,
  onRemoveFact,
  wording,
  onSetTier,
  onMoveFact,
}) => {
  const [term, setTerm] = useState("");
  // Which row is being dragged, for the duration of the drag only. Not state the
  // server knows or needs to: the drop is what gets written.
  const [dragging, setDragging] = useState<string | null>(null);
  // Whether the background pile is open. Seeded from the SERVER's decision (it
  // parsed the stored two-token setting), then follows the human's clicks for
  // this visit. `null` means "not told yet" — see the fallback below.
  const [backgroundOpen, setBackgroundOpen] = useState<boolean | null>(null);

  // Evidence first, then the human facts — the order the mockup shows, and the
  // order that reads as "what the record says, then what we know".
  // Ordered ONCE, here: weight first, then the human's own placement. An
  // untouched scenario comes out exactly as the server sent it (see
  // `orderedRows`), so this is a no-op until somebody drags something.
  const rows = useMemo(
    () => orderedRows([...includedRows(cards), ...humanFactRows(humanFacts)]),
    [cards, humanFacts],
  );
  const visible = useMemo(() => filterRows(rows, term), [rows, term]);
  // The background tier is FOLDED, never filtered away: the count travels with
  // the pile so the list can always say how much is down there.
  const { shown, background } = useMemo(() => splitBackground(visible), [visible]);
  const showBackground = backgroundOpen ?? !(wording?.fact_background_starts_collapsed ?? true);

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
        <>
          {shown.map((row) => (
            <FactRow
              key={row.graphNodeId}
              row={row}
              wording={wording}
              justArrived={arrived.has(row.graphNodeId)}
              // Item G: EVERY row can be taken out from where it is now. A human
              // fact is deleted outright (it exists nowhere else); an evidence
              // fact returns to the queue as not ruled. Two different acts, which
              // is why the confirmation names what will happen.
              //
              // The evidence control is WITHHELD until the wording has loaded —
              // `undefined`, not a control with an invented label (R4).
              onRemove={
                row.isHuman
                  ? () => onRemoveHumanFact(row.graphNodeId.replace(/^human:/, ""))
                  : wording
                    ? () => onRemoveFact(row.graphNodeId)
                    : undefined
              }
              // A human fact carries no weight tier (§8 — it is not evidence),
              // so it gets no weight control rather than a disabled one.
              onSetTier={
                row.isHuman ? undefined : (tier) => onSetTier(row.graphNodeId, tier)
              }
              onDragStart={row.isHuman ? undefined : () => setDragging(row.graphNodeId)}
              onDropOn={
                row.isHuman
                  ? undefined
                  : () => {
                      if (!dragging) return;
                      const pair = neighboursForDrop(rows, dragging, row.graphNodeId);
                      setDragging(null);
                      // A drop that cannot name a position (onto itself, or onto a
                      // row that has gone) is not sent: the server would only have
                      // to refuse it, and the human meant nothing by it.
                      if (pair) onMoveFact(dragging, pair.after, pair.before);
                    }
              }
              confirm={row.isHuman ? null : wording}
            />
          ))}

          {/* The background pile: folded, never hidden. The count is always on
              screen, so a curated fact can never silently vanish — which is the
              whole reason this tier is a fold and not a filter. */}
          {background.length > 0 && wording && (
            <div style={{ padding: "0.6rem 1rem", borderBottom: HAIRLINE }}>
              <button
                type="button"
                onClick={() => setBackgroundOpen(!showBackground)}
                style={{ ...ghostButtonStyle, fontSize: "0.78rem" }}
              >
                {showBackground
                  ? wording.fact_background_hide_label
                  : fillCount(wording.fact_background_count_template, background.length)}
              </button>
            </div>
          )}

          {showBackground &&
            background.map((row) => (
              <FactRow
                key={row.graphNodeId}
                row={row}
                wording={wording}
                justArrived={arrived.has(row.graphNodeId)}
                onRemove={wording ? () => onRemoveFact(row.graphNodeId) : undefined}
                onSetTier={(tier) => onSetTier(row.graphNodeId, tier)}
                onDragStart={() => setDragging(row.graphNodeId)}
                onDropOn={() => {
                  if (!dragging) return;
                  const pair = neighboursForDrop(rows, dragging, row.graphNodeId);
                  setDragging(null);
                  if (pair) onMoveFact(dragging, pair.after, pair.before);
                }}
                confirm={wording}
              />
            ))}
        </>
      )}

      </div>

      <div style={{ padding: "0.6rem 1rem", fontSize: "0.78rem", color: "var(--text-muted)" }}>
        {visible.length} of {rows.length} shown
      </div>
    </div>
  );
};

export default WorkingView;
