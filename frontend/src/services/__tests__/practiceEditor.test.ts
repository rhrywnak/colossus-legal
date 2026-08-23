/**
 * Service tests for the deck editor's writes.
 *
 * ## The URL guard, and the failure class it exists for
 *
 * The .377 build shipped a client calling a path the router did not serve: a
 * whole feature answering 404, with nothing on either side saying why. Part B
 * added seven routes at once. So the assertions below SPELL THE PATHS OUT
 * rather than building them from the same constants the code under test uses.
 *
 * ## The one thing that must never be on the wire (changed 2026-08-19)
 *
 * `editing_as` — and `author` on the two note writes. These used to be REQUIRED
 * arguments, filled by two dropdowns, because the design assumed a single shared
 * login; the tests below asserted they were sent. They are now asserted ABSENT.
 *
 * Attribution comes from the authenticated session on the server, and the
 * assertions are `toEqual` on the whole decoded body rather than `toContain` on
 * one field, so a re-added `editing_as` fails the test rather than passing it
 * unnoticed.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  addQuestion,
  reorderQuestion,
  signedInAs,
  editQuestion,
  hideQuestion,
  moveQuestion,
  saveNote,
  strikeNote,
} from "../practiceEditor";

const SLUG = "awad_v_catholic_family_service";
const SCENARIO = "11111111-1111-1111-1111-111111111111";
const QUESTION = "44444444-4444-4444-4444-444444444444";
const NOTE = "55555555-5555-5555-5555-555555555555";

afterEach(() => {
  vi.restoreAllMocks();
});

function okFetch(body: unknown = {}) {
  const mock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => body });
  // @ts-ignore
  global.fetch = mock;
  return mock;
}

function failFetch(status: number) {
  const mock = vi.fn().mockResolvedValue({ ok: false, status, text: async () => "" });
  // @ts-ignore
  global.fetch = mock;
  return mock;
}


describe("editQuestion", () => {
  it("POSTs the field and the value — and NOTHING naming who", async () => {
    const mock = okFetch({ question_id: QUESTION });

    await editQuestion(QUESTION, "watch_for", "Letter, date, stop.");

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/practice/questions/${QUESTION}/edit`);
    expect(options.method).toBe("POST");
    expect(JSON.parse(options.body)).toEqual({
      field: "watch_for",
      value: "Letter, date, stop.",
    });
  });

  it("sends null to CLEAR an optional field", async () => {
    // A blank watch-for is a watch-for somebody decided was wrong. The server
    // clears on null and refuses a blank question text; this only sends what it
    // was told.
    const mock = okFetch({ question_id: QUESTION });
    await editQuestion(QUESTION, "stronger", null);
    expect(JSON.parse(mock.mock.calls[0][1].body).value).toBeNull();
  });

  it("escapes the id rather than letting a slash become a path segment", async () => {
    const mock = okFetch({ question_id: QUESTION });
    await editQuestion("id/with/slashes", "text", "x");
    expect(mock.mock.calls[0][0]).toContain("id%2Fwith%2Fslashes");
  });

  it("reports a failure as an edit that was NOT saved", async () => {
    failFetch(400);
    await expect(editQuestion(QUESTION, "text", "")).rejects.toThrow(
      /was not saved \(HTTP 400/,
    );
  });
});

describe("moveQuestion", () => {
  it("POSTs the direction alone", async () => {
    const mock = okFetch({ question_id: QUESTION });
    await moveQuestion(QUESTION, "up");

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/practice/questions/${QUESTION}/move`);
    expect(JSON.parse(options.body)).toEqual({ direction: "up" });
  });

  it("reports a failure as a question that was NOT moved", async () => {
    failFetch(500);
    await expect(moveQuestion(QUESTION, "down")).rejects.toThrow(
      /was not moved \(HTTP 500/,
    );
  });
});

describe("reorderQuestion", () => {
  it("POSTs the neighbour the dropped question lands above", async () => {
    const mock = okFetch({ question_id: QUESTION });
    await reorderQuestion(QUESTION, "55555555-5555-5555-5555-555555555555");

    const [url, options] = mock.mock.calls[0];
    // Spelled out rather than composed from the same constant the code uses:
    // the .377 defect was a client calling a path the router did not serve.
    expect(url).toContain(`/api/practice/questions/${QUESTION}/reorder`);
    expect(options.method).toBe("POST");
    expect(JSON.parse(options.body)).toEqual({
      before: "55555555-5555-5555-5555-555555555555",
    });
  });

  it("sends null for a drop past the last row", async () => {
    // `null` means the end of the side. It must reach the wire AS null — an
    // omitted field would be indistinguishable from a malformed request, and
    // the server's `#[serde(default)]` would read it as the same thing by luck
    // rather than by contract.
    const mock = okFetch({ question_id: QUESTION });
    await reorderQuestion(QUESTION, null);
    expect(JSON.parse(mock.mock.calls[0][1].body)).toEqual({ before: null });
  });

  it("escapes the id rather than letting a slash become a path segment", async () => {
    const mock = okFetch({ question_id: QUESTION });
    await reorderQuestion("id/with/slashes", null);
    expect(mock.mock.calls[0][0]).toContain("id%2Fwith%2Fslashes");
  });

  it("reports a failure as a question that was NOT moved", async () => {
    // The same sentence `moveQuestion` uses, deliberately: to the person who
    // dragged it, the arrows and the drag are one operation, and two different
    // failure sentences for it would read as two different problems.
    failFetch(500);
    await expect(reorderQuestion(QUESTION, null)).rejects.toThrow(
      /was not moved \(HTTP 500/,
    );
  });
});

describe("hideQuestion", () => {
  it("POSTs the hidden flag both ways, and nothing else", async () => {
    const mock = okFetch({ question_id: QUESTION });

    await hideQuestion(QUESTION, true);
    expect(JSON.parse(mock.mock.calls[0][1].body)).toEqual({ hidden: true });

    await hideQuestion(QUESTION, false);
    expect(JSON.parse(mock.mock.calls[1][1].body)).toEqual({ hidden: false });
  });
});

describe("addQuestion", () => {
  it("POSTs to the case- and scenario-scoped URL, unsigned", async () => {
    // The only deck write that needs a case and a scenario: the other three
    // address a question by its own id, and only a CREATE has to be told where
    // to put the new row.
    const mock = okFetch({ question_id: QUESTION });

    await addQuestion(
      SLUG,
      SCENARIO,
      {
        kind: "redirect",
        text: "Tell the jury about the letter.",
        tactic: null,
        follows: "g1",
        watch_for: null,
        source_kind: "point",
        source_index: 1,
      },
    );

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/practice/questions`);
    const body = JSON.parse(options.body);
    expect(body.kind).toBe("redirect");
    expect(body.follows).toBe("g1");
    // The signature is the SESSION'S. A body carrying a name would mean the
    // screen could get the attribution wrong, which is the whole defect.
    expect(body).not.toHaveProperty("editing_as");
  });

  it("reports a failure as a question that was NOT added", async () => {
    failFetch(400);
    await expect(
      addQuestion(
        SLUG,
        SCENARIO,
        {
          kind: "cross",
          text: "",
          tactic: null,
          follows: null,
          watch_for: null,
          source_kind: null,
          source_index: null,
        },
      ),
    ).rejects.toThrow(/was not added \(HTTP 400/);
  });
});

describe("saveNote", () => {
  it("POSTs the target and the text — the author is the session's", async () => {
    const mock = okFetch({ id: NOTE, author: "Chuck", text: "x", when: "Tue 18 Aug" });

    await saveNote(SLUG, SCENARIO, { questionId: QUESTION, answerId: null }, "x");

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/practice/notes`);
    expect(JSON.parse(options.body)).toEqual({
      question_id: QUESTION,
      answer_id: null,
      text: "x",
    });
  });

  it("returns the note as the SERVER stored it", async () => {
    // The stored `created_at` is the server's. A panel that dated a new note by
    // the browser's clock would disagree with itself the moment it reloaded.
    okFetch({ id: NOTE, author: "Chuck", text: "x", when: "Tue 18 Aug", struck: null });
    const note = await saveNote(SLUG, SCENARIO, { questionId: null, answerId: null }, "x");
    expect(note.when).toBe("Tue 18 Aug");
    expect(note.struck).toBeNull();
  });

  it("reports a failure as a note that was NOT saved", async () => {
    failFetch(400);
    await expect(
      saveNote(SLUG, SCENARIO, { questionId: null, answerId: null }, "x"),
    ).rejects.toThrow(/was not saved \(HTTP 400/);
  });
});

describe("strikeNote", () => {
  it("POSTs an EMPTY body — the striker is the session's", async () => {
    const mock = okFetch({ struck: true });
    await strikeNote(NOTE);

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/practice/notes/${NOTE}/strike`);
    expect(JSON.parse(options.body)).toEqual({});
  });

  it("reports a failure as a note that was NOT struck", async () => {
    failFetch(404);
    await expect(strikeNote(NOTE)).rejects.toThrow(/was not struck \(HTTP 404/);
  });
});


describe("signedInAs", () => {
  /** One `/api/me` body, with only the two fields this helper reads varied. */
  const user = (display_name: string, username: string) => ({
    username,
    display_name,
    email: `${username}@example.test`,
    groups: [],
    permissions: { can_read: true, can_edit: true, can_use_ai: true, is_admin: false },
  });

  it("prints the display name when there is one", () => {
    expect(signedInAs(user("Chuck", "cparker"))).toBe("Chuck");
  });

  it("falls back to the username when the display name is blank", () => {
    // The same class of bug the Rust `attribution` tests guard: an Authentik
    // account with no display name set would otherwise render a sentence
    // reading "Saved as a change by  —", naming nobody.
    expect(signedInAs(user("", "cparker"))).toBe("cparker");
    expect(signedInAs(user("   ", "cparker"))).toBe("cparker");
  });

  it("returns an empty string while /api/me is still in flight", () => {
    // Honest about not knowing yet. A literal "someone" here would be the
    // screen inventing a person — and this is only ever a LABEL: the stored
    // attribution comes from the session on the server, never from this.
    expect(signedInAs(null)).toBe("");
  });
});
