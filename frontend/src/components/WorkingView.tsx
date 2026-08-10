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
  orderedRows,
  splitBackground,
} from "./factsTable";
import { ghostButtonStyle } from "./scenarioSectionStyles";
import type { FactTier, ScenarioCard } from "../services/scenarioCards";
import type { HumanFactDto } from "../services/scenarioAugmentation";
import {
  fillCounts,
  fillSlots,
  type AllegationOptions,
  type LinkPanelWording,
} from "../services/evidenceLinks";
import { tierLabels } from "./WeightPicker";
import type { ChipFilter } from "./evidenceCardModel";
import { matchesChip } from "./evidenceCardModel";
import { CARD_GAP_PX } from "./FactRow";
import FactStack from "./FactStack";
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
 * The rows' box — NO LONGER A SCROLLPORT (task R2).
 *
 * It was `maxHeight: 60vh; overflowY: auto`, chosen so that two stacked regions
 * (this one and the queue's 70vh) would each be partly visible on a laptop. That
 * reasoning conceded the problem it was managing: a page with two inner
 * scrollports has three scrollbars, and a wheel does something different
 * depending on which third of the page the pointer is over.
 *
 * Both are gone. Roman's acceptance for .391 is one continuous scroll from the
 * header to the watch-list, and this section sits between them — leaving 60vh
 * here would have moved the jam down the page rather than removing it.
 */
const factsScrollRegionStyle: React.CSSProperties = {
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
  /** The card's own words and fold thresholds (ONE_CARD_GRAMMAR). */
  options: AllegationOptions | null;
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
}

const WorkingView: React.FC<Props> = ({
  cards,
  humanFacts,
  onAdd,
  onRemoveHumanFact,
  onRemoveFact,
  wording,
  options,
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
  /**
   * The last weight change, as the sentence announcing it and the row it was
   * about (Piece 5a).
   *
   * Task 2.13c already said something when a card was folded away; this widens
   * it to EVERY weight change and adds the way back. Roman was moved to the
   * background pile by a control he had signed off three days earlier — an
   * acknowledgment with no undo tells you what happened and leaves you to
   * reverse it by hand.
   *
   * `pendingNodeId` is what keeps the card where it is while the sentence is on
   * screen: `splitBackground` honours it, so a demotion does not slide the list
   * out from under the cursor at the moment the human acts.
   */
  const [weightNotice, setWeightNotice] = useState<{
    text: string;
    nodeId: string;
    previous: FactTier;
  } | null>(null);
  /** A chip the human clicked, narrowing this list to it (Piece 7). */
  const [chipFilter, setChipFilter] = useState<ChipFilter | null>(null);
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
  const searched = useMemo(() => filterRows(rows, term), [rows, term]);
  // Piece 7: a clicked chip narrows this list to its raw value. Applied AFTER the
  // search box rather than instead of it — the two are different narrowings and a
  // human may want both, and matching on the payload value means a chip can never
  // surface a row whose chip says something else.
  const visible = useMemo(
    () =>
      chipFilter
        ? searched.filter((row) => row.card !== null && matchesChip(row.card, chipFilter))
        : searched,
    [searched, chipFilter],
  );
  // The background tier is FOLDED, never filtered away: the count travels with
  // the pile so the list can always say how much is down there. A row whose
  // demotion is still being acknowledged stays put — see `splitBackground`.
  const pendingDemotion = useMemo(
    () => new Set(weightNotice ? [weightNotice.nodeId] : []),
    [weightNotice],
  );
  const { shown, background } = useMemo(
    () => splitBackground(visible, pendingDemotion),
    [visible, pendingDemotion],
  );

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
    const row = rows.find((r) => r.graphNodeId === graphNodeId);
    if (!row || !wording || !options) return;
    const previous = row.tier ?? "backup";

    // Raised optimistically, because it describes what the human just asked for
    // and the row is held in place until they dismiss it. A refused write
    // RETRACTS it explicitly: an earlier version left the notice standing, so a
    // failed tier write would have rendered the error banner and "C-91 now reads
    // Background" at once — two contradictory messages, one of them a lie about
    // where a fact went.
    setWeightNotice({
      nodeId: graphNodeId,
      previous,
      text: fillSlots(options.card_grammar.weight_changed_template, {
        code: row.code ?? graphNodeId,
        tier: tierLabels(wording)[tier],
      }),
    });
    onSetTier(graphNodeId, tier).catch((e: unknown) => {
      // NOT a swallowed failure. `ScenarioFactsSection.changeTier` fills the
      // stored `fact_tier_save_failed_template` into the section's error banner
      // and RE-THROWS precisely so this handler can retract the optimistic
      // notice — the two halves are one flow, and the rejection arriving here is
      // how the retraction is triggered rather than a second thing to report.
      //
      // Retracting matters as much as reporting: an earlier version left the
      // notice standing, so a refused write rendered the error banner AND
      // "C-91 now reads Background" at once — two contradictory messages, one of
      // them a lie about where a fact went.
      //
      // The `warn` is what makes the chain legible from a console alone, so a
      // reader does not have to know the section above re-throws to be sure the
      // failure was not dropped here.
      setWeightNotice(null);
      console.warn(
        "a weight write was refused; the section's error banner carries the reason",
        e,
      );
    });
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
      {/* Piece 7: a list narrowed by a chip SAYS what it is narrowed to, and
          offers the way out. A filter with no visible exit is how a human ends
          up reporting that facts have gone missing. */}
      {chipFilter && options && (
        <div
          role="status"
          style={{
            display: "flex",
            gap: "0.5rem",
            alignItems: "center",
            padding: "8px 16px",
            fontSize: "0.82rem",
            color: "var(--text-secondary)",
            borderBottom: HAIRLINE,
          }}
        >
          <button
            type="button"
            onClick={() => setChipFilter(null)}
            style={{
              border: "1px solid var(--accent-primary)",
              background: "var(--state-info-bg-soft)",
              color: "var(--accent-primary)",
              borderRadius: "999px",
              padding: "3px 12px",
              cursor: "pointer",
              fontFamily: "inherit",
              fontSize: "0.8rem",
            }}
          >
            {fillSlots(options.card_grammar.chip_filter_clear_template, {
              value: chipFilter.value,
            })}
          </button>
        </div>
      )}

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
          weightNotice={weightNotice}
          onUndoWeight={() => {
            if (!weightNotice) return;
            // Undo re-applies the PREVIOUS tier through the same write path, so
            // the record shows what actually happened: two human acts, not one
            // that was rolled back behind the scenes.
            const { nodeId, previous } = weightNotice;
            setWeightNotice(null);
            onSetTier(nodeId, previous).catch((e: unknown) => {
              // Same chain as `setTierAnnouncing` above: `changeTier` shows the
              // stored refusal and re-throws. There is nothing to RETRACT here —
              // the notice was cleared on the line before the call, because undo
              // is a new act rather than a rollback of the one being announced —
              // so this handler exists only to keep the rejection from being
              // dropped without a word.
              console.warn(
                "an undo of a weight change was refused; the section's error banner carries the reason",
                e,
              );
            });
          }}
          onNoticeCleared={() => setWeightNotice(null)}
          options={options}
          onFilterChip={setChipFilter}
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

export default WorkingView;
