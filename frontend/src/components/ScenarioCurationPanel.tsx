// =============================================================================
// ScenarioCurationPanel — the saved-facts binder + candidate finder.
// =============================================================================
//
// Phase A curation, saved side. Replaces the old (broken) allegation-anchored
// timeline on the scenario detail page with the facts a human has actually
// curated onto the scenario: the saved set on top (each removable), and the
// CandidateFactsPanel below to add more. Owns the refresh cycle — an add or a
// remove bumps a key that re-fetches the saved list (the same idiom the Trial
// Prep dashboard uses after a create).

import React, { useEffect, useState } from "react";

import {
  listScenarioFacts,
  removeScenarioFact,
  type ScenarioFactDto,
} from "../services/scenarioFacts";
import EvidenceCard from "../pages/BiasExplorer/EvidenceCard";
import CandidateFactsPanel from "./CandidateFactsPanel";
import type { ScenarioDefinition } from "../pages/trialPrepData";

interface Props {
  slug: string;
  scenarioId: string;
  /** This scenario's authored definition (B2a). Pure pass-through — the binder
   *  does not consume it; it forwards it to CandidateFactsPanel for seeding. */
  definition?: ScenarioDefinition;
}

const sectionLabel: React.CSSProperties = {
  fontSize: "0.74rem",
  fontWeight: 600,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  margin: "1.5rem 0 0.75rem",
};

const messageStyle: React.CSSProperties = {
  fontSize: "0.84rem",
  color: "var(--text-muted)",
  padding: "0.5rem 0",
};

const errorStyle: React.CSSProperties = {
  margin: "0.5rem 0",
  padding: "0.6rem 0.8rem",
  backgroundColor: "var(--state-danger-bg-soft)",
  border: "1px solid var(--state-danger-border)",
  borderRadius: "6px",
  color: "var(--state-danger-strong)",
  fontSize: "0.82rem",
};

const staleCardStyle: React.CSSProperties = {
  backgroundColor: "var(--bg-surface)",
  border: "1px dashed var(--state-warning-strong)",
  borderRadius: "8px",
  padding: "0.85rem 1rem",
  display: "flex",
  alignItems: "center",
  gap: "0.6rem",
  fontSize: "0.82rem",
  color: "var(--text-secondary)",
};

const removeBtnStyle: React.CSSProperties = {
  padding: "0.2rem 0.6rem",
  fontSize: "0.74rem",
  fontWeight: 600,
  border: "1px solid var(--state-danger-border)",
  borderRadius: "5px",
  backgroundColor: "var(--state-danger-bg-soft)",
  color: "var(--state-danger-strong)",
  cursor: "pointer",
};

const ScenarioCurationPanel: React.FC<Props> = ({
  slug,
  scenarioId,
  definition,
}) => {
  const [facts, setFacts] = useState<ScenarioFactDto[] | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [removeError, setRemoveError] = useState<string | null>(null);
  // Bumped after an add or remove to re-fetch the saved list.
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    listScenarioFacts(slug, scenarioId)
      .then((data) => {
        if (cancelled) return;
        setFacts(data);
        setLoading(false);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        setError(
          err instanceof Error
            ? err.message
            : "Failed to load saved facts. Try reloading the page.",
        );
        setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId, refreshKey]);

  const bump = () => setRefreshKey((k) => k + 1);

  const handleRemove = (graphNodeId: string) => {
    setRemoveError(null);
    removeScenarioFact(slug, scenarioId, graphNodeId)
      .then(() => bump())
      .catch((err: unknown) => {
        setRemoveError(
          err instanceof Error ? err.message : "Failed to remove the fact.",
        );
      });
  };

  const savedIds = new Set((facts ?? []).map((f) => f.graph_node_id));

  return (
    <div>
      <div style={sectionLabel}>Curated facts</div>

      {removeError && <div style={errorStyle}>{removeError}</div>}

      {loading ? (
        <div style={messageStyle}>Loading saved facts…</div>
      ) : error ? (
        <div style={errorStyle}>{error}</div>
      ) : !facts || facts.length === 0 ? (
        <div style={messageStyle}>No facts added yet.</div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem" }}>
          {facts.map((fact) => {
            const remove = (
              <button
                type="button"
                style={removeBtnStyle}
                onClick={() => handleRemove(fact.graph_node_id)}
              >
                Remove
              </button>
            );
            // A stale reference (the graph node is gone) still shows — with its
            // id and a Remove control — rather than vanishing silently.
            if (!fact.content) {
              return (
                <div key={fact.graph_node_id} style={staleCardStyle}>
                  <span>
                    Saved fact <code>{fact.graph_node_id}</code> — content
                    unavailable (the source node may have been removed).
                  </span>
                  <span style={{ marginLeft: "auto" }}>{remove}</span>
                </div>
              );
            }
            return (
              <EvidenceCard
                key={fact.graph_node_id}
                instance={fact.content}
                action={remove}
              />
            );
          })}
        </div>
      )}

      <div style={sectionLabel}>Add facts</div>
      <CandidateFactsPanel
        slug={slug}
        scenarioId={scenarioId}
        savedIds={savedIds}
        onAdded={bump}
        definition={definition}
      />
    </div>
  );
};

export default ScenarioCurationPanel;
