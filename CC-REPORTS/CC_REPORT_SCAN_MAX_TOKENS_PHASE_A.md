=== CC REPORT ===

# CC_REPORT_SCAN_MAX_TOKENS_PHASE_A — pre-coding analysis

**Task:** CC_TASK_SCAN_MAX_TOKENS_SETTINGS_v1 (rides as beta.389)
**Branch:** `fix/scan-max-tokens-settings`, cut from `main` at `0387ba5`
(v2.0.0-beta.388 — the temperature/failed-honesty work is merged and on main).
**Working tree:** clean apart from `CC-REPORTS/CC_REPORT_BAKEOFF_SCORECARD.md`,
untracked — that is Piece 2a's file and it rides this branch by design.
**Date:** 2026-08-09. No code written, no file modified.

**[measured]** = a command or query produced it. **[read]** = the cited line says so.
Short, per the instruction.

---

## 1. Where the 512 lives — one constant, one use site [measured]

```
src/services/theme_scan.rs:72       pub const THEME_SCAN_MAX_TOKENS: u32 = 512;
src/services/theme_scan_provider.rs:69   max_tokens: ParamValue::Set(THEME_SCAN_MAX_TOKENS)
```

`grep -rn THEME_SCAN_MAX_TOKENS src` returns exactly those two lines plus one
doc-comment mention. It enters the resolver as the **TASK layer** of
`scan_task_spec()`, which `resolve_scan_provider` merges and constrains.

**The constant's own comment is the thing this task falsifies** — it currently
reads **[read]**:

> "the verdict token budget is a fixed protocol shape, **not a deployment knob**.
> A verdict is a tiny four-key JSON object; 512 is a generous ceiling that would
> only ever change if the verdict SHAPE changes … Roman pinned this as a named
> constant (no env) in the D2b decision."

That reasoning was sound for a model whose output was only the verdict. It stopped
being true when the judge became a model that thinks inside the same budget. The
comment does not merely need updating — its *premise* is what the Aug 9 measurement
overturned, and the replacement should say so rather than quietly disappearing.

## 2. Sibling compiled-in caps — none on the scan path, one worth naming [measured]

`grep -rn MAX_TOKENS src` finds four other caps. Three are **not** on the judge
call path: `CHAT_MAX_TOKENS = 4096` (`main.rs`, chat), `FALLBACK_MAX_TOKENS = 8000`
(`pipeline/providers.rs`, corrupt-row guard for extraction),
`DEFAULT_CHUNK_MAX_TOKENS = 8000` (`steps/llm_extract.rs`, extraction).

The fourth **is** on the path and deserves a sentence:

```
src/domain/llm_params.rs:84    pub const DEFAULT_MAX_TOKENS: u32 = 8000;
```

This is the resolver's **SYSTEM layer** — what a scan would get if no layer set
`max_tokens`. It never fires today because the task layer always sets 512. Note
what that means: **the system default is already 8000, within rounding of the 8192
this task wants.** The scan has been the only thing overriding it downward.

## 3. The settings plumbing — an exact sibling already exists [read]

`theme_scan_prefilter_min_chars` is the same shape as the row this task needs
(a `count` read at scan start), and `theme_scan_prompt_file` is the same *lifecycle*
(read at scan start, not cached, boot-asserted). Both are in `REQUIRED_KEYS`.

| Piece | Where | Sibling to copy |
|---|---|---|
| Key constant | `services/settings_store.rs:81` | `KEY_PREFILTER_MIN_CHARS` |
| Boot assertion | `REQUIRED_KEYS` (15 entries today) | same list |
| Parse | `count_of()` → `usize`, kind `count`, honours `min_value`/`max_value` | same |
| Snapshot field | `domain/settings.rs` | `theme_scan_prefilter_min_chars: usize` |
| Seed | one migration row, `value_kind = 'count'` | `20260808084539_…` |
| **Settings page** | **nothing to build** | the page lists **every** stored row and renders its `meaning`; `consumed_by IS NULL` sorts it into the LIVE group **[read** `app_settings.rs:69`, `api/settings.rs:72` **]** |

So Piece 1b is: one key, one `REQUIRED_KEYS` entry, one `Settings` field, one
migration row, one threaded argument, and the constant's deletion. **No frontend
change** — which is worth stating plainly, because the task's phrase "admin
Settings page surface" could be read as UI work and it is not.

## 4. Two mechanical points that need deciding before code

**4a. `usize` → `u32`.** `count_of` returns `usize`; `max_tokens` is `u32`. Standing
Rule 1 forbids an `as` cast here — a 64-bit row value silently truncating to a
32-bit cap is precisely the class of bug this task exists to end. It becomes a
`u32::try_from` with a typed error naming the key and the value, refusing at the
read. The row's `max_value` bound makes that path unreachable in practice; it is
written anyway because the bound is data and the cast is code.

**4b. `constrain` still clamps to the model's ceiling, and that matters here.**
Measured `llm_models` on DEV:

| Model | `max_output_tokens` | 8192 survives? |
|---|---|---|
| claude-opus-5 / 4-8 / 4-6, sonnet-5 / 4-6 | 64000 | yes |
| claude-opus-4-7 | 128000 | yes |
| **Qwen 14B / 32B (vllm)** | **2048** | **no — clamped to 2048** |

This is correct behaviour and needs no change: the clamp is what `constrain` is
for, and the clamped value is already recorded per-run in
`scan_runs.resolved_params` **[measured** — the Aug 9 run's snapshot shows
`"max_tokens": 512` **]**. But it means **raising the row to 8192 does not raise
the Qwen models above 2048**, and if a local model ever truncates the same way, the
fix is its registry row, not this setting. Recorded so nobody debugs it twice.

## 5. Plan

| # | Change | File |
|---|---|---|
| 1 | New `count` key + `REQUIRED_KEYS` entry + `Settings` field | `services/settings_store.rs`, `domain/settings.rs` |
| 2 | `scan_task_spec(max_tokens)` takes the resolved value; `resolve_scan_provider` reads `state.settings.current()` (it already holds `&AppState`) | `services/theme_scan_provider.rs` |
| 3 | Delete `THEME_SCAN_MAX_TOKENS`; replace its comment with what the measurement showed | `services/theme_scan.rs` |
| 4 | Migration: one row, value **8192**, `value_kind 'count'`, bounds, plain-language `meaning` | new pipeline migration |
| 5 | The three named tests | `theme_scan_provider`, `settings_store_tests`, + see Q3 |
| 6 | Commit `CC_REPORT_BAKEOFF_SCORECARD.md` (Piece 2a) | — |

**Migration↔struct table** (standing rule) — no DDL, one INSERT:

| Statement | Table.column | Struct field | Type |
|---|---|---|---|
| `INSERT INTO app_settings … 'theme_scan_max_tokens', '8192', 'count'` | `app_settings.value` | `Settings.theme_scan_max_tokens` | `usize` → `u32` at the resolver boundary |

**Standing rules:** no silent failures — the row is boot-asserted by name, the
narrowing is a typed error, and the resolved value is already recorded per run.
No hardcoded values — this task *removes* one and adds no other. Tutorial comments
— planned on the narrowing (`## Rust Learning: try_from over as`) and on why the
cap is read per scan rather than cached.

---

## 6. Open questions

**Q1 — the row's bounds.** I propose `min_value = 256`, `max_value = 64000`. The
floor is above the largest observed successful reply (575 chars ≈ 150 tokens) with
room for thinking; the ceiling is the lowest Anthropic `max_output_tokens` in the
registry, so a value the resolver would silently clamp cannot be typed in. Say if
you want it open-ended instead — every other `count` row except two leaves
`max_value` NULL.

**Q2 — `consumed_by`.** NULL (live) is right: this parameter is read on every scan.
Confirming because it is the column that decides whether the row shows in the LIVE
or DORMANT group on the Settings page.

**Q3 — how `no_compiled_in_cap_remains_on_the_judge_call_path` should be written.**
The constant is deleted, so no test can name it. The honest options:

  a. **Signature test** — `scan_task_spec` takes the cap as an argument, so a
     compiled-in value cannot be reintroduced without changing a signature the
     other tests pin. Cheap, and it proves the *shape* rather than the absence.
  b. **Source-scanning test** (Rule 21 disk/code pattern, as used for
     "no profile YAML carries `synthesis_model:`") — read `theme_scan.rs` and
     `theme_scan_provider.rs` and assert no numeric literal is bound to
     `max_tokens`. Proves the actual claim; brittle to formatting.

I recommend **(a) plus a one-line grep-style assertion inside it** naming the two
files, which gets most of (b)'s value without a regex over source text. Your call.

**Q4 — scope check.** The task says "Nothing else on the call changes." I read that
as: `scan_task_spec`'s `temperature: Set(0.0)` and `timeout_secs: Unset` stay
exactly as they are, even though the temperature line is now known to be
belt-and-braces after beta.388. Confirming rather than assuming.

---

**Read-only note:** two `SELECT`s against the DEV pipeline database (the model
registry and the existing `count` rows), no writes, no DDL. No file in `backend/`,
`frontend/` or `colossus-rs` was modified. The branch was cut and is empty.

**STOP. Awaiting rulings on Q1–Q4 before any code.**

=== END REPORT — VERDICT: STOPPED ===
