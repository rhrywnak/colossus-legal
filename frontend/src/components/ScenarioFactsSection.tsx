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

import React, { useState } from "react";

import AddHumanFactForm from "./AddHumanFactForm";
import WorkingView from "./WorkingView";
import {
  sectionHeaderStyle,
  sectionMetaStyle,
  sectionTitleStyle,
} from "./scenarioSectionStyles";
import { includedRows } from "./factsTable";
import type { ScenarioCard } from "../services/scenarioCards";
import type { HumanFactDto } from "../services/scenarioAugmentation";
import { deleteHumanFact } from "../services/scenarioAugmentation";
import { removeScenarioFact } from "../services/scenarioFacts";
import {
  fillCodeAndReason,
  fillDetail,
  type LinkPanelWording,
} from "../services/evidenceLinks";
import {
  clearFactOrder,
  setFactOrder,
  setFactTier,
} from "../services/scenarioFactCuration";
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
}

const ScenarioFactsSection: React.FC<Props> = ({
  slug,
  scenarioId,
  cards,
  humanFacts,
  onChanged,
  onFactRemoved,
  wording,
}) => {
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
   * Forget where one fact was placed (task 2.13c item 11).
   *
   * Same shape as the other two writes — call, then re-read the cards — because
   * the order on screen must come from what is stored, never from a local guess
   * about what the write did.
   */
  const unplaceFact = (graphNodeId: string) => {
    clearFactOrder(slug, scenarioId, graphNodeId)
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

  const included = includedRows(cards).length;
  const total = included + humanFacts.length;

  return (
    <section>
      <div style={sectionHeaderStyle}>
        <h2 style={sectionTitleStyle}>Scenario facts</h2>
        {/* D10's empty-state fix: different copy at zero, because one is an
            instruction and the other explains an emptiness. */}
        <span style={sectionMetaStyle}>
          {total === 0 ? (
            "facts appear here when you ✓ Include a candidate above"
          ) : (
            <>
              {included} included ·{" "}
              <span style={{ color: "var(--state-success-strong)" }}>✓ Include</span> moves a
              candidate here
              {humanFacts.length > 0 && ` · ${humanFacts.length} added by hand`}
            </>
          )}
        </span>
      </div>

      {/* Task 2.13c item 7: the surface discloses itself. The weights and the drag
          were discoverable only by hovering an icon, which meant a human who did
          not already know they existed never found them — Roman's ruling that a
          feature must announce itself on screen. Served from the store, so
          renaming a tier renames it here too. */}
      {wording && (
        <p
          style={{
            margin: "0 0 0.6rem",
            fontSize: "0.8rem",
            color: "var(--text-secondary)",
          }}
        >
          {wording.fact_weights_hint}
        </p>
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

      {/* C2 + C4 in ONE table (task 1.7D item 6). 1.7C rendered human facts as a
          separate list beneath the evidence, because a fact with no citation and a
          fact with a pinpoint are different kinds of thing and §8 requires the
          distinction be visible. That reasoning holds; what changed is HOW it is
          made visible — the v3 mockup carries it in a coloured left-edge stripe
          (green evidence, blue human) instead of splitting the reader's attention
          across two lists they have to mentally join.

          The row's provenance line still says which it is in words, so the stripe
          is a cue and never the only signal. `WorkingView` opens its pinpoints in
          the viewer WINDOW (D5); a human row has no pinpoint to open. */}
      <WorkingView
        cards={cards}
        humanFacts={humanFacts}
        onAdd={() => setAdding(true)}
        onRemoveHumanFact={removeHumanFact}
        onRemoveFact={removeFact}
        wording={wording}
        onSetTier={changeTier}
        onMoveFact={moveFact}
        onUnplaceFact={unplaceFact}
      />

      {adding && (
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
      )}
    </section>
  );
};

export default ScenarioFactsSection;
