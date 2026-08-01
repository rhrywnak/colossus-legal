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
// Task 1.3 swaps the workbench for the keyboard TRIAGE QUEUE (`CardQueue`): the
// same job — rule on every candidate — done at the speed the pool actually needs
// (399 of 525 candidates are unrulable-as-they-stand, so one-key defer is the
// common case). The queue renders the §7 card contract from the 1.2 payload.
//
// The orphan guarantee (a confirmed fact whose graph node vanished must still
// surface) moves WITH the surface: `CardQueue` makes the same second
// `listScenarioFacts` read and renders the strip below the queue.
//
// `CandidateFactsPanel` is no longer mounted anywhere. It is left in the tree
// rather than deleted in this task: removing 763 lines plus its shipped helper
// tests is its own reviewable change, and keeping it available through G1 means
// the old surface can be restored in one line if the queue disappoints on DEV.
// Flagged for removal in the 1.3 report.

import React from "react";

import CardQueue from "./CardQueue";
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
        Rule on every candidate from the keyboard, without leaving this queue.
        The source page sits beside each card.
      </p>
      <CardQueue slug={slug} scenarioId={scenarioId} externalRefresh={externalRefresh} />
    </div>
  );
};

export default ScenarioCurationPanel;
