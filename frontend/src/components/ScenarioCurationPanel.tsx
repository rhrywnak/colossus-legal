// =============================================================================
// ScenarioCurationPanel — host for the scenario candidate workbench (1a.6).
// =============================================================================
//
// Task 1.3 made the keyboard TRIAGE QUEUE (`CardQueue`) the surface where a
// human rules on candidates — the same job the old workbench did, at the speed
// the pool actually needs (399 of 525 candidates are unrulable as they stand, so
// one-key defer is the common case). This component is the thin host that frames
// the working view, the augmentation panel and the queue.
//
// The orphan guarantee (a confirmed fact whose graph node vanished must still
// surface) lives WITH the surface: `CardQueue` makes the same second
// `listScenarioFacts` read and renders the strip below the queue.
//
// The retired `CandidateFactsPanel` was kept in the tree through G1 as a
// one-line restore path in case the queue disappointed on DEV. It did not, and
// task 1.7B deletes it.

import React, { useCallback, useEffect, useState } from "react";

import AugmentationPanel from "./AugmentationPanel";
import CardQueue from "./CardQueue";
import WorkingView from "./WorkingView";
import { fetchScenarioCards, type ScenarioCard } from "../services/scenarioCards";

interface Props {
  slug: string;
  scenarioId: string;
  /** Bumped by the page when a scan merge writes candidate facts, so the
   *  working view and the queue re-read. This panel owns neither — it relays. */
  externalRefresh?: number;
}

const sectionLabel: React.CSSProperties = {
  fontSize: "0.74rem",
  fontWeight: 600,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  margin: "1.5rem 0 0.5rem",
};

const hintStyle: React.CSSProperties = {
  fontSize: "0.82rem",
  color: "var(--text-muted)",
  margin: "0 0 0.75rem",
};

const ScenarioCurationPanel: React.FC<Props> = ({
  slug,
  scenarioId,
  externalRefresh,
}) => {
  const [cards, setCards] = useState<ScenarioCard[]>([]);
  const [showAugmentation, setShowAugmentation] = useState(false);
  const [cardsError, setCardsError] = useState<string | null>(null);

  // The working view reads the same card payload the queue does. Loaded here
  // rather than in `WorkingView` so the two surfaces cannot disagree about what
  // is included — one read, one answer.
  const loadCards = useCallback(async () => {
    try {
      const loaded = await fetchScenarioCards(slug, scenarioId);
      setCards([...loaded.pool, ...loaded.set_aside]);
      setCardsError(null);
    } catch (e: unknown) {
      setCardsError(
        e instanceof Error ? e.message : "Failed to load this scenario's facts.",
      );
    }
  }, [slug, scenarioId]);

  useEffect(() => {
    void loadCards();
  }, [loadCards, externalRefresh]);

  return (
    <div>
      <div style={sectionLabel}>Scenario facts</div>
      {cardsError && (
        <div role="alert" style={{ color: "var(--state-danger-strong)", fontSize: "0.85rem" }}>
          {cardsError}
        </div>
      )}
      <p style={hintStyle}>
        What you have included, then the queue of what is left to rule on. The
        source page sits beside each card.
      </p>

      {/* The working view sits ABOVE the queue: it is the record of what the
          curation produced, and the queue below is the position still being
          worked. Task 1.4. */}
      <WorkingView cards={cards} onAdd={() => setShowAugmentation(true)} />

      {showAugmentation && (
        <div style={{ marginTop: "1rem" }}>
          <AugmentationPanel
            slug={slug}
            scenarioId={scenarioId}
            externalRefresh={externalRefresh}
          />
        </div>
      )}

      <div style={{ marginTop: "1.5rem" }}>
        <CardQueue slug={slug} scenarioId={scenarioId} externalRefresh={externalRefresh} />
      </div>
    </div>
  );
};

export default ScenarioCurationPanel;
