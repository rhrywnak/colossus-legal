// =============================================================================
// routePaths.test.ts — THE ROUTE-SIDE URL GUARD (the .377 law, third member)
// =============================================================================
//
// ## The law
//
// A URL is a contract between the screen that composes it and the router that
// declares it. Both halves must be edited to agree, or the mismatch is a
// FAILING TEST — never a 404 discovered live.
//
//   .377 (API paths, server half)  → backend/src/api/scenario_accusation_tests.rs
//   .377 (API paths, client half)  → services/__tests__/scenarioAccusation.test.ts
//   .382 (route paths, this file)  → every path routePaths.ts emits
//
// The .382 defect this closes: the rehearsal header linked to
// `/cases/:slug/scenarios/:id` from TWO controls. No such route has ever been
// declared. Both 404'd. Nothing could catch it — the string was well-typed and
// syntactically perfect, it simply pointed nowhere.
//
// ## How it reads App.tsx, and why from disk
//
// The route table is parsed out of the SOURCE rather than imported. Importing
// `App.tsx` would drag in the entire component tree (providers, pages, the
// router itself) into a node-environment test with no DOM — and, more to the
// point, it would be asking the thing under test to describe itself. The same
// reasoning is written out in the backend half of this law, which hand-writes
// its inventory because axum exposes none. Reading the declaration text is the
// closest a frontend test gets to an independent witness.
//
// The house already does this: `pages/__tests__/scenarioPageStructure.test.ts`
// and `styles/__tests__/visualLanguage.test.ts` both read source from disk.
//
// ## Why this test is written to be able to FAIL
//
// A guard that cannot fail is worse than no guard: it reports safety it never
// checked. Three ways this one could have passed vacuously, and the assertion
// that closes each, are the three tests in `the guard can fail` below. They are
// tests OF the guard, and they are the reason to trust the guard's verdict.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import {
  rehearsalPath,
  rehearsalScenarioPath,
  scenarioPagePath,
  trialPrepPath,
} from "../routePaths";

const APP_TSX = join(__dirname, "..", "..", "App.tsx");

/** React Router's catch-all. It matches everything, so it can guard nothing. */
const CATCH_ALL = "*";

/**
 * Every `path="…"` declared on a `<Route>` in App.tsx.
 *
 * A deliberately dumb regex over the source text. It cannot understand nested
 * routers or computed paths — and if either is ever introduced, the inventory
 * assertions below go red rather than quietly under-reporting, which is the
 * behaviour worth having from a parser this simple.
 */
function declaredRoutes(): string[] {
  const source = readFileSync(APP_TSX, "utf8");
  const matches = source.matchAll(/<Route\s+path="([^"]+)"/g);
  return [...matches].map((m) => m[1]);
}

/**
 * Compile one route pattern into a matcher.
 *
 * `:param` becomes "one path segment, at least one character" — NOT `.*`. That
 * distinction is the whole point of a parameterized match: `[^/]+` refuses to
 * let an unescaped `/` inside a value slide across a segment boundary and land
 * on a route the caller never intended.
 *
 * Every other character is escaped, so a literal `.` in a future route cannot
 * quietly become a wildcard.
 */
function matcherFor(pattern: string): RegExp {
  const body = pattern
    .split("/")
    .map((segment) => (segment.startsWith(":") ? "[^/]+" : segment.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")))
    .join("/");
  return new RegExp(`^${body}$`);
}

/** The routes a link may legitimately point at: everything but the catch-all. */
function realRoutes(): string[] {
  return declaredRoutes().filter((p) => p !== CATCH_ALL);
}

/** Which declared route a path lands on, or `null` for the 404 page. */
function routeFor(path: string): string | null {
  return realRoutes().find((pattern) => matcherFor(pattern).test(path)) ?? null;
}

// -----------------------------------------------------------------------------
// The guard itself
// -----------------------------------------------------------------------------

/**
 * Every builder, with the route it CLAIMS and arguments that exercise escaping.
 *
 * The awkward values are the interesting ones. A slug with a space proves the
 * encoded form still matches; a value containing `/` proves the builder does not
 * hand the router an extra path segment — the failure that would look perfectly
 * reasonable in the source and silently match a different route.
 */
const BUILDERS: Array<{ name: string; route: string; emit: () => string }> = [
  { name: "trialPrepPath", route: "/cases/:slug/trial-prep", emit: () => trialPrepPath("awad-v-cfs") },
  { name: "trialPrepPath (slug needs escaping)", route: "/cases/:slug/trial-prep", emit: () => trialPrepPath("awad v cfs/2") },
  {
    name: "scenarioPagePath",
    route: "/cases/:slug/trial-prep/:scenarioId",
    emit: () => scenarioPagePath("awad-v-cfs", "3f2b1c9e-0000-4a1b-8c7d-000000000001"),
  },
  {
    name: "scenarioPagePath (id needs escaping)",
    route: "/cases/:slug/trial-prep/:scenarioId",
    emit: () => scenarioPagePath("awad v cfs", "id/with/slashes"),
  },
  { name: "rehearsalPath", route: "/cases/:slug/rehearsal", emit: () => rehearsalPath("awad-v-cfs") },
  { name: "rehearsalPath (slug needs escaping)", route: "/cases/:slug/rehearsal", emit: () => rehearsalPath("awad v cfs/2") },
  {
    name: "rehearsalScenarioPath",
    route: "/cases/:slug/rehearsal/:code",
    emit: () => rehearsalScenarioPath("awad-v-cfs", "S-1"),
  },
  {
    name: "rehearsalScenarioPath (code needs escaping)",
    route: "/cases/:slug/rehearsal/:code",
    emit: () => rehearsalScenarioPath("awad v cfs", "S/1 draft"),
  },
];

describe("the route-side URL guard", () => {
  it.each(BUILDERS)("$name emits a path App.tsx actually declares", ({ route, emit }) => {
    const path = emit();
    // Naming the landed route rather than merely asserting non-null: a builder
    // that drifted onto a DIFFERENT real route would pass a null check while
    // sending the reader somewhere nobody asked for.
    expect(routeFor(path), `${path} matches no declared route — it would 404`).toBe(route);
  });

  it("emits no path under /scenarios/ — the .382 defect, by name", () => {
    // The two dead links were `/cases/:slug/scenarios/:id`, spelled identically
    // in the breadcrumb and in the header control. Pinned so the plausible-
    // looking spelling cannot come back.
    for (const { name, emit } of BUILDERS) {
      expect(emit(), `${name} composes the path that 404'd on .382`).not.toContain("/scenarios/");
    }
  });
});

// -----------------------------------------------------------------------------
// Tests OF the guard — the three ways it could have passed while checking nothing
// -----------------------------------------------------------------------------

describe("the guard can fail", () => {
  it("parsed a real route inventory, so a broken parse cannot pass silently", () => {
    const routes = declaredRoutes();
    // A regex that stopped matching (App.tsx reformatted, `<Route>` renamed)
    // would yield an empty list, every `find` would return undefined, and the
    // guard above would report... nothing to contradict. This is the assertion
    // that turns that into a red test.
    expect(routes.length).toBeGreaterThanOrEqual(20);
    expect(routes).toContain("/cases/:slug/trial-prep/:scenarioId");
    expect(routes).toContain("/cases/:slug/rehearsal/:code");
    expect(routes).toContain("/documents/:id");
  });

  it("excludes the catch-all, which exists to be excluded", () => {
    // `*` matches every string. Left in the match set, it would make the guard
    // above pass for literally any path, including the .382 defect. Asserting
    // it is BOTH present and excluded means removing it from App.tsx fails here
    // rather than quietly weakening the guard.
    expect(declaredRoutes()).toContain(CATCH_ALL);
    expect(realRoutes()).not.toContain(CATCH_ALL);
  });

  it("rejects an undeclared path — proven on the defect itself", () => {
    // The guard's own smoke test: feed it the exact string that shipped in
    // .382 and confirm it comes back unmatched. Without this, "everything
    // matched" and "the matcher matches everything" look identical.
    expect(routeFor("/cases/awad-v-cfs/scenarios/3f2b1c9e-0000-4a1b-8c7d-000000000001")).toBeNull();
    expect(routeFor("/cases/awad-v-cfs/not-a-page")).toBeNull();
  });

  it("keeps a parameter inside one segment", () => {
    // `:param` → `[^/]+`, not `.*`. If it were `.*`, an unescaped id containing
    // slashes would match its own route and the escaping tests above would
    // prove nothing at all.
    expect(matcherFor("/cases/:slug/trial-prep").test("/cases/a/b/trial-prep")).toBe(false);
    expect(matcherFor("/cases/:slug/trial-prep").test("/cases/a/trial-prep")).toBe(true);
    // An empty segment is not a parameter value.
    expect(matcherFor("/cases/:slug/trial-prep").test("/cases//trial-prep")).toBe(false);
  });
});
