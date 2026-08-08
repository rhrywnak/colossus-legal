# CC_REPORT_DEFER_FIX_AND_FACETS

**Task:** `CC_TASK_DEFER_FIX_AND_FACETS_v1` · **Date:** 2026-08-08
**Branch:** `fix/defer-and-facet-counts`, off `main` @ `ccc664b` (v2.0.0-beta.385)
**Built under:** the architect's Phase A rulings + the RE-AIM confirmation of 2026-08-08 (R1 scope-corrected, R2, R3 reframed, D3a, D3b reworded).
**Phase A diagnostic:** `CC-REPORTS/CC_REPORT_DEFER_FIX_AND_FACETS_PHASE_A.md`, including §6 — the DB measurement that overturned the premise.

Defer was never broken. It was **silent**, and the silence was indistinguishable from a dead button. Every ruling now says what it did.

---

## 1 — GATE RESULTS [measured]

| Check | Result |
|---|---|
| `cargo check --bins` | **clean** |
| `cargo test --lib` | **1723 passed, 0 failed, 2 ignored** (baseline on `main`: 1723 — this build adds no backend test; its behaviour is frontend) |
| `cargo clippy --workspace -- -D warnings` | **clean** |
| `cargo fmt --check` | **clean** |
| `npm run typecheck` | **clean** |
| `npx vitest run` | **847 passed, 63 files, 0 failed** (baseline 829 → **+18**) |
| `npm run build` | **clean** |
| `./scripts/check-migrations.sh` | **clean** — 70 pipeline migrations, no duplicates |
| Route-link guard | **passes** (no route added or removed) |

---

## 2 — WHAT WAS BUILT

### Defect 2 — the stale facet, fixed at its line

`applyRulingToCard` ([cardTriage.ts](frontend/src/components/cardTriage.ts)) now clears `proposed` on include, exclude and defer. A ruling makes the card human-touched, so precedence R-a stops the server projecting it — the patch now agrees with the payload it stands in for. Its doc records the measurement (heading 25 beside facet 27) and restates the rule it broke: the patch maintains *everything the screen reads*, not a fixed list of fields.

**R2 — undo restores the stash.** `LastRuling` carries the card's pre-ruling `proposed`, captured before the patch clears it, and `reopen`/`undrop` put it back. The browser restores what it was served and never invents a proposal — a claim only precedence can make. The next refetch confirms rather than discovers.

### The rider, reframed — every ruling acknowledges itself

New pure module [rulingAcknowledgment.ts](frontend/src/components/rulingAcknowledgment.ts) composes the sentence from stored templates; `useQueueReducer` reports **every** outcome — landed and refused — through one `RulingOutcome` channel, and the queue renders it **on the card it names**, falling back to a strip above the list when that card has left the view.

Three things it says, each answering something measured:

* **it landed** — "Saved — C-14 is now Included."
* **the vanish** — "C-14 has left the Proposed list — that is where ruled candidates go." *This is the sentence whose absence made a working defer read as a dead button.*
* **the locked one-press defer** — "Deferred. The reason recorded is: …" so the human can read the sentence they signed.
* **the refusal** — names the card, the cause, and the fact that the queue reconciled itself.

The link-write failure keeps its own reporter and its own banner: it has no ruled card and no verb, so folding it into a ruling outcome would have been a category error.

### R1 — the reason input moved onto the card

`DeferReasonForm` renders in [CandidateCard.tsx](frontend/src/components/CandidateCard.tsx) directly under the action row of the card being deferred. The old prompt rendered after the card list — below a `maxHeight: 70vh` window — where pressing Defer could open a form a full viewport away. **Placement only**: quick picks, free text, Enter commits, Esc cancels, all still the reducer's, all 106 of its tests unchanged in meaning. Locked cards keep the one-press commit, now visible.

### D3a — the locked card states its condition on its face

`CardHead` renders the stored `card_locked_condition_label` plus the backend's own reason as standing text, not a tooltip. This **deliberately overrides Q4's suppression**, and the comment says why: Q4 was right about clutter and wrong about discovery — a condition reachable only by hovering a disabled button is one most humans never find, and this one is also the *promise* that Defer will work here. The louder tinted variants (unsaved draft, refused keypress) are unchanged.

### D3b — reworded and satisfied

Defer on a locked card already worked [measured]. What was untrue was the screen; it now says so.

### Wording — five rows, one migration

[20260808171630_ruling_acknowledgment_wording.sql](backend/pipeline_migrations/20260808171630_ruling_acknowledgment_wording.sql): saved · left-the-filter · defer-recorded · failed · locked-condition. Four carry required placeholders, registered in `wording_templates` so an edit that drops one is refused. The migration's header records the measurement that produced it.

---

## 3 — TESTS [measured]

**+18**, all behaviour.

| Test | Module |
|---|---|
| `facet_counts_reconcile_after_an_in_page_ruling` — what the browser counts equals what a fresh fetch would report | `cardTriage.test` |
| clears `proposed` on include, exclude AND defer (defer via the real two-press path, asserting the reason survives) | `cardTriage.test` |
| undo restores the proposal the card arrived with (R2) · undo on a never-proposed card invents nothing | `cardTriage.test` |
| a ruling that lands names the card and its new state | `rulingAcknowledgment.test` |
| **SAYS SO when the ruling takes the card out of the list** — the vanish | `rulingAcknowledgment.test` |
| stays quiet about the list when the card stayed put | `rulingAcknowledgment.test` |
| a locked card's one-press defer shows the sentence the human signed · and still says where the card went | `rulingAcknowledgment.test` |
| **a failed ruling surfaces on screen**, naming card and cause · and still reports when the pool no longer holds the card | `rulingAcknowledgment.test` |
| says nothing before the wording loads (R4) · renders the stored template verbatim · leaves an unknown placeholder visible | `rulingAcknowledgment.test` |
| `matchesFilter` gives the same answer the list itself computes, across all seven facets · and is what tells the queue a ruled card has LEFT the proposed list | `candidateFilters.test` |
| `facetLabel` names each facet exactly as its own dropdown option does · falls back to the key rather than leaving a hole in the sentence | `candidateFilters.test` |

**Include/Exclude regression guard:** the 106 reducer tests and the 63-file suite pass unchanged; the include/exclude paths are exercised by the same tests that always covered them, now additionally asserting the cleared `proposed`.

**One test re-scoped, not deleted.** `NO OPTIMISTIC ROWS` in `scenarioPageStructure.test.ts` pinned the literal `"() => onRulingSaved(),"`. The success handler gained a second call, so the literal broke while the law did not. It now asserts the ORDER — `onRulingSaved` appears *after* the write is issued and inside its resolve handler — which is what R3 actually says.

---

## 4 — CORRECTION TO THE BETA.385 REPORT

`CC_REPORT_SCAN_TO_RULING_WORKFLOW.md` claimed *"no module this task touched is over 300"*. **That was wrong for two test modules** [measured]: at `b9d4dde`, `scenario_card_projection_tests.rs` was **313** and `scenario_card_assembly_tests.rs` was **330**. I measured before adding the gate-response tests and did not re-measure after the final amend.

Nothing about the shipped behaviour changes; the record does. Both are still over on this branch (untouched by this build), and `wording_tests.rs` — already over on `main` at 423 — is now 442. All three are test modules; splitting them is a cleanup this task's scope explicitly excludes, so they are **owed, not smuggled**.

---

## 4b — THE §11 GATE

All four agents ran against `296eed1`. **observability-checker: PASS.** **rules-enforcer: FAIL, 1.** **architecture-reviewer: FAIL, 1.** **test-auditor: FAIL, 2.** All four findings were inside this diff; all four are fixed and the commit amended.

* **test-auditor — two untested pure functions.** `facetLabel` was the one that mattered: it has a `?? facet` fallback and its return value NAMES THE LIST in the acknowledgment sentence, so a wrong return would announce the wrong list. It now asserts that every facet's label equals the option its own dropdown renders — two names for one list is the §9 defect in words rather than numbers — plus the fallback. `matchesFilter` is tested for the property that actually matters rather than the delegation: the queue's per-card question and the list's own filtering must agree across all seven facets, or the receipt would announce a card leaving a list it is still in.
* **architecture-reviewer — a convention, taken.** `UNNUMBERED`'s justification sat in a JSDoc block; the house form for a deliberately compiled-in string is an inline `// CONST:` (`cardTriage.ts:64`, `viewerWindow.ts:88`), which is what makes the check mechanical rather than a doc hunt. Converted. It separately confirmed the failure message did NOT lose its action verb — `applyFactAction` already embeds verb, node id, scenario id and HTTP status in the Error, so `{detail}` carries them and `{code}` adds the human handle.
* **rules-enforcer — Rule 13 on a migration path.** `ACKNOWLEDGMENT_MIGRATION` is a relative filename in `backend/src/`, which the rule covers with no test carve-out. Annotated `// STRUCTURAL:` with why it cannot vary per environment: the path IS part of the assertion, because this test reads the seed off disk to prove the fixture matches what the migration seeds — and a declared key with no row is a boot refusal (Rule 21).

**Filed, not fixed:** seven sibling migration-path constants in the same file predate this task and carry no annotation; and `DeferReasonForm`'s two literals ("Why defer this? …", "or type a reason") were literals in `CardQueue` before this build moved the component. Neither is in this task's scope.

---

## 5 — DEPLOYMENT

* **Migration: one**, pipeline database, additive seed, forward-only. All five keys are boot-required, so a roll-FORWARD without this file is a boot refusal; a rollback to an older image is safe.
* **New env vars: none.** No Ansible change owed.
* **Container rebuild: both.**
* **API: unchanged** — no route added, removed, or altered. The payload gains five wording strings on the existing link-options read.

---

## 6 — THE ACCEPTED RESIDUAL (architect's instruction, recorded here)

**The render-mount gap is invisible to this suite by construction.** Rule 30 records that component-test infrastructure (RTL, jsdom) is deliberately not set up, so the queue's reducer has 106 tests and its RENDER has none. That is exactly the gap a working defer fell into: 102 passing tests, a correct write measured in the database, and a screen that said nothing.

What this build can prove and does: the reducer's state transitions, the composed sentences, and the counts reconciling. What it cannot: that the sentence reaches the glass.

**Per the architect's ruling, that half is covered by the standing walk discipline — defer is driven in the browser on every deploy from now on.** Recorded as an accepted residual rather than a solved problem.

---

## 7 — VERIFICATION OWED ON DEV

Roman runs `BUILD_RUNBOOK_BETA_v1` for beta.386. The architect walks:

1. Defer C-13 (locked) — one press, no prompt, and the card now **says** what was recorded and that it has left the Proposed list.
2. Defer an unlocked card — the reason form appears **on the card**, under the buttons; type, Enter, and read the receipt.
3. Rule three cards and watch **every count on the page agree with itself** without a reload.
4. Force a refusal if convenient — the card says so, and the queue reconciles.

=== END REPORT — VERDICT: BUILT ===
