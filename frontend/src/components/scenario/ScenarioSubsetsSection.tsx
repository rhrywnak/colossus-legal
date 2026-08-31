// =============================================================================
// ScenarioSubsetsSection.tsx — attach and detach, where a reader can find them
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 4, approved as drawn, and
// design §11 item 4.
//
// ## ⚑ WHERE THIS LIVES, AND WHY IT IS NOT WHERE THE MOCKUP DREW IT
//
// Screen 4 draws this section on a scenario EDIT PAGE. **That page does not
// exist.** `App.tsx` declares no scenario-edit route; editing is
// `ScenarioIdentityModal`, opened from the detail page. The mockup drew a
// surface this app does not have — the same class of error as its dark frames.
//
// Roman ruled on 2026-08-31: do not build an edit route, do not put this in the
// identity modal, put it at the FOOT OF THE SCENARIO DETAIL PAGE as its own
// titled section. Everything inside it reproduces as drawn.
//
// It is written as ONE SELF-CONTAINED COMPONENT taking `(slug, scenarioId)` so
// that when a real Edit page arrives it moves there unchanged — no props to
// rethread, no state to lift.
//
// ## Why an explicit Attach/Detach button and not a ✓-toggle
//
// Defect D10: detaching was undiscoverable. The old control was a tick that
// read as a status, so nobody recognised it as the thing that would undo the
// attachment. Two named buttons, one visible state word, and a hint that says
// in as many words that detaching never deletes.
//
// ## ⚑ SAVE SCENARIO DOES NOT TOUCH ANY OF THIS
//
// Attach and Detach write IMMEDIATELY, each to its own endpoint, and each
// re-reads the list on success. There is nothing here for a Save button to
// commit, and the identity modal's Save has never known about attachments. A
// reader who attaches a subset and then presses Cancel on the identity form has
// still attached the subset — which is correct, and is why the buttons are
// named for the act rather than styled as form fields.

import React, { useCallback, useEffect, useState } from "react";

import {
  attachSubset,
  detachSubset,
  getScenarioSubsets,
  type AttachedSubset,
} from "../scenario-timeline/scenarioTimeline";
import { listSubsets, type SubsetSummary } from "../../services/caseTimelineSubsets";
import type { ScenarioIdentityWording } from "../../services/scenarioAugmentation";
import { timelinePath } from "../../utils/routePaths";
import ScenarioTimelineDock from "../scenario-timeline/ScenarioTimelineDock";
import { subsetRows, type SubsetRow } from "./editSubsets";
import * as ss from "./stripStyles";

type Props = {
  slug: string;
  scenarioId: string;
  /** The section's ten words, off the payload the detail page already reads. */
  wording: ScenarioIdentityWording;
};

const ScenarioSubsetsSection: React.FC<Props> = ({ slug, scenarioId, wording }) => {
  const [all, setAll] = useState<SubsetSummary[] | null>(null);
  const [attached, setAttached] = useState<AttachedSubset[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  /** The id currently being written, so its button can say it is busy. */
  const [busy, setBusy] = useState<string | null>(null);
  /**
   * The subset being PREVIEWED, if any.
   *
   * Held here rather than by the page, because the page has no other reason to
   * know this section exists — and a preview is this section's own transient
   * state, not a fact about the scenario.
   */
  const [preview, setPreview] = useState<string | null>(null);

  const load = useCallback(() => {
    setError(null);
    // BOTH reads, because a row needs the case's subsets AND this scenario's
    // attachments to know which button to draw. Either failure names itself.
    Promise.all([listSubsets(), getScenarioSubsets(slug, scenarioId)])
      .then(([cases, scenario]) => {
        setAll(cases);
        setAttached(scenario.subsets);
      })
      .catch((err: unknown) => {
        setError(err instanceof Error ? err.message : "unknown error");
      });
  }, [slug, scenarioId]);

  useEffect(() => load(), [load]);

  /**
   * Attach or detach, then RE-READ rather than patching local state.
   *
   * The re-read costs one request and buys correctness: `carried_by` on every
   * other row changes when this scenario takes a subset, and a local splice
   * would leave those stale. T1's endpoints answer with the new attachment list
   * and the errors are its named sentences, surfaced here rather than swallowed.
   */
  const write = useCallback(
    (id: string, attach: boolean) => {
      setBusy(id);
      setError(null);
      const call = attach
        ? attachSubset(slug, scenarioId, id)
        : detachSubset(slug, scenarioId, id);
      call
        .then(() => load())
        .catch((err: unknown) => {
          setError(err instanceof Error ? err.message : "unknown error");
        })
        .finally(() => setBusy(null));
    },
    [slug, scenarioId, load],
  );

  const rows: SubsetRow[] | null =
    all === null || attached === null ? null : subsetRows(all, attached);

  return (
    <section style={ss.section}>
      <h3 style={ss.sectionTitle}>{wording.edit_subsets_section_title}</h3>
      <div style={ss.sectionHint}>{wording.edit_subsets_section_hint}</div>

      {error !== null && <div style={ss.sectionError}>{error}</div>}

      {rows !== null &&
        rows.map((row) => (
          <div key={row.id} style={ss.subsetRow(row.attached)}>
            <div>
              <div style={ss.subsetName}>{row.name}</div>
              {row.description !== "" && (
                <div style={ss.subsetDescription}>{row.description}</div>
              )}
            </div>

            {/* The count comes from the CHRONOLOGY block's template, which is
                the one place that turns a number of references into words —
                the same row the window's title bar fills. */}
            <div style={ss.subsetCount}>{row.eventCount}</div>

            {/* The ✓ is furniture and lives in code; the words are the row. */}
            <div style={ss.subsetState(row.attached)}>
              {row.attached
                ? `✓ ${wording.edit_subsets_attached_state}`
                : wording.edit_subsets_not_attached_state}
            </div>

            <div style={ss.subsetActions}>
              <button
                type="button"
                style={ss.subsetLink}
                onClick={() => setPreview(row.id)}
              >
                {wording.edit_subsets_preview_link}
              </button>
              <button
                type="button"
                style={{
                  ...(row.attached ? ss.quietButton : ss.ghostButton),
                  ...ss.smallButton,
                  ...(busy === row.id ? { opacity: 0.5, cursor: "wait" } : {}),
                }}
                disabled={busy !== null}
                onClick={() => write(row.id, !row.attached)}
              >
                {row.attached
                  ? wording.edit_subsets_detach_button
                  : wording.edit_subsets_attach_button}
              </button>
            </div>
          </div>
        ))}

      {/* A NEW TAB, promised by the hint beside it: building a subset is a
          several-minute act on another screen, and the thing the reader came
          here to do is still open behind them. */}
      <div style={ss.createRow}>
        <a
          href={timelinePath()}
          target="_blank"
          rel="noopener noreferrer"
          style={ss.subsetLink}
        >
          + {wording.edit_subsets_create_link} →
        </a>
        <span style={ss.createHint}>({wording.edit_subsets_create_hint})</span>
      </div>

      {/* ⚑ PREVIEW: the floating window on one subset, attached or not.
          The dock is mounted a SECOND time here — the strip mounts one for the
          View Timeline button — and that is deliberate: this one is driven by
          `previewSubsetId` and draws no button of its own, so the two never
          render a control twice. On this page the window is z-40 with nothing
          above it, which is why Preview works plainly here and would not have
          inside the identity modal (z-9999). */}
      {preview !== null && (
        <ScenarioTimelineDock
          slug={slug}
          scenarioId={scenarioId}
          previewSubsetId={preview}
          onPreviewClosed={() => setPreview(null)}
        />
      )}
    </section>
  );
};

export default ScenarioSubsetsSection;
