=== CC REPORT — CC_TASK_R1_SURFACE_REPAIR_BATCH_v1 .390 COMPLETION — 2026-08-10 ===

# R1 .390 — the correctness batch, built and gated

**Branch:** `fix/r1-surface-repair-390` · **Commit:** `03866d7` (amended after the
gate) · **Base:** `main` @ `7527c4d` (v2.0.0-beta.389) · **Not pushed, not tagged.**

**Scope built:** Pieces 1, 2, 3, 4, 5, 6, 10c, 10d + both compiler flags, under
the rulings in `CC_TASK_R1_RULINGS_v1`. Pieces 7, 9, 10a, 10b, 10e, 10f are .391;
Piece 8 is .392.

---

## BUILD AND TEST RESULTS — true numbers

| Check | Result |
|---|---|
| `cargo build --workspace` | **clean** |
| `cargo test --lib` | **1757 passed**, 0 failed, 2 ignored (was 1753 on main; **+4**) |
| `cargo clippy --workspace -- -D warnings` | **clean** |
| `cargo fmt --check` | **clean** |
| `npm run typecheck` | **clean — with `noUnusedLocals` AND `noUnusedParameters` on** |
| `npx vitest run` | **897 passed**, 66 files (was 887; **+10**) |
| `npm run build` | **clean** (1.74 MB bundle, pre-existing chunk-size warning) |
| `./scripts/check-migrations.sh` | **OK** — 74 pipeline migrations, no duplicate versions |

**Two commands from CLAUDE.md §6 were NOT run, and cannot be:**

- `cargo test --workspace` — `backend/tests/` does not compile. `cargo test --test
  scenarios_integration --no-run` fails with **17 errors** against signatures that
  moved several releases ago (`insert_response_item` gained an argument, among
  others). **Verified pre-existing:** this commit changes zero files under
  `backend/tests/` (`git diff main...HEAD --name-only -- backend/tests/` returns
  nothing). The honest backend baseline is `cargo test --lib`, and that is what is
  reported above. There is also a stale `stash@{0}` from beta.343 sitting in the
  repo, which dates the breakage.
- `npm run lint` — **there is no lint script and no eslint config** in
  `frontend/`. `package.json` defines `dev, build, preview, test, typecheck`. The
  frontend baseline is typecheck + vitest + build, all three green.

**The migration was verified against the live DEV schema, not just written.** Both
statements were executed inside a transaction on `colossus_legal_v2` and rolled
back — `ALTER TABLE … ADD CONSTRAINT` and `DROP INDEX IF EXISTS` both succeed
against the real table. DEV is unchanged; no DDL was committed.

---

## WHAT SHIPPED, PIECE BY PIECE

### Piece 1 — the rehearsal link and page (audit defects 1–5)

The .389 round trip, mechanically: `ScenarioHeaderTiers.tsx:227` composed
`rehearsalPath(slug)` — the case-level address carrying no scenario — and rendered
identically at every status. On S-5 (Draft) it landed with no code in the URL,
`RehearsalPage`'s `notReady` test requires a code, so no notice rendered, the index
stayed clamped at 0, and S-2 rendered under S-2's title. Both exits then composed
their address from the scenario on screen.

| Sub | Change |
|---|---|
| 1a | `ScenarioHeaderTiers.tsx` composes `rehearsalScenarioPath(slug, code)` — the address routed and tested since .382 with **no producer anywhere in the app** |
| 1b | The link is live only at `status === "ready"`; otherwise an inert `<span>` with the reason beside it AND in its title, from a stored row |
| 1c | `RehearsalPage` derives **one mode** — `refusing` / `picking` / `rehearsing` — so a refusal can no longer render content |
| 1d | A bare `/rehearsal` renders `RehearsalPicker` (new, 118 lines): every Ready scenario by code and name, each linking to its own address |
| 1e | Two new `describe` blocks in `rehearsalAddress.test.ts` asserting the link's **arguments**, plus the picker added to `CONVERTED_FAMILY` |

**Ruling 8 implemented as designed:** `rehearsalBlockedTemplate` is `null` when the
augmentation payload has not loaded, and the control then renders **not at all** —
matching `AccusationSection`'s `if (!panel) return null`. The observability gate
independently verified this absence is never silent: `augmentation === null` occurs
only during the load state or behind the page's full error banner.

**One deliberate deviation from the specified wording, and why.** The instruction
gave `"Not in rehearsal — this scenario is Draft. Mark it Ready first."` The row
ships as a **`{status}` template**: `"Not in rehearsal — this scenario is {status}.
Switch it to Ready on this page first."`, registered in
`wording_templates::REQUIRED_PLACEHOLDERS` and filled from `statusMeta(status)`.
Reason: ruling 6 deliberately keeps `needs_evidence` **readable**, and the
`scenarios.status` CHECK still permits it — so a sentence hardcoding "Draft" would
state a falsehood on the one row it exists to explain. Same words, one placeholder.
**Flagging for the architect** as a change to specified copy.

### Piece 2 — the dead refresh wire (defects 7–9)

`onFactsChanged` is now called on run **completion** (`ThemeScanPanel.tsx`, beside
the existing `refreshRuns()`) and on run **deletion**. The prop's doc comment, which
still described a merge path that had left the component, was rewritten to say what
it now does and why the name outlived its first caller.

The over-refresh-on-delete caveat is kept as an explanatory sentence in the code,
per the accepted walk note.

### Piece 3 — the last-run line (defect 10)

`lastRunSummary` takes `runs.find(r => r.status === "completed")` instead of
`runs[0]`, adopting the selection its own collapsed card already used. The `status`
field is now required on the parameter type, so a caller cannot omit it.

**No visible acceptance on current DEV data** — every scenario's newest run is
`completed`. Acceptance is the three new unit tests (skips failed, skips running,
returns `null` when only a failed run exists) plus code review, per the accepted
walk note.

### Piece 4 — status honesty at create (defects 12–13)

The Status `<select>`, its state and `STATUS_OPTIONS` are gone; creation always
sends `CREATED_STATUS = "draft"`. `ALLOWED_STATUSES` narrows to `["draft", "ready"]`
on the create route — **a form is not a fence**.

**Ruling 6 honoured precisely:** the DB CHECK is untouched, `statusMeta` keeps its
third arm, and the existing read-path test
(`scenario_dashboard::tests::parse_status_maps_each_valid_token`) is named in a
comment beside the new write-side refusal rather than duplicated.

### Piece 5 — the identity modal stops losing text (defects 16, 20)

`targetWouldBeLost` becomes `definitionWouldBeLost`, returning
`"target" | "meaning" | null` rather than a boolean — two operationally distinct
states, two sentences. The modal maps each answer to its own stored row.

The discarded `catch` gains a `console.warn` that **names the scenario**
(`${slug}/${scenarioId}`), added under the observability gate.

### Piece 6 — talking points on a guaranteed row (defect 11)

`UNIQUE (scenario_id)` on `scenario_responses`; the superseded
`idx_scenario_responses_scenario` dropped in the same migration (ruling 14).

**All three `.first()` sites** now read through one new
`sole_response_for_scenario`, which carries the multi-row warning that previously
existed at only one of them — including the rehearsal read, the surface a witness
works from (ruling 14's "one rule, one shape, three sites").

**Precondition measured on DEV before shipping:** 1 `scenario_responses` row in the
whole database, 1 distinct `scenario_id`, **0** scenarios with more than one.

### Piece 10c / 10d

- The run-delete `window.confirm` that froze the walk adopts `ScenarioDeleteConfirm`.
  The table now **requests** the delete (carrying its already-filled stored
  sentence); the panel owns the dialog, the call, and the failure. `title` became
  optional on the dialog so no new literal heading was invented.
- The dead `+ Generate a scenario` tile is removed, with the reason recorded where
  it stood.

### Both compiler flags

`noUnusedLocals` **and** `noUnusedParameters`. **27 dead bindings cleared across 19
files** (28 minus `onFactsChanged`, fixed by Piece 2).

Two findings inside that sweep were **not** simple deletions:

1. **`ThemeScanPanel`'s localStorage collapse preference** (`COLLAPSE_KEY_PREFIX`,
   `readCollapsed`, `writeCollapsed`, and the `collapsed` state) was kept dormant in
   1.7D on the reasoning that "a dormant PREFERENCE is a decision already made",
   waiting for task 3.14. Ruling R7 has since decided the opposite for this whole
   family — collapse is deliberately **not** remembered. It was not waiting for a
   feature, it was contradicting a ruling. Removed, with that reasoning recorded.
2. **`CandidateCard`'s `onCorrectQuestion` / `onRevertQuestion` were PRESERVED, not
   deleted** — see the filed finding below.

---

## THE GATE — all four agents, and what changed because of them

| Agent | Verdict | Findings in this diff | Action |
|---|---|---|---|
| `rules-enforcer` | **PASS** | none | — |
| `architecture-reviewer` | REVIEW | 1 (+1 pre-existing) | fixed |
| `test-auditor` | **FAIL** | 2 | both addressed |
| `observability-checker` | REVIEW | 2 (both self-declared out of scope) | 1 fixed, 1 declined with reasons |

**Fixed in the amend:**

1. *(architecture-reviewer)* The delete-error path said what failed but not what to
   do. Both branches now state that the run has **not** been deleted and name the
   remedy.
2. *(observability-checker)* The new `console.warn` did not name the scenario. It
   now carries `slug/scenarioId`, with a note on why the effect's empty dependency
   list stays correct (the modal is mounted only while open).
3. *(test-auditor, gap 2)* `RehearsalPicker.tsx` composes routes and was missing
   from `CONVERTED_FAMILY`. Added; the anti-vacuity length bumped 5 → 6.
4. *(test-auditor, gap 1)* — see below.

**Test-auditor gap 1, addressed differently than asked — please read.** It asked
for two integration tests for `sole_response_for_scenario` in
`backend/tests/scenarios_integration.rs`. **That file does not compile** (17
pre-existing errors, untouched by this commit), so tests written there would never
run — worse than a recorded gap. Instead the SQL tail was hoisted to a `const` and
given two shape tests in the module, following the house pattern
(`scan_run_projection`'s three): that `ORDER BY created_at, id` is present, so
"first" means "the oldest, deterministically", and that the read is fenced to one
scenario. **The behavioural half — `None` for no row, `Some` for one — remains
uncovered and is filed below.**

**Observability finding declined, with reasons.** The agent asked that
`sole_response_for_scenario`'s multi-row warning also write to `pipeline_events`.
Declining because: the agent itself notes it is outside the check's
`pipeline/steps/` scope; the condition became **impossible by construction** in the
same migration; and it would put a database WRITE into a read path executed on
every rehearsal page load, to record a state that cannot occur. The `tracing::warn!`
stays. **Recorded here rather than dropped — overturn it if you disagree.**

**One pre-existing finding, filed not fixed** *(architecture-reviewer's own
recommendation)*: `validate_status`'s 400 does not echo the submitted value. The
sibling `validate_direction` has the identical gap; changing one would make the pair
inconsistent.

---

## FILED — not fixed here, needs a decision

1. **`QuestionLine` is unreachable.** Task 1.7F Part B built a complete, tested
   question-correction editor. **Nothing renders it** — its only reference outside
   its own file is a structure test. `CandidateCard` receives `onCorrectQuestion`
   and `onRevertQuestion`, wired from `CardQueue` through `CandidateList`, and drops
   both. **This is the same dead-wire shape as defect 7, one feature over.** The two
   props are preserved with `_` prefixes and the finding written on them, because
   retiring a built feature is a design decision, not a lint sweep's call.
2. **`sole_response_for_scenario` has no behavioural test** — blocked by the broken
   integration suite (above).
3. **`backend/tests/` has not compiled since ~beta.343.** 17 errors in
   `scenarios_integration.rs` alone. Out of scope here; it is why
   `cargo test --workspace` cannot be the standard target CLAUDE.md rule 28 says it
   is.
4. **`api/scenarios.rs` is over the 300-line limit** (326 by the enforcer's count).
   Pre-existing and unchanged — this commit added doc comments only.
5. **`validate_status` / `validate_direction` 400s omit the submitted value.**

---

## DEPLOYMENT IMPACT

- **Migration:** 1 new
  (`20260810094435_r1_390_rehearsal_gate_wording_and_response_uniqueness.sql`),
  pipeline DB. Applied at backend boot by the Migrator.
- **New env vars:** none. **No Ansible template change owed by this build.**
  (`THEME_SCAN_MODEL`/`THEME_SCAN_CONCURRENCY` still owe theirs from D2b — a
  separate repo and a separate task.)
- **New settings rows:** 3 — `scenario_rehearsal_link_blocked_reason`,
  `scenario_identity_meaning_needs_attack_text`, `rehearsal_picker_heading`.
- **Deploy ordering (the standing hazard):** all three are declared to the boot
  loader, and a declared key with no row **refuses start**. The migration must reach
  the database with or before the .390 image — which the boot Migrator guarantees on
  a normal deploy. The reverse is safe: rolling back to .389 leaves three unread rows
  and one harmless constraint.
- **Container rebuild:** both.
- **API endpoints:** none added; none changed. `POST /cases/:slug/scenarios` now
  **refuses** `status: "needs_evidence"` with a 400 — the only wire-visible change.
- **Rollback:** `git checkout main` and redeploy .389. The UNIQUE constraint and the
  three rows are forward-compatible with .389 and need no reversal.

`backend/Cargo.lock` is in the diff: it records the `2.0.0-beta.388 → beta.389`
version already in `Cargo.toml` at `7527c4d`, regenerated by `cargo check`. **No
version was bumped by this build.**

---

## THE WALK CHECKLIST

### Piece 1 — the acceptance the batch was written for

1. **S-5 (Draft) → the control is inert and says why.** Open S-5. "Rehearsal view →"
   is muted with `not-allowed`, and beside it: *"Not in rehearsal — this scenario is
   Draft. Switch it to Ready on this page first."* **Nothing navigates.**
2. **Flip S-5 to Ready → the link goes to S-5.** The URL must end `/rehearsal/S-5`
   and the page must show **S-5's** code and title. *(This is the exact click that
   produced S-2 on .389.)*
3. **From S-5's rehearsal, "Scenario page ↗" returns to S-5** — not S-2.
   *(The second half of the .389 round trip.)*
4. **Hand-type `/cases/<slug>/rehearsal/S-9`.** The stored not-ready sentence naming
   **S-9**, and **no scenario rendered anywhere on the page**. Not a 404.
5. **Hand-type `/cases/<slug>/rehearsal`.** A **list** headed *"Choose a scenario to
   rehearse"*. Nothing is opened for you. Clicking a row opens that scenario.
6. **S-2 unaffected** — flip it Ready if needed and confirm its rehearsal is
   unchanged.
7. **Take every scenario out of Ready** → the picker shows the stored
   nothing-ready sentence, no empty list under a heading.

### Piece 2 — the stale banner

8. Open a never-scanned scenario, run a scan, and **do not reload**. When it
   completes, the *"No scan has run yet"* banner must **disappear on its own** and
   the queue must fill.
9. Delete the projecting run from the history. The scan card's proposed count and
   the history's "proposing" marker must **stop naming the deleted run** without a
   reload.

### Piece 10c — the dialog that froze the walk

10. Delete a scan run. An **in-app** dialog appears with the stored sentence naming
    the run. **The browser does not freeze.** Cancel leaves the run. Confirm removes
    it and the table re-reads.

### Pieces 4 and 5

11. The create form has **no Status field**. A created scenario lands **Draft** on
    the dashboard and on its own page — the two must agree.
12. In the identity modal on a scenario with no attack text: type only into *"What
    that is meant to imply"*, press Save. It must **refuse by name**, and the typed
    text must still be on screen. *(On .389 it saved, closed, and lost the text.)*

### Behavioural probes for the .390 runbook — a badge has lied before

| Probe | Expected |
|---|---|
| `GET /api/cases/<slug>/rehearsal` | 200; JSON carries `wording.picker_heading` |
| `POST /api/cases/<slug>/scenarios` with `"status":"needs_evidence"` | **400**, message `status must be one of: draft, ready` |
| `GET /api/cases/<slug>/scenarios/<S-5 id>/augmentation` | `identity_wording` carries `meaning_needs_attack_text` **and** `rehearsal_link_blocked_reason`, the latter containing `{status}` |
| `SELECT conname FROM pg_constraint WHERE conrelid='scenario_responses'::regclass` | includes `scenario_responses_scenario_id_key` |
| `SELECT indexname FROM pg_indexes WHERE tablename='scenario_responses'` | `idx_scenario_responses_scenario` **absent** |
| `SELECT count(*) FROM app_settings WHERE key IN (…the three…)` | **3** |
| Backend boot log | no wording-key refusal; scenario-authoring count reads **16**, rehearsal **42** |

**If the backend refuses to start after deploy, the migration did not run.** That is
the designed failure, not a regression: check the Migrator's log line before
touching anything else.

=== END REPORT — VERDICT: PASS ===
