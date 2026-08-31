// =============================================================================
// ScenarioHeaderStrip.tsx — one header for every scenario surface
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 1, approved as drawn, and
// design §11 item 1. Light frames only — this app has one palette.
//
// ## ⚑ WHAT THIS ENDS
//
// Roman, 2026-08-31, on the header as it was: "It looks like crap … very
// chaotic." Five surfaces each owned a piece of it —
//
//   detail      ScenarioHeaderTiers   two tiers, six controls, and a sentence
//   rehearsal   RehearsalPageHeader   its own, used by that page alone
//   practice    PracticeTitleRow      a bare <h1> off the deck payload
//   dashboard   ScenarioCard          no per-scenario header at all
//   questions   —                     nothing
//
// — and each grew its own controls, its own spacing and its own idea of where
// Delete goes. This is the one strip. Four surfaces render it: the dashboard
// row, the detail page, practice and rehearsal. The questions page is
// deliberately NOT one of them (Roman, ruling 6): a full strip above a single
// cross-examination question is noise on a surface kept empty on purpose.
//
// ## ⚑ IT FETCHES ITS OWN DATA, AND THAT IS THE ONLY SHAPE THAT WORKS
//
// A props-only strip is impossible here, and the evidence is in the payloads:
// `PracticeDeck` and `RehearsalScenario` carry NO status and no direction;
// `ScenarioSummary` carries no direction. Passing the fields in would have meant
// widening three DTOs and teaching four pages about this component — which is
// exactly the shape `ScenarioTimelineDock` rejected, and recorded in its own
// header, before settling on a one-line mount.
//
// So it reads the augmentation panel: the one payload that already carries the
// code, the name, the direction, the header's own vocabulary — and, since T5,
// the status. One field on one DTO replaced three widened payloads.
//
// ## What the pages still own
//
// `onEdit` / `onDelete` / `onStatusChanged` are callbacks because the DIALOGS
// are the page's (the detail page owns the identity modal and the delete
// confirm, and re-reads itself after either). `children` is the slot for the
// two genuinely page-owned control groups the mockup does not draw: rehearsal's
// ‹ Previous / position / Next ›, and practice's print trio. Those are derived
// from page state — an index into a payload, a lock computed from a question
// list — and cannot come from `(slug, scenarioId)`.

import React, { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";

import ScenarioStatusControl from "../ScenarioStatusControl";
import ScenarioTimelineDock from "../scenario-timeline/ScenarioTimelineDock";
import {
  fetchAugmentationPanel,
  type ScenarioIdentityDto,
  type ScenarioIdentityWording,
} from "../../services/scenarioAugmentation";
import type { ScenarioStatus } from "../../pages/trialPrepData";
import { practicePath, rehearsalScenarioPath } from "../../utils/routePaths";
import { directionChip } from "../scenarioHeader";
import { isKnownDirection, stripControls } from "./headerStripRules";
import * as ss from "./stripStyles";

type Props = {
  slug: string;
  scenarioId: string;
  /** Surfaces where the action is meaningless hide it. Each hide is reported. */
  hidePractice?: boolean;
  hideRehearsal?: boolean;
  hideEdit?: boolean;
  hideDelete?: boolean;
  hideStatus?: boolean;
  /** The page owns the identity modal. Absent ⇒ Edit is not drawn. */
  onEdit?: () => void;
  /** The page owns the delete confirm. Absent ⇒ Delete is not drawn. */
  onDelete?: () => void;
  /** Called after the status control writes, so the page can re-read itself. */
  onStatusChanged?: () => void;
  /** Page-owned controls the mockup does not draw — rehearsal's nav, practice's print trio. */
  children?: React.ReactNode;
};

const ScenarioHeaderStrip: React.FC<Props> = ({
  slug,
  scenarioId,
  hidePractice = false,
  hideRehearsal = false,
  hideEdit = false,
  hideDelete = false,
  hideStatus = false,
  onEdit,
  onDelete,
  onStatusChanged,
  children,
}) => {
  const [identity, setIdentity] = useState<ScenarioIdentityDto | null>(null);
  const [wording, setWording] = useState<ScenarioIdentityWording | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => {
    let cancelled = false;
    fetchAugmentationPanel(slug, scenarioId)
      .then((panel) => {
        if (cancelled) return;
        setIdentity(panel.identity);
        setWording(panel.identity_wording);
      })
      .catch((err: unknown) => {
        // Never swallowed. A strip that failed to read renders its reason rather
        // than a headerless page, because a page whose title silently vanished
        // reads as a broken route.
        if (!cancelled) setError(err instanceof Error ? err.message : "unknown error");
      });
    return () => {
      cancelled = true;
    };
  }, [slug, scenarioId]);

  useEffect(() => load(), [load]);

  /** The status write must refresh BOTH this strip and the page under it. */
  const statusChanged = useCallback(() => {
    load();
    onStatusChanged?.();
  }, [load, onStatusChanged]);

  if (error !== null) return <div style={ss.sectionError}>{error}</div>;
  if (identity === null || wording === null) return null;

  const controls = stripControls(identity.status);
  const role = directionChip(identity.direction);
  const knownRole = isKnownDirection(identity.direction);

  return (
    // `data-scenario-strip` is a stable handle, in the manner of the header's
    // own `data-app-chrome`: it is what the harness selects to photograph one
    // strip out of four on the dashboard, and what a future test would assert
    // the count of. It carries no styling and no behaviour.
    <div style={ss.strip} data-scenario-strip>
      {/* Row 1 — ONE line: code · title · role · status · the two actions. */}
      <div style={ss.row1}>
        <span style={ss.code}>{identity.code}</span>
        <h1 style={ss.title}>{identity.name}</h1>
        {/* Mockup `.chip.role`, and the LABEL comes from `directionChip` — the
            one place that turns the stored `offense` into the word "Offensive".
            Rendering `identity.direction` raw is what this drew at first, and it
            put the database's token on screen where the drawing has English.

            An unrecognised direction is shown VERBATIM and amber by that same
            helper: "Defensive" on a scenario the database calls something else
            would be the page inventing a posture. Direction is read-only —
            flipping it would make this a different scenario, and the update
            route refuses it, which is what the chip's title says. */}
        <span style={ss.roleChip(knownRole)} title={role.title ?? undefined}>
          {role.label}
        </span>

        {!hideStatus && (
          <ScenarioStatusControl
            slug={slug}
            scenarioId={scenarioId}
            status={identity.status as ScenarioStatus}
            onChanged={statusChanged}
          />
        )}

        <span style={ss.actions}>
          {/* ⚑ VIEW TIMELINE lives here, inside the strip's action slot, and the
              dock draws it. The dock already hides its own button when the
              scenario carries no subset — Screen 1's "simply absent, nothing
              else shifts" — so the strip does not second-guess it with a count
              of its own. The dock also owns the floating window and portals it
              to `body`, so nothing about this position constrains it. */}
          <ScenarioTimelineDock slug={slug} scenarioId={scenarioId} />

          {!hidePractice && controls.practiceEnabled && (
            <Link to={practicePath(slug, scenarioId)} style={ss.solidButton}>
              {wording.practice_link_label}
            </Link>
          )}
        </span>
      </div>

      {/* Row 2 — Edit · Rehearsal view · … · Delete. */}
      <div style={ss.row2}>
        {!hideEdit && onEdit !== undefined && (
          <button type="button" style={ss.quietButton} onClick={onEdit}>
            ✎ Edit
          </button>
        )}

        {/* ⚑ THE SENTENCE IS GONE; THE TOOLTIP REMAINS.
            `ScenarioHeaderTiers` rendered the long blocked-reason TWICE — as
            this control's `title` and again as a visible line beside it. Screen
            1 keeps the tooltip and removes the line, and the tooltip is now the
            short stored row rather than the {status}-filled sentence: a control
            disabled for being Draft need not say "Draft" when the segmented
            control an inch to its left already does.

            Still a <span> and not a disabled <button> when inert, for the reason
            .390 recorded: it is not a control that failed, it is a destination
            that does not exist yet. */}
        {!hideRehearsal &&
          (controls.rehearsalEnabled ? (
            <Link
              to={rehearsalScenarioPath(slug, identity.code)}
              style={ss.quietButton}
            >
              Rehearsal view
            </Link>
          ) : (
            <span style={ss.quietDisabled} title={wording.rehearsal_disabled_tooltip}>
              Rehearsal view
            </span>
          ))}

        {children}

        {!hideDelete && onDelete !== undefined && (
          <span style={ss.row2Right}>
            <button type="button" style={ss.dangerButton} onClick={onDelete}>
              Delete
            </button>
          </span>
        )}
      </div>
    </div>
  );
};

export default ScenarioHeaderStrip;
