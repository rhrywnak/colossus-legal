// =============================================================================
// ScenarioStatusControl — one color-coded segmented control (task 1.7D, item 3)
// =============================================================================
//
// Replaces `ReadyToggle`. Mockup `.status-seg`: a chrome track holding two
// segments, the active one filled — amber for Draft, green for Ready.
//
// ## The defect this fixes
//
// The 1.7B/1.7C header carried a BUTTON reading "Mark ready to rehearse" (or
// "Remove from rehearsal"), plus a separate status chip. Roman's 2026-08-03
// session found the state unreadable at a glance: a button states the ACTION
// available, not the state you are in, and the reader has to invert it — "the
// button says Mark ready, therefore it is currently NOT ready". Two surfaces for
// one fact, neither of them the fact.
//
// The segmented control is the fix and the whole fix (item 10): the filled
// segment IS the state, so no toast, no confirmation banner, and no status chip
// (ruling R4 — the chip dies, the direction chip stays).
//
// ## Colour never stands alone
//
// The active segment carries a `●` glyph and its own word, so the state survives
// colourblindness and a greyscale print. That is the mockup's stated rule for
// every coloured control in v3.
//
// ## Zero backend change (confirmed in Phase A, ruling R3/R8)
//
// `setScenarioReady(slug, id, ready)` already POSTs `/scenarios/:id/ready` with a
// boolean and already handles BOTH directions — so Ready sends `true`, Draft sends
// `false`. The status-transitions ledger keeps recording, and the generic PUT route
// still refuses `status` outright. Nothing on the server moved for this.
//
// ## Why there is still no optimistic update
//
// Carried from `ReadyToggle` deliberately. This state decides whether a scenario
// appears in front of a witness, so the control waits for the server and reports
// what the server said. A segment that filled in instantly and silently failed
// would be the 1.3 ruling defect wearing a nicer coat.

import React, { useState } from "react";

import { setScenarioReady } from "../services/rehearsal";
import type { ScenarioStatus } from "../pages/trialPrepData";

/** Mockup `.status-seg`: the chrome track. */
const trackStyle: React.CSSProperties = {
  display: "inline-flex",
  background: "var(--v3-chrome-strong)",
  borderRadius: "10px",
  padding: "3px",
  gap: "2px",
};

/** Mockup `.status-seg span`: an inactive segment. */
const segmentStyle: React.CSSProperties = {
  fontFamily: "inherit",
  fontSize: "12.5px",
  fontWeight: 500,
  padding: "4px 14px",
  borderRadius: "8px",
  border: "none",
  background: "none",
  cursor: "pointer",
  color: "var(--text-muted)",
};

/** Mockup `.status-seg .active-draft` — and its green twin for Ready. */
const activeSegmentStyle: React.CSSProperties = {
  ...segmentStyle,
  color: "var(--v3-on-fill)",
  boxShadow: "0 1px 3px rgba(16,24,40,.2)",
};

// CONST: the mockup's tooltip copy, verbatim. User-facing text, not config.
const TOOLTIP = "Ready = appears in Marie's rehearsal view. Human-only switch.";

/**
 * Whether clicking a segment should actually change anything.
 *
 * Extracted as a pure rule rather than left inline because it is a CORRECTNESS
 * rule with a named consequence, not a visual convention: the ready route writes a
 * row to the status-transitions ledger, so firing it for a click on the segment
 * you are already on would record a transition that never happened. The ledger is
 * the forensic record of who declared a scenario rehearsable and when; a false
 * entry in it is worse than a missing feature.
 *
 * @param next Whether the clicked segment means "ready".
 * @param currentReady Whether the scenario is ready right now.
 */
export function shouldApplyStatus(next: boolean, currentReady: boolean): boolean {
  return next !== currentReady;
}

/**
 * One segment of the control.
 *
 * The `●` glyph is load-bearing, not decoration: it marks the active segment
 * without relying on the fill colour, so the state survives colourblindness and a
 * greyscale print. That is the mockup's stated rule for every coloured control.
 */
const Segment: React.FC<{
  label: string;
  active: boolean;
  /** The fill token used when this segment is the active one. */
  fill: string;
  disabled: boolean;
  onClick: () => void;
}> = ({ label, active, fill, disabled, onClick }) => (
  <button
    type="button"
    style={active ? { ...activeSegmentStyle, background: fill } : segmentStyle}
    aria-pressed={active}
    disabled={disabled}
    onClick={onClick}
  >
    {active && "● "}
    {label}
  </button>
);

/** Everything `applyStatusChange` needs, so the handler stays out of the render. */
interface ApplyArgs {
  slug: string;
  scenarioId: string;
  /** Whether the clicked segment means "ready". */
  next: boolean;
  /** Whether the scenario is ready right now. */
  ready: boolean;
  saving: boolean;
  setSaving: (v: boolean) => void;
  setMessage: (v: string | null) => void;
  setError: (v: string | null) => void;
  onChanged: () => void;
}

/**
 * Perform a status change, or decline to.
 *
 * Extracted from the component for the function-size limit (Rule 18), and it earns
 * the split: this is the only part of the control that TALKS TO THE SERVER, and
 * keeping it beside the render made it easy to miss that there is deliberately no
 * optimistic update here. The segment's fill derives from the `status` prop, so it
 * cannot move until the server has confirmed and the page has re-read.
 */
function applyStatusChange(args: ApplyArgs): void {
  const { slug, scenarioId, next, ready, saving, setSaving, setMessage, setError, onChanged } =
    args;

  // The `saving` half cannot live in `shouldApplyStatus` — it is React state — but
  // the rule that protects the ledger is pure and tested. See `shouldApplyStatus`.
  if (!shouldApplyStatus(next, ready) || saving) return;

  setSaving(true);
  setError(null);
  setScenarioReady(slug, scenarioId, next)
    .then((change) => {
      // The backend composes both sentences, including the one that matters —
      // "removed from rehearsal, nothing else changed". Demoting is the act a human
      // hesitates over, and the hesitation is *does this throw away the work*. It
      // does not, and the server is what says so.
      setMessage(change.message);
      setSaving(false);
      onChanged();
    })
    .catch((e: unknown) => {
      // Explicit error UI. A failed readiness change that looked successful would
      // leave someone believing a scenario is rehearsable when it is not.
      setError(e instanceof Error ? e.message : "That readiness change did not save.");
      setMessage(null);
      setSaving(false);
    });
}

interface Props {
  slug: string;
  scenarioId: string;
  /** The scenario's stored status. */
  status: ScenarioStatus;
  /** Called after a successful change so the page can re-read. */
  onChanged: () => void;
}

const ScenarioStatusControl: React.FC<Props> = ({
  slug,
  scenarioId,
  status,
  onChanged,
}) => {
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Two segments, not three (ruling R3). `needs_evidence` is DEAD VOCABULARY —
  // measured at zero rows in task 1.5, and its retirement from the enum is filed
  // as task 3.6. A scenario carrying it (there are none) renders with Draft
  // active, which loses no information because there is none to lose. Do not add
  // a third segment for it; retire the value instead.
  const ready = status === "ready";

  const apply = (next: boolean) =>
    applyStatusChange({ slug, scenarioId, next, ready, saving, setSaving, setMessage, setError, onChanged });

  return (
    <span style={{ display: "inline-flex", flexDirection: "column", gap: "0.3rem" }}>
      <span style={trackStyle} title={TOOLTIP} role="group" aria-label="Scenario status">
        <Segment
          label="Draft"
          active={!ready}
          fill="var(--state-warning-strong)"
          disabled={saving}
          onClick={() => apply(false)}
        />
        <Segment
          label="Ready"
          active={ready}
          fill="var(--state-success-strong)"
          disabled={saving}
          onClick={() => apply(true)}
        />
      </span>

      {saving && (
        <span style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>Saving…</span>
      )}
      {message && !saving && (
        <span style={{ fontSize: "11.5px", color: "var(--text-muted)" }}>{message}</span>
      )}
      {error && (
        <span role="alert" style={{ fontSize: "11.5px", color: "var(--state-danger-strong)" }}>
          {error}
        </span>
      )}
    </span>
  );
};

export default ScenarioStatusControl;
