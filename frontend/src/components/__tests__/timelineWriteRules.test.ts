/**
 * Pure-helper tests for the timeline's WRITE decisions (chronology Phase C).
 *
 * This project has no component-testing tier, so everything the add/edit form,
 * the card controls, the undo line and the history panel decide is a plain
 * function in `timelineWriteRules.ts` — and this file is the whole reason that
 * is worth doing. No DOM, no RTL.
 */
import { describe, expect, it } from "vitest";

import type {
  CaseTimelineEvent,
  ChronologyWording,
  TimelineEvent,
  TimelineHistory,
  TimelineNote,
} from "../../services/caseTimeline";
import {
  blankForm,
  type EventFormState,
  formFromEvent,
  formIsSubmittable,
  formToRequest,
  historyLine,
  noteIsDeletableBy,
  patchEventList,
  PRECISION_TOKENS,
  rowsForPhase,
  toggleTag,
} from "../timeline/timelineWriteRules";

/** An event carrying only what these rules read; the rest is realistic filler. */
const event = (id: string, overrides: Partial<TimelineEvent> = {}): TimelineEvent => ({
  id,
  event_date: "2012-04-12",
  date_precision: "day",
  approximate: false,
  phase: "appeals",
  title: "Judge Tighe Issues Post-Appeal Order",
  fact: "Judge Tighe issues Opinion and Order.",
  attributes: { tags: ["court_action"], source: "legacy_json" },
  tags: ["court_action"],
  links: [],
  note_count: 0,
  created_at: "2026-08-25T12:00:00Z",
  updated_at: "2026-08-25T12:00:00Z",
  ...overrides,
});

/** The detail shape a write answers with. */
const written = (
  id: string,
  overrides: Partial<CaseTimelineEvent> = {},
): CaseTimelineEvent => ({
  ...event(id),
  notes: [],
  history: [],
  ...overrides,
});

/** The words these rules speak, as the migration seeds them. */
const wording: ChronologyWording = {
  history_line_template: "{when} · {who} · {what}",
  history_created_label: "created",
  history_updated_label: "edited",
  history_deleted_label: "deleted",
  history_restored_label: "restored",
  history_unknown_template: "{action}",
};

const note = (id: string, createdBy?: string): TimelineNote => ({
  id,
  note: "Need the certified copy.",
  created_by: createdBy,
  created_at: "2026-08-25T12:00:00Z",
});

const history = (action: string, changedBy?: string): TimelineHistory => ({
  id: `h-${action}`,
  action,
  snapshot: {},
  changed_by: changedBy,
  changed_at: "2026-08-26T14:30:00Z",
});

describe("the form's starting state", () => {
  it("starts blank on the day and phase it is given", () => {
    const form = blankForm("2026-08-26", "estate");
    expect(form.event_date).toBe("2026-08-26");
    expect(form.phase).toBe("estate");
    // The two R11 required fields are the only ones with a value; everything
    // else starts empty, because R11 requires nothing else, forever.
    expect(form.title).toBe("");
    expect(form.fact).toBe("");
    expect(form.tags).toEqual([]);
    expect(form.date_precision).toBe("day");
    expect(form.approximate).toBe(false);
  });

  it("pre-fills from an event, and copies the tags rather than aliasing them", () => {
    const source = event("e1", { tags: ["filing", "financial"] });
    const form = formFromEvent(source);
    expect(form.title).toBe(source.title);
    expect(form.tags).toEqual(["filing", "financial"]);
    // ⚑ A shared array would make un-picking a tag in the form mutate the
    // event the list is still rendering — a card whose chips changed without a
    // write, which no reload would explain.
    form.tags.push("discovery");
    expect(source.tags).toEqual(["filing", "financial"]);
  });

  it("turns an absent fact into an empty box rather than the word undefined", () => {
    expect(formFromEvent(event("e1", { fact: undefined })).fact).toBe("");
  });

  it("pre-fills NO links, because an edit never replaces an event's link set", () => {
    // An edit that submitted the whole set would delete a colleague's link
    // while somebody re-typed a title. Links are added and removed one at a
    // time on the event page.
    const source = event("e1", {
      links: [{ target_type: "document", target_id: "doc-a", resolution: "resolves" }],
    });
    expect(formFromEvent(source).links).toEqual([]);
  });
});

describe("whether the form may be submitted", () => {
  const base = blankForm("2026-08-26", "estate");

  it("needs a title", () => {
    expect(formIsSubmittable({ ...base, title: "  " })).toBe(false);
    expect(formIsSubmittable({ ...base, title: "A hearing" })).toBe(true);
  });

  it("needs a date and a phase", () => {
    expect(formIsSubmittable({ ...base, title: "x", event_date: "" })).toBe(false);
    expect(formIsSubmittable({ ...base, title: "x", phase: "" })).toBe(false);
  });

  it("does NOT need a fact — R11 makes it encouraged, not required", () => {
    // A form that insisted on one would be enforcing a rule the design
    // explicitly declined to make.
    expect(formIsSubmittable({ ...base, title: "x", fact: "" })).toBe(true);
  });

  it("does not check the phase VOCABULARY, which only the server knows", () => {
    // A second copy of the phase list here would be a list that goes stale the
    // day a fifth phase is added as a row.
    expect(formIsSubmittable({ ...base, title: "x", phase: "not-a-real-phase" })).toBe(true);
  });
});

describe("what the form submits", () => {
  const filled: EventFormState = {
    event_date: "2012-04-12",
    date_precision: "month",
    approximate: true,
    title: "  Tighe order  ",
    fact: "  ",
    tags: ["court_action"],
    phase: "appeals",
    links: [{ target_type: "document", target_id: "doc-a" }],
  };

  it("trims the title and omits an empty fact entirely", () => {
    const request = formToRequest(filled, false);
    expect(request.title).toBe("Tighe order");
    // OMITTED, not sent as "". The server keeps NULL and "" apart so a history
    // snapshot can tell "he cleared it" from "he never wrote one".
    expect("fact" in request).toBe(false);
  });

  it("always sends tags, because absent and empty are different instructions", () => {
    // ⚑ Absent means "leave the stored tags alone"; empty means "remove them".
    // A form showing nothing selected means the second, so it must say so.
    expect(formToRequest({ ...filled, tags: [] }, false).tags).toEqual([]);
  });

  it("carries the precision and the approximate flag as separate facts", () => {
    const request = formToRequest(filled, false);
    expect(request.date_precision).toBe("month");
    expect(request.approximate).toBe(true);
  });

  it("includes links only when asked, which is only on a create", () => {
    expect(formToRequest(filled, true).links).toHaveLength(1);
    expect("links" in formToRequest(filled, false)).toBe(false);
  });

  it("omits an empty link list even on a create", () => {
    expect("links" in formToRequest({ ...filled, links: [] }, true)).toBe(false);
  });
});

describe("the tag picker", () => {
  it("adds at the end and removes in place, keeping the author's order", () => {
    // The FIRST tag decides the card's dot colour, so a re-order here would
    // silently recolour the event.
    expect(toggleTag(["filing"], "financial")).toEqual(["filing", "financial"]);
    expect(toggleTag(["filing", "financial"], "filing")).toEqual(["financial"]);
  });

  it("is idempotent in pairs", () => {
    expect(toggleTag(toggleTag(["filing"], "financial"), "financial")).toEqual(["filing"]);
  });
});

describe("the precision vocabulary", () => {
  it("is exactly the three the column's CHECK allows", () => {
    // `chronology_events_precision_valid` allows day | month | year, and
    // `domain::chronology::chronology_precisions()` derives the same three.
    // A fourth here would be an option the database refuses.
    expect([...PRECISION_TOKENS]).toEqual(["day", "month", "year"]);
  });
});

describe("how a history line reads", () => {
  const at = () => "Aug 26, 2026";

  it("renders the mockup's line, with the DISPLAY word for the action", () => {
    // ⚑ The stored token is `updated` and a human reads "edited": the database
    // says what happened to the row, the line says what the person did.
    expect(historyLine(history("updated", "Marie"), wording, at)).toBe(
      "Aug 26, 2026 · Marie · edited",
    );
  });

  it("has a distinct word for each of the four stored actions", () => {
    const words = ["created", "updated", "deleted", "restored"].map((action) =>
      historyLine(history(action, "Roman"), wording, at),
    );
    expect(new Set(words).size).toBe(4);
    expect(words[0]).toContain("created");
    expect(words[3]).toContain("restored");
  });

  it("renders an unknown action's raw token rather than a blank line", () => {
    // A vocabulary drift must be visible on the one screen where somebody could
    // notice it. An empty line would hide a real history row.
    expect(historyLine(history("archived", "Chuck"), wording, at)).toBe(
      "Aug 26, 2026 · Chuck · archived",
    );
  });

  it("renders an unattributed entry without inventing an author", () => {
    expect(historyLine(history("created", undefined), wording, at)).toBe(
      "Aug 26, 2026 ·  · created",
    );
  });
});

describe("who may delete a note (design R8)", () => {
  it("lets an author delete their own", () => {
    expect(noteIsDeletableBy(note("n1", "chuck"), "chuck")).toBe(true);
  });

  it("does not let one author delete another's", () => {
    // The one place the three authors are NOT equal: R2 makes them equal on
    // events, and a note is a signed remark.
    expect(noteIsDeletableBy(note("n1", "chuck"), "marie")).toBe(false);
  });

  it("lets nobody delete an unsigned note", () => {
    expect(noteIsDeletableBy(note("n1", undefined), "roman")).toBe(false);
  });

  it("lets an unnamed reader delete nothing", () => {
    // An anonymous session on the open read. `""` too, which is what a naive
    // `user?.username ?? ""` comparison against an unsigned note would let past.
    expect(noteIsDeletableBy(note("n1", "chuck"), null)).toBe(false);
    expect(noteIsDeletableBy(note("n1", undefined), "")).toBe(false);
  });

  it("matches exactly, not by prefix", () => {
    expect(noteIsDeletableBy(note("n1", "marie.awad"), "marie")).toBe(false);
  });
});

describe("patching the list after a write", () => {
  const older = event("a", { event_date: "2009-01-01" });
  const newer = event("c", { event_date: "2014-01-01" });

  it("replaces an edited event in place and re-sorts it by its new date", () => {
    const list = [older, event("b", { event_date: "2012-01-01" }), newer];
    const moved = patchEventList(list, written("b", { ...event("b"), event_date: "2020-01-01" }));
    expect(moved.map((e) => e.id)).toEqual(["a", "c", "b"]);
    expect(moved).toHaveLength(3);
  });

  it("inserts a created event where its date belongs, not at the end", () => {
    const list = [older, newer];
    const patched = patchEventList(list, written("b", { ...event("b"), event_date: "2012-01-01" }));
    expect(patched.map((e) => e.id)).toEqual(["a", "b", "c"]);
  });

  it("removes a deleted event, because the read endpoints would not return it", () => {
    // Keeping it would make the page disagree with its own next reload. The
    // undo line that stands in its place is PAGE state, not list state.
    const list = [older, newer];
    const patched = patchEventList(list, written("a", { deleted_at: "2026-08-26T15:00:00Z" }));
    expect(patched.map((e) => e.id)).toEqual(["c"]);
  });

  it("breaks a same-date tie by id, which is the API's own order", () => {
    const list = [event("b", { event_date: "2012-01-01" })];
    const patched = patchEventList(list, written("a", { ...event("a"), event_date: "2012-01-01" }));
    expect(patched.map((e) => e.id)).toEqual(["a", "b"]);
  });

  it("takes the note count from the notes the write returned", () => {
    // So a note added on the event page updates the card's badge without a
    // second read — the two are computed from the same rows on the server.
    const patched = patchEventList(
      [event("a")],
      written("a", { notes: [note("n1", "chuck"), note("n2", "marie")] }),
    );
    expect(patched[0].note_count).toBe(2);
  });

  it("does not leave the detail's own fields on a list event", () => {
    const patched = patchEventList([event("a")], written("a", { history: [history("created")] }));
    expect("history" in patched[0]).toBe(false);
  });
});

describe("where the undo line goes (design R10)", () => {
  const first = event("a", { event_date: "2009-01-01", phase: "estate" });
  const middle = event("b", { event_date: "2012-01-01", phase: "estate" });
  const last = event("c", { event_date: "2014-01-01", phase: "estate" });

  it("puts a deleted event's line exactly where its card was", () => {
    // ⚑ "The card is replaced IN PLACE by the undo line." With no confirmation
    // before the delete, the taking-back has to be where the reader is already
    // looking — between the same two neighbours.
    const rows = rowsForPhase([first, last], [middle], "estate");
    expect(rows.map((r) => `${r.kind}:${r.event.id}`)).toEqual([
      "event:a",
      "undo:b",
      "event:c",
    ]);
  });

  it("draws nothing for an event deleted from a different phase", () => {
    // Otherwise a delete inside an expanded phase would sprout a line in a
    // phase nobody is looking at.
    const rows = rowsForPhase([first], [event("z", { phase: "appeals" })], "estate");
    expect(rows.map((r) => r.kind)).toEqual(["event"]);
  });

  it("leaves an untouched phase exactly as it was", () => {
    const rows = rowsForPhase([first, middle], [], "estate");
    expect(rows.map((r) => `${r.kind}:${r.event.id}`)).toEqual(["event:a", "event:b"]);
  });

  it("keeps several undo lines each in their own place", () => {
    const rows = rowsForPhase([last], [first, middle], "estate");
    expect(rows.map((r) => `${r.kind}:${r.event.id}`)).toEqual([
      "undo:a",
      "undo:b",
      "event:c",
    ]);
  });
});
