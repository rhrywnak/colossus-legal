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

/**
 * Chuck's review sheets for one scenario — the print view.
 *
 * Declared in `App.tsx` as
 * `/cases/:slug/trial-prep/practice/:scenarioId/print`.
 *
 * ## Why the sheets have an address at all
 *
 * Because Chuck opens them in a second tab, looks, and only then prints — and
 * because a print STYLESHEET on the practice page cannot render the whole deck:
 * that page's list is conditionally rendered behind a fold and filtered by the
 * *Who's asking?* selector, so what is in its DOM is not what belongs on paper.
 * An address is also what lets the view be bookmarked and reached from a second
 * monitor, which is why it carries its own way back.
 *
 * @param slug the case slug, escaped here
 * @param scenarioId the scenario's UUID, escaped here
 */
export function practicePrintPath(slug: string, scenarioId: string): string {
  return `${practicePath(slug, scenarioId)}/print`;
}

/**
 * Chuck's printed ANSWERS — the same sheets, carrying what Marie wrote.
 *
 * Declared in `App.tsx` as
 * `/cases/:slug/trial-prep/practice/:scenarioId/print-answers`.
 *
 * A sibling address rather than a query parameter on `/print`: they are two
 * documents for two different acts — one is marked up, one is read — and Chuck
 * keeps both tabs open. A `?answers=1` would make them one page that sometimes
 * shows something else, which is exactly the shape whose Back button lies.
 */
export function practiceAnswersPath(slug: string, scenarioId: string): string {
  return `${practicePath(slug, scenarioId)}/print-answers`;
}

/**
 * ONE question — where Marie writes and Chuck reads.
 *
 * Declared in `App.tsx` as
 * `/cases/:slug/trial-prep/practice/:scenarioId/question/:questionId`.
 *
 * ## The same path, a different page
 *
 * This address held the retired REVIEW page — one question with every attempt
 * stacked and Chuck's notes on each. That page went with the notes ruling of
 * 2026-08-23. The address returns because the thing at it is what a person
 * always meant by "open this question": her current answer, editable, with the
 * earlier ones collapsed underneath.
 *
 * `question` is a literal segment for the reason `print-answers` is one: routes
 * under this prefix must be told apart by a WORD, not by which the matcher
 * happens to try first.
 */
export function practiceQuestionPath(
  slug: string,
  scenarioId: string,
  questionId: string,
): string {
  return `${practicePath(slug, scenarioId)}/question/${encodeURIComponent(questionId)}`;
}

/**
 * The practice walk, for one side.
 *
 * Declared as `/cases/:slug/trial-prep/practice/:scenarioId/walk`, with the side
 * in the query string.
 *
 * ## Why the side is a QUERY and not a segment
 *
 * It is a filter on one page, not a different page: reloading a walk should land
 * on the same walk, and switching sides is starting over rather than navigating
 * somewhere new. A segment would make two addresses for one screen.
 */
export function practiceWalkPath(
  slug: string,
  scenarioId: string,
  side: "george" | "chuck",
): string {
  return `${practicePath(slug, scenarioId)}/walk?side=${side}`;
}

/**
 * One SITTING of the practice drill — the address a reload lands back on.
 *
 * Declared in `App.tsx` as
 * `/cases/:slug/trial-prep/practice/:scenarioId/session/:sessionId`.
 *
 * ## Why a sitting has an address at all (Section B, item B10)
 *
 * Because the browser's Back button and the reload key exist. In .401 the four
 * screens were four states of one component at one address: Roman answered
 * question 1, left the page, came back — and started at question 1 again with no
 * sign his answer had been kept. It HAD been kept; the screen simply had no way
 * to say so or to return him to it. An address is what makes "where she was" a
 * thing the browser can hold.
 *
 * ## Why `session` is a literal segment
 *
 * The same reason `practice` is one in the parent: it stops a scenario id and a
 * session id being told apart by position alone. `…/practice/:scenarioId` and
 * `…/practice/:scenarioId/session/:sessionId` differ by two segments, one of
 * which is a word no uuid can be — so neither can shadow the other, and the
 * guard test says so.
 *
 * @param slug the case slug, escaped here
 * @param scenarioId the scenario's UUID, escaped here
 * @param sessionId the sitting's UUID, escaped here
 */
export function practiceSessionPath(
  slug: string,
  scenarioId: string,
  sessionId: string,
): string {
  return (
    `/cases/${encodeURIComponent(slug)}/trial-prep/practice/` +
    `${encodeURIComponent(scenarioId)}/session/${encodeURIComponent(sessionId)}`
  );
}


// ─── The navigation bar's own addresses (nav cleanup, Part 2) ────────────────
//
// ## Why the BAR moved in here, and what it buys
//
// Part 1 of the nav cleanup measured 30 hand-composed navigation call sites
// across 16 targets that did not go through this file — and every path the nav
// task touches was among them. `Header.tsx` held `/explorer`, `/bias-explorer`,
// `/settings` and a template-literal `/cases/${slug}/proof-matrix` as JSX
// literals, which is exactly the half-signed contract the .382 defect was.
//
// Putting the nav table through these builders means the guard test below
// (`__tests__/routePaths.test.ts`) covers the whole bar for free: removing a
// route while a menu entry still points at it is a RED TEST, not a 404 in front
// of Chuck. That is the entire reason the removals in this task are safe to make
// in one sweep.
//
// The builders that take no argument still exist as functions rather than
// constants. It is deliberate: the guard enumerates BUILDERS and calls each one,
// so a path that is a bare exported string would need a second mechanism to be
// checked. One shape, one guard.

/** The dashboard. Declared in `App.tsx` as `/`. */
export function homePath(): string {
  return "/";
}

/** The document list. Declared as `/documents`. */
export function documentsPath(): string {
  return "/documents";
}

/**
 * One document's workspace — the tabbed page.
 *
 * Declared as `/documents/:id`. This is where `AdminAudit`'s per-document link
 * now goes: it used to compose `/admin/documents/:id/audit`, an address
 * `App.tsx` has never declared, so following it landed on the 404 page. Part 1
 * found it live; this builder is what stops it recurring.
 *
 * @param id the document's id, escaped here — pass it raw
 */
export function documentPath(id: string): string {
  return `/documents/${encodeURIComponent(id)}`;
}

/** The RAG chat. Declared as `/ask`. The bar calls it "Chat". */
export function askPath(): string {
  return "/ask";
}

/** Everyone in the case. Declared as `/people`. */
export function peoplePath(): string {
  return "/people";
}

/** The case timeline. Declared as `/timeline`. */
export function timelinePath(): string {
  return "/timeline";
}

/**
 * One chronology event, in full. Declared as `/timeline/events/:id`.
 *
 * A SECOND level and not a third: the list's phase filter lives in a query
 * parameter on `/timeline`, so expanding a phase is still the same page (design
 * R16), and this is the only place a reader goes deeper.
 */
export function timelineEventPath(id: string): string {
  return `/timeline/events/${encodeURIComponent(id)}`;
}

/**
 * One subset, alone in a window with no app chrome. Declared as
 * `/timeline/subsets/:id/popout`.
 *
 * ## ⚑ A ROUTE AND NOT A QUERY PARAMETER, AND WHY THIS EXISTS AT ALL
 *
 * The FALLBACK half of Pop out (design §11 item 5). Chrome and Edge give the
 * story a real always-on-top window through the Document Picture-in-Picture
 * API, which needs no address at all — the same React tree is portalled into
 * the new document. Safari and Firefox have no such API, so there the button
 * opens a plain `window.open` popup, and a popup needs a URL.
 *
 * That URL renders the window body and NOTHING else: no header, no nav, no
 * page padding. A `?chrome=none` query on `/timeline` would have been the
 * smaller diff and the wrong shape — the thing at this address is a different
 * document, not the timeline page in a mode, and every surface in this app that
 * a person can be looking at has its own address (the Admin note in `App.tsx`
 * is the same argument).
 *
 * Behind the same auth as everything else: it is an ordinary route in the
 * ordinary app bundle, reached same-origin with the same session cookie.
 */
export function subsetPopoutPath(id: string): string {
  return `/timeline/subsets/${encodeURIComponent(id)}/popout`;
}

/**
 * The allegation list.
 *
 * Declared as `/allegations`. Reached from `AllegationDetailPage`'s Back button
 * when the page was opened COLD (a bookmark, with no history to go back to).
 * That fallback used to be `/explorer`, which this task removes — so the
 * re-point had to happen first, and it points at the list the page belongs to
 * and already names as its own breadcrumb parent.
 */
export function allegationsPath(): string {
  return "/allegations";
}

/**
 * The proof matrix for one case.
 *
 * Declared as `/cases/:slug/proof-matrix`. **The address does not change in this
 * task** — Roman's ruling 8. Only its menu placement and breadcrumb move.
 *
 * @param slug the case slug, escaped here
 */
export function proofMatrixPath(slug: string): string {
  return `/cases/${encodeURIComponent(slug)}/proof-matrix`;
}

/**
 * The proof matrix, opened on its Proof Review TAB.
 *
 * ## Why a query parameter and not a route
 *
 * Proof Review stops being a page and becomes a tab on the matrix (design §2).
 * A tab is a state of one page, not a second address — but `/…/proof-review` is
 * a real bookmark Roman has, so it must land somewhere exact rather than on the
 * matrix's default tab. `?tab=review` is that: one address, one page, and a
 * redirect from the old route that arrives on the right half of it.
 *
 * The guard compares PATHS, so the query string is stripped before the
 * comparison — see the builder's entry in the test.
 *
 * @param slug the case slug, escaped here
 */
export function proofReviewTabPath(slug: string): string {
  return `${proofMatrixPath(slug)}?tab=review`;
}

/**
 * Case health — the connection rate that must not be easy to overlook.
 *
 * Declared as `/cases/:slug/case-health`. Address unchanged (ruling 8); it moves
 * from a top-level bar item into Trial Prep ▾.
 *
 * @param slug the case slug, escaped here
 */
export function caseHealthPath(slug: string): string {
  return `/cases/${encodeURIComponent(slug)}/case-health`;
}

// ─── Admin, which becomes five addresses instead of nine tabs ────────────────
//
// The admin surface was ONE route (`/admin`) with nine tabs held in component
// state, so no tab was addressable, bookmarkable, or reachable by Back. The five
// groups below are real routes. The tabs WITHIN a group stay component state,
// which is the right level: "which of the five admin areas" is a place, "which
// of five prompt tables" is a view of it.

/** Admin overview — today's Metrics tab. Declared as `/admin`. */
export function adminPath(): string {
  return "/admin";
}

/**
 * Prompt Management — Prompts · System Prompts · Profiles · Schemas · Models,
 * the five existing admin tables re-homed under one address, unchanged inside.
 *
 * Declared as `/admin/prompts`.
 */
export function adminPromptsPath(): string {
  return "/admin/prompts";
}

/** Data — the store status and Indexing. Declared as `/admin/data`. */
export function adminDataPath(): string {
  return "/admin/data";
}

/** Logs — Chats and Audit. Declared as `/admin/logs`. */
export function adminLogsPath(): string {
  return "/admin/logs";
}

/**
 * Settings, at its new address.
 *
 * Declared as `/admin/settings`. The page itself is UNCHANGED inside; only where
 * it hangs moved. `/settings` redirects here for one release (removed in v2.1).
 */
export function adminSettingsPath(): string {
  return "/admin/settings";
}
