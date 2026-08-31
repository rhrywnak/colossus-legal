// =============================================================================
// subsetSaveModel.test.ts — what Save sends, and what the banner says
// =============================================================================
//
// T6.5's first two named suites. Both functions decide something the old code
// decided inside a callback: whether the second write happens at all, and what
// a reader is told when it fails after the first one committed.

import { describe, expect, it } from "vitest";

import type { ChronologyWording } from "../../../services/caseTimeline";
import type { SubsetDetail, SubsetEvent } from "../../../services/caseTimelineSubsets";
import type { Pick } from "../subsetPicker";
import { bannerModel, eventsAreDirty, type SaveFailure } from "../subsetSaveModel";

const wording = {
  subsets_saved_name_only_banner: "Name and description saved.",
  subsets_events_not_saved_banner_template:
    "The event list was not saved — the server refused it (HTTP {status}: {reason}). Fix and Save again; nothing you picked has been lost.",
} as unknown as ChronologyWording;

/** One stored reference, as `GET /subsets/:id` joins it. */
function ref(id: string, note = ""): SubsetEvent {
  return {
    event: { id } as SubsetEvent["event"],
    subset_note: note,
    removed: false,
  };
}

function detail(events: SubsetEvent[]): SubsetDetail {
  return { id: "s1", name: "The $50,000", description: "", events } as SubsetDetail;
}

const pick = (event_id: string, note = ""): Pick => ({ event_id, note });

describe("eventsAreDirty — is the second write worth making", () => {
  it("says NO for the exact list that was loaded", () => {
    // Roman's reproduction: fifteen events, all picked, none reordered, only
    // the name changed. This is the case that should now make ONE call.
    const original = detail([ref("a"), ref("b"), ref("c")]);
    expect(eventsAreDirty(original, [pick("a"), pick("b"), pick("c")])).toBe(false);
  });

  it("says YES when an event is added", () => {
    expect(eventsAreDirty(detail([ref("a")]), [pick("a"), pick("b")])).toBe(true);
  });

  it("says YES when an event is removed", () => {
    expect(eventsAreDirty(detail([ref("a"), ref("b")]), [pick("a")])).toBe(true);
  });

  it("says YES when the SAME events are reordered", () => {
    // The same fifteen events in a different order is a different story.
    expect(eventsAreDirty(detail([ref("a"), ref("b")]), [pick("b"), pick("a")])).toBe(true);
  });

  it("says YES when a note is typed, and when one is cleared", () => {
    expect(eventsAreDirty(detail([ref("a")]), [pick("a", "the key transfer")])).toBe(true);
    expect(eventsAreDirty(detail([ref("a", "the key transfer")]), [pick("a")])).toBe(true);
  });

  it("says NO for a note that is only whitespace either side of the wire", () => {
    // The payload builder trims, so a note typed and blanked back to spaces
    // would send identical bytes. That is not a change and must not write.
    expect(eventsAreDirty(detail([ref("a")]), [pick("a", "   ")])).toBe(false);
    expect(eventsAreDirty(detail([ref("a", "note")]), [pick("a", "  note  ")])).toBe(false);
  });

  it("says YES on the create path, where there is no original to compare", () => {
    expect(eventsAreDirty(null, [])).toBe(true);
  });

  it("says NO for an empty subset left empty", () => {
    expect(eventsAreDirty(detail([]), [])).toBe(false);
  });
});

describe("bannerModel — the first call failed", () => {
  it("draws the named sentence and NO green half", () => {
    // Nothing was attempted after it and nothing committed. A green line here
    // would be the old lie with the colours reversed.
    const failure: SaveFailure = {
      nameSaved: false,
      status: 409,
      reason: "a subset already has that name",
      sentence: "That subset was not saved (HTTP 409 — a subset already has that name).",
    };
    expect(bannerModel(wording, failure)).toEqual({
      saved: null,
      failed: "That subset was not saved (HTTP 409 — a subset already has that name).",
    });
  });
});

describe("bannerModel — the rename landed and the events did not", () => {
  it("says what saved, then what did not and why", () => {
    const failure: SaveFailure = {
      nameSaved: true,
      status: 422,
      reason: "position 8 has no event id",
      sentence: "That subset's events were not saved (HTTP 422 — position 8 has no event id).",
    };
    const banner = bannerModel(wording, failure);
    expect(banner.saved).toBe("Name and description saved.");
    expect(banner.failed).toBe(
      "The event list was not saved — the server refused it (HTTP 422: position 8 has no event id). Fix and Save again; nothing you picked has been lost.",
    );
  });

  it("carries the SERVER's own reason and not a word of our own", () => {
    // T1 answers 400/409/422 naming the offending field and value. Replacing
    // that would discard the only part of the message that says what to fix.
    const banner = bannerModel(wording, {
      nameSaved: true,
      status: 400,
      reason: "an event id appears twice",
      sentence: "…",
    });
    expect(banner.failed).toContain("an event id appears twice");
    expect(banner.failed).toContain("HTTP 400");
  });

  it("renders the status ALONE when the body carried no message", () => {
    // The 422 that started T6 was exactly this: Axum's extractor answers with
    // plain text, not the app's JSON envelope, so there is no message to quote.
    // "(HTTP 422: )" — a colon in front of nothing — is what must not appear.
    const banner = bannerModel(wording, {
      nameSaved: true,
      status: 422,
      reason: "",
      sentence: "…",
    });
    expect(banner.failed).toContain("(HTTP 422)");
    expect(banner.failed).not.toContain("422:");
    expect(banner.failed).toContain("nothing you picked has been lost");
  });

  it("treats a whitespace-only reason as no reason", () => {
    const banner = bannerModel(wording, {
      nameSaved: true,
      status: 500,
      reason: "   ",
      sentence: "…",
    });
    expect(banner.failed).toContain("(HTTP 500)");
  });

  it("falls back to the named sentence when NO server answered", () => {
    // A timeout has no status and no reason. Filling {status} with a made-up 0
    // would be a lie the banner then printed.
    const banner = bannerModel(wording, {
      nameSaved: true,
      status: null,
      reason: "",
      sentence: "That subset's events were not saved (The request timed out).",
    });
    expect(banner.saved).toBe("Name and description saved.");
    expect(banner.failed).toBe("That subset's events were not saved (The request timed out).");
  });
});
