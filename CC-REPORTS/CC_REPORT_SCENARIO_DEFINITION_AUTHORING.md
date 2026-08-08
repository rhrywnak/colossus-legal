# CC_REPORT_SCENARIO_DEFINITION_AUTHORING

**Task:** `CC_FIX_SCENARIO_DEFINITION_AUTHORING_v1` · **Date:** 2026-08-07
**Branch:** `fix/scenario-definition-authoring`, off `bad51ec` per Roman's Q1 ruling
**Commit:** `e06c2c1` — 48 files, +2477 / −597 (amended from `edf1574` with the §11 findings)
**Evidence base:** `CC-REPORTS/CC_REPORT_SCENARIO_COPY_DIAGNOSTIC.md`

Rulings applied as given: **Q1** branch off `bad51ec` · **Q2(a)** the accusation
sentence seeds both `attack_text` and `attack_meaning` · **Q3** the fallback dies
in all three `resolve_gather_subject` callers · **Q4** legacy rows show the
notice until edited.

---

## 1. WHAT CHANGED, AGAINST THE TWO DEFECTS

### Defect 1 — the create form could not produce a complete scenario

`ScenarioCreateRequest` no longer carries `definition: Option<Value>` at all. It
carries **`target`** and **`accusation`** as required fields, and the handler
composes the definition from them ([api/scenarios.rs:119](backend/src/api/scenarios.rs:119),
`authored_definition`).

Three consequences, each deliberate:

- **The un-authored state is now unconstructible through this route.** Not
  discouraged — the type has no shape for it. `ScenarioCreatePayload` in
  [scenarioCrud.ts:47](frontend/src/services/scenarioCrud.ts:47) is required-field
  too, so a create call that would produce an empty definition fails to compile.
- **The composition rule lives in the backend** (Rule 12). The browser sends what
  the human typed; that the accusation seeds both attack fields (Q2(a)) is one
  function, in one place, tested without a browser.
- **A caller can no longer post an arbitrary jsonb blob** into the column.
  Editing a definition past those two fields is `PUT`'s job, where the body is a
  fully typed `ScenarioDefinition`.

The form ([ScenarioCreateForm.tsx](frontend/src/components/ScenarioCreateForm.tsx))
gains a target `<select>` populated from `getAvailableFilters().subjects` — the
same endpoint the Bias Explorer's "About" filter reads, so the people offered and
the people the graph can gather evidence about are one list by construction — and
an accusation `<textarea>`. **No new read route was needed.**

**The target is deliberately NOT pre-selected**, though the task permitted it: a
pre-filled person is what the old silent fallback amounted to, and ten minutes
later a defaulted target is indistinguishable from a chosen one.

### Defect 2 — silent default substitution

The case-default fallback is **removed** from
[services/scenario_subject.rs](backend/src/services/scenario_subject.rs). The
resolver now reads the definition's `target` and nothing else.

Three things fell out of that, all kept:

- `resolve_scenario_subject` **stopped being `async`** and no longer takes
  `&AppState`. With no graph lookup there is no I/O left, so it is a pure sync
  `fn` over borrowed data — and the whole resolver is now unit-testable with no
  database and no graph.
- `SubjectResolveError` went from two variants to one (`NoTarget`).
  `DefaultLookupFailed` and `Unresolvable` described failures that can no longer
  happen.
- **`ThemeScanError::SubjectResolveFailed` is retired.** Its only cause was that
  graph lookup. A variant for an impossible failure is a message no operator will
  ever see and every reader has to reason about.

`SubjectUnresolvable`'s message was rewritten: it named `CASE_DEFAULT_SUBJECT_NAME`,
sending a human to an env var that was never their problem. It now names the
authoring action that fixes it.

**All three callers handle the new state** (Q3):

| Route | Behaviour with no target |
|---|---|
| `…/facts/cards` | **200**, empty `pool` + `set_aside`, `no_target_notice` set. The surface Roman saw. |
| `…/facts/gather` | **200**, empty pool, same notice. Returns before minting ordinals — numbering 148 candidates for a scenario that has chosen no subject would leave identity rows for a pool it may never gather. |
| `…/allegation-options` | Empty catalogue via `build_options(vec![], &[], settings)` — reusing the composer so the panel's wording is still built in exactly one place. Unreachable from the UI today (no cards ⇒ no link panels), which is not a contract. |
| Theme Scan | Refuses to start, by name, before any run row is written. |

`CASE_DEFAULT_SUBJECT_NAME` **stays in use** by the Bias Explorer's "About"
filter, where a default is a visible starting view the human can change — not a
silent substitution inside a stored definition.

### The edit path (item 4)

Largely already built, which shaped the whole plan: `ScenarioIdentityModal`
already authored name, `attack_text`, `attack_meaning`, `theme_statement`,
`motivation` and allegation chips through a `PUT` that already accepted a typed
`ScenarioDefinition`. It deliberately did **not** edit `target` — it carried it
through untouched, on the assumption that something else authored it. Nothing
did.

The modal now edits the target. That is how S-1 and S-3 are completed with no SQL.

**One silent-loss trap found and closed while doing it.** `patchFrom` omits the
entire definition when `attackText` is blank (because `attack_text` is required
by the parse contract). With a target in the draft, that omission would have
discarded a person the human had just chosen, with nothing said — the exact
defect class this task is about, reintroduced by the fix. So `targetWouldBeLost`
([scenarioIdentity.ts:120](frontend/src/components/scenarioIdentity.ts:120))
refuses that combination with a stored sentence, rather than a disabled button
that will not say why.

A target already stored but absent from the vocabulary is still offered by its
raw id, so it stays visible instead of vanishing from the list and being cleared
on the next save.

### Wording (item 5)

**Thirteen new stored rows**, seeded by
[20260807155419_scenario_authoring_wording.sql](backend/pipeline_migrations/20260807155419_scenario_authoring_wording.sql),
read through a new `domain::wording_scenario_authoring` block. Every new
user-facing string is a row; **no new wording is in code.**

The form's **pre-existing** literals ("Name", "Direction", "Status") were left
alone — moving them is a separate change with its own migration, and mixing it
into a defect fix would put untested string plumbing beside the fix Roman is
waiting on. Recorded here rather than smuggled.

Two wire DTOs rather than one 13-field block sent everywhere: three of the
thirteen never travel as a *vocabulary*. The two create refusals ride the 400
(so a client cannot show a different sentence than the server refused with), and
the no-target notice rides the gather/cards payloads, where its **presence** is
the signal — a client holding it unconditionally could render it beside a full
queue.

---

## 2. VERIFICATION [measured]

```
backend:  cargo build --lib --bins        OK
          cargo test --lib                1678 passed, 0 failed, 2 ignored
          cargo clippy --lib --bins -D warnings   clean
          cargo fmt --check               clean
frontend: npm run typecheck               clean
          npx vitest run                  822 passed, 61 files, 0 failed
          npm run build                   built in 1.64s
          route-link guard                13 passed (routePaths.test.ts)
```

**`cargo test --workspace` was NOT run, deliberately.** The `backend/tests/*.rs`
integration targets have not compiled since ~beta.343 (`AppState` has no
`theme_scan_provider`; `settings_store::load_settings` moved to `settings_boot`
in `f1767e3`). Both predate this branch and neither is touched by it —
`git log backend/tests/` confirms no commit here. `cargo test --lib` is the
honest baseline; `cargo clippy --lib --bins` for the same reason.

**No `npm run lint`** — the frontend has no lint script and no eslint config.

### Tests written, against Roman's new standing law

Behaviour only. Nothing that restates the code, no shape-pinning, no complement
assertions.

| Test | What breaks if it fails |
|---|---|
| `no_target_resolves_to_nothing_rather_than_to_the_case_default` | The regression test for 2026-08-07. `None`, `""` and `"   "` all resolved to `person-marie-awad` before; any of them resolving again means the fallback is back. |
| `a_created_scenario_gathers_over_the_target_the_human_chose` | What create writes must parse AND resolve to the chosen subject. The whole fix in one assertion. |
| `the_accusation_is_readable_by_the_scan_and_by_the_parser` | Q2(a). Filling only one attack field leaves either an unparseable definition or an unscannable scenario — both look like a working one until somebody opens the page. |
| `a_blank_target_or_accusation_is_refused_by_name_and_writes_nothing` | A `"   "` target would be stored, would parse, and would then match no graph node — a silent empty pool. |
| `an_unauthored_definition_gathers_nothing_rather_than_a_default_pool` | The gather fallback path specifically. |
| `display_subject_unresolvable_sends_the_human_to_the_identity_they_must_author` | The scan refusal must send the human to the authoring control, and must NOT name the retired env var. |
| `every_declared_key_is_seeded_with_the_value_this_build_expects` | A declared wording key with no row makes the backend **refuse to start**. This catches the deploy that would take DEV down. |
| `no_key_collides_with_another_surface_s_key` | `app_settings` is keyed by `key` alone; a collision would make editing the create form's label silently change a rehearsal sentence. |
| `is how a legacy scenario stops gathering nothing` (TS) | The S-3 completion path: a patch that dropped the target would appear to save and leave the scenario empty. |
| `refuses a target with no attack text instead of silently dropping it` (TS) | The trap described above, including proof that the omission it guards is real. |
| `keeps the wielders the modal does not edit` (TS) | The backend REPLACES the definition blob; a dropped field is a field deleted from the row. |

Removed: the two `map_subject_error` tests and the `SubjectResolveFailed`
display/mapping tests — they exercised code paths that no longer exist.

---

## 3. THINGS FOUND ON THE WAY, NOT IN THE ANALYSIS

**Two Rule 17 splits, both taken rather than deferred.**

- My wiring pushed `settings_store.rs` from 296 to **303** non-comment lines — a
  breach I introduced. Split at the seam that module's own header already argued:
  `settings_wording.rs` now holds `AllWording` and `build_all_wording` (the WORDS),
  leaving `settings_store.rs` to decide the NUMBERS. Now under the limit.
- `scenarios.rs` went 436 → 532 with the create-side composition. Its tests moved
  to `scenarios_tests.rs` via `#[path]`, the pattern its neighbours already use
  (`scenario_gather_tests.rs`, `scenario_cards_tests.rs`). **329 now — below the
  436 it was before this change.** No test changed in the move.

**Still over the limit, untouched, pre-existing** — flagged, not fixed, because
each is a separate refactor: `scenario_dashboard.rs` 474 → **477** (+3, my
`create_wording` parameter), `settings_store_tests.rs` 636 → **647**,
`scenarios.rs` **329**.

**A thirteenth wording row appeared mid-build.** The analysis planned twelve. The
`targetWouldBeLost` trap needs its own sentence — a refusal without one is the
"control that refuses without saying why" this codebase criticises elsewhere.

**`backend/Cargo.lock` is in the commit.** Not a version bump by CC: cargo
reconciled the lock to the `2.0.0-beta.382` that Roman's own `Cargo.toml` already
declares (the lock still said `beta.381` after `be9c819`). Nothing else in it
changed.

---

## 4. DEPLOYMENT

- **New env vars: none.** No Ansible template change owed.
- **Migration: one**, pipeline DB (`colossus_legal_v2`), additive — thirteen
  `app_settings` rows, `ON CONFLICT (key) DO NOTHING`, no DDL on existing tables.
- **Ordering hazard, the only one:** `SCENARIO_AUTHORING_WORDING_KEYS` declares
  all thirteen to the boot loader, and a declared key with no row is a **refusal
  to start** (v2 §2b — no compiled-in defaults). The backend applies these at
  boot via the runtime Migrator, so a normal deploy orders itself correctly.
- **Container rebuild: BOTH** (backend + frontend).
- **Rollback:** revert the merge. The migration is additive and rewrites no
  scenario row, so the prior image boots unchanged against the migrated DB.

### What Roman will see on DEV after deploy

| Scenario | Before | After |
|---|---|---|
| **S-2** (`target: person-marie-awad`) | 148 cards | **unchanged** — 148 cards, 52 included refs intact |
| **S-3** (`definition: {}`) | 148 borrowed cards | the no-target notice, until its target is authored in the modal |
| **S-1** (`definition: {}`) | 148 borrowed cards | the notice. **Its 8 ruling anchors sit behind it** until edited (Q4, confirmed) |
| New scenario | born with `{}` | cannot be created without a target and an accusation |

**Not deployed. Not pushed. Roman decides both.**

---

## 5. THE §11 GATE

All four agents ran against commit `edf1574`. It was committed FIRST on purpose:
those agents `git stash`, and in this tree the pop fails on `Cargo.lock` — so the
tree they might disturb was already safe in a commit.

| Agent | Verdict |
|---|---|
| `rules-enforcer` | **PASS** — all 13 mechanical rules, 43 production files |
| `observability-checker` | **PASS** |
| `test-auditor` | **FAIL** → 1 gap, fixed below |
| `architecture-reviewer` | 1 finding, fixed below |

### Finding 1 (test-auditor) — the notice itself was untested, and untestable

The resolver tests proved no subject is resolved. Nothing proved what the caller
then SENDS. Both `no_target_response` functions took `&AppState`, so the payload
they build could only be exercised by running the whole service — meaning an
early return silently changed to pass `None` for `no_target_notice` would have
shipped. The user-visible result of that: an empty queue with no explanation —
the same undiagnosed state this task exists to remove, reached by the other road.

Both now take the already-read notice as `&str`: the caller does the reading (it
holds the state), the function does the shaping. Two tests added
(`a_scenario_with_no_target_is_told_why_its_queue_is_empty`, on gather and on
cards). 1676 → 1678.

### Finding 2 (architecture-reviewer) — a helper shown where a refusal belonged

The create form's client-side validation displayed `target_helper` /
`accusation_helper` as its error text. A helper says what a field is FOR — "the
scan judges every candidate fact against these words, so write them the way you
would say them out loud" — which, read as an error, explains the feature instead
of the mistake.

The correctly-worded refusal rows already existed. **I had withheld them from
`ScenarioCreateWordingDto` on purpose**, reasoning that a client holding its own
copy could show a different sentence than the server's 400. That reasoning was
wrong, and the review named why: the form validates BEFORE it sends, so when its
own check fires no request is made and the server's sentence is never reached.
Withholding them did not prevent a second voice — it guaranteed a worse one.

Both surfaces now speak the same two stored rows. The DTO's module comment, which
argued the mistaken position, is corrected rather than deleted.

### Honest caveat on the amend

The agents inspected `edf1574`; the two fixes above are in `e06c2c1` and the
agents did not re-run over them. Both are small and neither introduces a new
pattern — one is a signature change from `&AppState` to `&str` plus two tests,
the other swaps which stored row a `setError` call reads. The full mechanical
gate (build · test · clippy `-D warnings` · fmt · typecheck · vitest · build) was
re-run over the amended tree and is green, counts in §2.

**Not deployed. Not pushed. Roman decides both.**
