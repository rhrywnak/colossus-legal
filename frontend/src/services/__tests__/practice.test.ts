/**
 * Service tests for the practice client (PRACTICE v0).
 *
 * ## The URL guards, and the failure class they exist for
 *
 * The .377 build shipped a client calling a path the router did not serve: a
 * whole feature answering 404, with nothing on either side saying why. The server
 * half of this guard lives in `api::practice`'s tests; this is the client half.
 * Between them a path can only drift if BOTH are edited to agree — which is why
 * the assertions below SPELL THE PATHS OUT rather than building them from the
 * same constants the code under test uses.
 *
 * ## Standing Rule 1 on this client
 *
 * A failed load, a shape mismatch and an empty deck are three different things
 * and produce three different outcomes. The third is not an error at all: an
 * un-seeded scenario resolves normally with no questions, and the page says so.
 *
 * Mocks `global.fetch` because `authFetch` calls it.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  closePracticeAnswer,
  endPracticeSession,
  fetchPracticeDeck,
  markHelpOpened,
  startPracticeSession,
  submitPracticeAnswer,
  wordingOf,
  type PracticeDeck,
} from "../practice";

const SLUG = "awad_v_catholic_family_service";
const SCENARIO = "11111111-1111-1111-1111-111111111111";
const SESSION = "22222222-2222-2222-2222-222222222222";
const ANSWER = "33333333-3333-3333-3333-333333333333";

afterEach(() => {
  vi.restoreAllMocks();
});

/** A minimal but complete deck payload, matching the backend DTO. */
function deck(questions: PracticeDeck["questions"] = []): PracticeDeck {
  return {
    scenario_id: SCENARIO,
    code: "S-5",
    title: "Marie refused to divide property amicably",
    questions,
    points: [{ position: 1, text: "I asked in writing.", exhibit: "my certified letter" }],
    last_session_line: "No session on this one yet.",
    wording: { start_label: "Start", empty_deck: "no practice deck yet — seed it" },
  };
}

/** Mock `fetch` with one OK response, and hand back the spy. */
function okFetch(body: unknown = {}) {
  const mock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => body });
  // @ts-ignore
  global.fetch = mock;
  return mock;
}

describe("fetchPracticeDeck", () => {
  it("GETs the case- and scenario-scoped practice URL", async () => {
    const mock = okFetch(deck());

    await fetchPracticeDeck(SLUG, SCENARIO);

    const [url] = mock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/practice`);
  });

  it("resolves normally on an EMPTY deck — that is a screen, not a failure", async () => {
    // The S-6 case. A client that treated no questions as an error would put a
    // red failure notice in front of a scenario nobody has seeded yet.
    okFetch(deck());
    const payload = await fetchPracticeDeck(SLUG, SCENARIO);
    expect(payload.questions).toEqual([]);
  });

  it("throws with the status on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 500, text: async () => "" });
    await expect(fetchPracticeDeck(SLUG, SCENARIO)).rejects.toThrow(/HTTP 500/);
  });

  it("throws on a payload missing the shapes the page cannot render without", async () => {
    // A missing `wording` would leave every label on all four screens blank —
    // silently, because React renders `undefined` as nothing at all.
    okFetch({ scenario_id: SCENARIO, code: "S-5", title: "x", questions: [], points: [] });
    await expect(fetchPracticeDeck(SLUG, SCENARIO)).rejects.toThrow(/contract mismatch/);
  });
});

describe("the write paths", () => {
  it("POSTs the session to the scenario-scoped sessions URL, carrying who", async () => {
    const mock = okFetch({ session_id: SESSION });

    await expect(startPracticeSession(SLUG, SCENARIO, "mixed")).resolves.toBe(SESSION);

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/practice/sessions`);
    expect(options.method).toBe("POST");
    expect(JSON.parse(options.body)).toEqual({ who: "mixed" });
  });

  it("POSTs an answer to /api/practice/answers, with no mark and no boxes", async () => {
    // Both are settled by `closePracticeAnswer`, because both are decided after
    // she has read the reveal this call produces. Sending them here would record
    // a mark she has not chosen yet.
    const mock = okFetch({ answer_id: ANSWER, read_text: "Fine.", read_ok: true });

    const result = await submitPracticeAnswer({
      sessionId: SESSION,
      questionId: "q1",
      answerText: "I asked in writing.",
      dontRecall: false,
    });

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain("/api/practice/answers");
    const body = JSON.parse(options.body);
    expect(body).toEqual({
      session_id: SESSION,
      question_id: "q1",
      answer_text: "I asked in writing.",
      dont_recall: false,
    });
    expect(result).toEqual({ answer_id: ANSWER, read_text: "Fine.", read_ok: true });
  });

  it("keeps a missing read as null rather than inventing a sentence", async () => {
    // The whole failure posture of the drill: no read is a THIRD state, and the
    // page shows the stored "no system read this time" line. A client-side
    // default here would put words on a witness-prep screen that no model said.
    okFetch({ answer_id: ANSWER });
    const result = await submitPracticeAnswer({
      sessionId: SESSION,
      questionId: "q1",
      answerText: "",
      dontRecall: false,
    });
    expect(result.read_text).toBeNull();
    expect(result.read_ok).toBeNull();
  });

  it("POSTs the mark and the boxes to the close URL", async () => {
    const mock = okFetch({ mark: "repeat" });

    await closePracticeAnswer(ANSWER, "repeat", {
      only_asked: false,
      accepted_premise: true,
      explained_unasked: false,
      guessed: false,
    });

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/practice/answers/${ANSWER}/close`);
    expect(JSON.parse(options.body).mark).toBe("repeat");
    expect(JSON.parse(options.body).self_check.accepted_premise).toBe(true);
  });

  it("POSTs the drawer to the help URL", async () => {
    const mock = okFetch({ help_opened: true });
    await markHelpOpened(ANSWER);
    expect(mock.mock.calls[0][0]).toContain(`/api/practice/answers/${ANSWER}/help`);
  });

  it("POSTs the end to the session URL and returns the sheet", async () => {
    const mock = okFetch({ kicker: "Session done", heading: "1 questions. 0 to repeat.", rows: [] });
    await endPracticeSession(SESSION);
    expect(mock.mock.calls[0][0]).toContain(`/api/practice/sessions/${SESSION}/end`);
  });

  it("escapes every path parameter", async () => {
    // An unescaped id containing a slash would become an extra path segment and
    // land on a different route — or none — while looking entirely reasonable in
    // the source.
    const mock = okFetch({ help_opened: true });
    await markHelpOpened("id/with/slashes");
    expect(mock.mock.calls[0][0]).toContain("id%2Fwith%2Fslashes");
  });

  it("reports a failed answer write as NOT recorded", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 503, text: async () => "" });
    await expect(
      submitPracticeAnswer({
        sessionId: SESSION,
        questionId: "q1",
        answerText: "x",
        dontRecall: false,
      }),
    ).rejects.toThrow(/was not recorded/);
  });
});

describe("wordingOf", () => {
  it("returns the stored string", () => {
    expect(wordingOf(deck().wording, "start_label")).toBe("Start");
  });

  it("throws NAMING the key rather than falling back to a literal", () => {
    // There is no literal to fall back to (the wording law). A `?? ""` here
    // would put a blank button in front of a witness; a `?? "Answer"` would put
    // a sentence in the product that no migration can change.
    expect(() => wordingOf(deck().wording, "answer_button")).toThrow(/answer_button/);
    expect(() => wordingOf({ blank: "   " }, "blank")).toThrow(/blank/);
  });
});
