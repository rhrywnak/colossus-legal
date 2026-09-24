// =============================================================================
// boardFourSurface.test.ts — the deck row's waiting mark, and the reply control
// =============================================================================
//
// CC_TASK_FOR_YOU_v1 L3. Source scans, for the standing reason (CLAUDE.md rule
// 30: no jsdom, no testing-library here) — and for a second reason this task
// learned the hard way: a component can be written, typed, tested at the helper
// level and rendered by NOTHING. `waiting` arrives on every question of every
// deck payload; what these assert is that something on screen reads it.
//
// They cannot prove the mark is legible, or that it sits where the mockup puts
// it. Roman's walk knows that; this knows the field is reachable at all.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const HERE = join(__dirname, "..");
const read = (file: string) => readFileSync(join(HERE, file), "utf8");

/** Source with its `//` comments removed — this repo documents beside its code. */
const withoutComments = (source: string): string =>
  source
    .split("\n")
    .map((line) => {
      const at = line.indexOf("//");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");

describe("the board-4 mark on a deck row", () => {
  const row = withoutComments(read("PracticeDeckRow.tsx"));

  it("renders the server's composed mark", () => {
    expect(row).toContain("question.waiting");
    expect(row).toContain("data-practice-waiting");
  });

  it("draws nothing when nothing waits", () => {
    // Absent, not blank: an empty chip beside a question reads as a tag that
    // failed to load, which is a different fact and the wrong one to show.
    expect(row).toContain("question.waiting !== undefined");
  });

  it("tints the row itself when something waits", () => {
    // CC_TASK_FOR_YOU_POLISH_v1 (GO ruling 1). The board draws BOTH: a mark on
    // the right, and a tinted ground under the whole row. The mark alone is
    // four words at the end of a long row; the tint is what makes the row
    // findable from the top of a seventeen-question list without reading any
    // of it. A refactor that dropped the tint and kept the mark would pass
    // every other assertion in this file — which is why this one names the
    // style, and names the condition it hangs off.
    expect(row).toContain("question.waiting !== undefined ? r.waitingRow");
    const styles = withoutComments(read("practiceReviewLoopStyles.ts"));
    expect(styles).toContain("export const waitingRow");
    expect(styles).toContain("var(--practice-waiting-row-bg)");
  });

  it("composes no sentence of its own", () => {
    // Which of four stored templates a row wears is decided on the server, by
    // what is unread FOR THIS READER. A component that assembled the sentence
    // would put those four settings rows beyond Roman's reach.
    // `answered` on its own is not searched for: `question.answered_on` is a
    // legitimate field of the same row, composed by the server too.
    expect(row).not.toMatch(/left a note|the question changed|\+\{?.*\}? more/);
  });
});

describe("the reply control", () => {
  const list = withoutComments(read("PracticeNoteList.tsx"));
  const panel = withoutComments(read("PracticeAnswerNotes.tsx"));

  it("is offered on a standing note, from the stored label", () => {
    expect(list).toContain('w("row_note_reply_label")');
    expect(list).toContain("onReply");
  });

  it("is not offered on a struck note", () => {
    // A withdrawn note cannot be answered — the route refuses it with a 400,
    // and the panel must not offer a button that can only fail.
    expect(list).toContain("note.struck === null && onReply !== undefined");
  });

  it("draws replies under the note they answer", () => {
    expect(list).toContain("threadNotes");
    expect(list).toContain("data-note-replies");
  });

  it("writes a reply through the reply call, not as a fresh note", () => {
    expect(panel).toContain("replyToNote(replyTo.id, text)");
  });

  it("says which note the box is answering", () => {
    expect(panel).toContain("data-reply-to");
  });
});
