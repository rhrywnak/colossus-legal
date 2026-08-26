// =============================================================================
// TimelinePage.tsx — the case chronology, read from the database
// =============================================================================
//
// Chronology Phase B, mockup v2 Screens 1 and 1b. This page used to fetch a
// static JSON file baked into the frontend image; it reads `GET /api/timeline`
// now, and the file is gone.
//
// ## ⚑ TWO DEFECTS DIE WITH THIS REWRITE
//
// The page it replaces did `fetch("/data/timeline.json").catch(() => {})` with
// no timeout at all. Both are closed here, not by care but by construction: the
// request goes through `authFetch`, which arms an `AbortController` at the
// standing ceiling, and every failure below reaches a rendered, stored sentence.
// A network failure and an empty case are DIFFERENT screens.
//
// ## And a third silent path, which was never even a defect report
//
// An event naming a phase that has no row used to fall out of the render
// entirely — it belonged to no section, so nothing drew it. It now renders in a
// loud row naming its id. An event nobody can see is an event nobody can fix.
//
// ## Phase C: the page writes
//
// + Add event, the always-visible ✎/🗑 on every card (R17), and the undo line
// that replaces a deleted card IN PLACE (R10, no confirm dialog anywhere).
//
// ## ⚑ THE ACTIVE FILTER SURVIVES EVERY WRITE (§C3)
//
// Nothing here refetches the payload after a write. The server's answer is
// PATCHED into the list the page already holds (`patchEventList`), so the tag
// chip, the search box, the date range and the expanded phase are never touched
// by a save, a delete or an undo. Refetching would be one line shorter and would
// re-run the load effect, which is how a filter gets quietly reset under
// somebody mid-read.
//
// ## What is STILL deliberately not here
//
// The People filter and the "Spine only" toggle. Neither has data behind it. The
// honest-gap law: a control that cannot work is not drawn.

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";

import { timelineEventPath } from "../utils/routePaths";

import TimelineEventCard from "../components/timeline/TimelineEventCard";
import TimelineEventForm from "../components/timeline/TimelineEventForm";
import TimelineFilterBar from "../components/timeline/TimelineFilterBar";
import TimelinePhaseSection from "../components/timeline/TimelinePhaseSection";
import {
  applyFilters,
  groupByPhase,
  isFiltered,
  NO_FILTERS,
  subtitleOf,
  type TimelineFilters,
  unknownPhaseEvents,
} from "../components/timeline/timelineFilters";
import * as s from "../components/timeline/timelineStyles";
import {
  blankForm,
  type EventFormState,
  formFromEvent,
  formToRequest,
  patchEventList,
} from "../components/timeline/timelineWriteRules";
import * as w from "../components/timeline/timelineWriteStyles";
import {
  BOOTSTRAP_TEXT,
  type CaseTimeline,
  type CaseTimelineEvent,
  cw,
  fill,
  getCaseTimeline,
  type TimelineEvent,
} from "../services/caseTimeline";
import {
  createTimelineEvent,
  deleteTimelineEvent,
  undeleteTimelineEvent,
  updateTimelineEvent,
} from "../services/caseTimelineWrites";

/** The query parameter that carries an expanded phase across a reload. */
const PHASE_PARAM = "phase";

/**
 * Today, as `<input type="date">` spells it.
 *
 * Read here rather than inside `blankForm`, which is pure and testable because
 * it takes the date as an argument. `sv-SE` is the locale trick this codebase
 * uses elsewhere for an ISO date in LOCAL time — `toISOString()` would be UTC,
 * which puts a new event on yesterday for anybody typing after 8pm Eastern.
 */
function todayIso(): string {
  return new Date().toLocaleDateString("sv-SE");
}

/** Which form, if any, is open — and on what. */
type FormState =
  | { kind: "closed" }
  | { kind: "adding"; form: EventFormState }
  | { kind: "editing"; id: string; form: EventFormState };

const TimelinePage: React.FC = () => {
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const [data, setData] = useState<CaseTimeline | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [local, setLocal] = useState<TimelineFilters>(NO_FILTERS);
  const [formState, setFormState] = useState<FormState>({ kind: "closed" });
  const [saving, setSaving] = useState(false);
  const [writeError, setWriteError] = useState<string | null>(null);
  // Events deleted in THIS visit, whose undo line stands where the card was
  // until the reader navigates (R10). Page state, not list state: the list holds
  // what the server would return, and the server returns no deleted event.
  const [undoable, setUndoable] = useState<TimelineEvent[]>([]);

  // The phase filter lives in the URL so an expanded phase survives a reload and
  // can be linked to — which is what the home band's pills point at.
  const filters: TimelineFilters = useMemo(
    () => ({ ...local, phase: params.get(PHASE_PARAM) }),
    [local, params],
  );

  useEffect(() => {
    let cancelled = false;
    getCaseTimeline()
      .then((payload) => {
        if (!cancelled) setData(payload);
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : "unknown error");
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const setPhase = useCallback(
    (phase: string | null) => {
      const next = new URLSearchParams(params);
      if (phase === null) next.delete(PHASE_PARAM);
      else next.set(PHASE_PARAM, phase);
      setParams(next, { replace: false });
    },
    [params, setParams],
  );

  const openEvent = useCallback(
    // `params`, not `window.location.search`: the router's state is the
    // authority here, and reading the URL directly returns the PREVIOUS value
    // while a queued update has not yet committed — which would carry the wrong
    // phase back with the reader, silently, with nothing to see or log.
    (id: string) => {
      const query = params.toString();
      navigate(timelineEventPath(id) + (query === "" ? "" : `?${query}`));
    },
    [navigate, params],
  );

  /**
   * Run one write and fold the server's answer back into the list.
   *
   * ⚑ ONE place, so the four write paths cannot disagree about what happens
   * after a write. Every one of them: clears the previous failure, marks the
   * controls busy, PATCHES the returned event into the list the page already
   * holds, and — if it fails — leaves a sentence on screen. The filters are
   * never read or written here, which is exactly why they survive (§C3).
   */
  const runWrite = useCallback(
    async (write: () => Promise<CaseTimelineEvent>) => {
      setWriteError(null);
      setSaving(true);
      try {
        const written = await write();
        setData((current) =>
          current === null
            ? current
            : { ...current, events: patchEventList(current.events, written) },
        );
        return written;
      } catch (err: unknown) {
        // Never swallowed, never a dead button: the sentence is rendered by the
        // form when one is open, and above the list when one is not.
        setWriteError(err instanceof Error ? err.message : "unknown error");
        return null;
      } finally {
        setSaving(false);
      }
    },
    [],
  );

  const saveForm = useCallback(async () => {
    if (formState.kind === "closed") return;
    // Narrowed by the discriminant rather than cast: `formState.id` only exists
    // on the editing arm, and letting the compiler prove that is what stops an
    // add from one day being sent to the edit endpoint with `undefined`.
    const request = formToRequest(formState.form, formState.kind === "adding");
    const written = await runWrite(() =>
      formState.kind === "adding"
        ? createTimelineEvent(request)
        : updateTimelineEvent(formState.id, request),
    );
    // The form stays OPEN on failure, holding what was typed. Closing it would
    // throw away the author's words along with the explanation of why they were
    // not saved.
    if (written !== null) setFormState({ kind: "closed" });
  }, [formState, runWrite]);

  const deleteEvent = useCallback(
    async (event: TimelineEvent) => {
      // No confirm dialog, by ruling R10 — the undo line below IS the safety.
      const written = await runWrite(() => deleteTimelineEvent(event.id));
      if (written !== null) {
        setUndoable((current) => [...current.filter((e) => e.id !== event.id), event]);
        // An open form on the event that just went is a form with nothing behind
        // it. Closed rather than left to submit into a deleted row.
        setFormState((current) =>
          current.kind === "editing" && current.id === event.id ? { kind: "closed" } : current,
        );
      }
    },
    [runWrite],
  );

  const undoDelete = useCallback(
    async (event: TimelineEvent) => {
      const written = await runWrite(() => undeleteTimelineEvent(event.id));
      if (written !== null) {
        setUndoable((current) => current.filter((e) => e.id !== event.id));
      }
    },
    [runWrite],
  );

  if (loading) {
    // See BOOTSTRAP_TEXT: the store that holds every other word on this page
    // has not arrived yet, and cannot, until this request finishes.
    return (
      <div style={s.state} aria-busy="true">
        {BOOTSTRAP_TEXT.loading}
      </div>
    );
  }
  if (error !== null || data === null) {
    // A network failure NAMES ITSELF. The page this replaces rendered nothing
    // at all here, having swallowed the rejection.
    return (
      <div style={s.errorState}>
        {BOOTSTRAP_TEXT.timelineFailed(error ?? "unknown error")}
      </div>
    );
  }

  const shown = applyFilters(data.events, filters);
  const groups = groupByPhase(data.phases, shown);
  const orphans = unknownPhaseEvents(data.phases, shown);
  const visiblePhases =
    filters.phase === null ? groups : groups.filter((g) => g.phase.id === filters.phase);

  return (
    <div style={s.page}>
      <div style={s.titleRow}>
        <h1 style={s.h1}>{cw(data.wording, "page_title")}</h1>
        <p style={s.subCount}>{subtitleOf(data, filters, shown.length)}</p>
        {/* Drawn for every reader, per R2: Roman, Chuck and Marie are equal
            authors, so there is no version of this page where the button
            exists for one of them and not another. An unauthenticated reader
            gets a 401 from the server with a sentence, which is a truthful
            answer — a hidden control would be a silent one. */}
        <button
          type="button"
          style={{ ...w.button, marginTop: "0.6rem" }}
          disabled={saving}
          onClick={() =>
            setFormState({
              kind: "adding",
              // The first stored phase, never a slug named in code: the phases
              // are data (design R15) and a compiled-in default would be the
              // one place a fifth phase could not reach.
              form: blankForm(todayIso(), data.phases[0]?.id ?? ""),
            })
          }
        >
          {cw(data.wording, "add_event_label")}
        </button>
      </div>

      {formState.kind !== "closed" && (
        <TimelineEventForm
          creating={formState.kind === "adding"}
          form={formState.form}
          onChange={(form) => setFormState({ ...formState, form })}
          tags={data.tags}
          phases={data.phases}
          wording={data.wording}
          saving={saving}
          error={writeError}
          // Links ride a CREATE only: an edit that replaced an event's link set
          // would delete a colleague's link while somebody re-typed a title.
          withLinks={formState.kind === "adding"}
          onSave={() => void saveForm()}
          onCancel={() => {
            setFormState({ kind: "closed" });
            setWriteError(null);
          }}
        />
      )}

      {/* A write that failed with no form open — a delete or an undo — still
          reaches a rendered sentence. §C3: never silent, never a dead button. */}
      {writeError !== null && formState.kind === "closed" && (
        <div style={w.writeError}>
          {fill(cw(data.wording, "write_failed_template"), { reason: writeError })}
        </div>
      )}

      <TimelineFilterBar
        tags={data.tags}
        phases={data.phases}
        wording={data.wording}
        filters={filters}
        onChange={(next) => {
          setLocal({ ...next, phase: null });
          if (next.phase !== filters.phase) setPhase(next.phase);
        }}
      />

      {/* `undoable` is part of BOTH conditions: deleting the only event in the
          case, or the only one the filters kept, must not replace its undo line
          with "no events" — that would take away the only way back before the
          reader had a chance to use it. */}
      {data.events.length === 0 && undoable.length === 0 ? (
        <div style={s.state}>{cw(data.wording, "empty_label")}</div>
      ) : shown.length === 0 && undoable.length === 0 && isFiltered(filters) ? (
        // A different sentence from the empty case, deliberately: "there is
        // nothing here" and "your filters hid everything" send a reader to two
        // different places.
        <div style={s.state}>{cw(data.wording, "no_matches_label")}</div>
      ) : (
        <>
          {orphans.length > 0 && (
            <div>
              {orphans.map((event) => (
                <div key={event.id} style={s.unknownPhase}>
                  {fill(cw(data.wording, "unknown_phase_template"), {
                    id: event.id,
                    phase: event.phase,
                  })}
                  {/* An unknown-phase event is EDITABLE, deliberately: this
                      loud row exists so the event can be corrected, and a row
                      you can see but not fix is only half the fix. */}
                  <TimelineEventCard
                    event={event}
                    tags={data.tags}
                    wording={data.wording}
                    onOpen={openEvent}
                    onEdit={(target) =>
                      setFormState({
                        kind: "editing",
                        id: target.id,
                        form: formFromEvent(target),
                      })
                    }
                    onDelete={(target) => void deleteEvent(target)}
                  />
                </div>
              ))}
            </div>
          )}

          {visiblePhases.map((group) => (
            <TimelinePhaseSection
              key={group.phase.id}
              phase={group.phase}
              events={group.events}
              tags={data.tags}
              wording={data.wording}
              windowEvents={data.phase_window_events}
              expanded={filters.phase === group.phase.id}
              undoable={undoable}
              onToggleExpand={() =>
                setPhase(filters.phase === group.phase.id ? null : group.phase.id)
              }
              onOpenEvent={openEvent}
              onEditEvent={(target) =>
                setFormState({ kind: "editing", id: target.id, form: formFromEvent(target) })
              }
              onDeleteEvent={(target) => void deleteEvent(target)}
              onUndoDelete={(target) => void undoDelete(target)}
            />
          ))}
        </>
      )}
    </div>
  );
};

export default TimelinePage;
