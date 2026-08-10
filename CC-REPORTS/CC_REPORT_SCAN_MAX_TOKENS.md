=== CC REPORT ===

# CC_REPORT_SCAN_MAX_TOKENS — build report

**Task:** CC_TASK_SCAN_MAX_TOKENS_SETTINGS_v1, as ruled in
CC_TASK_SCAN_MAX_TOKENS_PHASE_B_AUTHORIZATION_v1 (R1–R6 + Piece 2a).
**Branch:** `fix/scan-max-tokens-settings`, cut from `main` at `0387ba5`.
**Commit:** `c6332e4` (one commit, amended once after the §11 gate).
**Date:** 2026-08-09. Rides as **beta.389** — no version bump by CC.

**[measured]** = a command or query produced it. **[read]** = the cited line says so.

---

## 0. READ THIS FIRST — a behaviour change for the Qwen models, on a premise I got wrong

**`constrain` is clamp-by-ERROR, not clamp.** `max_tokens > ceiling` returns
`LlmConfigError::MaxTokensExceedsCeiling` and the scan **refuses to start** **[read**
`domain/llm_params.rs:656`, error at `:470` **]**:

> "requested max_tokens {requested} exceeds output ceiling {ceiling} for model
> '{model_id}' — lower max_tokens in the task spec or the model-default configuration"

Both vLLM rows carry `max_output_tokens = 2048` **[measured]**. So with this row at
**8192, a scan judged by either Qwen model now fails at parameter resolution** —
where the old compiled-in 512 sailed under the ceiling and worked.

**My Phase A report said the opposite.** It stated that 8192 "clamps to 2048" for
Qwen and called it correct behaviour needing no note, and **ruling R6 was made on
that premise**. The premise was wrong; observability-checker caught it by reading
the `constrain` source rather than my summary of it.

**What follows from it:**

- The failure is loud, named, and recoverable **without a rebuild** — the operator
  lowers the row on the Settings page, or raises the model's `max_output_tokens` if
  the server genuinely allows more. That is exactly the decision this row now
  exists to let a human make, so the system behaves correctly. It is the *default
  value* that is incompatible with two registry rows.
- The wrong claim is corrected in the migration, which now carries an explicit
  "read this before running a Qwen scan" warning instead of the clamp sentence.
- **8192 was left as seeded** — R6 said it stands, and changing it (or changing
  `constrain`'s semantics, or adding a per-model cap) is a design decision, not
  mine to take mid-build. **This needs your call before an Opus-5-and-Qwen mixed
  workflow.**

---

## 1. What shipped

**R1/R2 — the row.** `theme_scan_max_tokens`, seeded **8192**, `value_kind 'count'`,
bounds **256..=64000**, `consumed_by` NULL so it sorts into the LIVE group. The
floor sits above the largest observed successful reply (575 chars ≈ 150 tokens)
with room to think; the ceiling is the lowest Anthropic `max_output_tokens` in the
registry, so a value `constrain` would reject cannot be typed in.

**R3 — the cap is an ARGUMENT.** `scan_task_spec(max_tokens: u32)`. The function
cannot reach the settings store, so the only way a number enters the task layer is
for `resolve_scan_provider` to have read one — and a compiled-in default cannot
return without changing a signature the tests pin. Both named tests sweep the row's
whole legal span (256, 512, 4096, 8192, 64000) rather than checking one value, so a
smuggled `max(512, n)` or a clamp fails at the low end.

**R4 — frozen.** `temperature: Set(0.0)` and `timeout_secs: Unset` are
byte-identical.

**R5 — the comment.** The deleted constant's reasoning is **quoted**, the D2b
decision cited rather than erased, and the measurement that falsified it named with
the run id, the counts and the counter-intuitive tell (the failed replies were
*shorter* than the successful ones, because the loss was upstream of the text).

**Piece 2a.** `CC_REPORT_BAKEOFF_SCORECARD.md` rides this branch, as does the
Phase A report.

---

## 2. One thing found during the build that was not in the analysis

**`settings_store.rs` was already over Rule 17 on main** — **304** non-comment lines
before this task touched it **[measured]**, and the new key pushed it to 315. The
per-row readers are now `settings_row_readers.rs`, split on a real subject boundary
rather than an arbitrary cut: everything there answers *"what does this ONE row
say?"*, while the parent answers *"does the whole store make a usable snapshot?"* —
which is why the cross-row band invariant and the key lists stayed behind.

Parent is now **260** — better than it was on main. Visibility carries the boundary:
`pub(super)` for the parsers, and `require` / `text_of` re-exported through
`settings_store` so `settings_wording`'s import line did not have to change for an
internal reorganisation.

---

## 3. Verification [measured]

| Gate | Result |
|---|---|
| `cargo test --lib` | **1754 passed, 0 failed, 2 ignored** |
| `cargo fmt --check` | clean |
| `cargo clippy --lib --bins --tests` | no warnings attributable to this diff |
| `cargo check --bins` | clean |
| `scripts/check-migrations.sh` | clean — 73 pipeline migrations, no duplicate versions |
| `npx tsc --noEmit` | clean |
| `npx vitest run` | **880 passed** (no frontend change in this build) |
| Rule 17 on every touched production module | at or under 300 |

`cargo test --workspace` remains broken by stale files under `backend/tests/` that
reference a removed `AppState` field. Predates this work; no file under `tests/` is
in this diff. Three clippy warnings remain in the lib target, all in files **not**
in this diff (`scenario_dashboard.rs:456`, `scenario_code.rs:192`,
`scenario_proposal_lookup.rs:162`).

### The named tests

| Test | Where |
|---|---|
| `the_scan_judge_cap_is_read_from_settings_at_scan_start` | `theme_scan_provider.rs` |
| `no_compiled_in_cap_remains_on_the_judge_call_path` | `theme_scan_provider.rs` |
| `a_missing_cap_row_refuses_boot` | `settings_store_tests.rs` |
| `a_cap_too_large_for_the_wire_is_refused_rather_than_wrapped` | `settings_store_tests.rs` (added at the gate's request) |

### §11 gate agents

Run against `0737b40`, then a targeted re-pass against the amended `c6332e4`.

| Agent | First pass | Finding | Action |
|---|---|---|---|
| rules-enforcer | **PASS** | ruled the 773-line `settings_store_tests.rs` is **not** a Rule 17 violation — the file is entirely `#[cfg(test)]`, so its production count is zero | question settled, no split |
| architecture-reviewer | **PASS** | endorsed the module split as a real subject boundary, not a size cut | — |
| observability-checker | **PASS** | **the `constrain` clamp claim is false** (§0) | migration comment corrected |
| test-auditor | **FAIL** | the `u32::try_from` overflow branch in `token_count_of` was untested | test added; re-pass **PASS** |

Both FAIL-class findings were real and mine. test-auditor's argument was the
correct one: I justified handling that branch with *"the bound is DATA — a `psql`
edit can widen it"*, and a branch whose justification is "a data change can reach
this" is a branch a test has to reach now, or the justification is decoration.

It also caught **my own doc comment overstating its coverage** as "end to end" when
`resolve_scan_provider` has no integration test. The comment now says precisely
what the chain proves and names the one link it does not: the six lines that read
`state.settings.current()` rest on review, not on a test.

---

## 4. Deferred — recorded, not smuggled

1. **The Qwen ceiling collision (§0)** — needs your ruling, not a code change.
2. **`settings_boot` does not name the new cap in its startup-success log.**
   Several sibling parameters are called out there; this one is not, so the boot
   log alone cannot confirm which budget booted. Closed per-run by
   `scan_runs.resolved_params`. `settings_boot.rs` was not touched by this build.
3. **`resolve_scan_provider` has no integration test** — it needs a database, a
   graph and a registry. Named honestly in the test's own doc comment.
4. **`cargo test --workspace`** — the stale `tests/*.rs` files, still owed.
5. **The dead `temperature: Set(0.0)` line** in `scan_task_spec` — filed by R4 for
   the next pass through that file, not this build.

---

## 5. Deployment

- **New env vars:** none. **Ansible template:** unchanged. **Frontend:** unchanged
  — the Settings page lists every stored row and renders its `meaning`
  automatically.
- **Migration:** one, pipeline DB (`colossus_legal_v2`). One `app_settings` row, no
  DDL. Forward-only, idempotent.
- **Deploy ordering:** `theme_scan_max_tokens` is declared to the boot loader, and a
  declared key with no row makes the backend **refuse to start** — there is no
  compiled-in default left, which is the point of the change. The runtime Migrator
  applies this at boot before the settings load, so a normal deploy orders itself.
  Rollback to an older image is safe; roll-forward without this file is not.
- **Rebuild:** backend only.
- **Rollback:** `git revert c6332e4` restores the compiled-in 512 — and with it the
  truncation that killed 7 of 104 verdicts. The extra `app_settings` row is ignored
  by an older image.
- `Cargo.lock` carries a one-line change syncing `beta.387` → `beta.388` from your
  version bump. A lock catching up, not a bump.

**Step-6 probe**, as authorized: an Opus 5 scan on S-4 with **zero
truncation-class failures** — the 512-class cut-off extinct. Any failure that does
appear should carry a stored reason that is *not* a mid-JSON cut-off.

**STOPPING HERE.** Nothing pushed, no version bumped, no deploy attempted, and the
Qwen question in §0 is open.

=== END REPORT — VERDICT: COMPLETE ===
