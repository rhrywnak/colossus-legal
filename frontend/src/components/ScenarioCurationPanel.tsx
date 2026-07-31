// =============================================================================
// ScenarioCurationPanel — host for the scenario candidate workbench (1a.6).
// =============================================================================
//
// Phase 1a.6 merge: the old split — a separate "Curated facts" saved-list on top
// of a "Find candidate facts" finder — is collapsed into ONE gather-driven
// workbench. The saved-list is now just the workbench's `included` status
// filter, so this component no longer fetches or renders it: it is a thin host
// that frames the `CandidateFactsPanel` workbench and preserves the mount
// contract from `ScenarioDetailPage` (slug, scenarioId, definition).
//
// The orphan guarantee (a confirmed fact whose graph node vanished must still
// surface) moved DOWN into `CandidateFactsPanel` (its `listScenarioFacts` +
// `findOrphans` path), so it lives beside the gather fetch it reconciles against.

import React from "react";

import CandidateFactsPanel from "./CandidateFactsPanel";
import type { ScenarioDefinition } from "../pages/trialPrepData";

interface Props {
  slug: string;
  scenarioId: string;
  /** @deprecated (1a.6) Vestigial pass-through — the workbench resolves its
   *  subject server-side, so the definition no longer seeds anything. Kept to
   *  preserve the `ScenarioDetailPage` mount contract; a follow-up chunk removes
   *  it along with `candidateSeed`. */
  definition?: ScenarioDefinition;
  /** Forwarded to `CandidateFactsPanel` — see its `externalRefresh`. This panel is
   *  a thin wrapper, so it only relays the signal. */
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
  definition,
  externalRefresh,
}) => {
  return (
    <div>
      <div style={sectionLabel}>Scenario facts</div>
      <p style={hintStyle}>
        Browse every candidate about this scenario&rsquo;s subject and rule on
        each: <strong>include</strong> a fact, <strong>drop</strong> one, or{" "}
        <strong>un-drop</strong> it back to the pool. Filter by status to review
        what you have included so far.
      </p>
      <CandidateFactsPanel
        slug={slug}
        scenarioId={scenarioId}
        definition={definition}
        externalRefresh={externalRefresh}
      />
    </div>
  );
};

export default ScenarioCurationPanel;
