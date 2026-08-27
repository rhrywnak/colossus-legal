=== CC REPORT — DELETE_PURGES_THE_WORKFLOW_ENGINE + 3 — COMPLETION — 2026-08-27 ===

**Branch:** `fix/truncation-terminal-failure` · **Commits:** `04d763f` (truncation, previously reported) + `e6b6af1` (this follow-up) · not merged, not pushed, no version bump, no migration.

---

## 1. DELETE PURGES THE WORKFLOW ENGINE — done, and the orphan is gone

### What was actually wrong — two causes, not one

Restate workflow keys are **single-use**, and the key is the `doc_id`. An orphaned journal therefore does not merely leak; it makes that document **permanently unprocessable**. The audit log holds the entire story in three rows:

| Time | `purge_outcome` |
|---|---|
| 19:11:58 | `success` |
| 19:26:14 | `error: … "The invocation … is not yet completed."` ← **orphan born** |
| 19:41:14 | `skipped_no_id` |

**Cause A — purge cannot touch a running invocation.** Delete at 19:26 hit a live run, Restate answered 409, and the old code folded that into an opaque `Err` — indistinguishable from "Restate is down" — logged it, and proceeded. Postgres row deleted, journal left alive, key held forever.

**Cause B — we threw away the id that would have fixed it.** Restate answers `PreviouslyAccepted` **with the id of the invocation blocking the key**. `process.rs` discarded it and returned 409. That is the 19:41 row: `invocation_id: null`, so DELETE reported `skipped_no_id` while the orphan it needed to purge **was named in the very response it had just discarded**.

The loop you were stuck in: process → 409 → delete to clear → delete's purge 409s → re-upload → process → 409.

### The fix

**A.** `purge_restate_workflow` now returns typed `PurgeResult { Purged, NotFound, NotTerminal }`. A 409 is no longer an error — it is a first-class outcome the caller acts on: **hard-kill the invocation, then re-purge** with a bounded retry.

Kill, not graceful cancel. Restate 1.6.2 offers both; cancel is cooperative and our chunk loop only polls `is_cancelled` *between* LLM calls, so a cancel issued during a multi-minute Opus call is not observed until that call returns. At delete time the document is being destroyed — there is nothing left to unwind for.

**B.** `process.rs` persists the blocking invocation id before returning its 409.

Every outcome stays a distinct observable. `purged_after_kill` is its own audit string, separate from `success`, because "the run had already finished" and "we hard-stopped a run in flight" are different events. Both give-up paths name the invocation and the exact `PATCH` to issue — **in the stored string as well as the log**, because `document_audit_log` outlives log retention.

### The live orphan — purged, verified

```
PATCH http://10.10.100.220:9070/invocations/inv_14eUEIupiTWm1TXTZ2FOm0OZ7SPmlq7HUV/purge  →  HTTP 200
```

It was `status = completed` by the time I reached it — so it *was* purgeable and had simply never been purged, which corroborates Cause A exactly. Verified afterwards:

- `sys_invocation` holds **three** invocations, none for this document; the key is free.
- Postgres residue for `doc-phillips-motion-summary%`: **0 rows** across `documents`, `document_text`, `extraction_items`, `extraction_runs`, `pipeline_steps`.

### What I could NOT verify, and why

**The end-to-end re-process needs your PDF.** Both blockers are provably cleared, but the document row was deleted at 19:41 and step 7 of the delete handler removes the PDF from disk — `/data/documents/` has no `…07-10-2014` file, and `archive-20260724/` holds only YAML profiles. There is nothing on the server left to process. Re-upload from your own copy and it will run; the code fix then prevents recurrence.

Note the deployed DEV backend is still `main` — this branch is not deployed, so the *old* delete behaviour is what is live until it merges.

---

## 2. FAILED STATE EXPOSES THE CONFIG FORM — done

One gate widened in `ProcessingPanel.tsx`: the form now renders for `new` **and** `failed`.

The panel was already built for a non-NEW document — its own `previewDisabled` gate turns preview *on* once text has been extracted. Only the parent was hiding it. Verified there is no status gate on either side of the round trip: `GET /documents/:id/resolved-config` and `PATCH /documents/:id/config` both accept a FAILED document. Saving and pressing Process sends `new_settings`, which clears the previous extraction so the changed setting actually takes effect.

This closes the loop the truncation fix opened: the error now says "Raise max_tokens for this document type in its profile YAML", and the operator can finally do it from the same screen.

---

## 3. CARGO.LOCK — done

Regenerated against `Cargo.toml`. The diff is **one line** — `2.0.0-beta.402` → `2.0.0-beta.413` — with zero dependency movement (`cargo update --dry-run`: "Locking 0 packages"). `main` stops being self-dirtying on every cargo invocation.

---

## 4. READ-AND-REPORT — `motion_pass1_v5_3.md`

**Read only. Not edited.**

### The finding: there is no materiality bar, and the arithmetic makes 64k reachable by design

Grep for `materiality` across the pass-1 templates:

| Template | mentions |
|---|---|
| `court_transcript_pass1` | **6** — a §3 "Materiality — what is NOT worth a node" section, "Completeness is non-negotiable — **within the materiality bar**", and a checklist item that asks "Did I apply the materiality bar — SKIPPING …" |
| `motion_pass1_v5_3` | **0** |
| `appellate_brief_pass1_v5_3` | **0** |
| `complaint_pass1_v5_4` | **0** |
| `court_ruling_pass1_v5_4` | **0** |
| `affidavit_pass1_v5_3` | **0** |

The bar added at `577c9b6` went to the transcript template **only**. The two templates that have now truncated at 32k — appellate and motion — are both in the group without one. This is the same shape as the transcript over-extraction, in a family that never got the fix.

### The specific lines an aggressive model reads as open-ended enumeration

Six splitting/completeness pressures:

- **L16** (opening frame): "connects **every fact, every party, and every piece of evidence**" — a totalizing frame before any rule is stated.
- **L72**: "**Each lettered sub-item is one assertion**"
- **L73**: "**Each paragraph making a distinct claim is one assertion.**"
- **L225**: a quote AND the argument about it = **two** entities — an explicit doubling rule, applied to a document type whose own §2 says "**It quotes the opponent constantly.**"
- **L323**: "Do not merge `A.` through `G.` into one entity"
- **L397 / L400**: "Extract **ALL** Party entities" / "Extract **ALL** Evidence entities"
- **L536 / L544** (checklist): "a separate entity for **each** discrete assertion" · "tag as `recitation` **every** quoted opponent answer, complaint passage, prior brief, and earlier holding"

Against exactly **one** narrow subtractive rule (L83/L203/L373/L548): no entity for bare, unapplied legal-standard boilerplate. Nothing anywhere lets the model judge an assertion **not worth extracting**.

**The most enumeration-inviting artifact is a worked example.** Examples 1 and 2 (L286–302) split **two adjacent sentences** into two entities, with a note driving it home: "**Note this is a SEPARATE entity from Example 1**, though the two sentences are adjacent." Example 2's source span is the seven words *"The Defendant is intentionally sabotaging discovery."* — which the template then expands into a ~230-token JSON entity. That is the granularity the model is shown to imitate.

### The arithmetic — why 64k was reachable without any loop

One Example-3-shaped entity serialized as JSON is **919 characters ≈ 230 output tokens** (14 fields). So:

| | |
|---|---|
| Source | ~5,900 words ≈ **7,900 input tokens** |
| 32,000 cap | ≈ **139 entities** |
| 64,000 cap | ≈ **279 entities** |
| At 64k | one entity per **~21 words** of source |
| Output ÷ input at 64k | **~8×** |

Each entity restates its own quoted span about **2.5×** — `verbatim_quote`, then `title`, `summary`, and `significance` all re-say it. So a fully-enumerated motion emits several times its own input, by construction. At 5,900 words, exhaustive enumeration under these rules lands **at or beyond 64k**.

**This was not a runaway loop. It was the template's own cost model meeting a cap.**

### What follows (no edit made, your call)

Raising motion to 64k was necessary and buys real headroom, but it **doubles the room without touching the mechanism** — the next dense motion re-truncates. The durable fix is the one already proven on transcripts: a materiality bar giving the model explicit permission to skip an assertion not worth a node, plus a checklist item that asks whether it was applied. I did not touch the template, per instruction. Note the same gap sits in appellate, complaint, court_ruling, and affidavit.

---

## GATES

| Gate | Result |
|---|---|
| `cargo test --lib` | **2646 passed / 0 failed / 3 ignored** (15 new) |
| `cargo build` | clean |
| `cargo fmt --check` | clean |
| `npm run typecheck` · `npm run build` | clean |
| `npx vitest run` | **1389 passed / 99 files** |
| `cargo clippy --lib --tests` | 13 warnings — **all pre-existing on main**, verified individually; the `config.rs` one only shifted line number because I inserted above it |
| `Task(rules-enforcer)` | **PASS** (after 2 rounds — see below) |
| `Task(architecture-reviewer)` | **PASS** (after 2 rounds) |
| `Task(test-auditor)` | **PASS** (after 2 rounds) |
| `Task(observability-checker)` | **PASS** (1 finding, fixed) |

**The gates caught six real things. All six were mine and all six are fixed:**

1. **observability** — the kill-removed-it-outright path returned silently while every other exit logged. An operator would read "killing it" and then nothing.
2. **test-auditor** — three branches were structurally unreachable by the test responder (kill fails / post-kill 404 / post-kill 500). The responder took a single "how many 409s" count, so it could never drive kill and purge independently. Rewritten to `(kill_status, purge_statuses[])`; all three now covered.
3. **rules-enforcer + architecture** — `PURGE_RETRY_ATTEMPTS` / `PURGE_RETRY_DELAY` were compiled constants. Both agents were right: they are deployment values. Now `RestatePurgePolicy`, env-driven.
4. **architecture** — the kill-failure path stated the outcome but not the remedy; the post-kill-purge-error path omitted the invocation id too. Both now carry the exact `PATCH` calls.
5. **rules-enforcer** — the `Default` impl literals needed collocated `// DEFAULT:` markers naming their env vars.
6. **self-caught before the gates** — `delete_restate_purge.rs` hit **342** lines. Tests split to `delete_restate_purge_tests.rs`; runtime is now **194**.

**One agent argument I accepted rather than defended.** I first justified a new `RESTATE_KILL_TIMEOUT` as the fourth of four identical sibling constants. The rules-enforcer rejected that in advance — "adding a new constant that conforms to a pre-existing (and also-violating) pattern extends the debt rather than paying it down" — and it was right. Resolved by **deletion**: kill now shares `RESTATE_PURGE_TIMEOUT`, since a kill and the purge that follows it are two halves of one operation, so a second constant would have been a knob that must always turn in lockstep with the first. The diff introduces **no new `const` or `static` at all**.

---

## OWED / NOTED

1. **TWO ENV VARS OWED TO `colossus-ansible`** — `RESTATE_PURGE_RETRY_ATTEMPTS` (4) and `RESTATE_PURGE_RETRY_DELAY_MS` (250). One-repo-per-instruction, so the template change is deliberately not here. **They join the three Theme Scan vars already owed — five now, one instruction.** Unlike those three, these go through `parse_env_or`, so a **typo in the template refuses to boot** rather than degrading silently. Worth knowing before editing.

2. **`process.rs` `PreviouslyAccepted` persist path is not unit-tested.** Handler-level tests need an AppState + Postgres fixture that does not exist in this repo — a pre-existing constraint the test-auditor accepted as a documented limitation, not a blocking gap. The path is not a silent failure: it logs at error level with the operational consequence spelled out.

3. **`delete_restate_purge_tests.rs` is at 299 lines.** Under Rule 17, but one test from tipping over. Whoever adds the next one splits the file.

4. **One flaked test run.** Mid-session a single `cargo test --lib` reported `2642 passed; 1 failed`; the next four runs were clean and I did not capture the name. Consistent with the known `registry_tests` env-var race (tracker 2.7), but I am not claiming that identification — I did not see it.

5. **Three pre-existing Rule 13 siblings remain** — `RESTATE_CANCEL_TIMEOUT`, `RESTATE_PURGE_TIMEOUT`, `RESTATE_INVOKE_TIMEOUT`, all `Duration::from_secs(10)` with `// CONST:` markers. Both agents named them as pre-existing debt. If the family ever becomes configurable it should become ONE setting covering all of them.

6. **Parked, per your ruling:** chunked-path truncation tolerance. Untouched.

---

## NOT DONE, BY INSTRUCTION

Not merged. Not pushed. No version bump, no tag. `motion_pass1_v5_3.md` **not edited** — item 4 was read-and-report. `backend/twin_merge_human_queue.txt` was untracked before this session and remains untracked.

=== END REPORT — VERDICT: PASS ===
