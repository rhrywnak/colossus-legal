# CC_REPORT_MODEL_PARAMS_HONESTY_PHASE_A — pre-coding analysis

**Task:** CC_TASK_MODEL_PARAMS_AND_FAILED_HONESTY_v1 (hotfix, rides as beta.388)
**Branch:** `fix/model-params-and-failed-honesty`, cut from `main` at `1bab520`
(v2.0.0-beta.387 — the one-card-grammar work is merged).
**Date:** 2026-08-09 · **Type:** Phase A. No code written, no file modified.

**[measured]** = a query or a command produced it. **[read]** = the cited line says so.
Kept short per the instruction.

---

## 1. The headline: Piece 1 is almost entirely already built

The per-model temperature capability the task asks me to create **exists**, and the
provider already omits the key correctly. **[read]**

| Layer | Where | What it does |
|---|---|---|
| DB column | `llm_models.temperature_mode` (+ `default_temperature`) | `'omit'` / `'zero-ok'` / NULL |
| Typed | `domain::llm_params::TemperatureMode` (`:164`) | `Omit` / `ZeroOk` / `Unknown` |
| Derivation | `domain::llm_params::construction_temperature` (`:239`) | row → `Option<f64>` |
| Wire | `AnthropicProvider::build_body` (colossus-rs) | `if let Some(temp)` — **`None` omits the key entirely** |

`construction_temperature`'s own doc says it exists *because* the old hardcoded
constant "sent `temperature = 0` to EVERY Claude model — 400-ing
temperature-deprecated ones (e.g. `claude-opus-4-7`)". The mechanism was built for
exactly this failure. It did not fire.

### Why it did not fire — measured

**[measured]** the whole registry on DEV:

```
 claude-opus-4-6      | zero-ok
 claude-opus-4-7      | omit
 claude-opus-4-8      | omit
 claude-opus-5        |            ← NULL
 claude-sonnet-4-6    | zero-ok
 claude-sonnet-5      | omit
 Qwen…14B / …32B      | zero-ok (default_temperature 0.00)
```

**`claude-opus-5` is the only Anthropic row with no mode.** Every sibling that
needs one has `omit`. The row was added and the mode was never set.

And NULL is not inert. `construction_temperature:244` **[read]**:

```rust
TemperatureMode::ZeroOk | TemperatureMode::Unknown => record
    .default_temperature
    .or(Some(ZERO_OK_DEFAULT_TEMPERATURE)),   // → Some(0.0)
```

**NULL means "send 0.0".** So an un-onboarded model gets a temperature key it may
reject — 104 × HTTP 400 in five seconds. This is the Standing-Rule-1 defect
underneath the incident: *not recorded* is being treated as *safe to send*, and
Piece 1b's ruling says the opposite ("NULL/empty = the parameter is NOT SENT").

Note the contrast with the sibling capability: `StructuredOutputMode::Unknown` is
documented as treated **conservatively** — "NOT assumed capable just because
nobody said otherwise" (`:252`). Temperature's `Unknown` does the reverse.

**So Piece 1 is not "add a column". It is:** flip the `Unknown` default to omit ·
set `claude-opus-5` to `'omit'` · surface the field on Admin → Models.

## 2. Piece 1d — the feared parse failure does not exist

**[read]** `colossus-extract/src/providers/anthropic.rs:345-352`:

```rust
let text: String = parsed.content.iter()
    .filter(|block| block.kind == BLOCK_KIND_TEXT)
    .filter_map(|block| block.text.as_deref())
    .collect::<Vec<_>>().join("");
```

It selects **text blocks by kind and joins them** — there is no first-block
assumption. Thinking blocks are skipped by construction. An all-thinking response
raises a named error ("no text content blocks"), not a silent empty.

Confirmed against the API reference: on Claude Opus 5 `thinking.display` defaults
to `"omitted"`, so thinking blocks arrive with empty text but are still present —
this parser was already correct for that shape. **No change needed. No fixture
test needed beyond one that pins the behaviour.**

## 3. This task stays in ONE repo

The provider lives in `colossus-rs`, which CLAUDE.md rule 3 forbids me touching in
the same instruction. **It does not need touching:** 1b's fix is a Rust default +
a migration + an admin field, all in `colossus-legal`; 1d needs nothing. Flagged
because it would have been a hard stop if either fix had landed one layer down.

## 4. Piece 2 — the counts were right; the screen lied

**[measured]** the run history:

```
 run_id    | status    | model         | read | relevant | irrelevant | failed
 6a9fad89… | completed | claude-opus-5 |  148 |        0 |          0 |    104
 08baef15… | completed | claude-opus-4-8 | 148 |     30 |        117 |      1
```

**2a.** `scan_runs.failed_count` exists and recorded **104**. `finalize_scan_run_completed`
writes it **[read]**. The report omitted it because the report's tiles and
conservation line are built from `ScanConservation` (`dto/theme_scan.rs:345`),
whose six fields describe the **pre-filter only** — `pool`, three `excluded_*`,
`duplicates_collapsed`, `judged`. **There is no `failed` field on it**, so the
judge's failures were never on the wire to that surface.

The arithmetic is therefore visibly broken on screen and nothing said so:
`judged = 104` while `relevant + irrelevant = 0`. That is the §9 reconciliation
defect in its purest form.

**2c — confirmed, and worse than expected.** `fetch_projecting_run` binds
`SCAN_STATUS_COMPLETED` **[read]**, so a `failed` run cannot project and cannot
supersede. That half is a test, not a change — as the instruction predicted.

But the dead run is `completed`, so it **did** project: it took the
latest-completed slot from run `08baef15` (Opus 4.8, **30 relevant**) and projected
nothing. **S-4's queue silently lost 30 proposals.** Fixing the status is not only
honesty — it restores those proposals, which is a checkable acceptance outcome.

## 5. Other compiled-in sampling params — named, not converted

**[measured]** `grep -rn temperature`: the only sampling parameter anywhere is
`temperature`. **No `top_p`, `top_k`, `budget_tokens`, or `thinking` config exists
in either repo.** Nothing else to convert, and nothing else that would 400 on a
Claude 5 model.

## 6. Two risks the instruction does not name

1. **`max_tokens` now caps thinking + answer together.** Claude Opus 5 runs
   adaptive thinking when `thinking` is omitted (Opus 4.8 did not). The scan pins
   `max_tokens` to `THEME_SCAN_MAX_TOKENS` for a verdict-sized JSON reply. Once the
   400s stop, calls may truncate mid-verdict instead — trading 104 400s for N parse
   failures by a different route than 1d feared. **Worth a probe on the first
   re-run**; I would not change the cap blind.
2. **`stop_reason: "refusal"`.** Opus 5's classifiers can decline a request with
   HTTP 200 and empty content. This corpus is fraud/abuse-of-process litigation.
   The parser would surface that as "no text content blocks" — an honest error, so
   it fails loudly rather than silently. No change proposed; recorded.

---

## 7. Plan (small, one build)

| # | Change | Files |
|---|---|---|
| 1 | `TemperatureMode::Unknown` → omit, with the reasoning | `domain/llm_params.rs` |
| 2 | Migration: `claude-opus-5` → `temperature_mode = 'omit'` | new pipeline migration |
| 3 | `temperature_mode` on Admin → Models, plain-words description | `AdminModels.tsx`, `services/admin.ts`, DTO |
| 4 | `ScanConservation` gains `failed`; tiles + conservation line render it when nonzero | `dto/theme_scan.rs`, `theme_scan*.rs`, `ThemeScanPanel.tsx`, wording rows |
| 5 | All-failed run finalizes as `failed`, not `completed`; collapsed summary says so | `scan_runs.rs` / `theme_scan.rs`, wording rows |
| 6 | Tests per the instruction's list | Rust + vitest |

**Migration↔struct table** (standing rule) — the only schema touch is a data
UPDATE, no DDL:

| Statement | Table.column | Struct field | Type |
|---|---|---|---|
| `UPDATE llm_models SET temperature_mode='omit' WHERE id='claude-opus-5'` | `llm_models.temperature_mode` | `LlmModelRecord.temperature_mode` | `Option<String>` (unchanged) |

Wording rows owed: `failed` tile caption, the conservation line's failed clause,
the FAILED status word, the collapsed-summary failed sentence, and the admin
field's description. All seeded in the same migration as their declaration.

---

## 8. Rulings needed

**Q1 — the `Unknown` default (the systemic half).** Flip NULL/Unknown to **omit**?
It matches Piece 1b's ruling and the `StructuredOutputMode` precedent, and it makes
the next un-onboarded model fail safe. The cost: a new `zero-ok` model that relied
on the implicit 0.0 would stop getting it until its row says so — for extraction
determinism that is a real behavioural change. **I recommend flipping it**, and
setting the two Qwen rows explicitly (they already carry `default_temperature 0.00`,
so they are unaffected either way).

**Q2 — when is a run FAILED?** The instruction says "a run whose judged calls ALL
failed". Literally: `failed_count == judged && judged > 0`. Confirm that, or do you
want a threshold (e.g. any run with zero relevant AND any failures)? I recommend
the literal reading — it is unambiguous and cannot mislabel a run that did real work.

**Q3 — where does `failed` live on the wire?** `ScanConservation` currently means
"what the pre-filter did", and judge failures are a different stage. Add `failed` to
it anyway (one block, one arithmetic identity), or carry it as a sibling field?
**I recommend adding it** — the human reads one line and it must reconcile.

**Q4 — Piece 1b's admin field.** The task says "temperature becomes a nullable
per-model column … surfaced as an optional field". Given the column is
`temperature_mode` (a token) plus `default_temperature` (a number), do you want
**both** on the form, or just the mode? I recommend both, with the mode leading.

---

**Read-only note:** three `SELECT`s against the DEV pipeline database, no writes,
no DDL. No file in `backend/`, `frontend/`, or `colossus-rs` was modified and no
branch beyond the empty `fix/model-params-and-failed-honesty` was created.

**STOP. Awaiting rulings on Q1–Q4 before any code.**
