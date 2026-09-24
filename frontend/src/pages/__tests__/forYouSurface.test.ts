// =============================================================================
// forYouSurface.test.ts — the "For you" page renders what it must
// =============================================================================
//
// CC_TASK_FOR_YOU_v1 L1. A SOURCE SCAN, in the shape `practiceReviewSurface`
// and `onePageSurface` already use: there is no component-testing rig in this
// project (CLAUDE.md rule 30), and these are structural claims about the page
// rather than behaviour a pure helper could carry.
//
// ## What it is for, and what it is not
//
// It cannot prove the page LOOKS right — the journeys and the screenshots in
// the report do that, measured in a browser. It proves the claims that would
// fail silently: that no user-facing sentence is written in the source, that
// the page decides nothing about who may see what, that every wording call is
// a LITERAL the backend's reach scanner can see, and that the read-clear is
// conditional on having arrived from the list.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const PAGES = join(__dirname, "..");
const FOR_YOU = join(__dirname, "..", "..", "components", "forYou");

const read = (dir: string, name: string) => readFileSync(join(dir, name), "utf8");

/** Source with its `//` comments removed — this repo documents beside its code. */
function withoutComments(source: string): string {
  return source
    .split("\n")
    .map((line) => {
      const at = line.indexOf("//");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");
}

describe("the For you page", () => {
  const page = () => withoutComments(read(PAGES, "ForYouPage.tsx"));

  it("writes no sentence of its own — every string is the store's", () => {
    // The page renders `w("…")` for its own furniture and server-composed
    // fields for everything else. A sentence written here would be a string
    // Roman could not retune in Settings, on the one page whose entire content
    // is sentences.
    const source = page();
    for (const sentence of [
      "Nothing waiting",
      "Unread",
      "Everything",
      "TODAY",
      "YESTERDAY",
      "EARLIER",
      "For you",
    ]) {
      expect(source).not.toContain(`>${sentence}`);
      expect(source).not.toContain(`"${sentence}"`);
    }
  });

  it("asks for every wording key as a LITERAL the reach scanner can see", () => {
    // ⚑ The scanner matches `w("key")` exactly. `w(keyFor(day))` is invisible
    // to it, and a key missing from the wire would then throw in front of a
    // reader with nothing having warned us — which is how .407 shipped a blank
    // practice page.
    const source = page();
    const calls = [...source.matchAll(/\bw\(([^)]*)\)/g)].map((m) => m[1].trim());
    expect(calls.length).toBeGreaterThanOrEqual(5);
    for (const argument of calls) {
      expect(argument, `w(${argument}) is not a literal key`).toMatch(/^"[a-z0-9_]+"$/);
    }
  });

  it("decides nothing about whose list this is", () => {
    // Rule 12: the SERVER says which side the reader is on and composes the
    // sentences for it. The page compares no username and knows no reviewer.
    const source = page();
    expect(source).not.toContain("username");
    expect(source).not.toContain("is_admin");
    expect(source).not.toContain("reviewer");
    // The one thing it may read is the side the payload states.
    expect(source).toContain('page.side === "none"');
  });

  it("shows a failed read rather than an empty list", () => {
    // An empty list after a failed read says "nothing is waiting" — a
    // confident, false screen, and the one thing this page must never show.
    const source = page();
    expect(source).toContain("setError");
    expect(source).toContain('role="alert"');
    expect(source).toContain("console.error");
  });

  it("withholds the last-item line when there has never been one", () => {
    expect(page()).toContain("page.empty_last !== undefined");
  });

  it("withholds the tabs when BOTH lists are empty, and only then", () => {
    // Ruling GO-2.2 (board 2): two controls that both lead to the same empty
    // page are two things to try before believing the sentence under them.
    // "Both" is the whole of it — an empty Unread tab beside a full Everything
    // tab must keep its tabs, or the other list becomes unreachable.
    const source = page();
    expect(source).toContain(
      "page.unread.length === 0 && page.everything.length === 0",
    );
    expect(source).toContain("{!nothingAtAll && (");
    // The empty MESSAGE is not conditional on the same fact: an empty Unread
    // tab with a full Everything tab still shows it.
    expect(source).toContain("groups.length === 0 ? (");
  });
});

describe("a For you row", () => {
  const row = () => withoutComments(read(FOR_YOU, "ForYouRow.tsx"));

  it("is one link, with the whole row inside it", () => {
    // A row is one thing waiting; hunting for its clickable part is friction on
    // the page whose job is to remove friction. A real anchor, so the keyboard
    // and "open in new tab" work without this file knowing about them.
    const source = row();
    expect(source).toContain("<Link");
    expect(source.match(/<Link/g) ?? []).toHaveLength(1);
  });

  it("carries from=for-you into a question, and nothing into a deck", () => {
    // The read-clear hangs off that query (ruling Q4). A row about a whole
    // scenario has no question to open and must NOT claim to clear anything.
    const source = row();
    expect(source).toContain('"for-you"');
    expect(source).toContain("practicePath(slug, row.scenario_id)");
  });

  it("renders the server's three lines and composes none of them", () => {
    const source = row();
    for (const field of ["row.deck_line", "row.body", "row.byline", "row.when"]) {
      expect(source).toContain(field);
    }
  });

  it("draws the unread mark, and draws the read one hollow rather than absent", () => {
    // FOR_YOU_MOCKUP_v1 board 1. The dot is the third reading of "unread",
    // beside the tint and the edge, and it is the one that survives a reader
    // who cannot separate the two blues. The READ row keeps the circle, empty,
    // so the text of every row starts on the same vertical rule.
    const source = row();
    expect(source).toContain("s.dotRead");
    expect(source).toContain("s.dot");
    expect(source).toContain('aria-hidden="true"');
  });

  it("draws an unread row differently from a read one", () => {
    // The shipped page had this the wrong way round — the READ row carried the
    // tint. Two states that draw the same, or that draw each other's ground,
    // are one state as far as a reader is concerned.
    const source = row();
    expect(source).toContain("row.read ? s.rowRead : s.rowUnread");
    const styles = withoutComments(readFileSync(join(FOR_YOU, "forYouStyles.ts"), "utf8"));
    expect(styles).toContain("export const rowUnread");
    expect(styles).toContain("export const rowRead");
    // The unread row carries BOTH the board's ground and its edge.
    const unread = styles.slice(styles.indexOf("export const rowUnread"));
    expect(unread.slice(0, 220)).toContain("background: SOFT");
    expect(unread.slice(0, 220)).toContain("borderColor: NEW_BORDER");
  });

  it("puts the time in its own column, and drops it under the text on a phone", () => {
    // Board 5 (ruling §12.5): the CSS reading. A media query is one of the two
    // things a React style object cannot carry, so it is a real `<style>`
    // element — and it is useless unless the page renders it.
    const styles = readFileSync(join(FOR_YOU, "forYouStyles.ts"), "utf8");
    expect(styles).toContain("@media (max-width: 430px)");
    expect(styles).toContain("[data-for-you-when]");
    expect(row()).toContain("data-for-you-when");
    expect(read(PAGES, "ForYouPage.tsx")).toContain("s.PHONE_CSS");
  });
});

describe("the read-clear", () => {
  const hook = () => withoutComments(read(FOR_YOU, "useReadClear.ts"));
  const question = () => withoutComments(read(PAGES, "PracticeQuestionPage.tsx"));

  it("only fires when the reader arrived from the list", () => {
    // Arriving from the deck, a bookmark or a discussion link must not silently
    // mark somebody's notes as read.
    const source = hook();
    expect(source).toContain('get("from") === FROM_FOR_YOU');
    expect(source).toContain("if (!fromForYou");
    expect(source).toContain("markQuestionSeen(questionId)");
  });

  it("refreshes the menu count after clearing", () => {
    // The badge is stale by however many rows the clear removed, and only the
    // server knows the new number.
    expect(hook()).toContain("refresh()");
  });

  it("puts a failed write on screen rather than only in the console", () => {
    // Standing Rule 1: an authFetch failure gets a user-facing surface. The
    // question stays readable — this is a line, not a barrier.
    expect(hook()).toContain("setFailed(true)");
    const page = question();
    expect(page).toContain("readClearFailed");
    expect(page).toContain('w("row_read_failed")');
    expect(page).toContain('role="alert"');
  });
});

describe("the menu badge", () => {
  const badge = () => withoutComments(read(FOR_YOU, "ForYouBadge.tsx"));

  it("draws nothing at zero and a marker when the count could not be read", () => {
    // Three states, two of which draw something. "Nothing waiting" and "we
    // could not tell" must not look identical.
    const source = badge();
    expect(source).toContain("badgeLabel(count)");
    expect(source).toContain("return null");
    expect(source).toContain("data-for-you-badge-failed");
    expect(source).toContain("FOR_YOU_COUNT_UNAVAILABLE_LABEL");
  });
});
