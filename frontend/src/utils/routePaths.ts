// =============================================================================
// routePaths.ts — every in-app address, composed in exactly one place
// =============================================================================
//
// ## Why this file exists (the .382 defect, measured)
//
// The rehearsal page linked to `/cases/:slug/scenarios/:id` from two controls —
// the header's "Scenario page ↗" and the breadcrumb. That route has NEVER been
// declared in `App.tsx`. Both landed on the catch-all 404. The scenario page's
// real address is `/cases/:slug/trial-prep/:scenarioId`, and the working view
// had been spelling it correctly all along, in a different file.
//
// That is the whole failure mode: a URL is a CONTRACT between a screen that
// composes it and a router that declares it, and a template literal buried in
// JSX signs only one half of that contract. Two files spelled the same idea two
// ways, and nothing in the build, the type checker, or the test suite could
// notice — the string was well-typed and syntactically perfect. It just pointed
// nowhere.
//
// So: one builder, and a test that pins what it emits against what `App.tsx`
// declares (`__tests__/routePaths.test.ts`). A link to an undeclared route is a
// FAILING TEST now, not a 404 in front of a witness.
//
// ## The precedent this belongs to
//
// This is the ROUTE-side sibling of the post-.377 URL-guard law. The .377 build
// shipped a client calling an API path the axum router did not serve — same
// class, different layer. That guard has two halves that must be edited to agree
// (`backend/src/api/scenario_accusation_tests.rs` and
// `frontend/src/services/__tests__/scenarioAccusation.test.ts`). This file is
// the third member of the family: .377 guarded fetch paths, this guards
// navigation paths.
//
// ## What belongs here, and what emphatically does not
//
// IN:  in-app route paths — what you hand to `<Link to>` or `navigate()`.
// OUT: API paths (`/api/cases/:slug/rehearsal`). Those live with the service
//      that calls them, they carry `API_BASE_URL`, and they answer to the
//      backend router rather than to `App.tsx`. Mixing the two families in one
//      module would mean one guard test trying to check its outputs against two
//      unrelated route inventories.
//
// ## Scope today, honestly stated
//
// This builder covers the trial-prep / rehearsal family — the seven call sites
// in that feature. A survey found 23 hand-composed route strings across the
// frontend; the other 16 (documents, people, allegations, admin, timeline) are
// NOT routed through here yet and are therefore NOT guarded. That is a filed
// follow-on conversion task, not an oversight. This module is the pattern they
// convert into.
//
// ## TS note: template literals and `encodeURIComponent`
//
// Every segment is escaped on the way in. This is not decoration — a slug or an
// id containing `/` would otherwise silently become an EXTRA PATH SEGMENT, and
// the resulting URL would match a different route (or none) while looking
// entirely reasonable in the source. Before this file, `ScenarioHeaderTiers`
// escaped its slug and `RehearsalPageHeader` did not; the two disagreed about
// the same value. Escaping in the builder ends that by construction — a caller
// cannot forget what it has no opportunity to do.

/**
 * The trial-prep dashboard: every scenario for a case.
 *
 * Declared in `App.tsx` as `/cases/:slug/trial-prep`.
 *
 * @param slug the case slug, escaped here — pass it raw
 */
export function trialPrepPath(slug: string): string {
  return `/cases/${encodeURIComponent(slug)}/trial-prep`;
}

/**
 * One scenario's working page — where it is built and curated.
 *
 * Declared in `App.tsx` as `/cases/:slug/trial-prep/:scenarioId`.
 *
 * ## Why the name says "scenario page" and the path says "trial-prep"
 *
 * The path is historical; the name is what humans call the screen ("Scenario
 * page ↗" is the control's own label). Naming the FUNCTION after the route would
 * push every caller to think in URL structure, which is precisely the thinking
 * that produced the .382 defect — a developer reasoning "scenarios live under
 * /scenarios" and composing a path that felt right. Callers name the
 * DESTINATION; this file owns the spelling.
 *
 * @param slug the case slug, escaped here
 * @param scenarioId the scenario's UUID, escaped here
 */
export function scenarioPagePath(slug: string, scenarioId: string): string {
  return `/cases/${encodeURIComponent(slug)}/trial-prep/${encodeURIComponent(scenarioId)}`;
}

/**
 * Rehearsal mode for a case, with no scenario named.
 *
 * Declared in `App.tsx` as `/cases/:slug/rehearsal`. The page opens on the first
 * READY scenario.
 *
 * @param slug the case slug, escaped here
 */
export function rehearsalPath(slug: string): string {
  return `/cases/${encodeURIComponent(slug)}/rehearsal`;
}

/**
 * Rehearsal mode, positioned on one scenario.
 *
 * Declared in `App.tsx` as `/cases/:slug/rehearsal/:code`.
 *
 * ## Domain note: the code, not the id
 *
 * This address takes the scenario's CODE (`S-1`) rather than its UUID, and that
 * is deliberate rather than accidental: it is an address a human reads aloud and
 * types during trial prep. A code nobody declared ready gets the stored
 * not-ready sentence — never a 404 — because the address is legitimate and the
 * scenario simply is not ready. Two different states, two different observables.
 *
 * @param slug the case slug, escaped here
 * @param code the scenario code, escaped here
 */
export function rehearsalScenarioPath(slug: string, code: string): string {
  return `/cases/${encodeURIComponent(slug)}/rehearsal/${encodeURIComponent(code)}`;
}

/**
 * Marie's practice drill for one scenario.
 *
 * Declared in `App.tsx` as `/cases/:slug/trial-prep/practice/:scenarioId`.
 *
 * ## Why the address is case-scoped when the task wrote it shorter
 *
 * CC_TASK_PRACTICE_SESSION_V0_v1 names the route `/trial-prep/practice/:scenarioId`.
 * Every other trial-prep address in this app carries the case (`/cases/:slug/…`),
 * and for a reason the .382 defect made concrete: the page it is reached FROM
 * knows the slug, the backend fences every scenario read by it, and a bare
 * `/trial-prep/...` would be the one address in the family that could not say
 * which case it belonged to. So the task's path is preserved verbatim as the
 * tail, under the prefix the rest of the family uses.
 *
 * ## Domain note: the UUID, not the code
 *
 * Unlike `rehearsalScenarioPath`, this takes the id. The drill is reached by
 * CLICKING from the scenario page rather than by being typed or read aloud, and
 * the page it lands on addresses the backend by id.
 *
 * @param slug the case slug, escaped here
 * @param scenarioId the scenario's UUID, escaped here
 */
export function practicePath(slug: string, scenarioId: string): string {
  return (
    `/cases/${encodeURIComponent(slug)}/trial-prep/practice/` +
    `${encodeURIComponent(scenarioId)}`
  );
}
