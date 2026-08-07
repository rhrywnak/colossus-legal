# CC_REPORT_REHEARSAL_LINK_FIX — dead link, dead-looking buttons, and the guard

**Date:** 2026-08-06 · **Branch:** `fix/rehearsal-scenario-link` off `main` (`be9c819`)
**Instruction:** `CC_FIX_REHEARSAL_SCENARIO_LINK_v1` + `CC_FIX_REHEARSAL_LINK_RULINGS_v1`
**Verdict:** complete, committed, STOPPED — no build, no merge.

---

## 1. The two questions the instruction asked to be answered by measurement

### Did the breadcrumb share the defect?

**Yes — byte-identically.** Both controls in `RehearsalPageHeader.tsx` composed the
same wrong string:

| Line | Control | Emitted |
|---|---|---|
| 79 | breadcrumb scenario link | `/cases/${slug}/scenarios/${scenario.scenario_id}` |
| 102 | "Scenario page ↗" | `/cases/${slug}/scenarios/${scenario.scenario_id}` |

No `/cases/:slug/scenarios/*` route has ever been declared in `App.tsx`. Both
landed on the catch-all 404.

The compounding detail worth recording: that file's own header comment cites
**ruling 7 of the signed set** — two ways out, on purpose, because "the browser's
Back button must never be the only way out of a surface a witness is reading from
under stress." Both of those ways out were broken, in the same way, by the same
string. The redundancy the ruling bought protected nothing, because the failure
was correlated. Two copies of one contract is not redundancy.

### How many other composed route strings did the guard catch on its first run?

**One — and it was one my own survey had missed**, which is the more useful half
of the answer.

The manual survey grep matched `to={\`…\`}` and `navigate(\`…\`)`. It found 23
hand-composed route strings and reported 7 in the trial-prep/rehearsal family.
That number was wrong. The family-scan test found an **8th**:

```
frontend/src/pages/ScenarioDetailPage.tsx:117
const backCrumb = { label: "Trial Prep", to: `/cases/${slug}/trial-prep` };
```

An object-literal `to:` inside a crumb descriptor — not a JSX prop, so invisible
to a grep written around JSX props. It was pointing at a *correct* route, so it
was never going to 404; it was simply outside the contract, exactly the state the
two dead links were in the day before they broke.

**The measured value of the test on its first run: one call site that a careful
human survey, run specifically to find these, did not see.** That is the number
the instruction asked for. It also failed correctly on the reintroduced defect
(§3 below) and on nothing else.

The other 16 sites (documents, people, allegations, admin, timeline) are **filed
and remain UNGUARDED** per Q1's ruling. Stated plainly rather than left implied.

---

## 2. What was built

### The builder — `frontend/src/utils/routePaths.ts` (new)

Four functions: `trialPrepPath`, `scenarioPagePath`, `rehearsalPath`,
`rehearsalScenarioPath`. Every segment `encodeURIComponent`-ed, which ends a
measured inconsistency: `ScenarioHeaderTiers` escaped its slug and
`RehearsalPageHeader` did not — two files disagreeing about the same value.
A caller cannot now forget what it has no opportunity to do.

Scope boundary documented in-file: **in-app route paths only, never API paths.**
API paths carry `API_BASE_URL` and answer to the axum router; mixing the families
would leave one guard test checking its outputs against two unrelated inventories.

### The guard — `frontend/src/utils/__tests__/routePaths.test.ts` (new, 13 tests)

Parses every `<Route path="…">` out of `App.tsx` **from disk**, compiles each to a
parameterized matcher (`:param` → `[^/]+`, not `.*`), and asserts every builder
output lands on the route it claims — named, not merely non-null, so a builder
that drifted onto a *different* real route still fails.

Named in-file as the **route-side sibling of the .377 URL-guard law**, whose API
halves are `backend/src/api/scenario_accusation_tests.rs` and
`services/__tests__/scenarioAccusation.test.ts`. The precedent now reads as one
family: .377 guards fetch paths, .382 guards navigation paths.

### The call-site guard — `frontend/src/pages/__tests__/rehearsalAddress.test.ts` (new, 14 tests)

The guard above proves what the builders *emit*; it has no view of which screens
*use* them. This is the other half: a scan of all five converted files for
interpolated route literals, plus pins on the disabled style and the unified
mover. It carries a list-integrity test so a rename cannot empty the list and
leave it green.

### The disabled nav

**Correction to the instruction's diagnosis, and it made the fix smaller.** The
instruction reported the buttons "render enabled with nowhere to go." In fact
`disabled={atFirst}` / `disabled={atLast}` were already passed and the arithmetic
was already right — the buttons were genuinely inert. What was missing was the
*visual*: `navButtonStyle` had no disabled branch, so `cursor: pointer` and live
text survived. Inert-but-alive-looking, which is worse than either, because it
reads as broken rather than as unavailable.

`navButtonDisabledStyle` dims to the existing `--text-disabled` token and sets
`cursor: not-allowed`, keeping border/padding/font identical so only the aliveness
changes. No new wording row: a disabled control is its own disclosure.

### Q3, folded in

Keyboard and buttons now move through one `move` callback, so the URL updates
either way. Previously arrow-keying to S-3 produced a linkable page and clicking
Next to S-3 did not — the same position, two degrees of linkable, decided by input
method.

One thing changed beyond a straight lift: the old code called `navigate()` inside
a `setIndex(current => …)` updater. React may invoke an updater more than once, so
a side effect there can fire twice. `move` reads `index` from its own render
instead. The cost — the callback and its keydown listener rebuild when the index
moves — is the correct-by-construction trade, and the effect's cleanup tears down
the old listener first. Both gate agents examined this specifically and endorsed it.

---

## 3. Proof the guard can fail

A guard that cannot fail reports safety it never checked. The .382 defect was
reintroduced into `scenarioPagePath` and the suite re-run:

```
Tests  3 failed | 10 passed (13)
  → /cases/awad-v-cfs/scenarios/… matches no declared route — it would 404
  → /cases/awad%20v%20cfs/scenarios/id%2Fwith%2Fslashes … (escaped form too)
  → scenarioPagePath composes the path that 404'd on .382
```

Reverted; green again. Three further tests (`the guard can fail`) close the
vacuity routes named in the pre-coding analysis: a broken App.tsx parse (asserts
≥20 routes and three known literals), the catch-all `*` making everything match
(asserted both present and excluded), and `:param` being `.*` rather than one
segment.

---

## 4. Verification

| Command | Result |
|---|---|
| `npx vitest run` | **818 passed**, 61 files, 0 failed — the 791 that existed on `main`, plus 27 new here (13 in the route guard, 14 in the call-site/style pins). No existing test was modified. |
| `npm run typecheck` | clean |
| `npm run build` | ✓ built in 1.86s |
| `cargo test --lib` | **1675 passed**, 0 failed, 2 ignored |
| `cargo check --bins` | clean |
| `cargo clippy --lib --profile test` | clean |
| `cargo fmt --check` | clean |

`npm run lint` is **not** in this list because no `lint` script and no eslint
config exist in this repo. Flagged rather than silently skipped; the instruction's
own list correctly omits it.

Backend expected untouched, and the suite run is the proof — no `.rs` file appears
in the diff.

### Four-agent §11 gate

| Agent | Verdict |
|---|---|
| `rules-enforcer` | **PASS** |
| `architecture-reviewer` | **PASS** |
| `test-auditor` | **FAIL → fixed → PASS** |
| `observability-checker` | **PASS** |

`test-auditor`'s FAIL was correct and worth the round trip: the hand-route check
read only `RehearsalPage.tsx`, leaving `RehearsalPageHeader.tsx` — *the file both
dead links lived in* — unpinned. Fixing it by scanning the whole family rather
than adding one file is what surfaced the 8th call site in §1.

---

## 5. Files

| File | Change |
|---|---|
| `frontend/src/utils/routePaths.ts` | **new** — the builder |
| `frontend/src/utils/__tests__/routePaths.test.ts` | **new** — the route-side URL guard |
| `frontend/src/pages/__tests__/rehearsalAddress.test.ts` | **new** — call-site + disabled-state pins |
| `frontend/src/components/RehearsalPageHeader.tsx` | both dead links re-pointed; crumb through builder; disabled style at both bounds |
| `frontend/src/components/rehearsalStyles.ts` | `navButtonDisabledStyle` |
| `frontend/src/pages/RehearsalPage.tsx` | `move` callback; buttons and keys unified; builder |
| `frontend/src/pages/ScenarioDetailPage.tsx` | delete-redirect **and** back-crumb through builder |
| `frontend/src/components/ScenarioHeaderTiers.tsx` | rehearsal link through builder |
| `frontend/src/components/TrialPrepViews.tsx` | scenario card link through builder |

Zero interpolated `/cases/` route literals remain anywhere in `frontend/src`.

---

## 6. Observed during implementation, not in the analysis

1. **`backend/Cargo.lock` drifts on any cargo invocation.** HEAD's lock says
   `2.0.0-beta.381` while `Cargo.toml` says `.382` — the `.382` bump (`fb727f0`)
   did not reconcile the lock the way the `.381` bump did (`be9c819`). Every
   `cargo check` re-dirties it. **Restored, not committed** — version files are
   Roman's. It will keep reappearing until reconciled on `main`.

2. **The survey grep was wrong and the test was right.** Recorded because the
   lesson generalises past this task: a hand survey of call sites finds the shape
   you thought of. When the 16 filed sites are converted, the family-scan test
   should be extended file-by-file and allowed to find them, rather than a grep
   being trusted to enumerate them first.

3. **Admin dead links left exactly as found**, per Q2 — `AdminAudit.tsx:93` and
   `AdminDocuments.tsx:289` both navigate to `/admin/documents/:id/audit`, which
   `App.tsx` does not declare and `Admin.tsx` does not nest. Already owned by task
   2.14's deletions.

4. **No tech debt added.** No deferred items, no TODOs, no carve-outs taken.
