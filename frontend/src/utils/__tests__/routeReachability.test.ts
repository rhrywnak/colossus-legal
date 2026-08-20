// =============================================================================
// routeReachability.test.ts — the MIRROR of the route-link guard
// =============================================================================
//
// `routePaths.test.ts` asks "does every link point at a real route?". This file
// asks the opposite: "does every real route have a link into it?" — and the
// one-release redirects, which are the answer to "what happens to the addresses
// this release removed?".
//
// Split from its sibling rather than appended to it: that file doubled in size
// when the nav table went through the builders, and the two questions have
// different shapes. One walks a table of builders; this one walks the source
// tree. They share only the parsing helpers, which are re-declared here — three
// small readers, against a cross-file import of test internals.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  adminDataPath,
  adminLogsPath,
  adminPath,
  adminPromptsPath,
  adminSettingsPath,
  allegationsPath,
  askPath,
  caseHealthPath,
  documentPath,
  documentsPath,
  homePath,
  peoplePath,
  practicePath,
  practiceQuestionPath,
  practiceSessionPath,
  proofMatrixPath,
  proofReviewTabPath,
  rehearsalPath,
  rehearsalScenarioPath,
  scenarioPagePath,
  timelinePath,
  trialPrepPath,
} from "../routePaths";

const APP_TSX = join(__dirname, "..", "..", "App.tsx");
const CATCH_ALL = "*";

/** Every `<Route path="…">` App.tsx declares, in source order. */
function declaredRoutes(): string[] {
  const source = readFileSync(APP_TSX, "utf8");
  return [...source.matchAll(/<Route\s+path="([^"]+)"/g)].map((m) => m[1]);
}

/** The declared routes minus the catch-all, which matches everything. */
function realRoutes(): string[] {
  return declaredRoutes().filter((p) => p !== CATCH_ALL);
}

/**
 * Every route a builder emits, as a ROUTE PATTERN.
 *
 * The sibling file pairs each builder with its route and asserts they match.
 * Here only the route side is needed — this file asks which routes are pointed
 * at, not whether the pointing is correct.
 */
const BUILDER_ROUTES: string[] = [
  "/",
  "/documents",
  "/documents/:id",
  "/ask",
  "/people",
  "/timeline",
  "/allegations",
  "/cases/:slug/proof-matrix",
  "/cases/:slug/case-health",
  "/admin",
  "/admin/prompts",
  "/admin/data",
  "/admin/logs",
  "/admin/settings",
  "/cases/:slug/trial-prep",
  "/cases/:slug/trial-prep/:scenarioId",
  "/cases/:slug/rehearsal",
  "/cases/:slug/rehearsal/:code",
  "/cases/:slug/trial-prep/practice/:scenarioId",
  "/cases/:slug/trial-prep/practice/:scenarioId/session/:sessionId",
  "/cases/:slug/trial-prep/practice/:scenarioId/question/:questionId",
];

/**
 * The builders are CALLED, not merely listed.
 *
 * Without this the table above is a list of strings that could drift from the
 * module it claims to describe — and this file would then report orphans against
 * an inventory nobody maintains. Calling each one proves the export still exists
 * and still emits the shape the table names.
 */
const BUILDER_CALLS: Array<() => string> = [
  homePath,
  documentsPath,
  () => documentPath("d"),
  askPath,
  peoplePath,
  timelinePath,
  allegationsPath,
  () => proofMatrixPath("c"),
  () => proofReviewTabPath("c"),
  () => caseHealthPath("c"),
  adminPath,
  adminPromptsPath,
  adminDataPath,
  adminLogsPath,
  adminSettingsPath,
  () => trialPrepPath("c"),
  () => scenarioPagePath("c", "s"),
  () => rehearsalPath("c"),
  () => rehearsalScenarioPath("c", "S-1"),
  () => practicePath("c", "s"),
  () => practiceSessionPath("c", "s", "x"),
  () => practiceQuestionPath("c", "s", "q"),
];

// -----------------------------------------------------------------------------
// The MIRROR half: is every declared route reachable? (nav cleanup, Part 2)
// -----------------------------------------------------------------------------
//
// The guard above answers "does every link point at a real route?". This answers
// the opposite question — "does every real route have a link into it?" — and it
// is the half that finds a page nobody can get to, which is a different and
// quieter defect than a link that 404s.
//
// ## Why this is SOFT for now, and what soft means here
//
// Part 1 measured nine declared routes with no link into them from anywhere in
// the app. They are not defects to fix in this task and not removals anybody
// ruled on: some are probably dead, some are probably reachable by typing and
// deliberately so. Roman rules on them separately.
//
// So this test LISTS what it finds and passes. It is not `it.todo` — a todo runs
// nothing and would go stale silently. It runs the full scan every time, prints
// the current set, and asserts only that the set has not GROWN beyond the known
// nine. A tenth orphan — a route added with no way in, or a link removed from a
// page that had one — turns this red.

/**
 * Routes whose element is a REDIRECT rather than a page.
 *
 * These are deliberately unlinked — that is what a one-release redirect IS: an
 * address nothing points at any more, kept alive so an old bookmark lands
 * somewhere real. Counting them as orphans would make the assertion below
 * permanently red for doing its job correctly.
 *
 * Detected by reading the element out of `App.tsx` rather than by listing them,
 * so adding a redirect does not also mean remembering to exclude it here.
 */
function redirectRoutes(): string[] {
  const source = readFileSync(APP_TSX, "utf8");
  const out: string[] = [];
  for (const [, path, element] of source.matchAll(
    /<Route\s+path="([^"]+)"\s+element=\{<(\w+)/g,
  )) {
    if (/Navigate|Redirect/.test(element)) out.push(path);
  }
  return out;
}

/** Routes reachable only by typing, as measured in Part 1. Roman rules on these. */
const KNOWN_ORPHANS = [
  "/claims",
  "/contradictions",
  "/damages",
  "/decisions",
  "/graph",
  "/hearings",
  "/queries",
  "/search",
];

/**
 * Every route a builder or a literal link points at.
 *
 * Two sources, because the app has two kinds of navigation: builders in
 * `routePaths.ts` (which the table above already enumerates) and `<Link to="…">`
 * / `navigate("…")` literals still scattered through components — the 30 call
 * sites Part 1 measured. Reading both is what makes "unreachable" mean
 * unreachable rather than "not converted yet".
 */
function linkedRoutes(): Set<string> {
  const linked = new Set<string>(BUILDER_ROUTES);

  // Literal navigation targets, swept out of the source tree. Only paths with
  // no `:param` are matched: a literal carrying a real id cannot be compared to
  // a route pattern without re-implementing the matcher, and every parameterised
  // address in the app already goes through a builder.
  const walk = (dir: string): string[] => {
    const out: string[] = [];
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const full = join(dir, entry.name);
      if (entry.isDirectory()) out.push(...walk(full));
      else if (/\.tsx?$/.test(entry.name) && !entry.name.includes(".test.")) out.push(full);
    }
    return out;
  };
  for (const file of walk(join(__dirname, "..", ".."))) {
    const source = readFileSync(file, "utf8");
    for (const [, path] of source.matchAll(/(?:to=|navigate\()"(\/[a-z0-9/-]*)"/g)) {
      linked.add(path);
    }
  }
  return linked;
}

describe("every declared route is reachable", () => {
  it("finds no orphan beyond the nine Part 1 measured", () => {
    const linked = linkedRoutes();
    const redirects = redirectRoutes();
    const orphans = realRoutes()
      .filter((route) => !route.includes(":"))
      .filter((route) => !linked.has(route))
      .filter((route) => !redirects.includes(route))
      .sort();

    // Printed every run, so the list in the report cannot go stale without
    // somebody seeing it. This is the assertion's OUTPUT, not decoration.
    // eslint-disable-next-line no-console
    console.log(`unreachable routes (${orphans.length}): ${orphans.join(" ") || "none"}`);

    const surprises = orphans.filter((route) => !KNOWN_ORPHANS.includes(route));
    expect(
      surprises,
      "a route with no way into it — either link it, or remove it, or add it to KNOWN_ORPHANS with a reason",
    ).toEqual([]);
  });

  it("the sweep actually reads the tree, so an empty scan cannot pass", () => {
    // The anti-vacuity backstop, same shape as the one guarding `declaredRoutes`
    // above: a regex that stopped matching would yield an empty `linked` set,
    // every route would look orphaned, and... the assertion would fail loudly.
    // The dangerous direction is the other one — a sweep that matched EVERYTHING
    // would report zero orphans and check nothing. So pin a path that is
    // genuinely linked and one that genuinely is not.
    const linked = linkedRoutes();
    expect(linked.has("/documents"), "the nav bar links /documents").toBe(true);
    expect(linked.has("/graph"), "/graph is one of the orphans").toBe(false);
    // And the redirect detector must actually detect: an empty result here
    // would silently re-classify every redirect stub as an orphan, which is the
    // failure that would push somebody to widen KNOWN_ORPHANS instead.
    expect(redirectRoutes()).toContain("/settings");
    expect(redirectRoutes()).toContain("/pipeline");
    // And every builder still exists and still emits a path.
    for (const emit of BUILDER_CALLS) {
      expect(emit().startsWith("/"), "a builder must emit an absolute path").toBe(true);
    }
    expect(BUILDER_CALLS.length).toBe(BUILDER_ROUTES.length + 1);
  });
});

// -----------------------------------------------------------------------------
// The one-release redirects — REMOVED IN v2.1
// -----------------------------------------------------------------------------
//
// Each address below was real in v2.0 and is not real in v2.1. The tests pin
// two things: that the old address is still DECLARED (so a bookmark does not
// 404), and that what it redirects to is a route that exists. Deleting a
// redirect without deleting its test is a red build — which is what makes the
// v2.1 sweep a thing somebody has to do deliberately.

describe("the one-release redirects (removed in v2.1)", () => {
  const REDIRECTS: Array<{ from: string; toRoute: string; why: string }> = [
    { from: "/settings", toRoute: "/admin/settings", why: "Settings moved under Admin" },
    {
      from: "/cases/:slug/proof-review",
      toRoute: "/cases/:slug/proof-matrix",
      why: "Proof Review became a tab on the matrix",
    },
    { from: "/explorer", toRoute: "/cases/:slug/proof-matrix", why: "Evidence removed" },
    { from: "/evidence", toRoute: "/cases/:slug/proof-matrix", why: "Evidence removed" },
    { from: "/bias-explorer", toRoute: "/", why: "the Bias page removed; nothing replaces it" },
    { from: "/pipeline", toRoute: "/documents", why: "predates this task, dated with the rest" },
    { from: "/pipeline/:id", toRoute: "/documents", why: "predates this task, dated with the rest" },
  ];

  it.each(REDIRECTS)("$from still resolves ($why)", ({ from }) => {
    expect(
      declaredRoutes(),
      `${from} must stay declared for one release or the bookmark 404s`,
    ).toContain(from);
  });

  it.each(REDIRECTS)("$from points at a real route ($toRoute)", ({ toRoute }) => {
    expect(realRoutes(), `${toRoute} is the destination and must exist`).toContain(toRoute);
  });

  it("the removed pages' routes are GONE, not merely unlinked", () => {
    // The removals this task made. Asserted as absent so a revert that restored
    // the route without restoring the page — or vice versa — is caught.
    const routes = declaredRoutes();
    for (const removed of ["/cases/:slug/proof-review-old", "/analysis"]) {
      expect(routes).not.toContain(removed);
    }
    // `/explorer` and `/bias-explorer` are still DECLARED (as redirects) and
    // that is deliberate — the assertion above pins it. What must be gone is the
    // page behind them, which the import graph proves and `npm run build` would
    // fail on.
  });

  it("every redirect carries its removal date in a comment", () => {
    // A dated redirect is one somebody can sweep. An undated one becomes
    // permanent by default — which is what the two /pipeline redirects were
    // until this task dated them.
    const source = readFileSync(APP_TSX, "utf8");
    const marker = "REMOVED IN v2.1";
    const dated = source.split(marker).length - 1;
    expect(
      dated,
      "each redirect block in App.tsx must carry the REMOVED IN v2.1 marker",
    ).toBeGreaterThanOrEqual(5);
  });
});
