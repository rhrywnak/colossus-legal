// =============================================================================
// practiceWalkWrites.test.ts — the walk writes NOTHING
// =============================================================================
//
// ## ⚑ WHAT THIS FILE CANNOT DO, AND WHY IT IS NOT A NETWORK SPY
//
// The intended test is a spy on the network layer, driven across the whole loop:
// Start practising, Show my answer, Next through the end, Practise them again,
// asserting ZERO calls. It cannot be written here. This project has NO DOM test
// environment — no jsdom, no happy-dom, no `@testing-library`, vitest in the
// node environment — so the loop cannot be driven at all, because the components
// cannot be rendered.
//
// And a spy that cannot be driven is the WORST possible instrument for this
// particular claim, which is why no attempt is made to fake one:
//
//   ⚑ ZERO IS THE EASIEST NUMBER TO GET WRONG, BECAUSE IT IS WHAT A BROKEN
//     INSTRUMENT REPORTS. A spy never wired to the client reports zero calls
//     forever, no matter what the walk does.
//
// So this asserts the claim STRUCTURALLY, at the only place it can be made
// false: the walk's module graph. A write cannot happen if nothing that writes
// is reachable from the page.
//
// ## The mutation this file is proved against
//
// Make something HAPPEN — give Show my answer a POST — and these go red. Not
// "weaken the assertion and watch it still pass", which proves nothing.
//
// ⚑ THE DAY A DOM TIER EXISTS, replace this with the spy. It is a substitute.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const PAGES = join(__dirname, "..", "..", "..", "pages");
const HERE = join(__dirname, "..");
const read = (dir: string, file: string) => readFileSync(join(dir, file), "utf8");

/** Source with `//` comments stripped — this file's own prose names the very
 *  things it forbids. See `domain/wording_tests.rs` for why that is a rule. */
const withoutComments = (source: string): string =>
  source
    .split("\n")
    .map((line) => (line.includes("//") ? line.slice(0, line.indexOf("//")) : line))
    .join("\n");

/** Every name the walk page imports from a service module. */
function serviceImports(source: string): string[] {
  const names: string[] = [];
  // ⚑ NO STRAY `exec` BEFORE THE LOOP. A global-flag regex carries `lastIndex`,
  // so an extra `exec` advances past the FIRST import block and the loop below
  // never sees it. That is what the first version of this helper did: the page's
  // opening import line was silently unexamined while the test reported full
  // coverage. Found by the test-auditor gate, not by reading it.
  const pattern = /import\s*\{([^}]*)\}\s*from\s*"\.\.\/services\/[^"]+"/g;
  let m: RegExpExecArray | null;
  while ((m = pattern.exec(source)) !== null) {
    for (const raw of m[1].split(",")) {
      const name = raw.replace(/\btype\b/, "").trim();
      if (name !== "") names.push(name);
    }
  }
  return names;
}

/**
 * The service calls the walk is ALLOWED to make. Both are reads, both on mount.
 *
 * A list rather than a pattern: "anything not called post* is safe" is exactly
 * the assumption that lets `submitPracticeAnswer` through.
 */
const ALLOWED = ["fetchPracticeDeck", "fetchPracticeAnswers", "wordingOf"];

describe("the practice walk writes nothing", () => {
  const walk = () => withoutComments(read(PAGES, "PracticeWalkPage.tsx"));

  it("imports only reads from the service layer", () => {
    const imported = serviceImports(walk()).filter((name) => !name.startsWith("Practice"));

    // ANTI-VACUITY, and specific rather than "more than zero": the page imports
    // exactly these reads, and a helper that skipped an import block would find
    // fewer. `> 0` was what let the `lastIndex` bug hide.
    expect(imported.sort()).toEqual(["fetchPracticeAnswers", "fetchPracticeDeck", "wordingOf"]);
    for (const name of imported) {
      expect(ALLOWED, `${name} is reachable from the walk and is not a known read`).toContain(name);
    }
  });

  it("names no write verb anywhere in its source", () => {
    // MUTATION: give Show my answer a POST — `submitPracticeAnswer`, `method:
    // "POST"`, anything — and this goes red.
    const source = walk();
    for (const verb of [
      "submitPracticeAnswer",
      "openAnswerSession",
      "hideQuestion",
      "closePracticeAnswer",
      "savePracticeFlag",
      'method: "POST"',
      'method: "PUT"',
      'method: "DELETE"',
    ]) {
      expect(source, `the walk reaches a write: ${verb}`).not.toContain(verb);
    }
  });

  it("makes its two reads ONCE, on mount, and not per step", () => {
    // A fetch inside the reveal or the Next handler would be a request per
    // question — invisible on screen, and exactly the shape "nothing visible
    // happened" would hide.
    const source = walk();
    const effects = source.split("React.useEffect(");
    expect(effects.length, "the page must have a mount effect").toBe(2);

    const afterEffect = effects[1];
    const fetchesInEffect = (afterEffect.match(/fetchPractice(Deck|Answers)\(/g) ?? []).length;
    expect(fetchesInEffect, "both reads belong in the mount effect").toBe(2);

    const total = (source.match(/fetchPractice(Deck|Answers)\(/g) ?? []).length;
    expect(total, "no read outside the mount effect").toBe(2);
  });

  it("the walk's own decisions touch no network at all", () => {
    // `practiceWalk.ts` is pure: it is handed questions and answers and returns
    // steps. If it ever imported a service, the loop could write from inside a
    // decision, where no scan of the page would see it.
    const decisions = withoutComments(read(HERE, "practiceWalk.ts"));
    expect(decisions).not.toMatch(/from "\.\.\/\.\.\/services\/(?!practice"|practiceAnswers")/);
    expect(decisions).not.toContain("authFetch");
    expect(decisions).not.toContain("fetch(");
  });
});
