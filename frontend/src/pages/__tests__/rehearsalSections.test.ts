/**
 * Tests for `rehearsalSections` — two tiny rules whose failures are invisible.
 *
 * A default-state map wired to the wrong field opens the wrong section and looks
 * like a design choice. A placeholder left unfilled prints "{code}" to a witness.
 * Neither would produce an error anywhere, which is exactly why both are pinned.
 */
import { describe, expect, it } from "vitest";

import { fillCode, openSectionsFrom } from "../rehearsalSections";

describe("openSectionsFrom", () => {
  it("maps each stored default onto the section it belongs to", () => {
    // The failure this guards: a spread of the snake_case wire object would give
    // the component keys it never reads, so every section would be `undefined` —
    // which renders as closed. Every block silently shut, nothing in the log, on
    // a page whose design is that the accusation is open when you arrive.
    const open = openSectionsFrom({
      what_open: true,
      accusation_open: true,
      timeline_open: false,
      points_open: true,
      watch_for_open: false,
    });

    expect(open).toEqual({
      what: true,
      accusation: true,
      timeline: false,
      points: true,
      watchFor: false,
    });
  });

  it("carries the server's decision and never invents one", () => {
    // Every section shut is a legitimate stored configuration. A helper that
    // "helpfully" forced the accusation open would be overruling the store.
    const open = openSectionsFrom({
      what_open: false,
      accusation_open: false,
      timeline_open: false,
      points_open: false,
      watch_for_open: false,
    });

    expect(Object.values(open).every((v) => v === false)).toBe(true);
  });

  it("carries the fifth section the signed mockup added", () => {
    // Task 2.11 C: "What this is" folds like the others now. It was fixed open
    // in B2, so a mapping that forgot it would leave the section permanently
    // shut — the one failure mode a boolean nobody reads produces.
    const open = openSectionsFrom({
      what_open: true,
      accusation_open: false,
      timeline_open: false,
      points_open: false,
      watch_for_open: false,
    });

    expect(open.what).toBe(true);
    expect(open.accusation).toBe(false);
  });

  it("distinguishes the two fields whose names are nearly the same", () => {
    // `timeline_open` and `points_open` sit next to each other and a copy-paste
    // between them type-checks perfectly.
    const open = openSectionsFrom({
      what_open: false,
      accusation_open: false,
      timeline_open: true,
      points_open: false,
      watch_for_open: false,
    });

    expect(open.timeline).toBe(true);
    expect(open.points).toBe(false);
  });
});

describe("fillCode", () => {
  it("puts the handle a reader typed into the stored sentence", () => {
    expect(fillCode("{code} is not ready to rehearse yet.", "S-3")).toBe(
      "S-3 is not ready to rehearse yet.",
    );
  });

  it("substitutes nothing when there is no code, rather than printing undefined", () => {
    // Reachable only from the address-less route, where the sentence is not shown
    // at all — but "undefined is not ready to rehearse yet" is the kind of thing
    // that reaches a screenshot, so it cannot be possible.
    expect(fillCode("{code} is not ready.", undefined)).toBe(" is not ready.");
  });

  it("leaves a template with no slot untouched rather than appending", () => {
    // The settings write path refuses a template that drops {code}, so this state
    // means the store was edited around the API. The honest render is the
    // sentence as written — not one with a handle bolted onto the end.
    expect(fillCode("That scenario is not ready.", "S-3")).toBe(
      "That scenario is not ready.",
    );
  });
});
