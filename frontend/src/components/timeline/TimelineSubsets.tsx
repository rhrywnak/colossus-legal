// =============================================================================
// TimelineSubsets.tsx — the Subsets section and everything it opens
// =============================================================================
//
// Mockup Screen 2, below the phase sections, which do not change in any pixel.
// The whole feature mounts from ONE line on `TimelinePage`, deliberately: the
// page is a chronology of events and this is a read over it, so the page holds
// no subset state and cannot have its filters disturbed by one.
//
// ## ⚑ THE PAGE'S FILTERS SURVIVE THE MODAL (T2.3)
//
// Nothing here touches `TimelineFilters` or the URL. The picker is handed the
// UNFILTERED event list — a story is picked from the whole chronology, not from
// whatever the search box last narrowed — and closing the modal leaves the page
// exactly as it was found, because the page never learned the modal existed.
//
// ## Every visible word is a stored row
//
// Seventeen of them. The section reads ten; the modal reads the rest. There is
// no user-visible string in this file or its sibling — `cw` throws by name if
// the store and this build disagree, which is louder than a blank control.

import React, { useCallback, useEffect, useState } from "react";

import type {
  ChronologyWording,
  TimelineEvent,
  TimelinePhase,
} from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import {
  createSubset,
  deleteSubset,
  listSubsets,
  replaceSubsetEvents,
  SubsetWriteError,
  type SubsetDetail,
  type SubsetSummary,
  getSubset,
  undeleteSubset,
  updateSubset,
} from "../../services/caseTimelineSubsets";
import SubsetModal from "./SubsetModal";
import { gapLine, type Pick, toSubsetPayload } from "./subsetPicker";
import { eventsAreDirty, type SaveFailure } from "./subsetSaveModel";
import * as ss from "./subsetStyles";
import * as w from "./timelineWriteStyles";

type Props = {
  events: TimelineEvent[];
  phases: TimelinePhase[];
  wording: ChronologyWording;
};

/** Which modal is open, and on what. `loading` is the gap between click and read. */
type ModalState =
  | { kind: "closed" }
  | { kind: "loading" }
  | { kind: "adding" }
  | { kind: "editing"; subset: SubsetDetail };

const TimelineSubsets: React.FC<Props> = ({ events, phases, wording }) => {
  const [subsets, setSubsets] = useState<SubsetSummary[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [modal, setModal] = useState<ModalState>({ kind: "closed" });
  const [saving, setSaving] = useState(false);
  const [writeError, setWriteError] = useState<string | null>(null);
  // The modal's own banner (T6.4). Separate from `writeError` because it is a
  // richer thing — it must be able to say that HALF the save landed — and
  // because it belongs to a screen the section's delete/undo writes are not on.
  const [saveFailure, setSaveFailure] = useState<SaveFailure | null>(null);
  // Subsets deleted in THIS visit, whose undo line stands where the row was
  // (R10, the practice-page pattern — no confirm dialog anywhere).
  const [undoable, setUndoable] = useState<SubsetSummary[]>([]);

  const reload = useCallback(async () => {
    try {
      setSubsets(await listSubsets());
      setError(null);
    } catch (err: unknown) {
      // Never swallowed: the section renders this sentence in place of its list.
      setError(err instanceof Error ? err.message : "unknown error");
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  /** Run one write, then re-read the list — the section's single write path. */
  const runWrite = useCallback(
    async (write: () => Promise<unknown>): Promise<boolean> => {
      setWriteError(null);
      setSaving(true);
      try {
        await write();
        await reload();
        return true;
      } catch (err: unknown) {
        setWriteError(err instanceof Error ? err.message : "unknown error");
        return false;
      } finally {
        setSaving(false);
      }
    },
    [reload],
  );

  const openEdit = useCallback(async (id: string) => {
    setSaveFailure(null);
    setModal({ kind: "loading" });
    try {
      setModal({ kind: "editing", subset: await getSubset(id) });
    } catch (err: unknown) {
      // The modal never opens on a failed read — an empty picker over a subset
      // that exists would invite a Save that emptied it.
      setModal({ kind: "closed" });
      setError(err instanceof Error ? err.message : "unknown error");
    }
  }, []);

  /**
   * One Save: up to two writes, and an honest account of what landed (T6.4).
   *
   * ## ⚑ WHY THIS IS NOT `runWrite`
   *
   * `runWrite` reports one failure for one write. A save is two, and the whole
   * point of D2 is that the SECOND can fail after the FIRST has committed — so
   * this path has to remember which calls it got through before it threw. It
   * still ends where `runWrite` ends: the list is re-read, `saving` is cleared,
   * and nothing is swallowed.
   */
  const save = useCallback(
    async (name: string, description: string, picks: Pick[]) => {
      const events_ = toSubsetPayload(picks);
      const current = modal.kind === "editing" ? modal.subset : null;
      setSaveFailure(null);
      setWriteError(null);
      setSaving(true);
      // Tracks what has COMMITTED, so a failure knows which half of the banner
      // to draw. It is only ever set after an `await` returns without throwing.
      let nameSaved = false;
      try {
        if (current === null) {
          await createSubset(name, description, events_);
        } else {
          // Name/description only when CHANGED — an unchanged pair would still
          // be a legal write, but it would put an `updated` row in the history
          // for an act nobody performed.
          if (name !== current.name || description !== current.description) {
            await updateSubset(current.id, name, description);
            nameSaved = true;
          }
          // The COMPLETE ordered set, and only when it has actually moved.
          // T1's replace semantics: one human act, one history row, never
          // per-row calls. See `eventsAreDirty` for why the skip is here.
          if (eventsAreDirty(current, picks)) {
            await replaceSubsetEvents(current.id, events_);
          }
        }
      } catch (err: unknown) {
        // The modal stays OPEN, holding what was typed and picked, and the
        // banner says which of the two calls failed and what the server said.
        // ⚑ The `"unknown error"` fallback is the FOURTH site of a hardcoded
        // English string in this file (lines 82, 100, 118 are the others, all
        // pre-existing). It is kept identical to them deliberately: the fix is
        // one wording row that all four read, not a longer sentence written in
        // code here, which would make the inconsistency worse rather than
        // better. Filed in the T6 report under NEXT. The path is reachable only
        // if something that is not an `Error` is thrown — nothing in the
        // service does — and it is SHOWN rather than swallowed, so Rule 1 holds.
        const sentence = err instanceof Error ? err.message : "unknown error";
        setSaveFailure({
          nameSaved,
          status: err instanceof SubsetWriteError ? err.status : null,
          reason: err instanceof SubsetWriteError ? err.reason : "",
          sentence,
        });
        setSaving(false);
        return;
      }
      // ⚑ THE RE-READ IS OUTSIDE THE WRITE'S try, AND THAT IS THE POINT
      //
      // Everything above committed. If the list re-read then fails — a network
      // blip between two requests in one interaction — the save DID land, and a
      // banner reading "the event list was not saved" over a list that WAS
      // saved is the same lie D2 exists to end, wearing different clothes. So
      // the modal closes on the writes' success and the re-read reports itself,
      // in the section's own error line, where a failed read belongs.
      //
      // `reload` already swallows nothing: it sets `error`, which the section
      // renders in place of its list. Nothing is lost by letting it own this.
      try {
        await reload();
      } finally {
        setModal({ kind: "closed" });
        setSaving(false);
      }
    },
    [modal, reload],
  );

  const remove = useCallback(
    async (subset: SubsetSummary) => {
      const ok = await runWrite(() => deleteSubset(subset.id));
      if (ok) {
        setUndoable((c) => [...c.filter((s) => s.id !== subset.id), subset]);
        setModal({ kind: "closed" });
      }
    },
    [runWrite],
  );

  const undo = useCallback(
    async (subset: SubsetSummary) => {
      const ok = await runWrite(() => undeleteSubset(subset.id));
      if (ok) setUndoable((c) => c.filter((s) => s.id !== subset.id));
    },
    [runWrite],
  );

  return (
    <section>
      <div style={ss.sectionHead}>
        <span style={ss.sectionTitle}>{cw(wording, "subsets_section_title")}</span>
        <span style={ss.sectionSubtitle}>{cw(wording, "subsets_section_subtitle")}</span>
        <button
          type="button"
          style={ss.addButton}
          disabled={saving}
          onClick={() => {
            // A banner from the LAST save is not about this one. Cleared here,
            // on Cancel and on openEdit — the three doors into the modal.
            setWriteError(null);
            setSaveFailure(null);
            setModal({ kind: "adding" });
          }}
        >
          {cw(wording, "subsets_add_button")}
        </button>
      </div>

      {/* A read that failed NAMES ITSELF where the list would have been. */}
      {error !== null && <div style={w.writeError}>{error}</div>}

      {/* A write that failed with no modal open — a delete or an undo — still
          reaches a rendered sentence. Never silent, never a dead button. */}
      {writeError !== null && modal.kind === "closed" && (
        <div style={w.writeError}>
          {fill(cw(wording, "write_failed_template"), { reason: writeError })}
        </div>
      )}

      {/* `undoable` is part of the condition: deleting the only subset must not
          replace its undo line with the empty state, which would take away the
          only way back before the reader had a chance to use it. */}
      {error === null && subsets.length === 0 && undoable.length === 0 && (
        <div style={ss.emptyState}>{cw(wording, "subsets_empty_state")}</div>
      )}

      {subsets.map((subset) => (
        <div key={subset.id} style={ss.row}>
          <div>
            <div style={ss.rowName}>{subset.name}</div>
            {subset.description !== "" && (
              <div style={ss.rowDescription}>{subset.description}</div>
            )}
          </div>
          <div style={ss.rowCount}>
            {fill(cw(wording, "subsets_event_count_template"), { count: subset.event_count })}
            {subset.gap_count > 0 && (
              <div style={ss.rowGaps}>{gapLine(wording, subset.gap_count)}</div>
            )}
          </div>
          <div style={ss.rowCarriedBy}>
            {subset.carried_by.length > 0 && (
              <>
                {cw(wording, "subsets_carried_by_prefix")}{" "}
                {subset.carried_by.map((code) => (
                  <span key={code} style={ss.codeChip}>
                    {code}
                  </span>
                ))}
              </>
            )}
            <div style={ss.rowBy}>
              {subset.created_by} · {subset.created_at.slice(0, 10)}
            </div>
          </div>
          <div style={ss.rowActions}>
            {/* Open is the edit view in READ mode — the page's filter bar has no
                subset axis and cannot express "this subset", so the instruction's
                fallback applies. Stated in the report. */}
            <button
              type="button"
              style={ss.rowAction}
              onClick={() => void openEdit(subset.id)}
            >
              {cw(wording, "subsets_window_open_timeline")}
            </button>
            <button
              type="button"
              style={ss.rowAction}
              onClick={() => void openEdit(subset.id)}
            >
              {cw(wording, "edit_label")}
            </button>
          </div>
        </div>
      ))}

      {undoable.map((subset) => (
        <div key={`undo:${subset.id}`} style={w.undoLine}>
          <span>
            {cw(wording, "deleted_line_label")} {subset.name}
          </span>
          <button
            type="button"
            style={w.undoAction}
            disabled={saving}
            onClick={() => void undo(subset)}
          >
            {cw(wording, "undo_label")}
          </button>
        </div>
      ))}

      {(modal.kind === "adding" || modal.kind === "editing") && (
        <SubsetModal
          subset={modal.kind === "editing" ? modal.subset : null}
          events={events}
          phases={phases}
          wording={wording}
          saving={saving}
          failure={saveFailure}
          onSave={(name, description, picks) => void save(name, description, picks)}
          onCancel={() => {
            setModal({ kind: "closed" });
            setWriteError(null);
            setSaveFailure(null);
          }}
          onDelete={
            modal.kind === "editing"
              ? () => {
                  const row = subsets.find((s) => s.id === modal.subset.id);
                  if (row !== undefined) void remove(row);
                }
              : undefined
          }
        />
      )}
    </section>
  );
};

export default TimelineSubsets;
