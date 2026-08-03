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
  absentStyle,
  HAIRLINE,
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

  const included = includedRows(cards).length;
  const total = included + humanFacts.length;

  return (
    <section>
      <div style={sectionHeaderStyle}>
        <h2 style={sectionTitleStyle}>Scenario facts</h2>
        {/* D10's empty-state fix: different copy at zero, because one is an
            instruction and the other explains an emptiness. */}
        <span style={sectionMetaStyle}>
          {total === 0
            ? "facts appear here when you rule I on a candidate above"
            : `${included} included · ${humanFacts.length} human · an I ruling above moves an item here`}
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

      {/* C2 — the included evidence, search and pinpoints. Unchanged behaviour;
          `WorkingView` opens its pinpoints in the viewer WINDOW (D5) too. */}
      <WorkingView cards={cards} onAdd={() => setAdding(true)} />

      {/* C4 — human facts, tagged with author and date inline. They sit BELOW the
          evidence table rather than mixed into it: a fact with no citation and a
          fact with a pinpoint are different kinds of thing, and §8 requires the
          distinction be visible. */}
      <div style={{ marginTop: "0.9rem" }}>
        <div style={{ ...sectionMetaStyle, marginBottom: "0.35rem" }}>
          Human facts — knowledge in no document
        </div>
        {humanFacts.length === 0 ? (
          <p style={absentStyle}>
            None yet. Use “Add human fact” above for something you know that no
            document says.
          </p>
        ) : (
          humanFacts.map((fact) => (
            <div
              key={fact.id}
              style={{ borderBottom: HAIRLINE, padding: "0.5rem 0", fontSize: "0.9rem" }}
            >
              <div style={{ lineHeight: 1.6 }}>{fact.text}</div>
              <div
                style={{
                  display: "flex",
                  gap: "0.5rem",
                  alignItems: "baseline",
                  fontSize: "0.75rem",
                  color: "var(--text-muted)",
                }}
              >
                {fact.date_label && <span>{fact.date_label}</span>}
                {/* `authored_tag` ("Added by Roman") arrives COMPOSED — provenance
                    is not the browser's sentence to write. */}
                <span style={{ fontStyle: "italic" }}>
                  {fact.authored_tag}
                  {fact.edited && " · edited since written"}
                </span>
                <button
                  type="button"
                  onClick={() => {
                    deleteHumanFact(slug, scenarioId, fact.id)
                      .then(() => {
                        setError(null);
                        onChanged();
                      })
                      .catch((e: unknown) => {
                        setError(
                          e instanceof Error ? e.message : "That fact could not be removed.",
                        );
                      });
                  }}
                  style={{
                    marginLeft: "auto",
                    border: "none",
                    background: "none",
                    color: "var(--text-muted)",
                    cursor: "pointer",
                    fontFamily: "inherit",
                    fontSize: "0.75rem",
                  }}
                >
                  Remove
                </button>
              </div>
            </div>
          ))
        )}
      </div>

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
