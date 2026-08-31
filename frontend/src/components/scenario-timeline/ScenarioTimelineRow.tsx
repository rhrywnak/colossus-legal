// =============================================================================
// ScenarioTimelineRow.tsx — the Timeline row and Attach (mockup Screen 1)
// =============================================================================
//
// `Timeline: [The $50,000 · 15 events] Attach…` — the row under the scenario's
// name in the mockup's header. Design §5B.
//
// ## ⚑ ATTACH AND DETACH ARE THE SAME CONTROL
//
// The chooser lists every subset the case has, with a check beside the ones this
// scenario already carries. Picking an unchecked one attaches; picking a checked
// one detaches. One list, one gesture, and no separate "remove" affordance to
// go looking for — the state IS the control.
//
// A 409 from the attach means somebody else attached it first. That is not an
// error the reader caused or can act on, so it resolves the same way the truth
// does: the row simply shows checked, from the list the server returns.
//
// ## The one hard delete in the feature
//
// Detach removes the link row outright — no soft delete, no undo line. A link is
// the SCENARIO's fact about the subset, not the subset's content: the story
// itself is untouched and still on the timeline, one Attach away. That is why
// this is the one place in the feature with no undo and no confirm.

import React, { useCallback, useState } from "react";

import type { ChronologyWording } from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import { listSubsets, type SubsetSummary } from "../../services/caseTimelineSubsets";
import * as d from "./dockStyles";
import * as ws from "./windowStyles";
import { attachSubset, type AttachedSubset, detachSubset } from "./scenarioTimeline";

type Props = {
  slug: string;
  scenarioId: string;
  attached: AttachedSubset[];
  wording: ChronologyWording;
  /** Hand the server's new list back to the dock, which owns the state. */
  onChanged: (next: AttachedSubset[]) => void;
};

const ScenarioTimelineRow: React.FC<Props> = ({
  slug,
  scenarioId,
  attached,
  wording,
  onChanged,
}) => {
  const [open, setOpen] = useState(false);
  const [choices, setChoices] = useState<SubsetSummary[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const openChooser = useCallback(async () => {
    setOpen(true);
    setError(null);
    if (choices !== null) return;
    try {
      setChoices(await listSubsets());
    } catch (err: unknown) {
      // Never swallowed: the chooser opens onto a sentence rather than an empty
      // list, which would read as "this case has no stories".
      setError(err instanceof Error ? err.message : "unknown error");
    }
  }, [choices]);

  const toggle = useCallback(
    async (subsetId: string, isAttached: boolean) => {
      setBusy(true);
      setError(null);
      try {
        const next = isAttached
          ? await detachSubset(slug, scenarioId, subsetId)
          : await attachSubset(slug, scenarioId, subsetId);
        onChanged(next);
      } catch (err: unknown) {
        setError(err instanceof Error ? err.message : "unknown error");
      } finally {
        setBusy(false);
      }
    },
    [slug, scenarioId, onChanged],
  );

  const attachedIds = new Set(attached.map((s) => s.id));

  return (
    <div style={{ position: "relative" }}>
      <div style={d.timelineRow}>
        <span>{cw(wording, "scenario_timeline_row_label")}</span>
        {attached.map((s) => (
          <span key={s.id} style={d.subsetChip}>
            {s.name} ·{" "}
            {fill(cw(wording, "subsets_window_events_count_template"), { count: s.event_count })}
          </span>
        ))}
        <button type="button" style={d.attachLink} disabled={busy} onClick={() => void openChooser()}>
          {cw(wording, "scenario_attach_link")}
        </button>
      </div>

      {open && (
        <div style={d.attachList}>
          {error !== null && <div style={ws.errorState}>{error}</div>}
          {choices?.map((s) => {
            const on = attachedIds.has(s.id);
            return (
              <button
                key={s.id}
                type="button"
                style={d.attachItem}
                disabled={busy}
                onClick={() => void toggle(s.id, on)}
              >
                {/* The check IS the state and the control: picking a checked
                    one detaches. A glyph, not a word — there is no stored row
                    for "attached", and inventing one for a tick mark would be
                    a row nothing else ever says. */}
                <span style={d.attachCheck}>{on ? "✓" : ""}</span>
                <span>{s.name}</span>
              </button>
            );
          })}
          {/* Navigates to the Subsets section rather than opening the picker in
              place — creating a story is a different act from attaching one, and
              it belongs on the page that owns it. The stored words are the
              button's own, so the link and its destination cannot drift. */}
          <button
            type="button"
            style={d.attachFoot}
            onClick={() => window.open("/timeline", "_blank", "noopener")}
          >
            {cw(wording, "subsets_add_button")}
          </button>
        </div>
      )}
    </div>
  );
};

export default ScenarioTimelineRow;
