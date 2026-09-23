// Tests for `threadNotes` — the exchange, as a pure function.
//
// The one property that matters more than the shape: nothing is lost. A panel
// that quietly drops a note is the failure this feature exists to remove.

import { describe, expect, it } from "vitest";

import { threadNotes } from "../noteThreads";
import type { PracticeNote } from "../../../services/practice";

function note(id: string, answers?: string): PracticeNote {
  return {
    id,
    question_id: "q1",
    answer_id: null,
    author: "Chuck",
    text: `note ${id}`,
    when: "Tue 22 Sep",
    struck: null,
    ...(answers === undefined ? {} : { answers_note_id: answers }),
  };
}

/** How many notes a threading covers, roots and replies together. */
function covered(threads: ReturnType<typeof threadNotes>): number {
  return threads.reduce((n, thread) => n + 1 + thread.replies.length, 0);
}

describe("threadNotes", () => {
  it("puts a reply under the note it answers", () => {
    const threads = threadNotes([note("a"), note("b", "a")]);
    expect(threads).toHaveLength(1);
    expect(threads[0].note.id).toBe("a");
    expect(threads[0].replies.map((r) => r.id)).toEqual(["b"]);
  });

  it("keeps two replies to one note in the order they were written", () => {
    const threads = threadNotes([note("a"), note("b", "a"), note("c", "a")]);
    expect(threads[0].replies.map((r) => r.id)).toEqual(["b", "c"]);
  });

  it("leaves notes that answer nothing as their own rows", () => {
    const threads = threadNotes([note("a"), note("b")]);
    expect(threads.map((t) => t.note.id)).toEqual(["a", "b"]);
    expect(threads.every((t) => t.replies.length === 0)).toBe(true);
  });

  it("shows an orphan reply rather than dropping it", () => {
    // Its parent is not on this panel — struck, or on a superseded attempt.
    const threads = threadNotes([note("b", "missing")]);
    expect(threads).toHaveLength(1);
    expect(threads[0].note.id).toBe("b");
  });

  it("loses no note, whatever the shape", () => {
    const notes = [note("a"), note("b", "a"), note("c"), note("d", "gone"), note("e", "c")];
    expect(covered(threadNotes(notes))).toBe(notes.length);
  });

  it("returns nothing for no notes", () => {
    expect(threadNotes([])).toEqual([]);
  });
});
