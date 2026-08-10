=== CC REPORT — CC_TASK_SCENARIO_SURFACE_CODE_AUDIT_READONLY_v1 — 2026-08-10 ===

# Scenario-surface code audit — read-only, four sweeps

**Branch:** `main` · **Working tree:** clean · **Last commit:** `7527c4d chore: bump
version to v2.0.0-beta.389` — matches the DEV badge Roman walked. [measured:
`git branch --show-current`, `git log --oneline -1`, `git status --short` (empty)]

**Scope of the read.** Frontend: `pages/RehearsalPage.tsx`,
`pages/ScenarioDetailPage.tsx`, `pages/TrialPrepDashboardPage.tsx`,
`pages/rehearsalNav.ts`, `pages/trialPrepHelpers.ts`, `utils/routePaths.ts`,
`App.tsx`, and 18 components on those three surfaces. Backend:
`api/rehearsal.rs`, `api/scenarios.rs`, `services/scenario_readiness.rs`,
`services/rehearsal_assembly.rs`, `services/scenario_augmentation.rs`,
`services/scenario_card.rs`, `repositories/pipeline_repository/scan_runs.rs`,
`scan_run_projection.rs`, `scenario_store.rs`, `scenario_responses.rs`, plus the
`scenario_responses` migration. No writes, no queries, no scans, no builds.

---

## SWEEP 1 — routes, links, and IDs

### 1.1 The declared address space [measured]

`App.tsx` declares four routes in this family:

| Route | Line | Component | Param identity |
|---|---|---|---|
| `/cases/:slug/rehearsal` | App.tsx:79 | `RehearsalPage` | case slug only |
| `/cases/:slug/rehearsal/:code` | App.tsx:83 | `RehearsalPage` | scenario **code** (`S-5`) |
| `/cases/:slug/trial-prep` | App.tsx:84 | `TrialPrepDashboardPage` | case slug only |
| `/cases/:slug/trial-prep/:scenarioId` | App.tsx:85 | `ScenarioDetailPage` | scenario **UUID** |

Two different scenario identities are in play — code on rehearsal, UUID on the
working page. Both are legitimate (`routePaths.ts:106-124` states the reason for
the code), but every hand-off between the two surfaces has to translate, and the
translation is where the headline defect lives.

### 1.2 Every navigation edge on the three surfaces [measured]

Eight non-test call sites of the guarded builders. This is the complete list —
`grep` over `frontend/src` for `rehearsalPath(|rehearsalScenarioPath|scenarioPagePath|trialPrepPath`
returns exactly these eight outside `__tests__`.

| # | Site | Emits | Identity carried | Receiver resolves with | On failure |
|---|---|---|---|---|---|
| 1 | `ScenarioHeaderTiers.tsx:227` "Rehearsal view →" | `rehearsalPath(slug)` | **NONE** | — (no code in URL) | **falls through to `payload.scenarios[0]`** |
| 2 | `RehearsalPageHeader.tsx:81` breadcrumb "Trial Prep" | `trialPrepPath(slug)` | case | dashboard read | banner |
| 3 | `RehearsalPageHeader.tsx:87` breadcrumb scenario | `scenarioPagePath(slug, scenario.scenario_id)` | UUID **of the on-screen scenario** | `useParams.scenarioId` → `getScenarioDetailLive` | `null` → "Scenario not found." |
| 4 | `RehearsalPageHeader.tsx:110` "Scenario page ↗" | `scenarioPagePath(slug, scenario.scenario_id)` | UUID **of the on-screen scenario** | same | same |
| 5 | `ScenarioCard.tsx:67` dashboard card | `scenarioPagePath(slug, scenario.id)` | UUID | same | same |
| 6 | `ScenarioDetailPage.tsx:122` breadcrumb back | `trialPrepPath(slug)` | case | dashboard read | banner |
| 7 | `ScenarioDetailPage.tsx:245` post-delete | `trialPrepPath(slug)` | case | dashboard read | banner |
| 8 | `RehearsalPage.tsx:172` ‹Back / Next› + arrow keys | `rehearsalScenarioPath(slug, moved.code)` | code | `useParams.code` → `findIndex` | see 1.3 |

**The finding that matters most:** `rehearsalScenarioPath` — the per-scenario
rehearsal address, declared in `App.tsx:83`, built in `routePaths.ts:122`, guarded
by `utils/__tests__/routePaths.test.ts:128-136`, and documented as the address
"a human reads aloud and types during trial prep" — has **exactly one producer in
the entire application, and it is inside the rehearsal page itself** (row 8).
No entry point anywhere composes one. [measured: the grep above returns no other
call site.] The address exists, is tested, and has no caller.

Correspondingly, `rehearsalPath(slug)` — the identity-free form — is the **only**
way into rehearsal mode from anywhere in the app (row 1). `grep -rn "ehearsal"`
over `TrialPrepDashboardPage.tsx`, `TrialPrepViews.tsx`, `ScenarioCard.tsx`,
`Sidebar.tsx`, `Header.tsx` returns nothing: the dashboard has no rehearsal entry
at all, and the scenario page's header link is the sole door. [measured]

### 1.3 What the receiving end does with a missing / ineligible scenario [measured]

`RehearsalPage.tsx` loads `fetchRehearsal(slug)` — a case-level payload
(`services/rehearsal.ts`, `GET /api/cases/:slug/rehearsal`). The backend route
takes **no scenario parameter of any kind** (`api/rehearsal.rs:4`), and
`services/scenario_readiness.rs:229` filters the case's scenarios to
`is_ready(r)` before assembling. So the server never learns which scenario was
asked for and has no opportunity to refuse. Every selection decision is
client-side.

Then, in the page:

- `RehearsalPage.tsx:93` — on load, `setIndex(stepTo(current, length, null))`.
  `rehearsalNav.ts:49-56` **clamps** an out-of-range index into `[0, total-1]`;
  with `total = 1` that is unconditionally `0`.
- `RehearsalPage.tsx:128-131` — `addressed = scenarios.findIndex(s => s.code === code)`,
  which is `-1` when the code is absent from the payload, and `-1` when there is
  **no code at all** (the `?? -1` on an undefined payload, and `findIndex`
  against `undefined` on a loaded one).
- `RehearsalPage.tsx:136-138` — `if (addressed >= 0) setIndex(addressed)`. When
  `addressed < 0` the index is **left where it is**, i.e. at 0.
- `RehearsalPage.tsx:107-108` — `const scenario = payload?.scenarios[index]`, so
  `scenario` is `payload.scenarios[0]`.
- `RehearsalPage.tsx:298` — `notReady = code !== undefined && addressed < 0`.
  **With no code in the URL, `notReady` is `false`.** No notice renders.
- `RehearsalPage.tsx:338-351` — the blocks render `scenario`, i.e.
  `payload.scenarios[0]`, unconditionally whenever it exists.

On DEV, `payload.scenarios` is a one-element list holding S-2 (the only Ready
scenario). [state fact per the instruction; the filter at
`scenario_readiness.rs:229` is [measured], the DEV row set is taken as given.]

**Two distinct defects fall out of this, and they must not be conflated:**

**(a) The one Roman hit.** The link carries no code at all (row 1), so
`notReady` is `false` and the page silently renders index 0. The reader gets S-2's
complete rehearsal — title, code, accusation, timeline, points — with no notice,
no refusal, and a header reading "Scenario 1 of 1"
(`RehearsalPage.tsx:309` → `positionAt` → `payload.positions[0]`). Nothing on the
screen says a substitution happened.

**(b) The latent one, which the filed fix would not remove.** Even if the entry
link were repaired to carry `S-5`, `notReady` becomes `true` and the stored
not-ready sentence renders at `RehearsalPage.tsx:318-322` — **beside**, not
instead of, S-2's fully-rendered blocks at 338-351. The page would then say
"S-5 is not ready" while displaying S-2's content under S-2's title. That is a
worse state than today's, not a better one: it looks like a refusal that
nonetheless produced content.

### 1.4 The return trip [measured]

`RehearsalPageHeader.tsx:87` and `:110` both compose
`scenarioPagePath(slug, scenario.scenario_id)` where `scenario` is the prop the
page passed down — `payload.scenarios[index]`, i.e. S-2. So the breadcrumb and
"Scenario page ↗" are **correct code** operating on a substituted subject: they
faithfully link to the scenario the page is showing, which is not the scenario the
reader came from. Both exits go to S-2's working page. That is the second half of
Roman's round trip, and it is not a second bug — it is the first bug, observed
downstream.

### 1.5 Why the existing guard did not catch it [measured]

`pages/__tests__/rehearsalAddress.test.ts` pins four things about this family:
one mover (line 36-41), the builder is used (43-45), `replace: true` (47-49), and
that five named files contain no interpolated `/cases/…${…}` literal (60-81).
`utils/__tests__/routePaths.test.ts` pins what each builder **emits** against what
`App.tsx` **declares**.

Every one of those assertions passes on the current code. The guard proves the
*spelling* of a URL; nothing asserts anything about its *arguments*. A call site
that composes a perfectly-spelled, correctly-declared, correctly-escaped address
to the wrong subject — or to no subject — is invisible to both halves of the
guard. `ScenarioHeaderTiers.tsx` is even in `CONVERTED_FAMILY`
(rehearsalAddress.test.ts:63) and passes: it uses the builder, it just uses the
identity-free one, and it has `scenarioId` (prop, line 138), `code` (line 139)
and `status` (line 142) in hand while doing so.

### 1.6 No status gate on the entry [measured]

`ScenarioHeaderTiers.tsx:226-232` renders the "Rehearsal view →" `Link`
unconditionally. `status` is a declared prop (line 142) and is passed
(`ScenarioDetailPage.tsx:413`), but it is read only by `headerDescriptor`
(line 164) and `ScenarioStatusControl` (196-201). Nothing conditions the
rehearsal link on it. A Draft scenario's rehearsal control is byte-identical to a
Ready one's. The `title` attribute at line 229 — "shows scenarios marked Ready" —
is the only place on the surface where the eligibility rule is stated, and it is
a hover tooltip, in code, on a control that does not honour it.

### 1.7 Other identity-resolution sites on these surfaces [measured]

- `ScenarioDetailPage.tsx:120`, `TrialPrepDashboardPage.tsx:132` —
  `const slug = slugParam ?? DEFAULT_CASE_SLUG`. A missing case slug resolves to
  the hardcoded single-case constant (`services/caseHeader.ts:26`,
  `"awad_v_catholic_family_service"`) rather than refusing. Same class as the
  headline (an absent identity resolving to a live row), currently harmless
  because the deployment is single-case, and the constant's own comment says so.
  Flagged for completeness, not urgency.
- `RehearsalPage.tsx:306` — `slug={slug ?? ""}`. An empty slug composes
  `/cases//trial-prep`, which matches no declared route and lands on the
  catch-all 404 (`App.tsx:105`). Unreachable in practice (the route cannot match
  without a slug segment); noted because it is a fallback on an identity.
- `ScenarioDetailPage.tsx:391-398` — a `null` scenario after load renders
  "Scenario not found." rather than substituting. Correct; the contrast with the
  rehearsal page is instructive.

---

## SWEEP 2 — field wiring on the identity / pairing surfaces

### 2.1 The accusation-field map [measured]

Five stored fields carry some flavour of "what they are saying." Column homes
verified in `repositories/pipeline_repository/scenario_store.rs:59-105` and
`dto/scenario_crud.rs:218-221`.

| # | Store location | Seeded by | Edited by | Rendered on |
|---|---|---|---|---|
| A | `scenarios.name` | create form "Name" (`ScenarioCreateForm.tsx:184-194`) | identity modal "Name" (`ScenarioIdentityModal.tsx:316-328`) | dashboard card title (`ScenarioCard.tsx`), page header (`ScenarioDetailPage.tsx:411`), page breadcrumb (`:403`), delete dialog (`:565`), rehearsal title + breadcrumb (`RehearsalPageHeader.tsx:88,100`), delete-failure sentence (`TrialPrepDashboardPage.tsx:181`) |
| B | `definition->>'attack_text'` | create form "Accusation" (`api/scenarios.rs:141`) | identity modal "What they say — quote it if you can" (`ScenarioIdentityModal.tsx:334-346`) | identity block "The attack — what they claim" (`ScenarioIdentityBlock.tsx:151`) |
| C | `definition->>'attack_meaning'` | **the same create-form "Accusation"** (`api/scenarios.rs:142`) | identity modal "What that is meant to imply" (`ScenarioIdentityModal.tsx:348-359`) | **nowhere on any read surface** — no render site found |
| D | `scenarios.accusation_text` | **nothing** — no create-form field, no seed | accusation section `SentenceEditor` (`AccusationSection.tsx:203-219`), rehearsal accusation block | accusation section, rehearsal accusation block |
| E | `scenarios.motivation` | nothing | identity modal "What they want the jury to believe" (`ScenarioIdentityModal.tsx:411-422`) | identity block "Their motivation" (`ScenarioIdentityBlock.tsx:165`) |

Plus `scenarios.theme_statement` — "Our answer, in one sentence" in the modal
(`:397-409`), "Our theme — one sentence" in the block (`:159`).

**Wire note on the DTO name.** `services/scenario_dashboard.rs:255` and `:362`
both set `attack: record.name.clone()`. So the field the frontend calls
`scenario.attack` is column A — the scenario's *name* — not B, C or D. Three
different things in this table are called some form of "attack" in the code:
the DTO field `attack` (= name), `attack_text`, and `attack_meaning`.

### 2.2 The pairs that can silently diverge or duplicate [measured]

**(i) B and C are seeded byte-identical and then drift apart unwatched.**
`api/scenarios.rs:140-149` writes the one create-form accusation into both
`attack_text` and `attack_meaning`. The doc comment at `:90-111` records this as
Roman's ruling of 2026-08-07 and states the intent: the identity modal is where
the real quote replaces the placeholder. Two consequences, both structural:

  - C has **no read surface**. `ScenarioIdentityBlock` renders B, theme and
    motivation; there is no `attack_meaning` render site anywhere in
    `frontend/src` (`grep` for `attack_meaning` returns three files: the modal,
    `scenarioIdentity.ts`, and `trialPrepData.ts` — draft/type plumbing only).
    So the only way to see whether B and C still agree is to open the modal.
  - C is what the **Theme Scan judges against**
    (`services/theme_scan_judge.rs:249`, `services/theme_scan_validate.rs:131`).
    A scenario created through the UI and never edited therefore has its scan
    judging candidates against a duplicate of the wielder's-framing field, and
    nothing on any surface says so.

**(ii) A and B are seeded from *different* form fields with no relationship, and
A is what every surface calls the scenario.** The create form's "Name"
(placeholder `"e.g. Marie is obstructive and uncooperative"`,
`ScenarioCreateForm.tsx:192`) and its "Accusation" are two free-text boxes that
both invite the accusation in plain words. Whatever goes in "Name" becomes the
scenario's title on eight render sites (table row A); whatever goes in
"Accusation" becomes B and C and is rendered on one. Nothing compares them.

**(iii) D is orphaned at creation.** No create-form field and no seed writes
`accusation_text`. `dto/scenario_accusation.rs:119` and
`scenario_store.rs:86-92` both state, in terms, that it is *never* derived from
`attack_text` — that derivation is named as the beta.371 defect the column exists
to end. Correct as a law; the consequence is that **every newly-created scenario
opens with its accusation section in the honest-gap state**, while B and C hold
the human's accusation words, entered minutes earlier on the create form. The
human typed the accusation once and the surface that asks for "the accusation"
shows nothing.

**(iv) A typed `attack_meaning` can be silently discarded on save.**
`components/scenarioIdentity.ts:126-145`: `patchFrom` omits the **entire**
`definition` object when `attackText.trim()` is empty. `targetWouldBeLost`
(`:160-162`) guards exactly one field against that omission — `target` — and
`canSave` (`:177-179`) admits the draft otherwise. So: open the modal on a
scenario with no attack text, type into "What that is meant to imply", leave
"What they say" blank, click Save → `canSave` is true, the PUT carries `name`,
`theme_statement`, `motivation`, `anchor_allegation_ids` and **no definition**,
the write succeeds, the modal closes on success
(`ScenarioIdentityModal.tsx:281-288`), and the typed meaning is gone with no
message. [measured from the three pure functions; the same shape the `target`
guard was written for, with the guard covering one of the two fields it needs to.]

### 2.3 Label divergence across the identity surfaces [measured]

Same column, different words, on surfaces one click apart:

| Column | Read-only block | Editor modal | Create form |
|---|---|---|---|
| `name` | (rendered as the title, unlabelled) | "Name" (`Modal:318`) | "Name" (`Form:184`) |
| `attack_text` | "The attack — what they claim" (`Block:151`) | "What they say — quote it if you can" (`Modal:336`) | wording row `accusation_label` (`Form:262`) |
| `attack_meaning` | — (no surface) | "What that is meant to imply" (`Modal:350`) | same one box as `attack_text` |
| `theme_statement` | "Our theme — one sentence" (`Block:159`) | "Our answer, in one sentence" (`Modal:399`) | — |
| `motivation` | "Their motivation" (`Block:165`) | "What they want the jury to believe" (`Modal:413`) | — |
| `anchor_allegation_ids` | "Bears on" (`Block:171`) | "Complaint paragraphs this touches" (`Modal:444`) | "Anchor allegation id(s)" (`Form:276`) |
| `definition->>'target'` | — (not shown) | wording row `target_label` (`Modal:369`) | wording row `target_label` (`Form:240`) |

Every one of those left-column and middle-column strings is a code literal
(sweep 4). The two fields that *are* wording rows — `target` on both surfaces —
are the two that read identically on both. That is the whole argument for the
naming-unification fix, made by the code itself.

Note also: `motivation` is not shown on the create form at all, yet the modal's
label for it ("What they want the jury to believe") is the phrase the design
documents use for the *concept*, while the block's label for the same column is
"Their motivation" — the walk's OUR THEME / THEIR MOTIVATION / BEARS ON row.

### 2.4 Pairing panel and talking-points editor [measured]

`AccusationSection.tsx` is the cleanest surface audited. Every user-visible
string comes from `panel.wording` (`:143`, then `w.*` throughout); the two
mutually-exclusive count states are decided server-side (`:177-179`); the gap
kinds branch on a **token**, never on the message (`:324-335`), with the reason
stated. `dto/scenario_accusation.rs:111-131` backs it. No wiring defect found.

One benign fallback: `textOf` (`:168-169`) renders a raw `graph_node_id` when a
fact is not in `includedFacts`. That is visible, not silent — an id on screen is
self-announcing. Not flagged.

Talking points write through `scenario_augmentation.rs` and are read by both the
working page and the rehearsal page; see sweep 3 §3.4 for the row-uniqueness
issue that affects both.

---

## SWEEP 3 — silent fallbacks

### 3.1 The dead wire — one root under three of the four live-refresh gaps [measured]

`ThemeScanPanel` declares `onFactsChanged?: () => void` at line 140, destructures
it at line 147, and **never calls it.** `grep -n "onFactsChanged"
components/ThemeScanPanel.tsx` returns exactly two lines: the declaration and the
destructure. [measured]

The wire it is supposed to close: `ScenarioDetailPage.tsx:454`
`onFactsChanged={refresh}` → `ScanSection.tsx:120,162,289` → the panel prop →
nothing. `refresh` bumps `pageRefreshKey`, which is what re-runs the four-read
`Promise.all` at `ScenarioDetailPage.tsx:268-296` — the read that supplies
`cards`, `never_scanned_notice` (`:280`) and `proposal_source` (`:281`).

`tsconfig.json` sets `strict: true` but **not** `noUnusedLocals`, so an unused
destructured prop is not a type error and `npm run typecheck` stays green.
[measured: `frontend/tsconfig.json`]

The prop's doc comment (`ThemeScanPanel.tsx:136-139`) says it is "called after a
merge successfully writes candidate facts" — but merge left this panel (its own
comment at `:551-553`: "No findings list, no checkboxes, no Merge"). The caller
was never re-pointed when the callee's reason to call it was removed.

**What that costs, mechanically:**

- **Run completion.** `ThemeScanPanel.tsx:274-287` — on a poll that returns
  `completed`, the panel seeds its summary cache, selects the run, and calls
  `refreshRuns()` (line 281), which re-reads **only the run-history list**. The
  page's `cards`, `never_scanned_notice` and `proposal_source` are not re-read.
  The server now has a new projecting run
  (`scan_run_projection.rs:52-54,65-75`), so the queue's pool and the
  never-scanned verdict have both changed server-side while the page continues
  to render the pre-scan answers. This is the mechanism behind the queue frame's
  stale "No scan has run yet" banner: it renders from
  `neverScannedNotice` (`ScanSection.tsx:318-320`), which is set once at page
  load and only refreshed by `cardsRefreshKey`, which run completion never
  bumps.
- **Run deletion.** `ThemeScanPanel.tsx:403-420` — `deleteScanRun`, then
  `refreshRuns()` (line 411), then local selection/summary cleanup. Again no
  upward signal. Deleting the projecting run changes which run projects (or
  removes projection entirely); the page keeps serving the deleted run's
  `proposal_source` back down as a prop (`ScanSection.tsx:290` →
  `ThemeScanPanel.tsx:527-528` history table `proposingRunId` /
  `proposedCount`, and `:465` the collapsed card's proposed count). The panel can
  therefore attribute a proposed count to a run that no longer exists.
- **Ruling counts.** This one is *not* part of the same root and is wired
  correctly: `onRulingSaved` → `refreshCards` (`ScenarioDetailPage.tsx:457`) and
  the removal path bumps both keys (`:222-225`). The three-key design at
  `:177-225` is deliberate and documented. No defect found here.

So: run completion and run deletion share **one** root (the dead prop); ruling
counts are a separate, healthy path. [inferred from the three call graphs above;
each individual link is [measured].]

### 3.2 `runs[0]` — the last-run meta can describe a run that is not the last scan [measured]

`components/themeScanFormat.ts:81` — `const latest = runs[0]`, with no status
filter. Its only guard is `latest.candidates_total == null` (line 82).

`repositories/pipeline_repository/scan_runs.rs:561-566` — `LIST_SCAN_RUNS_SQL`
returns **every** run for the scenario (any status) `ORDER BY started_at DESC`.
`promote_scan_run_running` (`:188-214`) sets `candidates_total`;
`fail_scan_run` (`:364-378`) and `sweep_running_scan_runs` (`:387-398`) change
only `status`, `error` and `last_progress_at` — **neither clears
`candidates_total`.**

Therefore a failed run (including one killed by the boot sweep) that got as far
as promotion carries a non-null `candidates_total`, sits at `runs[0]`, and is
rendered by `ThemeScanPanel.tsx:515` as the scan control line's "last run"
summary — "*N* candidates · *date* · *model*" — with nothing marking it failed.

The same component gets this right eleven lines away: `latestCompleted` /
`latestSettled` / `latestFailed` at `ThemeScanPanel.tsx:444-451` do the
status-aware selection for the collapsed card, with a comment explaining exactly
why the distinction matters. So one screen can carry two different runs under
two labels: the collapsed card naming the failed run correctly, the control line
describing it as the last scan. Same class as the .385 latest-completed-slot
collision.

### 3.3 `LIMIT 1` without a uniqueness guarantee — the projecting run [measured, low]

`scan_run_projection.rs:52-54` — `ORDER BY started_at DESC LIMIT 1`, fenced to
`scenario_id` **and** `status = 'completed'`. Both fences are asserted by
SQL-shape tests (`:212-243`). The ordering caveat is stated honestly in the
comment at `:47-51`: there is no completion timestamp, so start order stands in
for completion order, justified by "scans are serialised per scenario." That
serialisation is a runtime convention, not a constraint — but the fences are
real and the failure mode requires two overlapping completed runs on one
scenario. Recorded as measured; not ranked as a live defect.

### 3.4 `.first()` on a table with no uniqueness constraint — talking points [measured]

`backend/pipeline_migrations/20260626135022_create_scenario_responses_tables.sql:29-43`
creates `scenario_responses` with `id` as the primary key, `scenario_id` as a
plain FK, and `CREATE INDEX idx_scenario_responses_scenario` — **a non-unique
index. There is no `UNIQUE (scenario_id)`.** [measured]

Three sites take `responses.first()` off
`list_responses_for_scenario` (`scenario_responses.rs:196-209`,
`ORDER BY created_at, id`):

| Site | What it is | Warns on >1 row? |
|---|---|---|
| `services/scenario_augmentation.rs:461` | working-page talking-points **read** | **yes** (`:452-462`, `tracing::warn!`) |
| `services/scenario_augmentation.rs:330` | talking-point **write** (edit one point) | no |
| `services/rehearsal_assembly.rs:256` | **rehearsal-page** talking-points read | no |

The invariant is enforced in one of three places. The warning's own text —
"something wrote around this service — which is exactly the kind of thing that
must not pass silently" — is the correct standard; the rehearsal read, which is
the surface a witness works from, has no equivalent. A second response row would
make the rehearsal page silently render the older row's points.

### 3.5 A match that maps an unexpected state to a working one [measured]

`components/ScenarioStatusControl.tsx:199` — `const ready = status === "ready"`.
The control renders two segments (Draft / Ready). `needs_evidence` therefore
renders **as Draft**, active. The comment at `:194-198` records this as
deliberate: the value is dead vocabulary, measured at zero rows in task 1.5, and
its retirement is filed as task 3.6.

The problem is that the vocabulary is not dead on the write side.
`ScenarioCreateForm.tsx:51` — `STATUS_OPTIONS = ["draft", "needs_evidence", "ready"]`
— and the create form offers all three in a `<select>` (`:220-232`), labelled
through `statusMeta` (`trialPrepHelpers.ts:99-107`), and the create route accepts
it (`api/scenarios.rs:76-85`, `ALLOWED_STATUSES`). So a human **can create a
scenario in `needs_evidence` today**, and when they open it:

  - the header status control shows it as **Draft** (`:199`),
  - clicking "Draft" is a no-op — `shouldApplyStatus(false, false)` is `false`
    (`:91-93`) — so the control offers no way out of the state,
  - the only reachable transition is → Ready,
  - the dashboard card, meanwhile, shows "Needs evidence"
    (`trialPrepHelpers.ts:104`, `ScenarioCard.tsx:85`).

Two surfaces on the same scenario disagree about its status, and one of them is
the control that is supposed to be *the single status surface* (ruling R4,
`ScenarioHeaderTiers.tsx:12-16`). The "measured at zero rows" premise holds only
because nobody has used a control that is on screen.

### 3.6 Position from a signed index — collision on corruption [measured, low]

`services/rehearsal_assembly.rs:284` and `api/scenario_augmentation_read.rs:159`
— `position: usize::try_from(item.item_index).unwrap_or(0) + 1`. A negative
`item_index` yields position 1. Two negative rows would both render as position
1, and the edit route addresses points by position. The comment at
`rehearsal_assembly.rs:277-283` states the reasoning (0 is a position no route
matches, so the edit is refused rather than mis-applied) and the writer only ever
inserts from `enumerate`. Recorded as measured; the reasoning is sound, and the
residual is a display collision rather than a mis-write.

### 3.7 One discarded error object [measured]

`ScenarioIdentityModal.tsx:250-256` — `.catch(() => { setSubjects([]) })`. The
error value is discarded entirely: no `console.warn`, no message. The state is
still observable (`subjectsFailed` at `:301` renders
`wording.target_options_failed_notice` at `:306-308`), so the human is told; but
an operator reading the console after a failed target-vocabulary load gets
nothing, and the comment at `:252-254` acknowledges the gap. Same file's sibling
effect (`:221-231`) surfaces the message properly. Rule 1's carve-out covers
cosmetic browser storage, not an `authFetch`.

### 3.8 Statuses that decode safely — checked and clear [measured]

For completeness, three places that could have been fallbacks and are not:
`domain/fact_status.rs:120-121` makes an undecodable status a **named error**,
not a default; `services/scenario_card.rs:527`'s `unwrap_or(FactStatus::Undecided)`
therefore only fires on a genuinely absent ref row (correct semantics);
`services/rehearsal_assembly.rs:141-155` compares the stored token and documents
that an unreadable status fails **out** of the included set, surfacing as a named
gap. `services/scenario_dashboard.rs:334` raises `UnknownStatus` rather than
defaulting. No defect in this family.

---

## SWEEP 4 — user-visible strings still in code

Inventory of code-literal user-visible strings on the six named surfaces. Every
one is a row the naming-unification fix cannot reach without a build.

### 4.1 Scenario detail page — header, identity block, sections

| String | Site | Surface |
|---|---|---|
| `"Scenario"` (eyebrow) | `ScenarioHeaderTiers.tsx:171` | header |
| `"Edit this scenario's identity — name, definition, attack, theme, motivation, allegations"` (title) | `:219` | header |
| `"Edit scenario identity"` (aria-label) | `:220` | header |
| `"✎ Edit"` | `:223` | header |
| `"Rehearsal view →"` | `:231` | header |
| `"Marie's testimony-prep view — shows scenarios marked Ready"` (title) | `:229` | header |
| `"Delete"` | `:252` | header |
| `"Delete this scenario — asks for confirmation first"` (title) | `:249` | header |
| `"Draft"` / `"Ready"` (segment labels) | `ScenarioStatusControl.tsx:208,215` | header |
| `"Ready = appears in Marie's rehearsal view. Human-only switch."` | `:76` (`TOOLTIP`) | header |
| `"Scenario status"` (aria-label) | `:206` | header |
| `"Saving…"` | `:224` | header |
| `"That readiness change did not save."` | `:169` | header |
| `"Draft"` / `"Needs evidence"` / `"Ready"` | `trialPrepHelpers.ts:102,104,106` | **card + create form + everywhere `statusMeta` is used** |
| `"The attack — what they claim"` | `ScenarioIdentityBlock.tsx:151` | identity block |
| `"No attack text written yet — the pencil opens the editor."` | `:153` | identity block |
| `"Our theme — one sentence"` / `"No theme written yet."` | `:159,161` | identity block |
| `"Their motivation"` / `"No motivation written yet."` | `:165,166` | identity block |
| `"Bears on"` / `"No allegations linked yet."` | `:171,173` | identity block |
| `"Edit — the same modal as the header button"` (title) | `:130` | identity block |
| `"Scenario facts"` | `ScenarioFactsSection.tsx:335` | facts section |
| `"That fact could not be removed."` | `:129` | facts section |
| `"The order could not be reset — please reload and try again."` | `:292` | facts section |
| `"Could not check this scenario for saved references whose content is unavailable."` | `ScenarioOrphanStrip.tsx:72` | orphan strip |
| `"Loading scenario…"` | `ScenarioDetailPage.tsx:379` | page |
| `"Failed to load the scenario. Try reloading the page."` | `:289` | page |
| `"Scenario not found."` | `:395` | page |
| `"Failed to delete the scenario. Try again."` | `:250` | page |
| `"Your ruling was saved. The facts list below could not be re-read…"` (both branches) | `:331,332` | page |
| `"Try again"` / `"Dismiss"` | `:472,475` | page |
| `"Trial Prep"` (crumb) / `"Dashboard"` / `"Scenario"` | `:122,371` | page |
| `"Delete this scenario?"` / `"Delete scenario"` | `scenarioDeleteCopy.ts:48,53` | delete dialog |
| `"Deleting…"` | `ScenarioDeleteConfirm.tsx:143` | delete dialog |

### 4.2 Queue frame (scan & candidates)

| String | Site |
|---|---|
| `"Scan & candidates"` (h2) | `ScanSection.tsx:262` |
| `"scans add candidates; your rulings drain them — rerunning never removes anything"` | `:266-269` |
| `"Keys: I E D U — or use the buttons"` | `:376-379` |
| `` `Candidates awaiting ruling — ${unruled}` `` | `queueRegion.ts:196` |
| `"from all scans"` | `:232` |
| `"Collapse the queue — only this arrow collapses it; keys pause while collapsed"` | `:234` |
| `"Expand the queue"` (twice) | `:165,235` |
| `` `Next up: ${nextCode}` `` | `:255` |
| `"Loading the candidate queue…"` | `CardQueue.tsx:431` |
| `"Move: ↑ ↓ or J K — moving never rules"` | `:477` |
| `"Retry"` | `:455` |
| `"Remove this run"` (title) | `ScanHistoryTable.tsx:238` |
| `` `Remove the scan run from ${row.when}` `` (aria-label) | `:237` |
| `` `${n} candidates` ``, `` `${signed} since the previous scan` `` | `themeScanFormat.ts:85,88` |
| `"Failed to load scan history."` / `"Failed to load the model catalog."` / `"Failed to start the scan."` / `"Failed to poll the scan."` / `"Failed to delete the run."` / `"The scan failed."` / `"That run failed: …"` / `"That run has no stored result to display yet."` / `"Failed to load the run."` | `ThemeScanPanel.tsx:264,288,325,289,408,283,354,357,362` |

### 4.3 Create form

| String | Site |
|---|---|
| `"Name"` | `ScenarioCreateForm.tsx:186` |
| `"e.g. Marie is obstructive and uncooperative"` (placeholder) | `:192` |
| `"Direction"` / `"Offense"` / `"Defense"` | `:200`, `:53-54` |
| `"Status"` | `:218` |
| `"Anchor allegation id(s)"` | `:277` |
| `"doc-…:allegation:<hash>"` (placeholder) | `:283` |
| `"Optional. Paste allegation node ids, comma- or newline-separated (a graph-backed picker comes later)."` | `:288-290` |
| `"Name is required."` | `:121` |
| `"Could not load the people in this case."` | `:109` |
| `"Failed to create the scenario. Please try again."` | `:173` |
| `"Cancel"` / `"Creating…"` / `"Create scenario"` | `:302,305` |

Wording rows already in use here: `target_label`, `target_unset_option`,
`target_helper`, `target_required`, `accusation_label`, `accusation_helper`,
`accusation_required` (7 of ~18 user-visible strings on the form).

### 4.4 Identity modal

| String | Site |
|---|---|
| `"Scenario identity"` (dialog title) | `ScenarioIdentityModal.tsx:304` |
| `"Name"` | `:318` |
| `"What they say — quote it if you can"` | `:336` |
| `"Their framing, in their words."` | `:345` |
| `"What that is meant to imply"` | `:350` |
| `"Our answer, in one sentence"` | `:399` |
| `"Read aloud in rehearsal mode."` | `:408` |
| `"What they want the jury to believe"` | `:413` |
| `"Direction"` | `:427` |
| `"Complaint paragraphs this touches"` | `:444` |
| `"None yet."` | `:464` |
| `"Add a complaint paragraph"` (aria-label) | `:468` |
| `"All paragraphs added"` / `"Add a paragraph…"` | `:479` |
| `` `Remove ${label}` `` (aria-label) | `:454` |
| `"Loading…"` | `:312` |
| `"Could not load this scenario's identity. Close and try again."` | `:229` |
| `"Failed to save. Your text is still here."` | `:292` |
| `"Cancel"` / `"Save"` / `"Saving…"` | `:496,506` |

Wording rows in use: `target_label`, `target_unset_option`, `target_helper`,
`target_needs_attack_text`, `target_options_failed_notice` (5 of ~23).

### 4.5 Rehearsal page

Three literals, all documented as necessarily un-storable: `LOADING`,
`LOAD_FAILED`, `RETRY` at `RehearsalPage.tsx:65-67`, with the reason stated at
`:63-64` ("the three strings that describe the absence of a payload, and
therefore cannot come from one"). Everything else on the page is served
(`payload.wording`, `payload.positions`, `payload.always`). **This surface is
compliant**; it is the model the other five are not yet.

### 4.6 Pairing panel

`AccusationSection.tsx` — **zero code literals**. Every string is `w.*` off
`panel.wording` (`:143`). Compliant.

### 4.7 Native `confirm()` sites [measured]

`grep -rn "window.confirm" frontend/src` returns four sites, one on the audited
surfaces:

| Site | Message source | Surface |
|---|---|---|
| `ScanHistoryTable.tsx:245` | `fillRun(wording.delete_confirm_template, row.when)` — **a wording row** | scan history / queue frame — **the one that froze the browser walk** |
| `HistoryCard.tsx:47` | `"Delete this question and answer?"` — literal | Ask/history, off-surface |
| `admin/AdminFileManager.tsx:247` | `` `Delete ${filename}?` `` — literal | admin |
| `admin/AdminModels.tsx:308` | `` `Delete model '${m.id}'?` `` — literal | admin |
| `admin/AdminProfiles.tsx:255` | literal | admin |

The scenario-page delete and the dashboard delete both use the in-app
`ScenarioDeleteConfirm` (`ScenarioDetailPage.tsx:561-574`,
`TrialPrepDashboardPage`), so the app-modal pattern already exists in this
family — the run delete is the one control on these surfaces that did not adopt
it. Its message is already a wording row, so the remaining gap is the dialog
mechanism, not the words.

---

## 1. VERDICT: MEASURED — the headline defect

The S-5 → S-2 substitution is a **link that drops the scenario's identity**,
landing on a page whose index-clamp then silently supplies a different scenario.
Concretely: `ScenarioHeaderTiers.tsx:227` composes the rehearsal control as
`rehearsalPath(slug)` — the case-level address, carrying no scenario at all —
even though the component holds `scenarioId`, `code` and `status` as props
(`:138,139,142`) and the identity-carrying builder `rehearsalScenarioPath` exists,
is routed (`App.tsx:83`), and is guarded by tests. `RehearsalPage` then receives
`code === undefined`, so `addressed` is `-1` (`:128-131`), the
"move to the addressed scenario" effect does not fire (`:136-138`), the index
stays at the value `stepTo(…, null)` clamped to `0` (`:93`,
`rehearsalNav.ts:52`), and `scenario = payload.scenarios[0]` (`:108`) — the only
Ready scenario the server returned, S-2. Because `notReady` requires a code to be
present (`:298`), **no notice renders**: the page shows S-2's complete rehearsal
under S-2's title with no indication that S-5 was asked for. The return trip
follows honestly from the substitution — `RehearsalPageHeader.tsx:87` and `:110`
compose `scenarioPagePath(slug, scenario.scenario_id)` from the on-screen
scenario, so both exits go to S-2's working page. The backend is not implicated:
`GET /cases/:slug/rehearsal` accepts no scenario parameter (`api/rehearsal.rs:4`)
and filters to ready (`scenario_readiness.rs:229`), so it never learns which
scenario was requested and has no opportunity to refuse. Per the ratified
requirements (§5/§10 — rehearsal serves READY scenarios through a mandatory human
gate), refusal here means the Draft scenario's rehearsal control must either not
render or must land on an address that names S-5 and is refused by name; note
that repairing only the link would expose defect 2 below, where the not-ready
sentence renders *alongside* index 0's full content rather than instead of it.

## 2. Defect list

1. **Identity-dropping link.** The only rehearsal entry point in the app composes
   the case-level address, discarding the scenario it is on —
   `ScenarioHeaderTiers.tsx:227`. *Class: silent substitution (route-link).*
2. **Partial refusal that still renders a substitute.** A named-but-not-ready
   code renders the not-ready notice *and* `scenarios[0]`'s full blocks —
   `RehearsalPage.tsx:298` + `:318-322` + `:338-351`. *Class: silent
   substitution (index fall-through).*
3. **No-code fall-through is entirely unannounced.** `notReady` requires a code,
   so the identity-free address produces a substitution with no notice at all —
   `RehearsalPage.tsx:298`. *Class: silent substitution (missing observable).*
4. **Rehearsal control renders regardless of status.** `status` is in scope and
   unused by the link; the eligibility rule lives only in a hover tooltip —
   `ScenarioHeaderTiers.tsx:226-232`, `:142`. *Class: missing gate.*
5. **The per-scenario rehearsal address has no producer.** `rehearsalScenarioPath`
   is routed, built and tested but called only from inside the page it addresses
   — `routePaths.ts:122`, `RehearsalPage.tsx:172`. *Class: dead contract.*
6. **The route guard checks spelling, not subject.** Both halves pass on the
   defective call site, which is itself in the guarded list —
   `rehearsalAddress.test.ts:60-81`, `routePaths.test.ts:113-136`. *Class: test
   blind spot.*
7. **Dead refresh wire.** `ThemeScanPanel` declares and destructures
   `onFactsChanged` and never calls it — `ThemeScanPanel.tsx:140,147`;
   `noUnusedLocals` is off in `frontend/tsconfig.json`. *Class: silent
   staleness (root).*
8. **Run completion does not refresh the page's card payload.** Only
   `refreshRuns()` fires — `ThemeScanPanel.tsx:274-287`; the stale
   "No scan has run yet" banner renders from `neverScannedNotice`
   (`ScanSection.tsx:318-320`, set at `ScenarioDetailPage.tsx:280`). *Class:
   silent staleness (defect 7).*
9. **Run deletion does not refresh `proposal_source`.** Only `refreshRuns()`
   fires — `ThemeScanPanel.tsx:403-420`; the panel keeps attributing proposals to
   the deleted run (`ThemeScanPanel.tsx:465,527-528`). *Class: silent staleness
   (defect 7).*
10. **`runs[0]` with no status filter.** The scan control line's "last run"
    summary can describe a failed or running run —
    `themeScanFormat.ts:81`, against `scan_runs.rs:561-566`; `fail_scan_run`
    (`:364-378`) does not clear `candidates_total`. Contradicts the same
    component's status-aware collapsed card (`ThemeScanPanel.tsx:444-451`).
    *Class: latest-slot collision (.385 class).*
11. **`.first()` on a non-unique table, unwarned, on the witness's surface.**
    `scenario_responses` has no `UNIQUE (scenario_id)` —
    migration `20260626135022…sql:29-43`; the rehearsal read
    (`rehearsal_assembly.rs:256`) and the write (`scenario_augmentation.rs:330`)
    lack the guard the working-page read has (`scenario_augmentation.rs:452-462`).
    *Class: LIMIT 1 without a uniqueness guarantee.*
12. **`needs_evidence` renders as Draft and cannot be left.**
    `ScenarioStatusControl.tsx:199`; the segment click is a no-op (`:91-93`)
    while the dashboard card shows the true status
    (`trialPrepHelpers.ts:104`). *Class: unexpected state mapped to a working
    one.*
13. **The create form offers a status the detail surface cannot represent.**
    `ScenarioCreateForm.tsx:51,220-232`, accepted by
    `api/scenarios.rs:76-85`. *Class: write-side vocabulary wider than the
    read-side.* (Precondition of defect 12.)
14. **One accusation input seeds two columns that then drift unwatched.**
    `api/scenarios.rs:141-142`; `attack_meaning` has no read surface anywhere in
    `frontend/src` and is what the Theme Scan judges against
    (`theme_scan_judge.rs:249`). *Class: silent duplication.*
15. **`accusation_text` is never seeded at creation.** No create-form field and
    no seed writes it, so every new scenario's accusation section opens empty
    while `attack_text`/`attack_meaning` hold the words the human just typed —
    `ScenarioCreateForm.tsx:154-161`, `scenario_store.rs:86-92`. *Class: write-once
    field with no writer.*
16. **A typed `attack_meaning` is silently discarded when `attack_text` is
    blank.** `patchFrom` omits the whole definition (`scenarioIdentity.ts:126-145`);
    `targetWouldBeLost` guards only `target` (`:160-162`); `canSave` admits the
    draft (`:177-179`); the modal closes on success
    (`ScenarioIdentityModal.tsx:281-288`). *Class: silent discard on save.*
17. **Six identity fields are labelled differently on the read block, the
    editor and the create form.** Full table at §2.3; every divergent string is a
    code literal. *Class: ONE THING ONE NAME unenforceable.*
18. **~90 user-visible strings on five of the six surfaces are code literals.**
    Full inventory at §4.1-4.4 with file:line; the status vocabulary
    (`trialPrepHelpers.ts:102-106`) and the queue frame's headings
    (`ScanSection.tsx:262,266-269`, `queueRegion.ts:165,196,232,234,235,255`) are
    the load-bearing ones. Rehearsal (§4.5) and the pairing panel (§4.6) are
    compliant. *Class: NO WORDING IN CODE violation.*
19. **The run-delete `confirm()` is the one native dialog left on these
    surfaces.** `ScanHistoryTable.tsx:245` — its message is already a wording row
    (`wording.delete_confirm_template`); the in-app modal pattern already exists
    in this family (`ScenarioDeleteConfirm`). *Class: native dialog on a curated
    surface.*
20. **A discarded error object in the identity modal.** `.catch(() => …)` with no
    message and no `console.warn` — `ScenarioIdentityModal.tsx:250-256`. The
    human-facing state is observable; the operator-facing cause is not. *Class:
    Standing Rule 1, diagnostic half.*
21. **Case identity falls back to a constant.** `slugParam ?? DEFAULT_CASE_SLUG`
    — `ScenarioDetailPage.tsx:120`, `TrialPrepDashboardPage.tsx:132`,
    `caseHeader.ts:26`. Harmless while single-case; same class as the headline.
    *Class: absent identity resolving to a live default.*
22. **Position derived from a signed index collides at 1 on corruption.**
    `rehearsal_assembly.rs:284`, `api/scenario_augmentation_read.rs:159`.
    Documented reasoning, low residual. *Class: unwrap_or on an ordinal.*
23. **Projecting-run `LIMIT 1` orders by start time, not completion.**
    `scan_run_projection.rs:52-54`; the uniqueness premise ("scans are
    serialised per scenario") is a comment, not a constraint. *Class: LIMIT 1
    ordering assumption.*

=== END REPORT — VERDICT: MEASURED ===
