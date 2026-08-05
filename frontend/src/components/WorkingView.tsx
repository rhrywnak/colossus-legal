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
import {
  fillCode,
  fillCount,
  fillCounts,
  type LinkPanelWording,
} from "../services/evidenceLinks";
import FactRow, { CARD_GAP_PX } from "./FactRow";
import { useDragAutoScroll } from "./dragAutoScroll";

const SURFACE = "var(--bg-surface)";
const HAIRLINE = "1px solid var(--border-default)";

// CONST: how long a newly-added card stays tinted, in milliseconds.
//
// Presentational, and therefore NOT a stored setting (ruling R5 draws exactly
// this line): it changes how long a colour fades, never what the list contains,
// which cards exist, or what any of them mean. A tunable changes BEHAVIOUR; this
// changes an animation.
const ARRIVAL_HIGHLIGHT_MS = 2400;

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
  // Ruling 2, the spacing rhythm: each fact is its own card and the gap BETWEEN
  // cards is decisively larger than any gap inside one — `CARD_GAP_PX` (20) against
  // `MAX_INTRA_GAP_PX` (6), a ratio of 3.33. Proximity is what separates the cards;
  // the hairline assists. `scenarioPageStructure.test.ts` asserts the ratio so the
  // two numbers cannot drift back together.
  //
  // Task 2.13c: that space is NOT a flex `gap` any more. A flex gap belongs to the
  // container, which has no drop handler, so 2.13b turned every seam between two
  // cards into 20px of dead zone — and the seam is exactly where you aim when you
  // want a card to go BETWEEN two others. Roman's drag landed there and nothing
  // happened, with no event to report. The space is now each card's own bottom
  // margin, so it is part of that card's drop target and every pixel of the list
  // belongs to something.
  display: "flex",
  flexDirection: "column",
  // The stack sits ON the tinted page rather than inside a white slab: a white
  // card on a white surface has nothing to be seen against, which is half of
  // "difficult to see where one card ends and the next starts".
  padding: `${CARD_GAP_PX}px 16px`,
  background: "var(--bg-page)",
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
  /** Record a fact's weight. RESOLVES when stored and REJECTS when refused —
   *  the rejection is what retracts an optimistic move notice. */
  onSetTier: (graphNodeId: string, tier: FactTier) => Promise<void>;
  /** Record where a dragged fact landed, named by its two new neighbours. */
  onMoveFact: (
    graphNodeId: string,
    after: string | null,
    before: string | null,
  ) => void;
  /** Forget where a fact was placed, returning it to the natural order. */
  onUnplaceFact: (graphNodeId: string) => void;
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
  onUnplaceFact,
}) => {
  const [term, setTerm] = useState("");
  // Which row is being dragged, for the duration of the drag only. Not state the
  // server knows or needs to: the drop is what gets written.
  const [dragging, setDragging] = useState<string | null>(null);
  // Whether the background pile is open. Seeded from the SERVER's decision (it
  // parsed the stored two-token setting), then follows the human's clicks for
  // this visit. `null` means "not told yet" — see the fallback below.
  const [backgroundOpen, setBackgroundOpen] = useState<boolean | null>(null);
  // The card that most recently moved into the background pile, so the list can
  // SAY so. Task 2.13c item 5 — Roman's report was that clicking the dot made a
  // card "silently vanish"; a fact that leaves the list must announce where it
  // went. Cleared when the pile is opened, because then it is no longer hidden.
  const [movedToBackground, setMovedToBackground] = useState<string | null>(null);
  // The facts list scrolls inside its own region, and nothing moved that region
  // during a drag — so a card could not be taken past the visible window and the
  // scrollbar was unreachable with a card in hand. See `dragAutoScroll`.
  const autoScroll = useDragAutoScroll();

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

  /**
   * Set a weight, and if that weight FOLDS the card away, say so.
   *
   * The notice is raised optimistically, because it describes what the human just
   * asked for and the card leaves the list the moment the re-read lands.
   *
   * ## Why the failure path is wired and not assumed
   *
   * An earlier version raised the notice and left it there, on the assumption
   * that a failed write would somehow drop it. Nothing did. A refused tier write
   * would have rendered the error banner AND "{code} moved to the background
   * pile" at once, with the card still sitting in the list — two contradictory
   * messages, one of them a lie about where a fact went. So the write's rejection
   * retracts the notice explicitly, and the error banner is left to speak alone.
   */
  const setTierAnnouncing = (graphNodeId: string, tier: FactTier) => {
    const code = rows.find((r) => r.graphNodeId === graphNodeId)?.code ?? null;
    setMovedToBackground(tier === "background" ? code : null);
    onSetTier(graphNodeId, tier).catch(() => setMovedToBackground(null));
  };
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
    <div
      style={{
        background: SURFACE,
        borderRadius: "var(--radius-card)",
        border: "1px solid var(--border-card)",
        overflow: "hidden",
      }}
    >
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
      <div
        ref={autoScroll.regionRef}
        style={factsScrollRegionStyle}
        // These fire by BUBBLING from the cards, which already call
        // `preventDefault` on dragover to make themselves legal drop targets.
        // The region only needs the cursor position and the moments a drag ends.
        onDragOver={autoScroll.onDragOver}
        onDrop={autoScroll.stop}
        onDragEnd={autoScroll.stop}
        onDragLeave={autoScroll.onDragLeave}
      >
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
        <FactStack
          movedToBackground={movedToBackground}
          onNoticeCleared={() => setMovedToBackground(null)}
          onUnplaceFact={onUnplaceFact}
          shown={shown}
          background={background}
          showBackground={showBackground}
          onToggleBackground={() => setBackgroundOpen(!showBackground)}
          rows={rows}
          arrived={arrived}
          wording={wording}
          dragging={dragging}
          setDragging={setDragging}
          onRemoveFact={onRemoveFact}
          onRemoveHumanFact={onRemoveHumanFact}
          onSetTier={setTierAnnouncing}
          onMoveFact={onMoveFact}
        />
      )}

      </div>

      {/* Task 2.13c item 9: the old line read "48 of 48 shown" while ten of those
          facts were folded away in the background pile. Both numbers are now
          named separately, from a stored template, because there is no honest
          single number for two different things. */}
      <div style={{ padding: "0.6rem 1rem", fontSize: "0.78rem", color: "var(--text-muted)" }}>
        {wording
          ? fillCounts(wording.fact_footer_template, shown.length, background.length)
          : null}
      </div>
    </div>
  );
};

/**
 * The cards themselves: the shown stack, then the folded background pile.
 *
 * Extracted from `WorkingView` in task 2.13b. That component was carrying the
 * search header, the arrival bookkeeping, the empty states AND every card's drag
 * wiring in one body; the card stack is the part with its own rules and it reads
 * better alone. Pure presentation — every decision it makes was made upstream.
 */
const FactStack: React.FC<{
  shown: WorkingRow[];
  background: WorkingRow[];
  showBackground: boolean;
  onToggleBackground: () => void;
  /** The FULL ordered list, which a drop needs to name its neighbours from. */
  rows: WorkingRow[];
  arrived: Set<string>;
  wording: LinkPanelWording | null;
  dragging: string | null;
  setDragging: (id: string | null) => void;
  onRemoveFact: (graphNodeId: string) => void;
  onRemoveHumanFact: (factId: string) => void;
  onSetTier: (graphNodeId: string, tier: FactTier) => void;
  onMoveFact: (graphNodeId: string, after: string | null, before: string | null) => void;
  onUnplaceFact: (graphNodeId: string) => void;
  /** The code of the card most recently folded into the pile, or `null`. */
  movedToBackground: string | null;
  onNoticeCleared: () => void;
}> = ({
  shown,
  background,
  showBackground,
  onToggleBackground,
  rows,
  arrived,
  wording,
  dragging,
  setDragging,
  onRemoveFact,
  onRemoveHumanFact,
  onSetTier,
  onMoveFact,
  onUnplaceFact,
  movedToBackground,
  onNoticeCleared,
}) => {
  /** One card's props — identical for the shown stack and the background pile. */
  const cardFor = (row: WorkingRow) => ({
    key: row.graphNodeId,
    row,
    wording,
    justArrived: arrived.has(row.graphNodeId),
    // Item G: EVERY card can be taken out from where it is now. A human fact is
    // deleted outright (it exists nowhere else); an evidence fact returns to the
    // queue as not ruled. Two different acts, which is why the confirmation
    // names what will happen. The evidence control is WITHHELD until the wording
    // loads — `undefined`, not a control with an invented label (R4).
    onRemove: row.isHuman
      ? () => onRemoveHumanFact(row.graphNodeId.replace(/^human:/, ""))
      : wording
        ? () => onRemoveFact(row.graphNodeId)
        : undefined,
    // A human fact carries no weight tier (§8 — it is not evidence), so it gets
    // no weight control rather than a disabled one.
    onSetTier: row.isHuman
      ? undefined
      : (tier: FactTier) => onSetTier(row.graphNodeId, tier),
    onDragStart: row.isHuman ? undefined : () => setDragging(row.graphNodeId),
    onDropOn: row.isHuman
      ? undefined
      : () => {
          if (!dragging) return;
          const pair = neighboursForDrop(rows, dragging, row.graphNodeId);
          setDragging(null);
          // A drop that cannot name a position (onto itself, or onto a card that
          // has gone) is not sent: the server would only have to refuse it, and
          // the human meant nothing by it.
          if (pair) onMoveFact(dragging, pair.after, pair.before);
        },
    confirm: row.isHuman ? null : wording,
    onUnplace: row.isHuman ? undefined : () => onUnplaceFact(row.graphNodeId),
  });

  return (
    <>
      {shown.map((row) => (
        <FactRow {...cardFor(row)} />
      ))}

      {/* Task 2.13c item 5: a card that just left the list SAYS where it went.
          The pile's own count already made the facts findable; this makes the
          MOVE observable, which is what Roman meant by "silent vanish". */}
      {movedToBackground && wording && (
        <div
          role="status"
          style={{
            padding: "0.4rem 0.6rem",
            fontSize: "0.8rem",
            color: "var(--text-primary)",
            background: "var(--state-warning-bg-soft)",
            borderRadius: "6px",
          }}
        >
          {fillCode(wording.fact_background_move_notice, movedToBackground)}
        </div>
      )}

      {/* The background pile: folded, never hidden. The count is always on
          screen, so a curated fact can never silently vanish — which is the
          whole reason this tier is a fold and not a filter. */}
      {background.length > 0 && wording && (
        <div style={{ padding: "0.2rem 0" }}>
          <button
            type="button"
            onClick={() => {
              onNoticeCleared();
              onToggleBackground();
            }}
            style={{ ...ghostButtonStyle, fontSize: "0.8rem" }}
          >
            {showBackground
              ? wording.fact_background_hide_label
              : fillCount(wording.fact_background_count_template, background.length)}
          </button>
        </div>
      )}

      {/* Same anatomy inside the pile (ruling 3) — these are the same cards, not
          a reduced rendering of them. */}
      {showBackground && background.map((row) => <FactRow {...cardFor(row)} />)}
    </>
  );
};

export default WorkingView;
