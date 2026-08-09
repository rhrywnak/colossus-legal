=== CC REPORT ===

# CC_REPORT_MODEL_PARAMS_HONESTY — build report

**Task:** CC_TASK_MODEL_PARAMS_AND_FAILED_HONESTY_v1, as amended by
CC_TASK_MODEL_PARAMS_PHASE_B_AUTHORIZATION_v1 (rulings R1–R7).
**Branch:** `fix/model-params-and-failed-honesty`, cut from `main` at `1bab520`.
**Commit:** `82674ea` (one commit, amended once after the §11 gate).
**Date:** 2026-08-09. Rides as **beta.388** — no version bump by CC.

**[measured]** = a command or query produced it. **[read]** = the cited line says so.

---

## 1. What shipped, ruling by ruling

### R1 — the unsafe default dies

`domain/llm_params.rs`. `TemperatureMode::Unknown` resolved to `Some(0.0)`: "nobody
recorded what this model does" was being read as "safe to send it a zero". It now
resolves to **send nothing**, matching the `StructuredOutputMode::Unknown`
discipline the sibling capability already followed.

**Both paths were defective, and only fixing both fixes the incident.** Phase A
named `construction_temperature`; the theme scan does not use it. The scan pins
`Set(0.0)` in its task spec and goes through **`constrain`**, whose
`ZeroOk | Unknown => resolved.temperature` arm was the one that actually put
`temperature: 0` on the wire for run 6a9fad89. Both arms now split `Unknown` off
to `None`. A fix to `construction_temperature` alone would have passed review,
passed its tests, and left the 400s exactly where they were.

Two existing tests **pinned the defect** and were rewritten rather than deleted —
`construction_temperature_null_mode_behaves_like_zero_ok` and
`unknown_temperature_mode_preserves_resolved_temperature` both asserted that an
unrecorded model gets a zero. They are now
`an_unconfigured_model_omits_temperature` and
`…_through_the_constraint_pass`, one per path, each with an anti-vacuity check.

### R2 — the `claude-opus-5` row

Migration `20260809153630_seed_opus_5_temperature_mode_and_failed_honesty_wording.sql`
sets `temperature_mode = 'omit'`, guarded `AND temperature_mode IS NULL` so a
human's value stands.

**The interim was NOT applied — see §5.** It was blocked, and I did not work
around the block.

### R3 — an all-failed run records FAILED

`repositories/pipeline_repository/scan_runs.rs`. New pure predicate
`final_status(&ScanRunFinal)`: `attempted > 0 && failed == attempted` → `failed`,
everything else → `completed`. `finalize_scan_run_completed` binds it instead of
the constant it used to hardcode.

The honesty and the recovery are **one change**: the projection query binds
`SCAN_STATUS_COMPLETED` **[read]**, so a run that records `failed` is invisible to
it — the previous good run keeps the projecting slot and its proposals stay in the
queue. That is why S-4 lost 30 proposals on 2026-08-09: the dead run recorded
`completed` and won the slot.

Three shapes must still record `completed`, and each is asserted: one dead call
out of 148 (the Aug 7 run), 124 judged with none relevant, and an empty pool. A
predicate that returned `failed` for everything would satisfy the headline
assertion and break every real run *silently*, because a `failed` run simply stops
projecting.

**No `error` sentence is frozen onto the row.** The reason is a number already in
its own column, now rendered; and the per-call causes are stored verbatim per
candidate in `scan_run_verdicts.error` by `handle_failed`, so nothing is lost.

### R4 — failed on the wire, and the reconciliation law

`ScanConservation` gains `failed`. This is the field whose absence caused the
screen to lie: `scan_runs.failed_count` had recorded 104 all along and
`ThemeScanSummary.failed` carried it, but the report's tiles and the conservation
sentence are built from *this block*, which had no term for it.

**A design tension is recorded honestly in the code.** `scan_conservation.rs`
documents that `relevant` deliberately lives on the summary and not in the block —
the block describes the *input*, the summary carries the *outcomes* — and
`ScanConservation` is built by `prepare_pool` **before any call**, so it cannot
know `failed` at construction. R4 puts `failed` in the block anyway, and the
reason is stated at the field: the identity the tiles must show is unprovable from
a block carrying the left side and not the right. The consequence — `prepare_pool`
writes `0`, `persist_and_summarize` overwrites it — is documented at both ends and
asserted at both ends.

**The law is now EVALUATED, not merely written down.** It had been sitting in a
doc comment on `ThemeScanSummary` **[read]** while a run reporting `judged = 104`
against `relevant + irrelevant = 0` shipped to screen. `ScanConservation::reconciles()`
is called at write time (`theme_scan_persist`, `tracing::error!` naming the run and
all four counts) and again at read time (`scan_conservation`, `tracing::warn!`),
the second catching stored records from before the first existed.

Five wording rows: the failed clause, the failed tile, the two status pills, the
collapsed card's line for a dead run. The conservation template gains a `{failed}`
**slot** by guarded UPDATE — it renders as nothing on a clean run, so no reader is
taught to skip a permanent "· 0 failed".

### R5 — the admin form

Admin → Models grows the control that did not exist. Ninth wording block
(`domain/wording_model_params.rs`, seven rows, DTO mirror with the parity fence).
The dropdown asks *"does this model accept a temperature?"* in those words; the
`zero-ok` / `omit` tokens never reach the screen, and a test asserts they do not.
The numeric value is editable only under the mode that sends one, and travels only
with it.

The write path for `temperature_mode` / `default_temperature` was explicitly
deferred by Chunk A ("operator-writable in a later chunk" **[read]**). This is that
chunk. `validate_temperature_mode` refuses an unknown token **at the write**, which
matters because the resolver's own refusal happens at *call* time, mid-scan — a bad
token would otherwise sit in the registry looking settled until the next run died.

**One asymmetry, deliberate and documented:** the form can RECORD a capability and
cannot un-record one. The backend UPDATE is a `COALESCE`, and "nobody has said what
this model does" is a gap to close, not a state to choose. The unset option is
shown (so a row reads honestly) and disabled (so it cannot be chosen).

### R6 / R7

No action, as ruled. `max_tokens` untouched; the first Opus 5 re-run is the probe.
Run 6a9fad89 is Roman's ✕ to delete.

---

## 2. Two things found during the build that were not in the analysis

1. **`S.completePill` was referenced and never defined.** `ThemeScanPanel`'s `S` is
   a `Record<string, CSSProperties>`, so `S.completePill` resolved to `undefined`
   and the word "Complete" has been rendering unstyled beside a styled model chip
   for as long as the pill has existed. Both pills are defined now — a FAILED pill
   that looked identical to a Complete one would be this whole fix defeated by its
   own CSS.

2. **A stored string cannot begin with a space.** The failed clause was designed to
   carry its own separator (`" · {failed} failed"`), and `text_of` trims every value
   on the way out of the settings store — the sentence came out as
   `"124 judged· 1 failed"`. Caught by `the_domain_test_snapshot_matches_the_seeded_store`,
   which is the test existing precisely to catch a fixture and the store disagreeing.
   The clause is now `"· {failed} failed"` and the renderer supplies the single
   joining space, with a comment saying why one character of whitespace is not
   language.

---

## 3. Verification [measured]

| Gate | Result |
|---|---|
| `cargo test --lib` | **1750 passed, 0 failed, 2 ignored** |
| `cargo fmt --check` | clean |
| `cargo clippy --lib --bins --tests` | no warnings attributable to this diff |
| `cargo check --bins` | clean |
| `npx tsc --noEmit` | clean |
| `npx vitest run` | **880 passed, 66 files** |
| `npm run build` | clean (chunk-size advisory only, pre-existing) |
| Rule 17 on every touched module | at or under 300 |

**`cargo test --workspace` is still broken** by stale files under `backend/tests/`
(they reference `AppState.theme_scan_provider`, a field that no longer exists).
This predates the work — it has been broken since ~beta.343 — and no file under
`tests/` is in this diff. `cargo test --lib` is the honest baseline.

Three clippy warnings remain in the lib target, all in files **not** in this diff:
`scenario_dashboard.rs:456` (dead test helper), `scenario_code.rs:192`
(`is_ascii` idiom), `scenario_proposal_lookup.rs:162` (items after a test module).

**Rule 17 correction:** `scan_runs.rs` went 278 → 331 non-comment lines when the
status tests landed inline. Split to `scan_run_status_tests.rs` — the third sibling
test module on that file, and the module's own comment already records that the
first split happened for exactly this reason. Now 289.

### Named tests

| Test | Where |
|---|---|
| `an_unconfigured_model_omits_temperature` | `domain/llm_params.rs` |
| `…_omits_temperature_through_the_constraint_pass` | `domain/llm_params.rs` |
| `a_fully_failed_run_records_failed_status_not_completed` | `scan_run_status_tests.rs` |
| `a_failed_run_can_never_project_or_supersede` | `scan_run_status_tests.rs` |
| `conservation_reconciles_judged_equals_relevant_plus_irrelevant_plus_failed` | `scan_conservation.rs` |
| `a_partially_failed_run_reports_its_failed_count_in_conservation` | `theme_scan_persist_tests.rs` |
| `a_fully_failed_run_still_reconciles_with_nothing_relevant` | `theme_scan_persist_tests.rs` |
| `failed_counts_render_in_tiles_and_summary_line` | `scanFailureHonesty.test.ts` |
| `an_unknown_temperature_mode_is_refused_before_it_reaches_the_registry` | `models_tests.rs` |
| `a_model's_temperature_capability_is_recorded_never_guessed` (3 cases) | `temperatureForm.test.ts` |

### §11 gate agents

Run against `38fbbf0`, then a targeted re-pass against the amended `82674ea`.

| Agent | First pass | Finding | Action |
|---|---|---|---|
| architecture-reviewer | **PASS** | — | — |
| observability-checker | **PASS** | corrected me: per-call 400 causes are stored in `scan_run_verdicts.error`, not only logged — my doc comment understated it | comment fixed |
| test-auditor | **FAIL** | `validate_temperature_mode` untested (3 paths); `prepare_pool`'s `failed: 0` unasserted | both closed |
| rules-enforcer | **FAIL** | `ModelParamsWordingDto` missing `#[serde(deny_unknown_fields)]` | fixed |

All four findings were real and in my diff. Each pre-existing finding the agents
reported was in untouched code of a touched file and is listed in §4.

---

## 4. Deferred — recorded, not smuggled

1. **Per-verdict error strings have no UI surface.** `scan_run_verdicts.error`
   stores each call's cause verbatim; an operator can now see the aggregate count
   in the tiles and the sentence, but reading *why* individual calls died still
   needs a DB query. Pre-existing; observability-checker's own note. Worth a task.
2. **Four `Deserialize` structs in `models.rs` / `repositories/models.rs` lack
   `deny_unknown_fields`** — pre-existing, in code this commit did not change.
3. **The five `models.rs` handlers have no tests** — pre-existing; they need a live
   `AppState`.
4. **`ThemeScanPanel.tsx` carries dozens of pre-law literals** ("Relevant findings",
   the merge confirm) — recorded by the module's own comment before this task; only
   the two pills, which acquired a sibling they can be wrong about, were converted.
5. **`cargo test --workspace`** — the stale `tests/*.rs` files.
6. **`THEME_SCAN_MODEL` / `THEME_SCAN_CONCURRENCY` Ansible template rows** still
   owed from D2b. **No new env vars in this build**, so nothing new is owed.

---

## 5. THE ONE THING THAT DID NOT HAPPEN

**The R2 interim was not applied.** You authorized it; I attempted it; the
permission classifier declined the write, and I did not work around the denial.
Nothing was written to the DEV registry.

The `.388` migration seeds the row on deploy regardless, so this only matters if
you want the bake-off unblocked before the deploy. `'omit'` is the exact value read
from the `claude-opus-4-8` row in Phase A, and the migration's `IS NULL` guard means
running this first is harmless:

```sql
SELECT id, temperature_mode FROM llm_models
 WHERE id IN ('claude-opus-4-8', 'claude-opus-5');

UPDATE llm_models SET temperature_mode = 'omit'
 WHERE id = 'claude-opus-5' AND temperature_mode IS NULL;

SELECT id, temperature_mode FROM llm_models WHERE id = 'claude-opus-5';
```

---

## 6. Deployment

- **New env vars:** none. **Ansible template:** unchanged.
- **Migration:** one, pipeline DB (`colossus_legal_v2`). Twelve new `app_settings`
  rows, one guarded correction, one `llm_models` UPDATE. Forward-only, idempotent.
- **Deploy ordering:** all twelve keys are declared to the boot loader, and a
  declared key with no row makes the backend **refuse to start**. The runtime
  Migrator applies this at boot before the settings load, so a normal deploy orders
  itself. Rollback to an older image is safe; roll-forward without this file is not.
- **Rebuild:** backend **and** frontend.
- **Rollback:** `git revert 82674ea`. The extra `app_settings` rows are ignored by
  an older image; the `claude-opus-5` row reverts to NULL only if you clear it by
  hand, and under the OLD code NULL means "send 0.0" — i.e. reverting restores the
  400s. That is the honest statement of what this rollback costs.
- `Cargo.lock` carries a one-line change: it still read `beta.386` against a
  `beta.387` `Cargo.toml` and the build synced it. A lock catching up, not a bump.

**Step-6 probes**, as authorized: (1) re-run the Opus 5 scan on S-4 — it JUDGES, or
the run reads FAILED with its count; no third outcome. (2) a deliberately failing
call shows failed in the tiles and the summary line. (3) Admin → Models shows the
mode and value with plain descriptions. (4) a 4.8 scan behaves exactly as before.
(5) after your ✕ on 6a9fad89, the Aug 7 proposals are back in the queue.

**STOPPING HERE.** Nothing pushed, no version bumped, no deploy attempted.

=== END REPORT — VERDICT: COMPLETE ===
