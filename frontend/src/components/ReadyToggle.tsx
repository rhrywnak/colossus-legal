// =============================================================================
// ReadyToggle — the ready gate on the scenario header (task 1.5, v2 §5/§6)
// =============================================================================
//
// One button, two directions, and a plain sentence after either. Visible to all
// three humans (§5: Roman, Marie or Chuck may declare a scenario ready; §5a:
// nothing waits on Chuck).
//
// ## Why the confirmation is rendered verbatim
//
// The backend composes both sentences — "S-2 is ready — Marie can now rehearse
// this scenario" and "S-2 removed from rehearsal — nothing else changed". The
// second one is the important one: demoting is the act a human hesitates over,
// and the hesitation is *does this throw away the work*. It does not. Saying so
// is what makes the gate usable in both directions, and it is case language, so
// it lives on the backend like every other user-visible string.
//
// ## Why there is no optimistic update
//
// Task 1.3's ruling failure was an optimistic advance over a `console.error`. The
// state here decides whether a scenario appears in front of a witness, so the
// button waits for the server and reports what the server said — nothing more.

import React, { useState } from "react";

import { setScenarioReady } from "../services/rehearsal";

interface Props {
  slug: string;
  scenarioId: string;
  /** Whether the scenario is in rehearsal right now. */
  ready: boolean;
  /** Called after a successful change so the page can re-read. */
  onChanged: () => void;
}

const ReadyToggle: React.FC<Props> = ({ slug, scenarioId, ready, onChanged }) => {
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const apply = async () => {
    setSaving(true);
    setError(null);
    try {
      const change = await setScenarioReady(slug, scenarioId, !ready);
      setMessage(change.message);
      onChanged();
    } catch (e: unknown) {
      // Explicit error UI. A failed readiness change that looked successful
      // would leave someone believing a scenario is rehearsable when it is not.
      setError(e instanceof Error ? e.message : "That readiness change did not save.");
      setMessage(null);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "0.35rem" }}>
      <button type="button" onClick={() => void apply()} disabled={saving}>
        {saving
          ? "Saving…"
          : ready
            ? "Remove from rehearsal"
            : "Mark ready to rehearse"}
      </button>

      {message && (
        <div style={{ fontSize: "0.8rem", color: "var(--text-muted)" }}>{message}</div>
      )}
      {error && (
        <div role="alert" style={{ fontSize: "0.8rem", color: "var(--state-danger-strong)" }}>
          {error}
        </div>
      )}
    </div>
  );
};

export default ReadyToggle;
