// =============================================================================
// timelineWriteRules.ts — every decision the WRITE surfaces make, as pure code
// =============================================================================
//
// The Phase B pattern (F10), continued. This project has no component-testing
// tier, so anything decided inside a component is decided where no test can
// reach it. Everything here is a plain function over plain data: what the form
// starts as, whether it may be submitted, what it submits, how a history line
// reads, which notes carry a delete control, and how a list is patched after a
// write. The components below become arrangement.
//
// ## ⚑ WHY THE CLIENT VALIDATES AT ALL, GIVEN THE SERVER DOES
//
// It does not RE-validate: the server is the authority, and [`formIsSubmittable`]
// deliberately checks only the two fields R11 makes required forever. The point
// is a disabled Save rather than a round trip that comes back 400 — and the
// difference matters because a disabled control with a visible reason is a
// faster answer than a sentence under the form. Everything else (the phase
// vocabulary, the tags, the document's existence) is the server's, because only
// the server knows what rows exist.

import type {
  CaseTimelineEvent,
  ChronologyWording,
  TimelineEvent,
  TimelineNote,
  TimelineHistory,
} from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import type { SubmittedEvent, SubmittedLink } from "../../services/caseTimelineWrites";

/** Everything the add/edit form holds while it is being filled in. */
export type EventFormState = {
  /** ISO `YYYY-MM-DD`, as `<input type="date">` gives it. */
  event_date: string;
  date_precision: string;
  approximate: boolean;
  title: string;
  fact: string;
  /** Tag ids, in the order the author picked them. */
  tags: string[];
  /** A phase id. */
  phase: string;
  /**
   * Links the form will create WITH the event.
   *
   * Only ever non-empty on an add: the edit path submits no links, because an
   * edit that replaced an event's link set would delete a colleague's link while
   * somebody re-typed a title. The event page adds and removes links one at a
   * time through their own endpoints.
   */
  links: SubmittedLink[];
};

/**
 * The stored precision tokens, and nothing else.
 *
 * These are the three the `chronology_events_precision_valid` CHECK allows and
 * the three `domain::chronology::chronology_precisions()` derives. They are
 * TOKENS, not words: the words a human reads are stored wording rows, which is
 * why this list carries no labels.
 */
export const PRECISION_TOKENS = ["day", "month", "year"] as const;

/**
 * An empty form, ready for a new event.
 *
 * `today` and `defaultPhase` are passed IN rather than read here, for two
 * different reasons. The date because `new Date()` inside a pure function makes
 * it untestable and its result depends on the reader's clock; the phase because
 * the phases are data — the first stored phase is a sensible starting point and
 * the code must not name one.
 */
export function blankForm(today: string, defaultPhase: string): EventFormState {
  return {
    event_date: today,
    date_precision: "day",
    approximate: false,
    title: "",
    fact: "",
    tags: [],
    phase: defaultPhase,
    links: [],
  };
}

/**
 * The same form, pre-filled from an event — the Edit path.
 *
 * `links` is deliberately EMPTY even though the event has some: see
 * [`EventFormState.links`]. The event's real links are shown and changed on the
 * event page, one at a time.
 */
export function formFromEvent(event: TimelineEvent): EventFormState {
  return {
    event_date: event.event_date,
    date_precision: event.date_precision,
    approximate: event.approximate,
    title: event.title,
    fact: event.fact ?? "",
    tags: [...event.tags],
    phase: event.phase,
    links: [],
  };
}

/**
 * May this form be submitted?
 *
 * Exactly R11's two required fields, and no more. A form that also insisted on a
 * fact would be enforcing a rule the design explicitly declined to make ("one
 * sentence encouraged but optional"), and one that checked the phase vocabulary
 * would be a second copy of a list only the server has.
 */
export function formIsSubmittable(form: EventFormState): boolean {
  return form.title.trim() !== "" && form.event_date.trim() !== "" && form.phase.trim() !== "";
}

/**
 * What the form sends.
 *
 * Empty optional fields are OMITTED rather than sent as `""`, with one
 * deliberate exception: `tags` is always sent, because absent and empty are
 * different instructions to the server ("leave them" versus "remove them") and
 * a form that showed no tags selected means the second.
 */
export function formToRequest(form: EventFormState, includeLinks: boolean): SubmittedEvent {
  const fact = form.fact.trim();
  const request: SubmittedEvent = {
    event_date: form.event_date,
    title: form.title.trim(),
    phase: form.phase,
    date_precision: form.date_precision,
    approximate: form.approximate,
    tags: form.tags,
  };
  if (fact !== "") request.fact = fact;
  if (includeLinks && form.links.length > 0) request.links = form.links;
  return request;
}

/** Add or remove one tag, keeping the author's order. */
export function toggleTag(tags: string[], id: string): string[] {
  return tags.includes(id) ? tags.filter((tag) => tag !== id) : [...tags, id];
}

/**
 * How one history entry reads.
 *
 * The stored ACTION token and its display word are deliberately different —
 * `updated` reads as "edited", because the database says what happened to the
 * row and a human reads what the person did. An action this build has no word
 * for renders its raw token rather than an empty line: a vocabulary drift must
 * be visible on the one screen where somebody could notice it.
 */
export function historyLine(
  entry: TimelineHistory,
  wording: ChronologyWording,
  formatDate: (iso: string) => string,
): string {
  return fill(cw(wording, "history_line_template"), {
    when: formatDate(entry.changed_at),
    who: entry.changed_by ?? "",
    what: actionWord(entry.action, wording),
  });
}

/**
 * The display word for one stored action token.
 *
 * ⚑ Four LITERAL `cw` calls, not a lookup table of key names. The backend's
 * reach guard (`dto::chronology_wording_reach_tests`) scans this source for
 * `cw("…")` and requires every key it finds to be a field on the wire; a key
 * held in a variable is invisible to it, so four rows would be declared,
 * seeded, requested at runtime, and guarded by nothing. The guard's own header
 * names this limit, and this is how these surfaces stay inside it.
 */
function actionWord(action: string, wording: ChronologyWording): string {
  if (action === "created") return cw(wording, "history_created_label");
  if (action === "updated") return cw(wording, "history_updated_label");
  if (action === "deleted") return cw(wording, "history_deleted_label");
  if (action === "restored") return cw(wording, "history_restored_label");
  // Not a throw, deliberately, and the difference from the form's precision
  // select is the stakes: an unreadable history LINE must not take down a page
  // whose event is otherwise fine, whereas a blank option in a precision select
  // is how a fabricated day gets stored. The raw token renders, loudly enough
  // to notice and quietly enough to keep reading.
  return fill(cw(wording, "history_unknown_template"), { action });
}

/**
 * May this reader delete this note?
 *
 * ## Domain note — the one place the three authors are NOT equal
 *
 * R2 makes them equal on EVENTS: anyone may add, edit or delete any event. A
 * note is a signed remark rather than a shared field (R8), so only its author
 * may withdraw it. The SERVER enforces this — this function only decides whether
 * to DRAW the control, because offering a button that always fails is worse than
 * not offering it.
 *
 * An unsigned note (`created_by` absent) is deletable by nobody, which matches
 * the server. `username` of `null` is a reader this build cannot name — an
 * anonymous session on the open read — and they may delete nothing.
 */
export function noteIsDeletableBy(note: TimelineNote, username: string | null): boolean {
  if (username === null || username === "") return false;
  return note.created_by === username;
}

/**
 * Put the server's version of one event back into a list, in date order.
 *
 * ## ⚑ THE FILTER SURVIVES BECAUSE THE LIST IS PATCHED, NOT REFETCHED
 *
 * §C3: "the active filter survives every write and undo". The page holds its
 * filters in its own state and the URL; replacing the whole payload after a
 * write would be simpler and would also re-run every effect that reads them. So
 * a write patches the ONE event it changed into the list it already had, and the
 * filters are never touched.
 *
 * A DELETED event is removed from the list — the read endpoints never return one
 * either, so keeping it would make the page disagree with its own next reload.
 * The undo line the surface draws in its place is page state, not list state.
 *
 * The order is `(event_date, id)`, which is the order the API sorts by. Re-
 * sorting here rather than appending is what makes an edited date move the card
 * to where it belongs instead of leaving it where it was.
 */
export function patchEventList(
  events: TimelineEvent[],
  written: CaseTimelineEvent,
): TimelineEvent[] {
  const without = events.filter((event) => event.id !== written.id);
  if (written.deleted_at !== undefined && written.deleted_at !== null) {
    return without;
  }
  const next = [...without, stripDetail(written)];
  next.sort((a, b) => {
    if (a.event_date !== b.event_date) return a.event_date < b.event_date ? -1 : 1;
    return a.id < b.id ? -1 : a.id > b.id ? 1 : 0;
  });
  return next;
}

/**
 * The list's view of an event, from the detail the write returned.
 *
 * The list renders a note COUNT and the detail carries the notes themselves, so
 * the count is taken from the array rather than from the detail's own
 * `note_count` — the two are computed from the same rows on the server, and
 * preferring the array means a note added on the event page updates the badge
 * without a second read.
 */
function stripDetail(written: CaseTimelineEvent): TimelineEvent {
  const { notes, history: _history, ...event } = written;
  return { ...event, note_count: notes.length };
}

/** One row of a phase's body: a live card, or the undo line that replaced one. */
export type TimelineRow =
  | { kind: "event"; event: TimelineEvent }
  | { kind: "undo"; event: TimelineEvent };

/**
 * A phase's rows, with each deleted event's undo line WHERE ITS CARD WAS.
 *
 * ## ⚑ "In place" is the whole ruling (R10)
 *
 * "On delete the card is replaced IN PLACE by the undo line ('Deleted — Undo')
 * until navigation; no confirm dialog." A toast in a corner would satisfy the
 * words and not the intent: there is no confirmation before the delete, so the
 * taking-back has to be where the reader is already looking — in the row they
 * just emptied.
 *
 * The order is the list's own `(event_date, id)`, so the line sits between the
 * same two neighbours the card sat between. A deleted event whose phase is not
 * this one contributes nothing here, which is how a delete inside an expanded
 * phase does not sprout a line in a phase nobody is looking at.
 */
export function rowsForPhase(
  events: TimelineEvent[],
  undoable: TimelineEvent[],
  phaseId: string,
): TimelineRow[] {
  const rows: TimelineRow[] = events.map((event) => ({ kind: "event", event }));
  for (const event of undoable) {
    if (event.phase === phaseId) rows.push({ kind: "undo", event });
  }
  rows.sort((a, b) => {
    if (a.event.event_date !== b.event.event_date) {
      return a.event.event_date < b.event.event_date ? -1 : 1;
    }
    return a.event.id < b.event.id ? -1 : a.event.id > b.event.id ? 1 : 0;
  });
  return rows;
}
