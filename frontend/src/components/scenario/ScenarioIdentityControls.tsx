// =============================================================================
// ScenarioIdentityControls.tsx — the role chip and the Draft/Ready switch
// =============================================================================
//
// The two identity controls, on their own, for a surface that already has a
// title of its own and does not want a second header above it.
//
// ## ⚑ Why this exists (Roman, 2026-09-07, from a screenshot)
//
// The Practice page rendered `ScenarioHeaderStrip` above its own "PRACTICE
// SESSION / S-11 · The $50,000" card, and the two said the same thing: the same
// code, the same title, twice, an inch apart. The strip came off that page and
// its three live pieces moved onto the card's title line — the chip and the
// switch (here) and the View Timeline dock (mounted beside this one).
//
// ## ⚑ Why this FETCHES rather than taking props, which is the whole problem
//
// The chip needs `direction` and the switch needs `status`, and the Practice
// deck payload carries NEITHER — `ScenarioIdentityDto`'s own doc records that,
// and it is why the strip was built self-fetching in the first place:
//
//   "A strip taking status as a prop would have needed three payloads widened
//    and four pages taught about it — the shape `ScenarioTimelineDock` already
//    rejected for the same reason."
//
// That reasoning did not move when the controls did. Threading identity down
// through `PracticePage` → `PracticeStart` → `PracticeTitleRow` would teach
// three presentational components about a payload none of them otherwise reads,
// to save one request the strip was already making from the same page. So this
// reads the augmentation panel exactly as the strip does — the same call, the
// same error handling — and the page is no busier than it was.
//
// `ScenarioTimelineDock` is the precedent, not an exception: it is a one-line
// mount that fetches its own subsets, and it is mounted beside this on the same
// line for the same reason.

import React, { useCallback, useEffect, useState } from "react";

import ScenarioStatusControl from "../ScenarioStatusControl";
import { directionChip } from "../scenarioHeader";
import { isKnownDirection } from "./headerStripRules";
import * as ss from "./stripStyles";
import {
  fetchAugmentationPanel,
  type ScenarioIdentityDto,
} from "../../services/scenarioAugmentation";
import type { ScenarioStatus } from "../../pages/trialPrepData";

type Props = {
  slug: string;
  scenarioId: string;
  /**
   * Called after the status write lands, ON TOP of this component's own re-read.
   *
   * Optional, and its absence is a real answer rather than a gap: the strip took
   * the same callback and the Practice page passed it none, because nothing on
   * that page is derived from the scenario's status. The switch's own display
   * refreshes either way — see `statusChanged`. A surface that DOES derive
   * something from status passes a handler and gets both.
   */
  onStatusChanged?: () => void;
};

/**
 * The role chip and the Draft/Ready switch, in that order.
 *
 * Renders NOTHING until the identity lands, and renders its error rather than
 * swallowing it — the two states are different and a caller can tell them apart
 * on screen. It draws no title and no code: the surface that mounts this has its
 * own, which is the entire point of the split.
 */
const ScenarioIdentityControls: React.FC<Props> = ({
  slug,
  scenarioId,
  onStatusChanged,
}) => {
  const [identity, setIdentity] = useState<ScenarioIdentityDto | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    let cancelled = false;
    fetchAugmentationPanel(slug, scenarioId)
      .then((panel) => {
        if (cancelled) return;
        setIdentity(panel.identity);
        setError(null);
      })
      .catch((err: unknown) => {
        // Never swallowed (Standing Rule 1). Two controls silently missing from
        // a title line is indistinguishable from a scenario that has no status —
        // which is not a state this app has.
        if (!cancelled) setError(err instanceof Error ? err.message : "unknown error");
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId]);

  useEffect(() => load(), [load]);

  /**
   * The status write refreshes THIS component and then whatever the caller owns.
   *
   * The re-read is not optional and does not belong to the caller: the switch
   * shows `identity.status`, so without it the control would write "ready" to the
   * server and go on rendering "draft" — the screen disagreeing with the database
   * about a human's own act. Carried over from the strip unchanged.
   */
  const statusChanged = useCallback(() => {
    load();
    onStatusChanged?.();
  }, [load, onStatusChanged]);

  if (error !== null) return <span style={ss.sectionError}>{error}</span>;
  if (identity === null) return null;

  const role = directionChip(identity.direction);

  return (
    <>
      {/* The LABEL comes from `directionChip` — the one place that turns the
          stored `offense` into the word "Offensive". An unrecognised direction is
          shown verbatim and amber by that same helper: "Defensive" on a scenario
          the database calls something else would be the page inventing a posture.
          Direction is read-only — flipping it would make this a different
          scenario, and the update route refuses it, which is what the title says. */}
      <span
        style={ss.roleChip(isKnownDirection(identity.direction))}
        title={role.title ?? undefined}
      >
        {role.label}
      </span>

      <ScenarioStatusControl
        slug={slug}
        scenarioId={scenarioId}
        status={identity.status as ScenarioStatus}
        onChanged={statusChanged}
      />
    </>
  );
};

export default ScenarioIdentityControls;
