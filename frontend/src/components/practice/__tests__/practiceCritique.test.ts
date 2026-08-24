// =============================================================================
// practiceCritique.test.ts — the critique block's five states
// =============================================================================
//
// Every branch here is a state Roman has seen or ruled on, and two of them are
// states that must render NOTHING. Those two are the ones worth the file: an
// empty three-part scaffold with blank headings reads as a broken page, and this
// is the only thing that will notice if one appears.

import { describe, expect, it } from "vitest";

import { citedSources, critiqueFor } from "../practiceCritique";
import type { AnswerResult } from "../../../services/practice";

const result = (over: Partial<AnswerResult> = {}): AnswerResult => ({
  answer_id: "a1",
  read_text: null,
  read_ok: null,
  read_parts: null,
  read_sources: [],
  ...over,
});

describe("what the critique block shows", () => {
  it("shows nothing before she has answered", () => {
    expect(critiqueFor(null)).toEqual({ kind: "idle" });
  });

  it("shows the three parts when the read produced them", () => {
    const view = critiqueFor(
      result({
        read_text: "a composed line",
        read_ok: false,
        read_parts: { call: "You fixed the trap.", why: "because", pointers: ["do this"], keys: ["S2"] },
      }),
    );
    expect(view.kind).toBe("parts");
  });

  it("shows ONE SENTENCE for an answer written before T1 — the common case", () => {
    // Measured on DEV: 10 of 14 answers carry text and no parts. An empty
    // three-part scaffold for those would make most of her history look broken.
    const view = critiqueFor(result({ read_text: "the older one-line read", read_ok: true }));
    expect(view).toEqual({ kind: "sentence", text: "the older one-line read", ok: true });
  });

  it("shows NOTHING when she stopped waiting or the read failed", () => {
    // Her answer is saved either way. An empty bordered box would say
    // "something should be here" and invite her to wait for what is not coming.
    expect(critiqueFor(result())).toEqual({ kind: "none" });
  });

  it("prefers the parts over the text, and never shows both", () => {
    // `read_text` is a LOSSY PROJECTION — `compose_read_text` drops `why` and
    // keeps only the first pointer — so rendering both prints the call and the
    // first pointer twice.
    const view = critiqueFor(
      result({
        read_text: "You fixed the trap. Put the date on it.",
        read_parts: { call: "You fixed the trap.", why: "w", pointers: ["Put the date on it."], keys: [] },
      }),
    );
    expect(view.kind).toBe("parts");
    expect(view).not.toHaveProperty("text");
  });
});

describe("the source list — the one place a bad read can be caught", () => {
  it("pairs each cited key with the words that were SENT to the model", () => {
    const cited = citedSources(
      result({
        read_parts: { call: "c", why: "w", pointers: [], keys: ["S2", "R1"] },
        read_sources: [
          { key: "R1", text: "your certified letter" },
          { key: "S2", text: "CFS sworn answer, p. 5" },
          { key: "P1", text: "a point nobody cited" },
        ],
      }),
    );

    expect(cited).toEqual([
      { key: "S2", text: "CFS sworn answer, p. 5" },
      { key: "R1", text: "your certified letter" },
    ]);
  });

  it("lists only what was cited, in the order it was cited", () => {
    // The payload carries every citable source. Listing all of them would bury
    // the two she needs under eleven she does not.
    const cited = citedSources(
      result({
        read_parts: { call: "c", why: "w", pointers: [], keys: ["R1"] },
        read_sources: [
          { key: "S1", text: "one" },
          { key: "R1", text: "two" },
        ],
      }),
    );
    expect(cited).toHaveLength(1);
    expect(cited[0].key).toBe("R1");
  });

  it("KEEPS a cited key that has no source, rather than hiding it", () => {
    // It should be impossible — the read refuses a key it was not sent. If one
    // appears anyway, showing the key with nothing behind it is how anybody
    // finds out. Dropping it would hide the failure this list exists to expose.
    const cited = citedSources(
      result({
        read_parts: { call: "c", why: "w", pointers: [], keys: ["S9"] },
        read_sources: [],
      }),
    );
    expect(cited).toEqual([{ key: "S9", text: null }]);
  });

  it("cites nothing when there are no parts", () => {
    expect(citedSources(result({ read_text: "older" }))).toEqual([]);
  });
});
