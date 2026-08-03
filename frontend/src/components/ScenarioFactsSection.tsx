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

interface Props {
  slug: string;
  scenarioId: string;
  /** The card payload — the included ones become C2 rows. */
  cards: ScenarioCard[];
  /** C4: facts a human wrote, with no citation by design. */
  humanFacts: HumanFactDto[];
  /** Re-read after a human fact is added or removed. */
  onChanged: () => void;
}

const ScenarioFactsSection: React.FC<Props> = ({
  slug,
  scenarioId,
  cards,
  humanFacts,
  onChanged,
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
