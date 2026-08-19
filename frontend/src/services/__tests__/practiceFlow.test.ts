/**
 * Service tests for the practice flow client — the sitting's own address.
 *
 * ## The URL guard, and the failure class it exists for
 *
 * The .377 build shipped a client calling a path the router did not serve: a
 * whole feature answering 404, with nothing on either side saying why. Section B
 * added four routes at once, which is four chances to repeat it. So the
 * assertions below SPELL THE PATHS OUT rather than building them from the same
 * constants the code under test uses — a path can then only drift if both
 * halves are edited to agree.
 *
 * ## Standing Rule 1 on this client
 *
 * Every one of these four throws on a non-2xx with the status in the message,
 * and the two that return a sitting refuse a body that is not one BY NAME. A
 * cast would have put `undefined.length` in the middle of a witness's session
 * with no clue where it came from.
 *
 * Mocks `global.fetch` because `authFetch` calls it.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  fetchSitting,
  resumeSitting,
  skipPracticeQuestion,
  startOverSitting,
} from "../practiceFlow";

const SESSION = "22222222-2222-2222-2222-222222222222";
const SCENARIO = "11111111-1111-1111-1111-111111111111";
const QUESTION = "44444444-4444-4444-4444-444444444444";

afterEach(() => {
  vi.restoreAllMocks();
});

/** A complete sitting body, matching the backend DTO. */
function sitting(overrides: Record<string, unknown> = {}) {
  return {
    session_id: SESSION,
    scenario_id: SCENARIO,
    who: "george",
    queue: ["q1", "q2"],
    answered: ["q1"],
    ended: false,
    ...overrides,
  };
}

/** Mock `fetch` with one OK response, and hand back the spy. */
function okFetch(body: unknown = {}) {
  const mock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => body });
  // @ts-ignore
  global.fetch = mock;
  return mock;
}

/** Mock `fetch` with one failing response. */
function failFetch(status: number) {
  const mock = vi.fn().mockResolvedValue({ ok: false, status, text: async () => "" });
  // @ts-ignore
  global.fetch = mock;
  return mock;
}

describe("fetchSitting", () => {
  it("GETs the session's own URL, with a timeout", async () => {
    const mock = okFetch(sitting());

    const result = await fetchSitting(SESSION);

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/practice/sessions/${SESSION}`);
    // A reload lands here, so it must not hang forever on a slow database.
    expect(options.signal).toBeDefined();
    expect(result.queue).toEqual(["q1", "q2"]);
    expect(result.answered).toEqual(["q1"]);
    expect(result.ended).toBe(false);
  });

  it("escapes the id rather than letting a slash become a path segment", async () => {
    const mock = okFetch(sitting({ session_id: "id/with/slashes" }));
    await fetchSitting("id/with/slashes");
    expect(mock.mock.calls[0][0]).toContain("id%2Fwith%2Fslashes");
  });

  it("resolves an EMPTY queue rather than treating it as a failure", async () => {
    // A sitting opened before flow v1 carries no stored queue and the server
    // sends `[]`. That is a real state — the page goes back to the start card —
    // and a client that threw here would show a red failure notice instead.
    const result = await (okFetch(sitting({ queue: [], answered: [] })), fetchSitting(SESSION));
    expect(result.queue).toEqual([]);
  });

  it("reports a 404 as a session that could not be opened", async () => {
    failFetch(404);
    await expect(fetchSitting(SESSION)).rejects.toThrow(/could not be opened \(HTTP 404/);
  });

  it("refuses a body that is not a sitting, by name", async () => {
    // The contract-mismatch guard. A cast would put `undefined.length` in the
    // middle of a witness's session with no clue where it came from.
    okFetch({ session_id: SESSION, who: "george" });
    await expect(fetchSitting(SESSION)).rejects.toThrow(/contract mismatch/);
  });

  it("refuses a body missing its side", async () => {
    okFetch({ session_id: SESSION, queue: [], answered: [] });
    await expect(fetchSitting(SESSION)).rejects.toThrow(/contract mismatch/);
  });
});

describe("resumeSitting", () => {
  it("POSTs to the session's resume URL and returns the sitting", async () => {
    const mock = okFetch(sitting());

    const result = await resumeSitting(SESSION);

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/practice/sessions/${SESSION}/resume`);
    expect(options.method).toBe("POST");
    expect(result.who).toBe("george");
  });

  it("is a POST and not a GET, because it CLOSES the older open sittings", async () => {
    // Pressing Resume is the first moment she has said which sitting she means,
    // and the server retires the others then. A GET that had that effect would
    // be a read with a side effect.
    const mock = okFetch(sitting());
    await resumeSitting(SESSION);
    expect(mock.mock.calls[0][1].method).toBe("POST");
  });

  it("reports a failure as a session that could not be resumed", async () => {
    failFetch(500);
    await expect(resumeSitting(SESSION)).rejects.toThrow(/could not be resumed \(HTTP 500/);
  });

  it("refuses a body that is not a sitting, by name", async () => {
    okFetch({ nonsense: true });
    await expect(resumeSitting(SESSION)).rejects.toThrow(/contract mismatch/);
  });
});

describe("startOverSitting", () => {
  it("POSTs to the session's start-over URL", async () => {
    const mock = okFetch({ also_closed: 2 });

    await startOverSitting(SESSION);

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/practice/sessions/${SESSION}/start-over`);
    expect(options.method).toBe("POST");
  });

  it("reports a failure as a session that could not be closed", async () => {
    failFetch(503);
    await expect(startOverSitting(SESSION)).rejects.toThrow(/could not be closed \(HTTP 503/);
  });
});

describe("skipPracticeQuestion", () => {
  it("POSTs both ids to /api/practice/answers/skip", async () => {
    const mock = okFetch({ answer_id: "a1", read_text: null, read_ok: null });

    await skipPracticeQuestion(SESSION, QUESTION);

    const [url, options] = mock.mock.calls[0];
    expect(url).toContain("/api/practice/answers/skip");
    expect(options.method).toBe("POST");
    expect(JSON.parse(options.body)).toEqual({
      session_id: SESSION,
      question_id: QUESTION,
    });
  });

  it("sends NO answer text — the stored phrase is the server's to write", async () => {
    // A skip records the stored "doesn't fit" sentence, not anything she typed.
    // A client that sent one would be putting words in a witness's mouth on
    // Chuck's sheet.
    const mock = okFetch({});
    await skipPracticeQuestion(SESSION, QUESTION);
    const body = JSON.parse(mock.mock.calls[0][1].body);
    expect(body).not.toHaveProperty("answer_text");
    expect(body).not.toHaveProperty("points_to");
  });

  it("reports a failure as a skip that was NOT recorded", async () => {
    // She pressed a control and the question was NOT set aside. Saying nothing
    // would leave her believing it had been.
    failFetch(500);
    await expect(skipPracticeQuestion(SESSION, QUESTION)).rejects.toThrow(
      /was not recorded \(HTTP 500/,
    );
  });
});
