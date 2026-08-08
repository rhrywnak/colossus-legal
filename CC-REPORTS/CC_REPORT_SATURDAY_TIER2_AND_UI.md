# CC_REPORT_SATURDAY_TIER2_AND_UI

**Task:** `CC_TASK_SATURDAY_TIER2_AND_UI_v1` · **Date:** 2026-08-08
**Branch:** `fix/tier2-scan-and-ui` off `main` @ `aee4c3f` (beta.383)
**Commit:** the branch tip — one commit, 71 files. It began as `93f6109`, was
amended to `f689405` with the §11 agents' findings (all four re-inspected THAT SHA
and returned PASS), and was amended again only to carry this report and its
wording. No code changed after the passing re-pass; `git show --stat` on the tip is
the authority for the file list, and the gate below was re-run on it.
**Rulings applied:** R1–R7 (architect, 2026-08-08). Phase A analysis:
[CC_REPORT_SATURDAY_TIER2_AND_UI_PHASE_A.md](CC_REPORT_SATURDAY_TIER2_AND_UI_PHASE_A.md).

Everything marked **[measured]** was read out of this tree or observed in a test
run today.

---

## PIECE 1 — TIER-2 SCAN MECHANICS

### 1a — Byte-identical quotes are judged once, and one ruling covers the set

`services::theme_scan_prefilter` (new, pure) folds byte-identical quotes into one
`CandidateGroup` carrying every node id it speaks for. The judge sees one group per
LLM call; the persist pass writes a `scan_run_verdicts` row for **every member**
(R2), and the pick the human sees carries `covers_node_ids` so Merge writes the
judgment onto the twin as well.

The measured harm this ends **[measured, 2.15 report §4]**: the 07-29 S-4 run
admitted three byte-duplicate PAIRS, so Roman was asked to rule the same sentence
twice and paid twice to be asked.

**Why every member still gets a verdict row.** The scorecard joins Roman's ledger
to `scan_run_verdicts` on `graph_node_id`. A folded twin with no row would be
scored as a statement the scan LOST — the scan would look worse than it is, on the
one instrument built to measure it. `PRIMARY KEY (run_id, graph_node_id)` permits
one row per member with no schema change **[measured]**.

The card says `×2` when a pick settles more than one row. Without it the pool would
shrink by two on a single click with nothing having said why.

### 1b — Content-free statements never reach the judge

Three rules, in order of specificity: **empty** (no text, or whitespace) →
**statement kind** (a kind the settings row names; seeded `referral`) → **too short
with no paired question**.

The length rule's safety is the no-question clause, and it has its own test: a
four-word answer whose interrogatory is in evidence is a full candidate and
survives, because the judge is shown both (`build_user_message`). Only an
unanchored fragment is set aside.

**`statement_type` did not reach Rust** and now does (R1): one column in the gather
RETURN, one `.ok()` decode, one `Option<String>` on `BiasInstance` — the exact
`question` precedent. No schema change **[measured]**.

Both dials are settings rows, editable with no build:

| key | seed | note |
|---|---|---|
| `theme_scan_prefilter_min_chars` | `60` | `0` switches the rule off |
| `theme_scan_prefilter_statement_types` | `referral` | comma-separated; the word `none` switches it off |

**On the seed of 60 (R4).** Its blast radius is still unmeasured — the 2.15 report
counts 41 of 148 pool quotes under 60 characters but does not report how many of
those also lack a question, which is the clause that decides. The first run's
conservation counts answer it, and the row is how the answer gets acted on.

### 1c — Conservation, on screen

```
pool = excluded_empty + excluded_statement_type + excluded_too_short
     + duplicates_collapsed + judged
```

Asserted in a test over a pool exercising every branch, logged at scan start, and
frozen into the run's summary (JSONB — no migration). One reconciliation line
renders under the run's counts:

> `148 gathered · 3 duplicates folded · 21 set aside before judging · 124 judged · 22 relevant`

**The line is composed at READ time** from the run's frozen counts and the stored
template. That split is deliberate: the record is immutable, the wording is
editable, and a run scanned in July reads in today's words. A run recorded before
this task carries no counts and gets **no line at all** — "0 gathered" would be a
claim about a run nobody measured.

**Two columns, two meanings (R5).** `candidates_read` is now the POOL (the history
table's Candidates column and its +Δ delta measure evidence that exists, which a
pre-filter setting must not appear to change); `candidates_total` is the judged
denominator, so "43 of 124" counts the work actually happening. Both columns
already existed — no migration. The results tile reads **Judged**, not Read.
History row shape is unchanged, as ruled.

### 1d — The prompt pointer is a settings row

`theme_scan_prompt_file`, seeded `theme_scan_prompt_v3.md`, read at SCAN START from
the live snapshot. `THEME_SCAN_PROMPT_FILE`, `resolve_theme_scan_prompt_file` and
the compiled default are **deleted** from `config.rs` along with their four tests —
they tested a function that no longer exists.

**No Ansible change is owed**, and the reason is measured: the env var was never set
on DEV, so a compiled constant silently decided which prompt judged every scan.
There is nothing to remove from the template.

**R3, both halves, same commit:**

- **Boot refuses to start** when the named file does not resolve
  (`assert_scan_prompt_deployed`), naming the file and the full path.
- **The write path refuses** a filename that does not resolve, as a 400 naming the
  path (`check_named_file` → `SettingsError::FileNotFound`).

The second is what keeps `trial_snapshot`'s promise intact — that a change accepted
on the Settings page can never produce a store that refuses to boot. Without it, a
typo would commit, and the crash would arrive at the next restart with nothing on
the page having said so.

`THEME_SCAN_MODEL` / `THEME_SCAN_CONCURRENCY` are **untouched**: the design proposed
retiring them, this instruction did not, and widening scope was not mine to do.

### 1e — The scorecard

[scripts/scan-scorecard.sql](../scripts/scan-scorecard.sql) — the §6 query with a
usage header, the two ids as psql variables, how to read each column, the 08-06
baseline, and both stated limitations (the `source_run_id` trap; a ledger with 52
includes and 3 drops measures recall well and precision barely). It also records
that a folded duplicate still writes a verdict row, because if that ever changes
this scorecard starts reporting folded rows as `includes_lost`.

---

## PIECE 2 — VISIBLE DELETE (D7 OVERRULED FOR DELETE)

Both surfaces carry a visible **Delete**; `ScenarioKebab.tsx` is deleted from the
tree (it had no consumer left — `git rm` succeeded, so nothing is owed to Roman
here).

- **Trial Prep card:** bottom-right, diagonally opposite the title and as far from
  "Open scenario →" as the card allows. Still a SIBLING of the `<Link>`, not a
  child: a `<button>` inside an `<a>` is invalid markup and every click would also
  navigate. That reasoning outlived the kebab it was written for.
- **Scenario header:** last in the actions row, behind a visible separator. The
  readiness control lives in the identity row ABOVE, so nothing destructive is
  adjacent to anything routine — which is the half of D7's concern that survives.

The confirm dialog is unchanged: same `scenarioDeleteCopy`, same
`ScenarioDeleteConfirm`, names the scenario, stays open on failure. The D7 comment
history is amended in both components and in the structure test.

---

## PIECE 3 — THE PRE-SCAN SCREEN TELLS THE TRUTH

**The signal had to come from the server, and this is the finding worth keeping.**
Both numbers on screen that look like they answer "has this been scanned" do not
**[measured]**: the queue's count is derived from the card pool (nothing to do with
scans), and the "Never scanned (148)" facet counts cards with no confidence — which
a card only gains when a scan verdict is **merged**. A scenario scanned three times
with nothing merged reads as never scanned by that test.

So `GET …/facts/cards` now carries `never_scanned_notice`, present only when the
scenario has **no COMPLETED scan run** — the same shape as the 2026-08-07
`no_target_notice` beside it. A run that failed judged nothing and does not count;
it stays visible in the history, where it belongs.

- **Never-scanned:** the section leads with the served notice, and the raw pool sits
  behind `Browse the raw evidence pool (148)` — collapsed, named, one click away.
  Nothing is removed: same cards, same queue, same keys.
- **"from all scans" is now earned:** the clause appears only when something in the
  pool actually carries a scan's score (`anyScanScored`).
- **Scan-run state is untouched** — the queue renders exactly as before.

No ruled/queue mechanics were redesigned. This is ordering plus an empty state, as
scoped.

---

## PIECE 4 — RESULTS REACHABLE AFTER A REFRESH

**4a is an affordance correction, per R6 — no second render path was built.** The
panel now auto-opens its most recent COMPLETED run on arrival (once per scenario,
via a ref, so clicking the open row still collapses it), and each completed history
row carries a visible **View results** control. Failed and running rows get none —
offering one would promise a panel that opens onto an error.

**4b:** the ✕ asks first, naming the run ("Remove the scan run from Aug 7, 21:14?
Its verdicts are deleted with it…"). If the wording has not loaded, the ✕ is not
rendered at all rather than deleting unguarded or refusing silently.

---

## TESTS — behaviour only, per the standing law

**Backend, 1701 unit tests pass [measured]** (`cargo test --lib`). New:

| test | what breaks if it fails |
|---|---|
| `byte_identical_quotes_are_judged_once_and_the_group_names_both_nodes` | the same sentence is judged and ruled twice |
| `a_collapsed_duplicate_is_counted_once_and_merged_as_a_set` | the run over-reports its judged count, or a merge misses the twin |
| `every_member_of_a_collapsed_group_gets_its_own_verdict_row` | the scorecard scores a folded row as a statement the scan lost |
| `quotes_that_merely_resemble_each_other_are_not_collapsed` | two different statements are silenced as one |
| `a_short_answer_survives_when_its_question_gives_it_meaning` | the C-51 class is set aside — the filter's one real hazard |
| `a_blank_question_does_not_rescue_a_fragment` | two surfaces disagree about whether a quote is rulable |
| `a_referral_is_set_aside_and_named_by_its_kind` | a content-free cross-reference is admitted as relevant again |
| `a_statement_kind_nobody_configured_is_judged` | the filter drops kinds nobody asked it to |
| `a_minimum_of_zero_judges_everything_that_has_text` | the documented "off" value does not turn the rule off |
| `conservation_holds_over_a_pool_that_exercises_every_branch` | a candidate goes missing and nothing says so |
| `the_pools_own_order_is_preserved` | two scans of one pool judge in different orders |
| `the_scan_reads_the_prompt_file_the_settings_row_names` | changing the row does not change what judges |
| `a_missing_prompt_names_the_path_it_looked_for` | a scan fails without saying which file is absent |
| `the_line_reports_the_runs_own_numbers_and_they_reconcile` | the reconciliation line does not reconcile |
| `a_run_recorded_before_this_task_gets_no_line_rather_than_a_zeroed_one` | a July run claims it measured a pool of zero |
| `a_damaged_conservation_block_is_left_alone` | a partial block renders as if it added up |
| `a_scenario_nothing_has_scanned_carries_the_empty_state` | the 2026-08-07 screen returns |
| `one_completed_run_withdraws_the_notice_for_good` | the notice contradicts the history table above it |
| `a_named_file_that_exists_resolves` / `a_typo_and_a_directory_both_fail_the_check` | the Settings page accepts a filename that arms a crash |
| `the_seeded_conservation_line_carries_all_five_numbers` | the shipped default is a value the write path would refuse |
| `every_declared_key_is_seeded_…` (scan wording) | a declared key with no row — a backend that will not start after deploy |
| `a_prompt_filename_that_is_not_deployed_is_refused_with_its_path` | the Settings page accepts a typo that kills the next restart |
| `a_row_that_names_no_file_is_never_checked_against_the_filesystem` | the file check widens to every parameter and refuses the whole page |
| `params_snapshot_freezes_the_prefilter_dials_that_chose_the_pool` | two runs differ and the audit trail cannot say which dial turned |

**Frontend, 825 tests pass [measured]** (`npx vitest run`). New/amended:

- `never claims a scan parentage the pool does not have` — the measured 08-07 defect
  as a test: the count survives, the provenance claim is withdrawn.
- `reads scan parentage off the CARDS, and says no before the pool is read`.
- The D7 structure test is **amended, not extended**: both surfaces raise the
  request and neither calls the delete service; Delete is not adjacent to the status
  control; the kebab is gone from disk.
- The scan-runs service test now pins that runs and their control words travel
  together, and that a payload with neither yields `null` rather than invented
  defaults.

**No render-restating tests for a button's existence**, as instructed. The
card/header delete behaviour is asserted through the routing test plus the existing
`deleteScenario` service tests — with no RTL in this frontend (CLAUDE.md Rule 30),
that is the honest reach.

---

## GATE

| check | result |
|---|---|
| `cargo check --bins` | clean |
| `cargo clippy --lib --bins -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `cargo test --lib` | **1701 passed, 0 failed, 2 ignored** |
| `npm run typecheck` | clean |
| `npx vitest run` | **825 passed, 0 failed** |
| `npm run build` | clean (pre-existing chunk-size warning only) |
| route-link guard | green (`routePaths.test.ts`) |
| §11 agents on the final SHA | see below |

`cargo test --workspace` is NOT the target: it is broken on `main` by stale
`tests/*.rs` files (pre-existing, unrelated, untouched here). `cargo test --lib` is
the honest baseline.

### Rule 17 (module size) — one violation introduced and removed

`settings_store.rs` went to 312 non-comment lines when the file check landed in it.
The check moved to `services::settings_template_file` (its natural home) — 296 now.
`main.rs` was over the limit before this task (≈435); rather than add 25 lines to
it, the boot assertion is a one-line call into that same module (444 → the
remaining growth is the call plus its comment). Both are noted rather than hidden.

### §11 AGENT RESULTS

All four ran against `93f6109`. **Three returned FAIL and every finding was
fixed**; the fourth's findings are almost entirely pre-existing code and are
reported below rather than silently absorbed.

**`test-auditor` — FAIL → fixed.** Two gaps, both real: `check_named_file` (the
write-path refusal, R3's second half) had no test at all, and its KEY SPECIFICITY
was untested — a `match` arm that fires on one key ever tested could silently
become a wall over every parameter. Two behaviour tests added:
`a_prompt_filename_that_is_not_deployed_is_refused_with_its_path` (the refusal
names key, value and path; the deployed file still passes) and
`a_row_that_names_no_file_is_never_checked_against_the_filesystem` (a band cutoff
is not checked as a filename). It confirmed all six mandated behaviours covered.

**`observability-checker` — FAIL → fixed.** One finding, and a good one:
`params_snapshot` froze the LLM parameters and the prompt filename but NOT the two
pre-filter dials, which this task made into settings a human can change between
runs. The conservation block recorded that fifteen quotes were set aside for
length; nothing recorded the threshold that set them aside, so an operator
comparing two runs a week apart could not tell a prompt change from a settings
change. `prefilter_min_chars` and `prefilter_statement_types` now ride the
snapshot, with `params_snapshot_freezes_the_prefilter_dials_that_chose_the_pool`
pinning it. Everything else passed, including the conservation counts being STORED
rather than only logged.

**`architecture-reviewer` — FAIL → fixed.** Five findings, all one mistake of
mine: I retired `THEME_SCAN_PROMPT_FILE` and left it named in five places that
tell an operator what to do. The blocking one was the `PromptFileMissing` error
message itself, which sent them hunting for an env var this build no longer reads;
it now names the file, the path, and the `theme_scan_prompt_file` row. Four stale
comments corrected (`theme_scan.rs` ×3, `theme_scan_start.rs` ×1), plus one the
agent did not see: `scenario_theme_scan_tests.rs` asserted the message contains the
retired env var, so the test would have enforced the wrong instruction. Domain
boundaries, configuration sourcing and reusability passed clean.

**`rules-enforcer` — FAIL (30), of which ONE is in my diff and is fixed.** The
agent audited whole files rather than the diff. Mine: the new
`statement_type: row.get(…).ok()` in `bias/aggregation.rs` carried an explanatory
comment but not the mechanical `// best-effort:` prefix. Fixed, and the comment now
says why an absent column decoding to `None` is the honest reading rather than a
swallowed failure.

**The other 29 are PRE-EXISTING lines in files this task touched, and I have not
changed them** — CLAUDE.md §13 says a bug found while implementing a feature is
documented, not fixed in the same instruction. Listed so Roman can rule on them:

| where | what | mine? |
|---|---|---|
| `main.rs` (444 lines) | over the 300-line limit | no — it was ≈435 before this task; my addition is a one-line call plus its comment, and the assertion body lives in `settings_template_file` precisely to avoid growing it further |
| `main.rs` ×12 | `.expect()` without a `// SAFETY:` prefix | no — startup code, untouched. One at :337 has a justification comment in the wrong FORM |
| `main.rs:24, :30, :720` | `DEFAULT_CHAT_MODEL`, `CHAT_MAX_TOKENS`, `DEFAULT_STARTUP_SCHEMA_FILE` config-shaped consts | no — untouched |
| `main.rs:131, :132` | HTTP client timeouts without a `// DEFAULT:` prefix | no — untouched |
| `bias/aggregation.rs` ×9 | `.ok()` without the `// best-effort:` prefix | no — the pre-existing column decodes, including `question`, whose comment my new one was written to match |
| `theme_scan_persist.rs:36` | `THEME_SCAN_REJECTED_SAMPLE_SIZE` | no — untouched, and it already carries a `// CONST:` justification |

The agent also flagged that it audited 27 of the ~66 changed files. The unaudited
remainder are test fixtures, the migration, and small frontend diffs; the other
three agents covered the substantive new modules between them.

### RE-INSPECTION OF THE FINAL COMMIT

The commit was amended after the agent pass, so per the standing rule **all four
agents re-inspected the final SHA `f689405`**, each scoped to `git diff 93f6109
f689405`. The mechanical gate above was re-run in full after every fix.

**All four returned PASS.**

- **`rules-enforcer`** — the `// best-effort:` prefix is in the required form; no
  unwrap/expect, discarded `Result`, magic value or hardcoded config in the new
  code; no file crossed the 300-line limit *because of* these changes.
- **`test-auditor`** — both gaps genuinely closed. It confirmed the new tests
  exercise `check_named_file` itself (not the helper beneath it), that the
  key-specificity test is load-bearing on the `match` (an empty scratch directory
  makes it fail if the guard is ever widened), and that neither test violates the
  standing law.
- **`architecture-reviewer`** — all five findings fixed; every remaining mention of
  the retired env var reads as history, not as instruction. It also independently
  confirmed the fifth site I found and it had not (`scenario_theme_scan_tests.rs`).
- **`observability-checker`** — it traced `prepare_scan` → `PrefilterSnapshot` →
  `params_snapshot` and confirmed a single `settings.current()` read gates both the
  config the pre-filter APPLIED and the values the snapshot RECORDS, so a divergence
  between them is structurally impossible rather than merely unlikely.

### One measurement disagreement, resolved

The re-pass reported `theme_scan.rs` at 421 lines and therefore already over Rule
17. By **the project's own command** (CLAUDE.md §8, which strips `^\s*//` and so
excludes doc comments) it is **292** — under the limit, though not by much. The
agent counted doc comments; Rule 17 says "excluding doc comments". Recording both
so nobody re-derives the discrepancy later. The §8 command over every backend file
this commit touched reports four over the limit, all pre-existing:
`main.rs` (444), `scenario_card_tests.rs` (971), `settings_store_tests.rs` (678),
`wording_tests.rs` (410). I added small amounts to all four — 9, 1, ~30 and ~4
lines — without splitting them; three are test files and `main.rs` is discussed
above.

---

## OBSERVED, NOT IN THE ANALYSIS

1. **`backend/Cargo.lock` is in the commit.** It carried `2.0.0-beta.382` while
   `Cargo.toml` says `.383`; the first build regenerated it. This is the lock
   catching up to Roman's own version bump, not a version bump by me.
2. **`main.rs` is 444 non-comment lines** and was already over Rule 17 before this
   task. Not this task's to split.
3. **The judge's inline tests moved** to `theme_scan_judge_tests.rs` (the house
   `#[path]` pattern) — that module had no room under Rule 17 for the group change.
   Not one assertion was edited.
4. **The seeded `theme_scan_prompt_file` is `theme_scan_prompt_v3.md`.** DEV already
   has that file (commit `eb22353`). The row seeds only where no row exists, so a
   re-run never overwrites an edit.
5. **Roman's OPS note stands:** once .384 is verified, restore the true v2 text to
   DEV's `theme_scan_prompt_v2.md` via `./push-templates.sh theme_scan_prompt_v2.md`.
6. **Deploy ordering:** six new keys are declared to the boot loader, so the
   migration must be applied by or with the backend image that reads them. The
   runtime Migrator does this at boot, so a normal deploy orders itself. A
   roll-forward WITHOUT the migration would refuse to start.

**Not deployed. Not pushed. Roman decides both.**
