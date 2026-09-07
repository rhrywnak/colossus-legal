// =============================================================================
// ScenarioFactsSection — C2 + C4 in one table (§2.4)
// =============================================================================
//
// What the scenario HAS: the evidence a human ruled in (C2) and the human facts
// they wrote themselves (C4), in one section rather than in a table plus a form
// three panels apart.
//
// ## The empty-state affordance (defect D10)
//
// The 1.7B section header was the word "Scenario facts" and a count. A human
// arriving at an empty scenario had no way to learn that an `I` ruling in the queue
// is what fills it. §2.4's copy says so — and says it differently at zero, because
// "an I ruling above moves the item here" is instruction and "they appear here when
// you rule I above" is an explanation of an emptiness.
//
// ## What is reserved and renders nothing
//
// The List / Timeline / By-allegation view toggle (2.6), hazard and ammunition cut
// tags (2.3), and per-fact annotations (§2d, Phase 2). All three have their place
// in this section in the design; none of them exists yet, and the Phase-1 law says
// a component that does not exist renders NOTHING rather than a greyed hint.
//
// ## ⚑ v2.1 (ruling R35): THREE SECTIONS BECAME ONE LIST
//
// "Scan & candidates" and "The accusation, and every time they made it" are no
// longer rendered by the scenario page. What they showed about the EVIDENCE is
// here, behind a three-way filter — Included · Candidates · All — over one card.
//
// The reason is a reader's, not an engineer's. Three stacked lists over one pool
// meant "is this fact already in?" was answered by scrolling between sections and
// holding two counts in your head. One list with three filters answers it in a
// click, and the filter names what you are looking at, which the three headings
// never quite did.
//
// ## What this section now owns, and what it borrows
//
//   the header      `ScenarioFactsHeader` — heading, last-scan sentence, Scan
//                   again, history, Reset order, the fold
//   the filter      `FactsFilterBar` over the pure `factsFilter`
//   Included        `WorkingView`, unchanged
//   Candidates      `CardQueue`, unchanged — the same rows, the same Include /
//                   rule controls and the same `CandidateFilterBar` the retired
//                   section showed, mounted here instead
//   the scan engine `ThemeScanPanel`, MOUNTED AND CHROMELESS. It must stay
//                   mounted (architect ruling R3): its mount effect calls
//                   `gatherCandidates`, the one place candidate ordinals are
//                   minted. It lends this section its three scan controls
//                   through `ScanHeaderApi` and draws no card of its own.
//
// ## The fold governs the LISTS, never the filter row
//
// Collapsed is still the arrival state and still remembered per scenario. What
// changed is what a closed section says: the pills stay on screen with their
// counts, so a folded list still declares how many facts are in it and how many
// candidates are waiting — which is what makes folding honest rather than a
// hiding place.

import React, { useState } from "react";

import AddHumanFactForm from "./AddHumanFactForm";
import CardQueue from "./CardQueue";
import FactsFilterBar from "./FactsFilterBar";
import ScenarioFactsHeader from "./ScenarioFactsHeader";
import ThemeScanPanel from "./ThemeScanPanel";
import { factsCounts, showsCandidates, showsIncluded, type FactsFilter } from "./factsFilter";
import { useSectionOpen } from "./sectionCollapse";
import WorkingView from "./WorkingView";
import { sectionPanelStyle } from "./scenarioSectionStyles";
import type { ProposalSource, ScenarioCard } from "../services/scenarioCards";
import type { TalkingPointDto } from "../services/scenarioAugmentation";
import type { HumanFactDto } from "../services/scenarioAugmentation";
import { deleteHumanFact } from "../services/scenarioAugmentation";
import { removeScenarioFact } from "../services/scenarioFacts";
import {
  fillCodeAndReason,
  fillDetail,
  type AllegationOptions,
  type LinkPanelWording,
} from "../services/evidenceLinks";
import {
  setFactOrder,
  setFactTier,
} from "../services/scenarioFactCuration";
import { resetFactOrder } from "./factsOrderReset";
import FactsResetOrder from "./FactsResetOrder";
import type { FactTier } from "../services/scenarioCards";

interface Props {
  slug: string;
  scenarioId: string;
  /** The card payload — the included ones become C2 rows. */
  cards: ScenarioCard[];
  /** C4: facts a human wrote, with no citation by design. */
  humanFacts: HumanFactDto[];
  /** Re-read the WHOLE page after a human fact is added or removed.
   *
   *  Human facts live in the augmentation payload, which only the page-level read
   *  covers — so this one has to be the heavy refresh. */
  onChanged: () => void;
  /**
   * Re-read only the CARDS, after an evidence fact is removed (task 2.12, G).
   *
   * ## Why this is not `onChanged`
   *
   * Removing an evidence fact IS a ruling — it goes through `record_removal` and
   * is ledgered as one. `ScenarioDetailPage` states the rule directly: the
   * page-level refresh "is correct after an edit to the scenario's own content,
   * and wrong after a ruling: it would disturb the queue's selection mid-triage,
   * which is precisely the class of defect task 1.7G spent two builds fixing."
   *
   * Measured on DEV (beta.374): wiring this to the page refresh collapsed the
   * candidate queue region on every removal, throwing the human out of the list
   * they were working — the two-pass problem this task exists to remove, in a new
   * costume. The cards-only read updates BOTH surfaces, because the facts list is
   * derived from the same cards the queue counts are.
   */
  onFactRemoved: () => void;
  /**
   * The stored words this section needs (task 2.12, item G).
   *
   * `null` until the scenario's panel wording has loaded. The Remove control is
   * then not rendered — there is no literal to fall back to (R4), and a control
   * that cannot state what it is about must not be offered at all.
   */
  wording: LinkPanelWording | null;
  /**
   * The whole allegation-options payload — the card's words and its two fold
   * thresholds (ONE_CARD_GRAMMAR).
   *
   * Separate from `wording` because that field is the LINK PANEL's block and
   * this section's controls read it directly; the fact CARD needs the grammar
   * block beside it, and threading the payload rather than a second wording prop
   * keeps one read reaching both.
   */
  options: AllegationOptions | null;
  // ── v2.1 (change D): what the retired "Scan & candidates" section used ─────
  /**
   * `true` while the page's card read is still in flight.
   *
   * The filter pills need it and `cards` cannot carry it: this section takes an
   * ARRAY (`WorkingView` maps over it) so an unread pool and an empty one arrive
   * identically. Collapsing those two is what put "No candidates gathered yet"
   * over a pool of 148 in .374 — the pills show their word with no number until
   * this goes false.
   */
  loading: boolean;
  /** Bumped when a scan merge writes candidate facts; relayed to the queue. */
  externalRefresh: number;
  /**
   * The served sentence for a scenario NO scan has ever touched, or `null`.
   *
   * Served, never inferred — the browser cannot tell "nothing has been scanned"
   * from "nothing scanned has been merged". Stands where the last-scan sentence
   * would be; see `ScenarioFactsHeader`.
   */
  neverScannedNotice: string | null;
  /** Which completed run is proposing the candidates, or `null`. Served. */
  proposalSource: ProposalSource | null;
  /**
   * A scan completed or a run was deleted — the page's whole payload is stale.
   *
   * The HEAVY refresh, and correctly so: a completed run changes the pool, the
   * proposal attribution and the served never-scanned notice at once.
   */
  onFactsChanged: () => void;
  /**
   * A ruling the SERVER confirmed, from the candidate queue (task 1.7F Part A).
   *
   * The LIGHT re-read: it updates this list and the queue's counts together
   * without disturbing the queue's selection mid-triage, which a page-level
   * refresh would — the class of defect task 1.7G spent two builds fixing.
   */
  onRulingSaved: () => void;
  /**
   * The scenario's live talking points, for each card's Backs picker (change E).
   *
   * This list is the ONE surface that offers the choice. See `FactCardBody` for
   * why the copy of a card under a talking point does not.
   */
  points: TalkingPointDto[];
}

const ScenarioFactsSection: React.FC<Props> = ({
  slug,
  scenarioId,
  cards,
  humanFacts,
  onChanged,
  onFactRemoved,
  wording,
  options,
  loading,
  externalRefresh,
  neverScannedNotice,
  proposalSource,
  onFactsChanged,
  onRulingSaved,
  points,
}) => {
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);
  /**
   * Whether the facts list itself is shown (task R4, P1b).
   *
   * COLLAPSED on arrival, and remembered per scenario (ruled 2026-08-28). It was
   * open on arrival on the reasoning that "the section is the scenario's
   * evidence and the page exists to show it" — true, and still true, but the
   * page shows two long sections stacked and opening both put the human several
   * screens from the top on every single visit. See `sectionCollapse` for what
   * this supersedes and why a folded section is not a hidden one: the count line
   * in the header below stays on screen, so a closed list still says how many
   * facts are in it.
   */
  const [open, toggleOpen] = useSectionOpen(scenarioId, "facts");
  /**
   * Which of the three lists is showing (v2.1, change D).
   *
   * `included` on arrival and deliberately NOT remembered, unlike the fold. The
   * fold is a preference about how much page you want; this is a place in a
   * workflow, and a section that greeted you with the candidate queue because
   * that is where you left off three days ago would answer a question you did
   * not ask. The default is the scenario's own content.
   */
  const [filter, setFilter] = useState<FactsFilter>("included");
  /**
   * Whether the run-history table is open.
   *
   * Behind a link rather than on the page, because it is the only surface in
   * this app that shows scan history and it is read rarely — checked before
   * this change: `ScanHistoryTable` has exactly one mount site, inside
   * `ThemeScanPanel`. Dropping it would have taken the only way to see, compare
   * or delete a run with it.
   */
  const [historyOpen, setHistoryOpen] = useState(false);
  /** Whether the Reset-order confirmation is open. */
  const [confirmingReset, setConfirmingReset] = useState(false);
  /** What the last reset did, or `null`. Every action acknowledges itself. */
  const [resetNotice, setResetNotice] = useState<string | null>(null);

  /** Remove one human fact, surfacing any refusal (Standing Rule 1). */
  const removeHumanFact = (factId: string) => {
    deleteHumanFact(slug, scenarioId, factId)
      .then(() => {
        setError(null);
        onChanged();
      })
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : "That fact could not be removed.");
      });
  };

  /**
   * Take one EVIDENCE fact back out of the scenario (item G).
   *
   * ## Why this is the removal path and not an exclude
   *
   * `removeScenarioFact` deletes the scenario's reference to the candidate. With
   * no reference the card is served with no ruling at all, so it re-enters the
   * queue as NOT RULED — which is what the human means by "not this scenario".
   * An exclude would mean "this is bad evidence", a different and much stronger
   * claim, and it would leave the card in the set-aside list rather than back in
   * the queue where it can be reconsidered.
   *
   * It is ledgered: the route goes through `record_removal`, which writes an
   * anchor row alongside the delete in one transaction, so the act survives the
   * row it removed.
   */
  const removeFact = (graphNodeId: string) => {
    removeScenarioFact(slug, scenarioId, graphNodeId)
      .then(() => {
        setError(null);
        // The 1.7F seam, and the LIGHT half of it: one cards read refreshes this
        // list and the queue's counts together, because both are derived from the
        // same payload — without disturbing the queue's selection, which a
        // page-level refresh would (see `onFactRemoved`).
        onFactRemoved();
      })
      .catch((e: unknown) => {
        // The words are the store's (R4); only the failure's own text is dropped
        // into the slot the sentence leaves for it — the same shape the link
        // writes use. `wording` is non-null here by construction: the control
        // that reaches this code is not rendered until it has loaded.
        const detail = e instanceof Error ? e.message : String(e);
        setError(
          wording
            ? fillDetail(wording.fact_remove_failed_template, detail)
            : detail,
        );
      });
  };

  /**
   * The code a message should name a fact by.
   *
   * `C-14` when it has one, the node id when it does not. A failure that cannot
   * name which row it is about is nearly useless on a list of forty-six that
   * look alike, and an un-numbered candidate still has to be nameable.
   */
  const codeFor = (graphNodeId: string): string =>
    cards.find((card) => card.graph_node_id === graphNodeId)?.code ?? graphNodeId;

  /**
   * Record a fact's weight, then re-read the cards.
   *
   * ## Why there is no optimistic update
   *
   * Server state is the ONE source of truth for tier and order (the structure
   * this task is tested for). Painting the star before the write lands would put
   * a second copy of the weight in this component, and the two disagree the
   * moment a write fails — leaving a star lit for a judgment that was never
   * stored. The re-read is the same light `onFactRemoved` uses, so the queue's
   * selection is not disturbed.
   */
  const changeTier = (graphNodeId: string, tier: FactTier): Promise<void> =>
    setFactTier(slug, scenarioId, graphNodeId, tier)
      .then(() => {
        setError(null);
        onFactRemoved();
      })
      .catch((e: unknown) => {
        const reason = e instanceof Error ? e.message : String(e);
        setError(
          wording
            ? fillCodeAndReason(
                wording.fact_tier_save_failed_template,
                codeFor(graphNodeId),
                reason,
              )
            : reason,
        );
        // Re-thrown so the caller can retract anything it showed optimistically —
        // the background-move notice is raised on the click and must not outlive
        // a write that was refused.
        throw e;
      });

  /**
   * Record where a dragged fact landed, then re-read the cards.
   *
   * The neighbours were computed from the rows on screen; the ORDINAL is the
   * server's, derived from what is stored. A refusal — a neighbour that has gone,
   * or no room left between two facts — arrives as the backend's own words and is
   * shown verbatim, because the human is the only one who can act on either.
   */
  const moveFact = (
    graphNodeId: string,
    after: string | null,
    before: string | null,
  ) => {
    setFactOrder(slug, scenarioId, graphNodeId, after, before)
      .then(() => {
        setError(null);
        onFactRemoved();
      })
      .catch((e: unknown) => {
        const reason = e instanceof Error ? e.message : String(e);
        setError(
          wording
            ? fillCodeAndReason(
                wording.fact_order_save_failed_template,
                codeFor(graphNodeId),
                reason,
              )
            : reason,
        );
      });
  };

  /**
   * Forget where EVERY fact in this scenario was placed (Piece 5b).
   *
   * The reasoning — why one header control replaced forty-six on the cards, why
   * it is a loop over the per-fact route rather than a new bulk endpoint, and
   * why a partial failure is counted and reported — moved with the code to
   * `resetFactOrder` when change D pushed this file past the 300-line limit.
   * That module is where a reader should go, and it is now testable without
   * mounting a section.
   */
  const resetOrder = () => {
    setConfirmingReset(false);
    void resetFactOrder({
      slug,
      scenarioId,
      cards,
      grammar: options?.card_grammar ?? null,
      setError,
      setNotice: setResetNotice,
      onDone: onFactRemoved,
    });
  };

  // `null` while the read is in flight — see the `loading` prop for why the two
  // states must not be collapsed, and what happened the day they were.
  const counts = factsCounts(loading ? null : cards);

  return (
    <section>
      {/* ⚑ THE SCAN ENGINE, MOUNTED AND CHROMELESS (architect ruling R3).

          It draws no card: it hands this section's header its three controls
          and renders only what a header cannot — the running view, the scan's
          refusals and the run report. It may NOT be unmounted; its mount effect
          calls `gatherCandidates`, and gather is the one place candidate
          ordinals are minted (every card's `C-14` handle). See `ScanHeaderApi`.

          The header renders INSIDE this component's tree — the panel's own box
          is `display: contents`, so what it returns becomes a child of the
          section rather than a wrapper inside it. */}
      <ThemeScanPanel
        slug={slug}
        scenarioId={scenarioId}
        proposalSource={proposalSource}
        onFactsChanged={onFactsChanged}
        header={(scan) => (
          <>
            <ScenarioFactsHeader
              scan={scan}
              neverScannedNotice={neverScannedNotice}
              historyOpen={historyOpen}
              onToggleHistory={() => setHistoryOpen(!historyOpen)}
              grammar={options?.card_grammar ?? null}
              onResetOrder={() => setConfirmingReset(true)}
              open={open}
              onToggleOpen={toggleOpen}
            />

            {/* The run history, behind the header's `history` link. It opens
                BELOW the header row it is reached from, at full width, and the
                TABLE is the panel's own element — so there is one history, one
                selection and one delete path however it is reached. */}
            {historyOpen && (
              <div
                style={{
                  ...sectionPanelStyle,
                  marginBottom: "0.75rem",
                  padding: "0 18px",
                }}
              >
                {scan.history}
              </div>
            )}
          </>
        )}
      />

      {options && (
        <FactsResetOrder
          wording={options.card_grammar}
          asking={confirmingReset}
          notice={resetNotice}
          onConfirm={resetOrder}
          onCancel={() => setConfirmingReset(false)}
        />
      )}

      {error && (
        <div
          role="alert"
          style={{
            color: "var(--state-danger-strong)",
            fontSize: "0.82rem",
            marginBottom: "0.5rem",
          }}
        >
          {error}
        </div>
      )}

      {/* ONE CARD, styled exactly as the watch-list's (mockup 3): the filter
          band at the top, the list inside it. `sectionPanelStyle` is the shared
          v3 panel — white, radius 12, no border, its edge is the shadow. */}
      <div style={sectionPanelStyle}>
        {/* OUTSIDE the fold, deliberately. A closed section still says how many
            facts are in it and how many candidates are waiting, which is what
            makes folding honest rather than a hiding place. */}
        <FactsFilterBar active={filter} counts={counts} onPick={setFilter} />

        {open && showsIncluded(filter) && (
          <WorkingView
            cards={cards}
            humanFacts={humanFacts}
            // FACT_CARD_v2 §2: a card is SCENARIO-scoped, so an edit is written
            // against this pair. `onChanged` re-reads the deck, which is what
            // makes the screen match the store after a field is stored.
            slug={slug}
            scenarioId={scenarioId}
            onCardEdited={onChanged}
            onAdd={() => setAdding(true)}
            // P3: the form renders AT its button, inside the view that owns the
            // button's position — not after a scroll region holding forty-six
            // rows, which is where it used to land and why the control read as
            // dead. The state and the write stay here; only the placement moved.
            addForm={
              adding ? (
                <AddHumanFactForm
                  slug={slug}
                  scenarioId={scenarioId}
                  onSaved={() => {
                    setAdding(false);
                    setError(null);
                    onChanged();
                  }}
                  onCancel={() => setAdding(false)}
                />
              ) : null
            }
            onRemoveHumanFact={removeHumanFact}
            onRemoveFact={removeFact}
            wording={wording}
            options={options}
            onSetTier={changeTier}
            onMoveFact={moveFact}
            // v2.1 (change E): the Backs picker, offered on THIS surface only.
            points={points}
          />
        )}

        {/* The candidate queue, exactly as the retired section mounted it —
            same rows, same Include / rule controls, same `CandidateFilterBar`,
            same one-key triage. Only its address changed.

            `keyboardActive` is the fold AND the filter: a `<details>`-style body
            that stays mounted would keep the one-key rulings firing on cards
            nobody can see (ruling R7), and under the Included filter the queue
            is not on screen at all. */}
        {open && showsCandidates(filter) && (
          <CardQueue
            linkOptions={options}
            slug={slug}
            scenarioId={scenarioId}
            externalRefresh={externalRefresh}
            // The heading this fed lived on the retired section's head row. The
            // pills carry the count now, so the frame is reported to nobody —
            // the prop stays required by the queue and is deliberately a no-op
            // rather than a widened signature this task has no reason to change.
            onFrameChanged={() => {}}
            keyboardActive={open}
            onRulingSaved={onRulingSaved}
          />
        )}
      </div>
    </section>
  );
};

export default ScenarioFactsSection;
