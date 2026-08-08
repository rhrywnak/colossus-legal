# CC_REPORT_SATURDAY_TIER2_AND_UI — PHASE A (pre-coding analysis)

**Task:** `CC_TASK_SATURDAY_TIER2_AND_UI_v1` · **Date:** 2026-08-08
**Branch:** `fix/tier2-scan-and-ui`, created off `main` at `aee4c3f` (beta.383),
working tree clean.
**Status: STOPPED at the Phase A gate.** No code written. Seven rulings are
needed before implementation starts — §12.

Everything marked **[measured]** below was read out of this tree today; nothing
is assumed from the design docs where the code could be read instead.

---

## 1 — TASK UNDERSTANDING

Four bounded pieces on one branch, riding as beta.384:

1. **Tier-2 scan mechanics** — collapse byte-identical quotes before judging,
   pre-filter content-free statements, report conservation on screen, move the
   judging-prompt filename from an env var into a settings row, and file the
   scorecard SQL in `scripts/`.
2. **Visible Delete buttons** — retire the ⋯ kebab on the Trial Prep card and on
   the scenario detail header (D7 overruled for Delete).
3. **The pre-scan screen tells the truth** — a never-scanned scenario stops
   leading with 148 raw pool cards labelled "from all scans".
4. **Scan results re-render from history** — a completed run's results panel is
   reachable after a cold load, and the ✕ that destroys a run asks first.

---

## 2 — BRANCH VERIFICATION

- Current branch: `fix/tier2-scan-and-ui` (off `main`)
- Working tree clean: **YES**
- Last commit: `aee4c3f chore: bump version to 2.0.0-beta.383`

---

## 3 — WHAT THE CODE ACTUALLY SAYS (the Phase A measurements)

### 3.1 The scan path today **[measured]**

| stage | file | today |
|---|---|---|
| read the prompt | `theme_scan_validate.rs:39` `load_scan_prompt` | filename from `state.config.theme_scan_prompt_file` (env `THEME_SCAN_PROMPT_FILE`, compiled default `theme_scan_prompt_v2.md` at `config.rs:364`) |
| gather | `bias/queries.rs:201` `all_evidence_about_subject` | ungated; **no dedup, no pre-filter** |
| judge | `theme_scan_judge.rs:63` `judge_all` | one LLM call per pool row |
| persist | `theme_scan_persist.rs:68` | one `scan_run_verdicts` row per pool row; summary counts |
| history row | `dto/theme_scan.rs:145` `ScanRunHeader` | `candidates_read` + `candidates_total`, both set to the pool size at promote (`scan_runs.rs:176`) |

### 3.2 `statement_type` is NOT available to the scan **[measured]** — 1b's one surprise

The pre-filter needs `statement_type = 'referral'`. The gathering Cypher
(`bias/queries.rs:201-231`) does **not** project `statement_type`, and
`BiasInstance` (`bias/dto.rs:164`) has no such field. The measurement report says
the property exists on 148/148 nodes in the graph — it simply never reaches Rust.

This is the "name it in Phase A if that assumption breaks" case. It is still **no
schema change**: the fix follows the `question` precedent exactly (added the same
way, `bias/aggregation.rs:83`) — one line in the RETURN, one `.ok()` decode, one
`Option<String>` field with `skip_serializing_if`.

### 3.3 The dedup decision has a consequence the instruction does not settle

`scan_run_verdicts` is `PRIMARY KEY (run_id, graph_node_id)` **[measured]** and
the scorecard query joins Roman's ledger to it **on `graph_node_id`** (the
measurement's own §6 warning: never `source_run_id`). So if a collapsed twin
simply never gets a verdict row, the scorecard reports that node as
`includes_lost` — the scan would be scored down for a node it deliberately
collapsed. See ruling **R2**.

### 3.4 The prompt-file settings row has a boot-refusal hazard **[measured]**

`settings_store::trial_snapshot` exists precisely so "a change accepted here can
never produce a store that refuses to boot". A `text` row is validated by
`validate_wording_candidate` — non-blank plus placeholder retention — and
**nothing checks that a filename resolves**. So a boot assertion on the prompt
file, added alone, re-opens exactly the hazard that function closes: a typo typed
into the Settings page would be accepted, and the next restart would `exit(1)`
with the page having said nothing. Ruling **R3**.

### 3.5 Piece 4's first half does not reproduce from the source **[measured]**

The instruction says the scan-history row is "display-only" and the results panel
"cannot be reopened". The code says otherwise:

- `ScanHistoryTable.tsx:111` — every `<tr>` has `onClick={() => onToggle(row.runId)}`.
- `ThemeScanPanel.tsx:511` — `onToggle` is `onSelectRun`.
- `onSelectRun` (`:324-350`) fetches `GET .../scan-runs/:run_id`, caches the
  summary and selects the run.
- `ThemeScanPanel.tsx:539` — `hasSelectedResults` then renders `ResultsArea`,
  the same component the post-run view uses, with merge controls intact.
- The backend serves it: `get_scan_run_status` returns the stored `summary_json`
  annotated with per-pick `ordinal` and `applied` (`theme_scan_run.rs:46`), and
  its doc says in as many words that this exists "because a reopened HISTORICAL
  run needs it just as much as the one just merged".

So either the path is broken at runtime for a reason the source does not show
(the history `<details>` is collapsed by default and the rows carry no visible
affordance — a plausible "display-only" reading), or something fails in the
browser. I will not build a second re-render path over a working one. Ruling
**R6**: I verify this on DEV first (browser, per the standing recipe) and report
what actually happens before writing anything for 4a.

Piece 4's **second half is unambiguous and real**: `ScanHistoryTable.tsx:133-152`
fires `onDelete(row.runId)` on the first click, with **no confirm** anywhere in
the path (`ThemeScanPanel.tsx:359` calls `deleteScanRun` immediately). One stray
click destroys the run. That gets built regardless of R6.

### 3.6 Piece 3: the honest signal is NOT the one on screen **[measured]**

- "Candidates awaiting ruling — 148" + "from all scans" is
  `queueRegion.ts:120-143`, fed by `progressFromCards(cards)` — the page's card
  pool, which has nothing to do with scans.
- "Never scanned (148)" is the filter facet `candidateFilters.ts:304`, counted by
  `isScanScored(card)` = `card.confidence.band !== "unscored"`
  (`candidateFilters.ts:97`).

Those two disagree because the second one is not a scan-history test either: a
card is "scan-scored" only once a scan verdict has been **merged** onto it. A
scenario that has been scanned three times and merged nothing still counts 148
"never scanned" cards. The only truthful "this scenario has never been scanned"
is `scan_runs` for the scenario, which lives in the pipeline DB.

There is a precedent for exactly this shape, built on 2026-08-07: the target-less
scenario. `scenario_cards.rs:96` returns an empty payload carrying
`no_target_notice` from a stored wording row, and
`dto/scenario_card.rs:419` is the field. Piece 3 is the same move with a
different condition.

### 3.7 Piece 2's blockers **[measured]**

- `ScenarioKebab` has exactly two consumers: `ScenarioCard.tsx:114` and
  `ScenarioHeaderTiers.tsx:202`. After both go, nothing imports it.
- `pages/__tests__/scenarioPageStructure.test.ts:163-181` is a **D7 guard**: it
  asserts the header contains `ScenarioKebab` and does not contain
  `Delete</button>`, and it reads `ScenarioKebab.tsx` from disk. That test must
  be amended in the same commit or the suite fails on the overruled ruling.
- Deleting the file: `rm` is denied to me in this environment. The deletion is
  planned LAST and handed to Roman as a one-line command if `git rm` is refused
  too — the report will say which.
- Placement: `ScenarioStatusControl` ("Mark ready to rehearse") is in the tier
  row ABOVE the actions row; Delete goes at the far right of the actions row
  (where the kebab already sits), after `✎ Edit` and `Rehearsal view →`.

### 3.8 Module-size headroom **[measured]** (Rule 17, non-comment lines / 300)

`theme_scan.rs` 265 · `theme_scan_judge.rs` 285 · `theme_scan_persist.rs` 199 ·
`theme_scan_start.rs` 196 · `dto/theme_scan.rs` 122 · `settings_store.rs` 267 ·
`domain/settings.rs` 242 · `config.rs` 164 · `scenario_cards.rs` 288 ·
`scan_runs.rs` 265.

Three of those cannot absorb this work: `theme_scan_judge.rs`,
`settings_store.rs`, `scenario_cards.rs`. The plan below puts the new logic in
new modules and moves `theme_scan_judge.rs`'s inline `mod tests` out to a sibling
`theme_scan_judge_tests.rs` (the house pattern every other scan module already
uses) rather than growing any of them.

---

## 4 — FILES TO MODIFY

| File | Change | Est. lines |
|---|---|---|
| `backend/src/bias/queries.rs` | project `e.statement_type` in `all_evidence_about_subject` | 3 |
| `backend/src/bias/aggregation.rs` | `BiasRow.statement_type` + `.ok()` decode + assign | 8 |
| `backend/src/bias/dto.rs` | `BiasInstance.statement_type: Option<String>` | 12 |
| `backend/src/services/theme_scan.rs` | `PreparedScan` carries judge groups + conservation; `prepare_scan` runs the prefilter | 40 |
| `backend/src/services/theme_scan_judge.rs` | judge one REPRESENTATIVE per group; move inline tests to a sibling file | 25 (−150 moved) |
| `backend/src/services/theme_scan_persist.rs` | fan verdicts out to group members; conservation into the summary | 45 |
| `backend/src/services/theme_scan_start.rs` | thread pool-size vs judged-count | 15 |
| `backend/src/services/theme_scan_validate.rs` | `load_scan_prompt` reads the settings row | 12 |
| `backend/src/dto/theme_scan.rs` | conservation block on `ThemeScanSummary`; `duplicate_count` + member ids on `ThemeScanSuggestion` | 55 |
| `backend/src/repositories/pipeline_repository/scan_runs.rs` | `ScanRunStart.candidates_read` separate from `candidates_total` | 8 |
| `backend/src/domain/settings.rs` | 3 new `Settings` fields + `for_test` | 18 |
| `backend/src/services/settings_store.rs` | 3 KEY consts, `REQUIRED_KEYS`, `build_settings` | 12 |
| `backend/src/config.rs` | **retire** `theme_scan_prompt_file`, its env read, `resolve_theme_scan_prompt_file` and its two tests | −45 |
| `backend/src/main.rs` | boot assertion: the named prompt file resolves | 20 |
| `backend/src/api/scenario_cards.rs` | never-scanned branch delegated to a new service helper | 10 |
| `backend/src/dto/scenario_card.rs` | `never_scanned_notice: Option<String>` | 15 |
| `backend/src/domain/wording_scenario_authoring.rs` | 1 new stored string (+ key, + keys list) | 14 |
| `frontend/src/services/themeScan.ts` | types: conservation, duplicate fields | 25 |
| `frontend/src/components/ThemeScanPanel.tsx` | conservation line; run-delete confirm state | 40 |
| `frontend/src/components/ScanHistoryTable.tsx` | ✕ asks first (names the run) | 25 |
| `frontend/src/components/ScanSection.tsx` | never-scanned ordering + opt-in pool | 45 |
| `frontend/src/components/queueRegion.ts` | scope clause gated on scan-scored counts | 20 |
| `frontend/src/services/scenarioCards.ts` | `never_scanned_notice` on the type | 6 |
| `frontend/src/components/ScenarioCard.tsx` | visible Delete, kebab out | 30 |
| `frontend/src/components/ScenarioHeaderTiers.tsx` | visible Delete, kebab out | 25 |
| `frontend/src/pages/__tests__/scenarioPageStructure.test.ts` | amend the D7 guard (overruled 2026-08-07) | 25 |
| `frontend/src/components/__tests__/queueRegion.test.ts` | the new scope rule | 20 |

## 5 — FILES TO CREATE

| File | Purpose | Est. lines |
|---|---|---|
| `backend/src/services/theme_scan_prefilter.rs` | PURE: pool → judge groups + exclusions by reason + conservation counts | 220 |
| `backend/src/services/theme_scan_prefilter_tests.rs` | dup collapse, each exclusion reason, conservation identity | 200 |
| `backend/src/services/theme_scan_judge_tests.rs` | the moved inline tests, unchanged | 150 |
| `backend/src/services/scenario_cards_scan_state.rs` | "has this scenario ever been scanned" + the notice, so `scenario_cards.rs` stays under 300 | 70 |
| `backend/pipeline_migrations/<ts>_theme_scan_prompt_row_and_prefilter.sql` | seeds `theme_scan_prompt_file` = `theme_scan_prompt_v3.md`, the two prefilter rows, and the never-scanned notice | 60 |
| `scripts/scan-scorecard.sql` | the §6 scorecard with a usage header | 45 |

## 6 — FILES TO DELETE (LAST, and possibly Roman's hand)

| File | Why |
|---|---|
| `frontend/src/components/ScenarioKebab.tsx` | no consumer left after Piece 2 |

---

## 7 — DEPENDENCIES / EXTERNAL SURFACE

- **New crates:** None. **Removed crates:** None.
- **New env vars:** None. **Retired env var:** `THEME_SCAN_PROMPT_FILE` (never
  set on DEV — measured in the 2.15 report, `printenv | grep -c THEME_SCAN` = 0),
  so no Ansible template change is owed, exactly as the instruction says.
  `THEME_SCAN_MODEL` / `THEME_SCAN_CONCURRENCY` are **untouched** — the design
  proposed retiring them, this instruction did not, and I am not widening scope.
- **New settings rows (3 + 1):**
  | key | kind | seed | meaning |
  |---|---|---|---|
  | `theme_scan_prompt_file` | text | `theme_scan_prompt_v3.md` | which judging prompt a scan reads at start |
  | `theme_scan_prefilter_min_chars` | count | see **R4** | shortest quote (with no paired question) that still reaches the judge |
  | `theme_scan_prefilter_statement_types` | text | `referral` | comma-separated statement types that never reach the judge |
  | `scenario_never_scanned_notice` | text | "No scan has run yet…" | Piece 3's empty state |
- **API endpoints added:** None. **Changed (additive fields only):**
  `GET .../scan-runs/:run_id` summary, `GET .../facts/cards`.
- **Migration:** one, via `./scripts/new-migration.sh pipeline`. Note for the
  record: the migration chain still does not apply from zero (8-digit prefixes
  sort before 14-digit ones) — a known, separate defect, untouched here.

---

## 8 — THE FOUR PIECES, AS BUILT

### 8.1 Piece 1a/1b/1c — one pure module, three observable numbers

`theme_scan_prefilter.rs` takes the gathered `Vec<BiasInstance>` and returns:

```
PreparedPool {
    groups:      Vec<CandidateGroup>,   // what the judge sees; one call each
    excluded:    Vec<(BiasInstance, ExclusionReason)>,
    conservation: Conservation,          // pool, by-reason counts, judged
}
CandidateGroup { representative: BiasInstance, members: Vec<String> /* node ids */ }
ExclusionReason { Empty, TooShortNoQuestion, StatementType(String) }
```

Pure — no I/O, no state, every branch unit-testable. Order is preserved
(`evidence_id` ascending, as the Cypher returns it) so a re-run judges in the
same order.

**Conservation identity, asserted in a test and rendered on screen:**

```
pool = excluded_empty + excluded_short + excluded_statement_type
     + duplicates_collapsed + judged
judged = relevant + irrelevant + failed
verdict_rows = pool − excluded            (every non-excluded node has a verdict)
```

Where it shows: the run's stored summary carries the block (JSONB — no
migration), and the results panel renders one reconciliation line under the
tiles. The history TABLE keeps its existing "Candidates" column, which becomes
the **pool** number (`candidates_read`), while `candidates_total` — the progress
denominator — becomes the **judged** number. Both columns already exist; only
what is written into them at promote changes. See **R5** on whether Roman wants
the by-reason breakdown in the history row itself rather than in the results
panel.

### 8.2 Piece 1d — the prompt filename becomes a row

`load_scan_prompt` reads `state.settings.current().theme_scan_prompt_file`
instead of `state.config`. The read still happens at **scan start**, so changing
the row changes the next scan with no restart — the freshness law already
guarantees the snapshot is swapped before the PUT returns. `config.rs` loses the
field, the env read, the resolver and its two tests. The per-run provenance
(`scan_runs.resolved_params.prompt_file`) is unchanged and still records which
file judged which run.

Boot assertion in `main.rs`, placed after the registry is built (the registry
owns the template dir) and before the server binds. See **R3** for whether it
refuses to start or logs loudly.

### 8.3 Piece 2 — visible Delete

Card: the `<Link>` and the button stay siblings inside the positioned wrapper —
that reasoning (a `<button>` inside an `<a>` is invalid markup and would
navigate) survives the ruling; only the kebab's popover goes. Detail header:
Delete at the far right of the actions row, two rows away from
`ScenarioStatusControl`. Both keep `scenarioDeleteCopy` and
`ScenarioDeleteConfirm` byte-for-byte. The D7 comment history gets its one-line
amendment in both files and in the structure test.

### 8.4 Piece 3 — the pre-scan screen

Backend: `GET .../facts/cards` gains `never_scanned_notice`, present **only**
when the scenario has zero `scan_runs` rows. The pool is still served — this is
an ordering and honesty fix, not a data change.

Frontend, `ScanSection`: when the notice is present, the section leads with it,
and the queue region renders **collapsed behind an explicit "Browse the raw
evidence pool (148)" control**. `queueRegion`'s `scope: "from all scans"` clause
is gated on a new argument — whether any card is scan-scored — and is `null`
otherwise. Nothing else about the queue changes: no reordering of the ruled/queue
mechanics, no new surface.

### 8.5 Piece 4 — the ✕ asks first

`ScanHistoryTable` gains a confirm that names the run ("Remove the scan run from
Aug 7, 21:14? Its verdicts are deleted with it."), same shape as the existing
merge confirm. The panel keeps the network call and the error box. The re-render
half waits on **R6**.

---

## 9 — TESTS (Roman's standing law: behaviour only, no shape-pinning)

| Test | Asserts | Where |
|---|---|---|
| `dup_collapse_judges_once_and_covers_the_set` | two byte-identical quotes produce ONE group whose members list both node ids | `theme_scan_prefilter_tests.rs` |
| `collapsed_twin_receives_the_same_verdict_row` | persist writes a verdict for every member, so the scorecard join finds both | `theme_scan_persist_tests.rs` |
| `merging_a_collapsed_pick_rules_every_member` | one ruling covers the set | `theme_scan_persist_tests.rs` |
| `prefiltered_items_are_counted_by_reason_and_never_judged` | each reason; the excluded node appears in no group | `theme_scan_prefilter_tests.rs` |
| `conservation_identity_holds_for_a_mixed_pool` | pool = excluded + collapsed + judged, on one pool exercising every branch | `theme_scan_prefilter_tests.rs` |
| `scan_start_reads_the_prompt_the_settings_row_names` | change the row → the next scan loads the other file | `theme_scan_start_tests.rs` |
| `never_scanned_scenario_payload_carries_the_notice` | the notice is present, and absent once a run exists | `scenario_cards` tests |
| `scope_clause_is_absent_until_something_is_scan_scored` | no "from all scans" on an unscanned pool | `queueRegion.test.ts` |
| `delete_from_the_card_and_from_the_header_both_remove_the_scenario` | both call the same delete path and the dialog names the scenario | existing behaviour tests, amended |

No render-restating tests for a button's existence (the instruction forbids it);
the D7 structure test is **amended, not extended** — it stops asserting the
kebab and starts asserting that neither surface can delete without the confirm.

---

## 10 — STANDING RULE COMPLIANCE

- **No silent failures.** Every exclusion is counted by reason and rendered; a
  collapsed twin still gets a verdict row; the prompt row is asserted at boot;
  the run delete asks first. The conservation identity is the rule made
  arithmetic — a lost candidate shows as a sum that does not add up.
- **No hardcoded values.** The min-length threshold and the excluded statement
  types are settings rows, not constants — `referral` is domain vocabulary and
  must not be compiled in (Rule 2). The prompt filename stops being a compiled
  default entirely.
- **Tutorial comments.** New Rust gets `## Rust Learning:` headers where it earns
  them (the group/borrow shape in the prefilter, `Option` as a third state on
  `statement_type`), and `// Why:` on the two real decisions (verdict fan-out;
  pool vs judged in the two existing columns).

---

## 11 — VERIFICATION AND ROLLBACK

- Local: `cargo build`, `cargo test --lib` (the honest baseline — `--workspace`
  is broken on `main` by stale `tests/*.rs`, unrelated to this task and NOT fixed
  here), `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`,
  `cargo check --bins`, `npm run typecheck`, `npx vitest run` (there is no lint
  script in this frontend), `npm run build`, route-link guard green.
- §11 agents on the FINAL commit; if the commit is amended after review, the
  agents re-inspect the amended SHA. I will not `git stash` — the gate agents do,
  and the pop fails on `Cargo.lock`.
- DEV: Roman runs `BUILD_RUNBOOK_BETA_v1` for beta.384; the architect walks the
  deployed build. The one thing I verify on DEV **before** coding is R6.
- Rollback: the branch is unmerged and the migration is additive (three seed rows
  and one notice row). Reverting the commit restores beta.383 behaviour; the
  seeded rows become unread rows, which the store tolerates by design.

---

## 12 — RULINGS NEEDED BEFORE I WRITE CODE

**R1 — `statement_type` reaches Rust via `BiasInstance`.** §3.2. The `question`
precedent, applied again: one column in the gather RETURN, one optional field on
the shared DTO. It touches a DTO three surfaces render. Approve, or name another
route.

**R2 — a collapsed twin still gets its own verdict row.** §3.3. My
recommendation: judge the group ONCE, then write the identical verdict for every
member, so the scorecard's `graph_node_id` join still finds Roman's ledger rows
and the audit trail has no holes. The alternative — verdicts only for the
representative — makes the scan look worse on the scorecard than it is. Confirm.

**R3 — what the boot assertion does when the named prompt file is missing.**
§3.4. Options: (a) refuse to boot **and** make the Settings write path check that
the filename resolves, so the page refuses a typo instead of arming a crash at
the next restart — my recommendation, and the only one consistent with
`trial_snapshot`'s promise; (b) refuse to boot, no write-path check (a typo on
the Settings page becomes a failure to start, hours later); (c) log loudly at
boot and let the scan-start error carry it (weaker than the instruction's word
"assertion"). Recommend (a).

**R4 — the pre-filter's min-length seed.** The measurement counts 41 of 148
quotes under 60 characters, but does **not** report how many of those also lack a
paired question — so the blast radius of the design's "under N chars with no
question" rule is unmeasured. I recommend seeding `theme_scan_prefilter_min_chars
= 60` per the design, because the first run then REPORTS the count by reason and
you can lower it from the Settings page with no build. Confirm 60, or name
another number.

**R5 — where the by-reason breakdown lives.** The instruction says conservation
must reconcile "in the scan history entry". The counts fit the stored summary
(JSONB, no migration) and render in the results panel when a run is open; the
history TABLE row would show pool → judged only, unless you want three more
columns there. Recommend summary + results panel, history row unchanged in shape.

**R6 — Piece 4a may already work.** §3.5. Clicking a history row is wired
end-to-end in the source, including the backend annotation written for exactly
this case. Before building a second path over a working one, I verify on DEV
(browser, your password) and report. If it reproduces, I fix the real cause; if
it does not, the honest fix is an affordance — the history disclosure is
collapsed by default and the rows do not look clickable — and that is a different,
smaller change I would bring back for your word.

**R7 — the ScenarioKebab deletion.** `rm` is denied to me. I will remove every
import and usage, and if `git rm` is also refused, the report hands you the
one-line command. Confirm you would rather have that than the file left orphaned
in the tree (the structure test can be made to fail until it is gone).

---

**Nothing built. Nothing committed beyond this file's branch. Awaiting rulings
R1–R7.**
