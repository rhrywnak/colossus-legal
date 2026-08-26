// =============================================================================
// TimelineEventPage.tsx — one event, in full (mockup v2 Screen 2, read-only)
// =============================================================================
//
// Chronology Phase B, §B5. Everything the list could not fit: the whole fact,
// the document links with their pinpoints and resolution states, the notes as
// individual attributed entries (design R8), and the history.
//
// ## The empty panels are RENDERED, not hidden
//
// Notes and history are empty for every event today — nothing writes either
// until Phase C. They are drawn anyway, each with its stored empty line. A
// hidden panel reads as a feature that does not exist; an empty one reads as a
// feature with nothing in it yet, which is the truth.
//
// ## What is deliberately absent
//
// The mockup's People panel, the Add-note input, and Edit/Delete. People has no
// data behind it, and the other two are writes. The honest-gap law: a control
// that cannot work is not drawn.

import React, { useEffect, useState } from "react";
import { Link, useLocation, useParams } from "react-router-dom";

import { API_BASE_URL } from "../services/api";
import {
  BOOTSTRAP_TEXT,
  type CaseTimelineEvent,
  type ChronologyWording,
  cw,
  getCaseTimeline,
  getTimelineEvent,
  type TimelinePhase,
  type TimelineTag,
} from "../services/caseTimeline";
import { formatEventDate, linkRendering, tagOf } from "../components/timeline/timelineFilters";
import * as s from "../components/timeline/timelineStyles";

type Loaded = {
  event: CaseTimelineEvent;
  wording: ChronologyWording;
  tags: TimelineTag[];
  phases: TimelinePhase[];
};

const TimelineEventPage: React.FC = () => {
  const { id = "" } = useParams();
  const location = useLocation();
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    // Two reads, in parallel: the event itself, and the timeline payload that
    // carries the words and the tag vocabulary. The event endpoint deliberately
    // does not repeat twenty-nine strings for one row.
    Promise.all([getTimelineEvent(id), getCaseTimeline()])
      .then(([event, timeline]) => {
        if (!cancelled) {
          setLoaded({
            event,
            wording: timeline.wording,
            tags: timeline.tags,
            phases: timeline.phases,
          });
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) setError(err instanceof Error ? err.message : "unknown error");
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [id]);

  if (loading) {
    return (
      <div style={s.state} aria-busy="true">
        {BOOTSTRAP_TEXT.loading}
      </div>
    );
  }
  if (error !== null || loaded === null) {
    // See BOOTSTRAP_TEXT — the words for this sentence ride the request that
    // failed.
    return (
      <div style={s.errorState}>{BOOTSTRAP_TEXT.eventFailed(error ?? "unknown error")}</div>
    );
  }

  const { event, wording, tags, phases } = loaded;
  const phase = phases.find((p) => p.id === event.phase);

  return (
    <div style={s.page}>
      {/* The filter travels back with the reader: `location.search` still holds
          whichever phase was expanded when the card was clicked. */}
      <div style={s.crumb}>
        <Link to={`/timeline${location.search}`} style={s.crumbLink}>
          {cw(wording, "back_label")}
        </Link>
        {/* The phase, after the link — mockup Screen 2's "← Case Timeline · COA".
            The slug shows if the phase has no row, for the same reason the list
            renders such an event loudly rather than hiding it. */}
        {" · "}
        {phase?.label ?? event.phase}
      </div>

      <h1 style={s.eventTitle}>{event.title}</h1>
      <div style={s.when}>
        {formatEventDate(event.event_date, event.approximate, event.date_precision)}
        {event.tags.map((tagId) => {
          const tag = tagOf(tags, tagId);
          return (
            <span key={tagId} style={s.tagChip(tag?.color ?? "var(--text-disabled)")}>
              {tag?.label ?? tagId}
            </span>
          );
        })}
      </div>

      {event.fact && <p style={s.fullFact}>{event.fact}</p>}

      <div style={s.panel}>
        <h4 style={s.panelHeading}>{cw(wording, "documents_heading")}</h4>
        {event.links.length === 0 ? (
          <div style={s.panelEmpty}>{cw(wording, "no_document_label")}</div>
        ) : (
          event.links.map((link) => {
            const rendered = linkRendering(link, wording);
            return (
              <div key={`${link.target_type}:${link.target_id}`} style={s.linkRow}>
                {rendered.kind === "link" ? (
                  <>
                    <a
                      href={`${API_BASE_URL}/api/documents/${encodeURIComponent(link.target_id)}/file`}
                      target="_blank"
                      rel="noopener noreferrer"
                      style={s.docLink}
                    >
                      {rendered.label}
                    </a>
                    <span style={s.pinpoint}>
                      {rendered.pinpoint ?? cw(wording, "no_pinpoint_label")}
                    </span>
                  </>
                ) : (
                  <span
                    style={rendered.kind === "missing" ? s.noDoc : s.unchecked}
                    title={link.target_id}
                  >
                    {rendered.label}
                  </span>
                )}
              </div>
            );
          })
        )}
      </div>

      <div style={s.panel}>
        <h4 style={s.panelHeading}>{cw(wording, "notes_heading")}</h4>
        {event.notes.length === 0 ? (
          <div style={s.panelEmpty}>{cw(wording, "no_notes_label")}</div>
        ) : (
          event.notes.map((note) => (
            <div key={note.id} style={s.note}>
              {note.note}
              <div style={s.noteBy}>
                {note.created_by ?? ""} · {new Date(note.created_at).toLocaleDateString("en-US")}
              </div>
            </div>
          ))
        )}
      </div>

      <div style={s.panel}>
        <h4 style={s.panelHeading}>{cw(wording, "history_heading")}</h4>
        {event.history.length === 0 ? (
          <div style={s.panelEmpty}>{cw(wording, "no_history_label")}</div>
        ) : (
          event.history.map((entry) => (
            <div key={entry.id} style={s.history}>
              {new Date(entry.changed_at).toLocaleDateString("en-US")} ·{" "}
              {entry.changed_by ?? ""} · {entry.action}
            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default TimelineEventPage;
