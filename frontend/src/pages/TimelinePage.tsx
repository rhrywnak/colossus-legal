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
// ## What is deliberately NOT here
//
// Add event, the ✎/🗑 controls, the People filter and the "Spine only" toggle.
// Writes are Phase C; People and spine have no data behind them. The honest-gap
// law: a control that cannot work is not drawn.

import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useNavigate, useSearchParams } from "react-router-dom";

import { timelineEventPath } from "../utils/routePaths";

import TimelineEventCard from "../components/timeline/TimelineEventCard";
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
  BOOTSTRAP_TEXT,
  type CaseTimeline,
  cw,
  fill,
  getCaseTimeline,
} from "../services/caseTimeline";

/** The query parameter that carries an expanded phase across a reload. */
const PHASE_PARAM = "phase";

const TimelinePage: React.FC = () => {
  const navigate = useNavigate();
  const [params, setParams] = useSearchParams();
  const [data, setData] = useState<CaseTimeline | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [local, setLocal] = useState<TimelineFilters>(NO_FILTERS);

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
      </div>

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

      {data.events.length === 0 ? (
        <div style={s.state}>{cw(data.wording, "empty_label")}</div>
      ) : shown.length === 0 && isFiltered(filters) ? (
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
                  <TimelineEventCard
                    event={event}
                    tags={data.tags}
                    wording={data.wording}
                    onOpen={openEvent}
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
              onToggleExpand={() =>
                setPhase(filters.phase === group.phase.id ? null : group.phase.id)
              }
              onOpenEvent={openEvent}
            />
          ))}
        </>
      )}
    </div>
  );
};

export default TimelinePage;
