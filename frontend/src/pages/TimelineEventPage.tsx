// =============================================================================
// TimelineEventPage.tsx — one event, in full (mockup v2 Screen 2)
// =============================================================================
//
// Everything the list could not fit: the whole fact, the document links with
// their pinpoints and resolution states, the notes as individual attributed
// entries (design R8), and the history.
//
// ## Phase C: the page writes
//
// ✎ Edit and 🗑 Delete on the action bar, the Add-a-note input, + Link a
// document, and ✕ on each link. The history panel now renders real entries as
// they accrue — "Aug 26, 2026 · Marie · edited" — and the stored empty line
// still stands for an untouched event.
//
// ## The empty panels are RENDERED, not hidden
//
// A hidden panel reads as a feature that does not exist; an empty one reads as a
// feature with nothing in it yet, which is the truth.
//
// ## ⚑ ONE WRITE, ONE REPLACED EVENT
//
// Every write below answers with the whole composed event, and the page simply
// takes it — no refetch, no local patching, no optimistic state (§C3). That is
// why a note added here updates the history panel in the same render: the note's
// write landed a history row, and the response carries it.
//
// ## What is STILL deliberately absent
//
// The mockup's People panel. No event carries people, and the honest-gap law
// says a panel with nothing behind it is not drawn.

import React, { useCallback, useEffect, useState } from "react";
import { Link, useLocation, useNavigate, useParams } from "react-router-dom";

import { useAuth } from "../context/AuthContext";
import { API_BASE_URL } from "../services/api";
import {
  BOOTSTRAP_TEXT,
  type CaseTimelineEvent,
  type ChronologyWording,
  cw,
  fill,
  getCaseTimeline,
  getTimelineEvent,
  type TimelinePhase,
  type TimelineTag,
} from "../services/caseTimeline";
import {
  addTimelineNote,
  deleteTimelineEvent,
  deleteTimelineNote,
  linkTimelineDocument,
  unlinkTimelineDocument,
  updateTimelineEvent,
} from "../services/caseTimelineWrites";
import { formatEventDate, linkRendering, tagOf } from "../components/timeline/timelineFilters";
import TimelineDocumentPicker from "../components/timeline/TimelineDocumentPicker";
import TimelineEventForm from "../components/timeline/TimelineEventForm";
import * as s from "../components/timeline/timelineStyles";
import {
  type EventFormState,
  formFromEvent,
  formToRequest,
  historyLine,
  noteIsDeletableBy,
} from "../components/timeline/timelineWriteRules";
import * as w from "../components/timeline/timelineWriteStyles";

type Loaded = {
  event: CaseTimelineEvent;
  wording: ChronologyWording;
  tags: TimelineTag[];
  phases: TimelinePhase[];
};

/**
 * How a date reads on this page: "Aug 26, 2026".
 *
 * One function rather than three `toLocaleDateString` calls with the same
 * arguments, so the notes panel, the history panel and any future stamped line
 * cannot drift into three date formats on one screen.
 */
function stamp(iso: string): string {
  return new Date(iso).toLocaleDateString("en-US");
}

const TimelineEventPage: React.FC = () => {
  const { id = "" } = useParams();
  const location = useLocation();
  const navigate = useNavigate();
  // Who is reading, so a note carries a delete control only for its own author
  // (design R8). The SERVER enforces the rule; this only decides what to draw,
  // because a button that always fails is worse than no button.
  const { user } = useAuth();
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [editForm, setEditForm] = useState<EventFormState | null>(null);
  const [noteDraft, setNoteDraft] = useState("");
  const [pickerOpen, setPickerOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [writeError, setWriteError] = useState<string | null>(null);

  /**
   * Run one write and take the server's event as the new truth.
   *
   * ⚑ ONE place, so the six write paths on this page cannot disagree about what
   * happens after a write. There is no local patching and no optimistic state:
   * the response is the whole composed event, including the history row the
   * write just landed, so the panels below simply re-render from it (§C3).
   */
  const runWrite = useCallback(
    async (write: () => Promise<CaseTimelineEvent>): Promise<CaseTimelineEvent | null> => {
      setWriteError(null);
      setSaving(true);
      try {
        const written = await write();
        setLoaded((current) => (current === null ? current : { ...current, event: written }));
        return written;
      } catch (err: unknown) {
        // Never swallowed. The sentence is rendered below the action bar.
        setWriteError(err instanceof Error ? err.message : "unknown error");
        return null;
      } finally {
        setSaving(false);
      }
    },
    [],
  );

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

  const saveEdit = useCallback(async () => {
    if (editForm === null) return;
    // `false`: an edit never carries links. They are added and removed one at a
    // time below, because an edit that replaced the whole set would delete a
    // colleague's link while somebody re-typed a title.
    const written = await runWrite(() => updateTimelineEvent(id, formToRequest(editForm, false)));
    // The form stays OPEN on failure, holding what was typed — closing it would
    // throw the author's words away along with the explanation.
    if (written !== null) setEditForm(null);
  }, [editForm, id, runWrite]);

  const removeEvent = useCallback(async () => {
    // No confirm dialog (R10). The event page has no row to replace with an undo
    // line, so it returns to the list — where the line IS drawn, in the place the
    // card was, because the list holds this delete in its own undo state.
    const written = await runWrite(() => deleteTimelineEvent(id));
    if (written !== null) navigate(`/timeline${location.search}`);
  }, [id, location.search, navigate, runWrite]);

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

      {editForm !== null && (
        <TimelineEventForm
          creating={false}
          form={editForm}
          onChange={setEditForm}
          tags={tags}
          phases={phases}
          wording={wording}
          saving={saving}
          error={writeError}
          withLinks={false}
          onSave={() => void saveEdit()}
          onCancel={() => {
            setEditForm(null);
            setWriteError(null);
          }}
        />
      )}

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
                {/* Removable in every state, including "no document yet": a link
                    that points at nothing is exactly the one somebody wants to
                    take back, and hiding the control on a dead link would leave
                    the ten-dead-links state permanent. */}
                <button
                  type="button"
                  style={w.rowAction}
                  disabled={saving}
                  onClick={() =>
                    void runWrite(() =>
                      unlinkTimelineDocument(event.id, link.target_type, link.target_id),
                    )
                  }
                >
                  {cw(wording, "remove_link_label")}
                </button>
              </div>
            );
          })
        )}

        {pickerOpen ? (
          <TimelineDocumentPicker
            wording={wording}
            onPick={(choice, pinpoint) => {
              setPickerOpen(false);
              void runWrite(() =>
                linkTimelineDocument(event.id, {
                  target_type: "document",
                  target_id: choice.id,
                  label: choice.title,
                  // Absent stays ABSENT — the absence is what marks the link
                  // "no pinpoint" on every surface that draws it (design R9).
                  ...(pinpoint.trim() === "" ? {} : { pinpoint: pinpoint.trim() }),
                }),
              );
            }}
          />
        ) : (
          <button
            type="button"
            style={w.button}
            disabled={saving}
            onClick={() => setPickerOpen(true)}
          >
            {cw(wording, "link_document_label")}
          </button>
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
                {note.created_by ?? ""} · {stamp(note.created_at)}
                {/* ⚑ The one place the three authors are NOT equal. R2 makes
                    them equal on EVENTS; a note is a signed remark (R8), so
                    only its author may withdraw it. The control is not drawn
                    for anyone else rather than drawn and refused. */}
                {noteIsDeletableBy(note, user?.username ?? null) && (
                  <button
                    type="button"
                    style={w.rowAction}
                    disabled={saving}
                    onClick={() => void runWrite(() => deleteTimelineNote(event.id, note.id))}
                  >
                    {cw(wording, "delete_note_label")}
                  </button>
                )}
              </div>
            </div>
          ))
        )}

        <div style={w.addNoteRow}>
          <input
            style={w.input}
            placeholder={cw(wording, "add_note_placeholder")}
            aria-label={cw(wording, "add_note_placeholder")}
            value={noteDraft}
            onChange={(e) => setNoteDraft(e.target.value)}
          />
          <button
            type="button"
            style={w.primaryButton(noteDraft.trim() === "" || saving)}
            disabled={noteDraft.trim() === "" || saving}
            onClick={() =>
              void runWrite(() => addTimelineNote(event.id, noteDraft.trim())).then((written) => {
                // Cleared only on success: a failed note that also erased what
                // was typed would lose the words AND the reason.
                if (written !== null) setNoteDraft("");
              })
            }
          >
            {cw(wording, "add_note_button_label")}
          </button>
        </div>
      </div>

      <div style={s.panel}>
        <h4 style={s.panelHeading}>{cw(wording, "history_heading")}</h4>
        {event.history.length === 0 ? (
          <div style={s.panelEmpty}>{cw(wording, "no_history_label")}</div>
        ) : (
          // "Aug 26, 2026 · Marie · edited" — composed by a pure function, so
          // the stored token (`updated`) and its display word (`edited`) are
          // mapped where a test can see it. The stored empty line still stands
          // for an untouched event.
          event.history.map((entry) => (
            <div key={entry.id} style={s.history}>
              {historyLine(entry, wording, stamp)}
            </div>
          ))
        )}
      </div>

      {/* ⚑ A failed write reaches a rendered sentence, always. When the edit
          form is open it renders there instead, so the message is beside the
          control that produced it rather than at the foot of the page. */}
      {writeError !== null && editForm === null && (
        <div style={w.writeError}>
          {fill(cw(wording, "write_failed_template"), { reason: writeError })}
        </div>
      )}

      {/* Mockup Screen 2's action bar. Muted, always visible (R17), and there
          is no confirm dialog behind Delete (R10). */}
      <div style={w.actionBar}>
        <button
          type="button"
          style={w.button}
          disabled={saving || editForm !== null}
          onClick={() => setEditForm(formFromEvent(event))}
        >
          {cw(wording, "edit_label")}
        </button>
        <button
          type="button"
          style={w.dangerButton}
          disabled={saving}
          onClick={() => void removeEvent()}
        >
          {cw(wording, "delete_label")}
        </button>
      </div>
    </div>
  );
};

export default TimelineEventPage;
