/**
 * Service tests for the API-backed case timeline.
 *
 * `getCaseTimeline` read a static file until Phase B; it reads
 * `GET /api/timeline` now, through the credentialed client. Standing Rule 1:
 * every distinct failure — network, non-2xx, unparseable body, and a payload
 * missing a load-bearing shape — produces its own observable error, and none of
 * them is swallowed.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  BOOTSTRAP_TEXT,
  cw,
  fill,
  getCaseTimeline,
  getTimelineEvent,
} from "../caseTimeline";

afterEach(() => {
  vi.restoreAllMocks();
});

/** A minimal well-formed payload; each test varies one thing about it. */
const payload = (over: Record<string, unknown> = {}) => ({
  phases: [],
  tags: [],
  events: [],
  wording: { page_title: "Case Timeline" },
  phase_window_events: 4,
  ...over,
});

/** `authFetch` is a plain fetch wrapper, so mocking global fetch reaches it. */
function respondWith(body: unknown, ok = true, status = 200) {
  // @ts-ignore — minimal Response mock
  global.fetch = vi.fn().mockResolvedValue({ ok, status, json: async () => body });
}

describe("getCaseTimeline", () => {
  it("returns the payload when every load-bearing shape is present", async () => {
    respondWith(payload({ phase_window_events: 6 }));
    const data = await getCaseTimeline();
    expect(data.phase_window_events).toBe(6);
    expect(data.wording.page_title).toBe("Case Timeline");
  });

  it("throws a contextual error on a non-2xx", async () => {
    respondWith({}, false, 503);
    await expect(getCaseTimeline()).rejects.toThrow(/the case timeline \(HTTP 503\)/);
  });

  it("throws when the network fails, naming the cause", async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error("connection refused"));
    await expect(getCaseTimeline()).rejects.toThrow(/connection refused/);
  });

  it("throws when the body is not JSON", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => {
        throw new Error("bad json");
      },
    });
    await expect(getCaseTimeline()).rejects.toThrow(/did not come back as valid JSON/);
  });

  it.each([
    ["phases", { phases: "no" }],
    ["tags", { tags: null }],
    ["events", { events: 3 }],
  ])("throws when %s is not an array", async (_name, over) => {
    respondWith(payload(over));
    await expect(getCaseTimeline()).rejects.toThrow(/without its phases, tags or events/);
  });

  it("throws when the payload carries no wording at all", async () => {
    // The one failure that would otherwise surface as a page of thrown errors,
    // one control at a time. It is named once, at the boundary.
    respondWith(payload({ wording: undefined }));
    await expect(getCaseTimeline()).rejects.toThrow(/came back with no wording/);
  });

  it("falls back to showing every event when the window size is nonsense", async () => {
    // A window of zero would render every phase empty and read as a data
    // failure. Visibly wrong beats invisibly empty.
    respondWith(payload({ events: [{}, {}, {}], phase_window_events: 0 }));
    expect((await getCaseTimeline()).phase_window_events).toBe(3);

    respondWith(payload({ events: [{}], phase_window_events: undefined }));
    expect((await getCaseTimeline()).phase_window_events).toBe(1);
  });
});

describe("getTimelineEvent", () => {
  it("returns the event with its lists defaulted", async () => {
    respondWith({ id: "e1", title: "An event" });
    const event = await getTimelineEvent("e1");
    expect(event.links).toEqual([]);
    expect(event.notes).toEqual([]);
    expect(event.history).toEqual([]);
  });

  it("throws when the event comes back without an id or a title", async () => {
    respondWith({ id: "e1" });
    await expect(getTimelineEvent("e1")).rejects.toThrow(/without an id or a title/);
  });

  it("names the event resource, not the timeline, in its errors", async () => {
    respondWith({}, false, 404);
    await expect(getTimelineEvent("e1")).rejects.toThrow(/this timeline event \(HTTP 404\)/);
  });
});

describe("cw", () => {
  it("returns a stored string", () => {
    expect(cw({ a: "x" }, "a")).toBe("x");
  });

  it("throws BY NAME rather than rendering a control with no label", () => {
    expect(() => cw({}, "missing_key")).toThrow(/no stored wording for "missing_key"/);
    expect(() => cw({ blank: "   " }, "blank")).toThrow(/no stored wording for "blank"/);
  });
});

describe("fill", () => {
  it("replaces every occurrence of each placeholder", () => {
    expect(fill("{a} and {a} and {b}", { a: 1, b: "two" })).toBe("1 and 1 and two");
  });

  it("leaves a placeholder nobody supplied visible rather than blanking it", () => {
    // A silently emptied placeholder reads as a missing number; the token
    // reads as a bug, which is what it is.
    expect(fill("{a} of {b}", { a: 1 })).toBe("1 of {b}");
  });
});

describe("BOOTSTRAP_TEXT", () => {
  it("is the only place these three sentences exist", () => {
    // They cannot be settings rows: the wording store is delivered by the very
    // request whose failure they describe. Pinned so the exception stays small.
    expect(BOOTSTRAP_TEXT.timelineFailed("boom")).toContain("boom");
    expect(BOOTSTRAP_TEXT.eventFailed("boom")).toContain("boom");
    expect(BOOTSTRAP_TEXT.loading.trim()).not.toBe("");
  });
});
