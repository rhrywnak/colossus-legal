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
  fillSlots,
  type AllegationOptions,
  type LinkPanelWording,
} from "../services/evidenceLinks";
import {
  clearFactOrder,
  setFactOrder,
  setFactTier,
} from "../services/scenarioFactCuration";
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
}) => {
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<string | null>(null);
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
   * ## Why one control in the header replaced forty-six on the cards
   *
   * "Clear my order" sat on every placed card to do a thing a human does once,
   * and it competed for the footer with Remove — two controls a click apart, one
   * of which discards a position and the other of which takes the fact out of
   * the scenario. The footer now keeps only Remove.
   *
   * ## Why it is a loop over the existing per-fact route
   *
   * There is no bulk endpoint and this task deliberately adds none: the ruling
   * and order write paths are audited surfaces, and a second writer for "clear
   * them all" would be a new path to guard for a control that runs a handful of
   * times. The loop is bounded by the number of PLACED facts — a fact nobody
   * dragged has nothing to clear — which is a few, not the whole list.
   *
   * ## Partial failure is REPORTED, never swallowed
   *
   * Each write can fail independently, so the outcome is counted and the
   * sentence names how many landed. A loop that stopped at the first refusal and
   * said "reset" would leave the order half-cleared with the screen claiming
   * otherwise (Standing Rule 1).
   */
  const resetOrder = () => {
    const placed = cards.filter((card) => card.sort_ordinal != null);
    setConfirmingReset(false);

    // The control is withheld until the words load, so this branch is
    // unreachable through the UI. It is not a silent `return` all the same: a
    // second caller wired up without the wording would otherwise get a confirm
    // dialog, a click, and nothing at all — the exact shape Standing Rule 1
    // exists to forbid. The refusal SAYS so, in the one place this component can
    // speak without a stored sentence to speak it with.
    if (!options) {
      // eslint-disable-next-line no-console -- there is no stored sentence for a
      // state that cannot happen through the UI, and inventing one would be the
      // literal the language law deletes.
      console.warn(
        "Reset order was invoked before the card wording loaded; nothing was written.",
      );
      setError("The order could not be reset — please reload and try again.");
      return;
    }

    Promise.allSettled(
      placed.map((card) => clearFactOrder(slug, scenarioId, card.graph_node_id)),
    ).then((results) => {
      const cleared = results.filter((r) => r.status === "fulfilled").length;
      const failed = results.length - cleared;

      if (failed > 0) {
        const first = results.find((r) => r.status === "rejected");
        const reason =
          first && first.status === "rejected"
            ? first.reason instanceof Error
              ? first.reason.message
              : String(first.reason)
            : "";
        setError(
          fillSlots(options.card_grammar.reset_order_failed_template, { reason }),
        );
      } else {
        setError(null);
      }

      // The count is what ACTUALLY cleared, not what was attempted: a sentence
      // reporting the intention would be the screen agreeing with the click
      // rather than with the database.
      setResetNotice(
        fillSlots(options.card_grammar.reset_order_done_template, {
          count: String(cleared),
        }),
      );
      onFactRemoved();
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

        {/* Piece 5b: ONE Reset order in the section header, confirmed. It
            discards work across the whole list — the sequence IS the argument —
            which is why it asks first, unlike the per-card control it replaces.
            Withheld until the words load: there is no fallback vocabulary, and a
            destructive control that cannot state what it does must not be
            offered at all (R4). */}
        {options && (
          <span style={{ marginLeft: "auto", display: "flex", gap: "0.5rem" }}>
            <button
              type="button"
              onClick={() => setConfirmingReset(true)}
              style={{
                border: "none",
                background: "none",
                padding: 0,
                color: "var(--accent-primary)",
                cursor: "pointer",
                fontFamily: "inherit",
                fontSize: "0.8rem",
              }}
            >
              {options.card_grammar.reset_order_label}
            </button>
          </span>
        )}
      </div>

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
        options={options}
        onSetTier={changeTier}
        onMoveFact={moveFact}
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
