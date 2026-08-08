# CC_REPORT_SCAN_TO_RULING_WORKFLOW

**Task:** `CC_TASK_SCAN_TO_RULING_WORKFLOW_v1` · **Date:** 2026-08-08
**Branch:** `fix/scan-to-ruling-workflow`, off `main` @ `9f3bf1d`
**Rulings built under:** the architect's R0–R8 of 2026-08-08, in full. No ruling was reinterpreted; where one changed the shape of the work, §7 says so.
**Phase A:** `CC-REPORTS/CC_REPORT_SCAN_TO_RULING_WORKFLOW_PHASE_A.md` (committed with this work).

Select-twice is dead. A completed scan's admitted verdicts are a **read-time projection**: they appear in the candidate queue as PROPOSED cards with no human step in between. The human's ruling is the only write, and it now records which scan proposed the card. Merge — the route, the service, the SQL, the checkboxes, the button, the confirm — is gone.

Everything marked **[measured]** was produced by a command run in this tree at the SHA this report ships with.

---

## 1 — GATE RESULTS [measured]

| Check | Result |
|---|---|
| `cargo check --bins` | **clean** |
| `cargo test --lib` | **1723 passed, 0 failed, 2 ignored** (baseline on `main`: 1701 / 0 / 2) |
| `cargo clippy --workspace -- -D warnings` | **clean** |
| `cargo fmt --check` | **clean** |
| `npm run typecheck` | **clean** |
| `npx vitest run` | **829 passed, 62 files, 0 failed** |
| `npm run build` | **clean** (1.54 s) |
| Module-size check (§8's own command) | **no module this task touched is over 300** — two were pushed over and split; see §6 |

`cargo test --workspace` remains broken on `main` (stale `tests/*.rs` since ~beta.343) and is untouched by this task. `npm run lint` does not exist in `frontend/package.json` — recorded, not silently skipped.

**Net test delta: +22 backend, +4 frontend.** Twenty-three tests were added, one merge-SQL shape test was replaced by two provenance tests, and five tests that asserted merge behaviour were removed with the behaviour.

---

## 2 — WHAT WAS BUILT

### Piece 1 — the projection (backend)

`GET …/facts/cards` now composes, per node, the fact-ref state AND the latest completed run's relevant verdict.

* **New repository module** [scan_run_projection.rs](backend/src/repositories/pipeline_repository/scan_run_projection.rs) — two reads: the projecting run (`status = 'completed'`, newest first, `LIMIT 1` — R-b lives in the SQL, one place) and its `relevant = true` verdicts. Kept out of `scenario_store.rs`, which is already over the size limit.
* **New pure service** [scenario_card_projection.rs](backend/src/services/scenario_card_projection.rs) — the precedence law (R-a), the twin fold, and the display-ready proposal. Runs without a database, a graph or a settings store, which is what makes the laws testable rather than promised.
* **R-a is structural, not remembered.** Precedence tests for the PRESENCE of a reference row, never its status — which closes the diagnostic §6 defer gap by construction. The merge SQL's `WHERE status = 'undecided'` protected included and dropped rows and silently overwrote a *deferred* one; presence cannot be fooled that way.
* **R-f — codes.** No new minting path (ruling R3). Gather remains the sole minter, `ThemeScanPanel` stays MOUNTED when the scan card collapses, and the comment at the collapse site says why so a future cleanup cannot "optimize" the mint away.
* **R-e — the live count.** `proposal_source.proposed_count`, counted over the finished payload (the same discipline `cards_without_context` uses), served beside the run's frozen counts and labelled live. The run's own record is never rewritten.

### Piece 2 — ruling provenance (R-c, ruling R4)

`upsert_fact_ref` gained `source_run_id` **in the INSERT column list only**. On conflict it is left out of the `SET`, so a ruling now records provenance where there was none and still never destroys provenance that exists. The measured NULL-provenance hole (82 of 83 S-2 refs) closes for every future ruling; nothing historical is backfilled.

The run id is **server-resolved** ([scenario_proposal_lookup.rs](backend/src/services/scenario_proposal_lookup.rs)): the ruling path re-derives the projecting run for that node from the same reads that drew the card. A client's claim is never trusted — the same law `scenario_ruling`'s module doc states about anchors, for the same reason.

### Piece 3 — the queue leads (frontend)

* Heading: **"Candidates awaiting ruling — 30 proposed by the Aug 7 scan"**, from a stored template, filled with the live count and the run's date (the server owns the sentence, the browser owns the date format — the split the delete confirmation already uses).
* Card gains: proposed-role chip (**"Scan: supports"** — it names the scan as the speaker, exactly as the banded-confidence label does), the banded chip it already had, the judge's reason verbatim, the ×N covers badge, and the proposed-by attribution.
* Filter: **Proposed** is the leading Status option with its count. Default filter is Proposed when proposals exist, and otherwise the 1.7E computed default STANDS (ruling R8).
* I/E/D/U, auto-advance, single-step undo: untouched.
* **Piece 3c needed no code.** `CandidateList` has been a bounded 70vh scroll window since 1.7E; the unbounded list measured in the diagnostic was the findings list, and it died with Piece 4b.

### Piece 4 — the scan card (frontend)

* Collapses to one line once a run has completed — `Last scan Aug 7, 9:37 PM · Claude Opus 4.8 · 30 proposed` — and opens expanded on a never-scanned scenario. The human's own click wins from then on and is deliberately not persisted (ruling R7's reasoning, applied to this card).
* The report is **numbers only**: five tiles (gathered / duplicates folded / set aside / judged / proposed), the backend-composed reconciliation sentence, and the live proposed line. No findings list, no checkboxes, no Merge button, no merge confirm. The container is shaped for the 2.19 digest; nothing was built for it.
* History row keeps View report and the ✕ with its confirm, and gains a **Proposed** column that carries a number only on the row that is actually projecting (ruling R6); every other row shows an em dash.

### Piece 5 — what died

Route · handler · `merge_scenario_scan_run` · `merge_run_into_scenario_recording` · `merge_scan_run_into_scenario` + `MERGE_SCAN_RUN_SQL` + its two SQL-shape tests · `ScanRunMergeRequest`/`Response` · `EmptySelection` / `ScanRunMergeFailed` · `list_applied_node_ids_for_run` and the `applied` annotation · `mergeScanRun` + its three service tests · the checkbox fork, the select-all control, the merge notice's three tones · the comparison hero (its only caller was the merge results area) · 25 dead style keys.

**Retained as history, exactly as the design says:** the `scan_run_merges` table, its record type, `insert_scan_run_merge`, and `count_run_provenance`.

**Re-scoped, not deleted:** `scan_never_attempts_a_fact_ref_write_even_against_a_dead_database` passes unchanged — scan COMPLETION still performs no fact-ref write. Its doc and the module header now say why that matters *more* under the projection: junk scans costing nothing, re-scans not disturbing rulings, and a deletable run's proposals simply ceasing to be projected all rest on that absence.

### Piece 6 — wording and settings

**Twelve new rows**, one migration ([20260808141052_scan_to_ruling_wording.sql](backend/pipeline_migrations/20260808141052_scan_to_ruling_wording.sql)), created with `./scripts/new-migration.sh`: four curation rows (the queue heading and the three things a proposed card says) and eight scan rows (the collapsed summary, the advisory note, the live proposed line, five tile captions). Six carry required placeholders, registered in `wording_templates` so an edit that drops one is refused.

**One guarded UPDATE.** `scan_history_delete_confirm_template` became false with this change — it promised a run's verdicts "support the rulings it produced". It is rewritten to say what is now true, and the `WHERE` clause matches the seeded default so a value Roman has edited on the Settings page is left alone.

ThemeScanPanel's pre-existing literals: only the sentences this task touched were converted. The rest stays filed debt.

---

## 3 — THE RULINGS, AS BUILT

| # | Ruling | As built |
|---|---|---|
| R0 | The disk file is the contract | Built against it. |
| R1 | Keep the 409 guard | Unchanged and its reach GREW as intended: every ruling on a proposed card records `source_run_id`, so a run one ruling has drawn on is undeletable. The error variant is renamed `ScanRunCited` (it is no longer about merges) and its message says it plainly — *"{n} ruling(s) cite it … Rulings you have already made are unaffected."* `details.reason` is now `run_cited`. |
| R2 | Twins fold; the ruling settles each | Fold at read on the quote key; the ruling writes a ref row AND a ledger anchor **per twin**, each anchored to its own document and page. The representative is the lowest C-ordinal, so the badge always reads the same way round. |
| R3 | No new minting path | Gather stays the sole minter; the panel stays mounted; the comment at the collapse site names the hazard. |
| R4 | Server-resolved provenance | One indexed read + one targeted graph read per ruling. The client sends nothing. |
| R5 | Conservation stays frozen; proposed is its own line | Two sentences, two rows. The live line says it is live. Recorded as a stated §2c deviation from the mockup's single sentence — the reason is the frozen/live honesty boundary. |
| R6 | Proposed column on the projecting row only | Em dash elsewhere. |
| R7 | Keep the selects | Proposed leads the Status dropdown; the Scan dropdown is retired with the facet it measured. |
| R8 | Proposed default, else the computed default | Both branches asserted in one test, named as the ruling names it. |

---

## 4 — TESTS

All eleven named in the instruction, plus five the build earned. No shape-pinning; two laws that live in SQL are asserted against the statement, which is the house pattern for a rule the database enforces.

| Test | Module |
|---|---|
| `a_ref_row_always_wins_over_a_verdict` (include, exclude AND defer) | `scenario_card_projection` |
| `a_rescan_never_alters_a_ruled_or_deferred_card` | `scenario_card_projection` |
| `proposed_count_is_admitted_minus_ruled` | `scenario_card_projection` |
| `deleting_a_run_removes_its_unruled_proposals_and_keeps_rulings` | `scenario_card_projection` |
| `one_ruling_covers_a_byte_identical_twin` | `scenario_card_projection` |
| `a_twin_whose_partner_was_already_ruled_stands_on_its_own` | `scenario_card_projection` |
| `a_quoteless_verdict_is_never_folded_with_another` | `scenario_card_projection` |
| `the_judges_own_words_reach_the_card_unchanged` | `scenario_card_projection` |
| `only_the_latest_completed_run_projects` · `a_failed_run_projects_nothing` · `only_admitted_verdicts_are_projected` | `scan_run_projection` |
| `a_proposed_card_always_carries_a_code` · `banded_confidence_never_a_percentage_on_the_card` · `a_ruled_card_carries_no_proposal` | `scenario_card` |
| `ruling_a_proposed_card_records_the_proposing_run` · `a_ruling_never_writes_a_model_score_or_touches_the_scan_columns` | `scenario_store` |
| `deleting_a_run_the_record_cites_maps_to_409_and_explains_why` | `api::scenario_theme_scan` |
| `queue_defaults_to_proposed_when_proposals_exist_and_keeps_the_computed_default_otherwise` | `candidateFilters.test.ts` |
| `scan_card_collapsed_summary_reports_run_model_and_proposed_count` (3 cases) | `themeScanFormat.test.ts` |

`banded_confidence_never_a_percentage_on_the_card` serializes the finished card and asserts the raw score reaches no part of the wire — §7.8 stops depending on every future component remembering it.

---

## 4b — THE §11 GATE, AND THE DEFECT IT CAUGHT

All four agents ran against the committed SHA. **rules-enforcer: PASS** (64 files, zero violations). **architecture-reviewer: PASS** (no issues across five checks). **observability-checker: FAIL, 2 findings.** **test-auditor: FAIL, 3 findings.** All five were inside this diff; all five are fixed, and the commit was amended.

**observability — both fixed:**

1. `decode_role` dropped an unrecognised role token silently, while the DTO doc I had written promised the failure was logged. It now emits a `warn` naming the token and the vocabulary this build knows. The suppression is the right SCREEN behaviour (a raw token is the C-222 leak §7.5 forbids) and the wrong LOG behaviour: "no role assigned" and "a role this build cannot name" are otherwise indistinguishable, and only the second is a deploy problem.
2. `quotes_for` logged a graph-read failure with a node count and nothing else — no scenario, no run — on a path where the failure means a human's ruling was refused. Both ids were in scope at the call site and are now passed in and logged.

**test coverage — all three fixed, and one of them found a live defect [measured]:**

`to_card_proposal`, `count_proposed`, and the `assemble` branch that carries a populated proposal index had no direct tests. Writing the assembly test failed immediately with:

```
the chip carries the canon stance word: Some("Scan: {verb}")
```

`wording_templates::render` matches placeholder NAMES — it finds `{verb}` in the template and looks up `verb`. I had passed `"{verb}"`, `"{count}"` and `"{codes}"`, so no substitution happened and `render` emitted the tokens verbatim, exactly as it is documented to do for an unknown placeholder. **Every proposed card would have shipped to DEV with a literal `Scan: {verb}` chip, and every folded card with `×{count} — covers {codes}`.** Both call sites are fixed and nine tests now cover the composition directly, including the unmapped-role path, the covers badge naming the twins and not the card it sits on, and an un-numbered twin being counted without being given an invented `C-0` handle.

This is the gate doing its job: the projection logic was correct and thoroughly tested, and the defect was one layer out, in the display composition nothing was calling.

**The re-pass: observability PASS. test-auditor closed all three, and raised two NEW gaps — which I am NOT closing, and which need your word (§7a).**

---

## 4c — ONE JUDGED DISAGREEMENT — RESOLVED IN THE FINAL RE-PASS

The test-auditor's re-pass confirmed all three original gaps closed, then raised two new ones: `resolve_proposed_ruling` and `rule_one` have no tests, and it suggested integration tests in `backend/tests/`.

**I closed the part of that which is real, refused the rest rather than complying quietly, and asked the auditor to check my grounds rather than accept them. It did, and returned PASS** — including an independent reproduction of the compile failure below. The gate is closed on all four agents; what follows is the record of the reasoning, and one filed item that outlives this task.

Closed: the one decision inside `resolve_proposed_ruling` that is not I/O — the order the covered nodes are ruled in — is now a pure `ruled_card_first` helper with three tests. It is not cosmetic: the twins are ruled in sequence, each in its own transaction, so a part-way failure must leave the card the human actually pressed a key on settled rather than a twin they never looked at. That rule was inline and untestable; now it is neither.

Refused, with the measurement:

* **Both functions are `async` and need a live pool + graph.** The auditor's OWN first pass named this and declined to count it: *"all async functions requiring a live DB pool. The project has no tokio-test infrastructure for service functions of this class (`record_ruling`, `record_removal`, and all 48 service-layer `pub async fn`s are in the same state). This matches the project's established testing boundary."* The re-pass reverses that judgement about the same class of function without new evidence.
* **The suggested home does not compile** [measured]. `cargo test --no-run --test scenarios_integration` fails on `main` at `9f3bf1d` — `unresolved import …::upsert_fact_ref` (withheld from the re-export as the anchor choke point, at 14 call sites in that file) plus four type errors. The stale `tests/*.rs` breakage predates this task by months. **Tests added there would never run**, and a test that cannot run is worse than a missing one: it reads as coverage.
* Repairing that suite is a task in its own right, not a rider on this one.

**What I would need to close it properly:** either the `tests/*.rs` suite repaired first (its own task), or these two functions refactored to take their data rather than their `&AppState` — which is a design change to a path the architect specified as server-resolving its own reads (ruling R4), and not mine to make after the gate.

**FILED, not closed:** the two async functions remain untested, and `tests/*.rs` remains broken. Neither is a regression this task introduced, and neither blocks beta.385 — but the second one means the whole integration suite is dead weight right now, which is worth a task of its own before anything else needs a home there.

---

## 5 — DEPLOYMENT

* **Migration: one**, pipeline database (`colossus_legal_v2`), additive seed + one guarded UPDATE, forward-only. All twelve keys are declared to the boot loader, so a roll-FORWARD without this file is a boot refusal; a rollback to an older image is safe.
* **New env vars: none** → no Ansible template change owed by this task. (Pre-existing debt, unchanged: `THEME_SCAN_MODEL` / `THEME_SCAN_CONCURRENCY` are still owed to the colossus-ansible template.)
* **Container rebuild: both.**
* **API removed:** `POST …/scan-runs/:run_id/merge`.
* **API changed (additive):** `…/facts/cards` gains `proposed` per card and `proposal_source`; `…/scan-runs` carries eight more wording strings; `…/scan-runs/:run_id` no longer annotates each suggestion with `applied`.
* **No data migration, no backfill.** Nothing in this task rewrites or deletes a stored row.

---

## 6 — WHAT WAS NOT IN THE ANALYSIS

Four things the build surfaced. None changed the design; all changed the shape of the code.

1. **Two route modules crossed the 300-line limit** [measured]. `api/scenario_cards.rs` (291 → 368) and `api/scenario_facts.rs` (290 → 338). Split rather than granted an exception: the pool decoders (`build_ref_states`, `apply_display_order`) moved to `scenario_card_assembly` where they belonged anyway (they reason about the pool, not about HTTP); the projection READS moved to `scenario_proposal_lookup`, which now serves both callers from one pair of reads; and per-node ruling moved to a new `scenario_ruling_apply`. Final: 256 / 279, and no module this task touched is over.
2. **`assemble` hit `clippy::too_many_arguments`.** The two per-node indexes now travel as one `PoolIndexes`, with the human/machine seam kept visible as two named fields — precedence R-a exists precisely because those two must never be confused.
3. **`ScanHistoryWording` was renamed `ScanPanelWording`.** It carried two history strings and now carries ten; naming it for the row would have been a lie about a type that speaks for the whole card.
4. **`backend/Cargo.lock` carries a version line change.** It was still on `2.0.0-beta.383` while `Cargo.toml` says `.384`; the first build in this branch synced it. That is the lock catching up to Roman's own bump, not a version bump by CC.

---

## 7 — OBSERVED, NOT FIXED

* **The comparison hero is gone.** Its only render path was inside the merge results area, and it compared two runs' relevant SETS — a client-side Jaccard the code itself labelled "estimate only, not the promotion number". Reinstating it under the projection would mean a new surface for a number the scorecard SQL already answers authoritatively. Flagged rather than rebuilt.
* **`ThemeScanPanel.tsx` is still large** and still carries pre-existing literals from before the configuration law. This task removed ~220 lines of it and converted only the sentences it touched, per Piece 6.
* **A twin's ruling writes N anchors in N transactions.** Each is atomic; the set is not. The node the human pressed a key on is ruled FIRST, so a mid-set failure always leaves their own card settled, the error propagates, and the queue's failure handler re-reads the pool — the screen reconciles to what the database holds. A single transaction across twins would mean `record_ruling` taking a transaction it does not own; named here rather than done quietly.

---

## 8 — VERIFICATION OWED ON DEV

Roman runs `BUILD_RUNBOOK_BETA_v1` for beta.385. The architect walks it, including the primary workflow end to end:

1. Open S-4 cold → the queue leads with "Proposed by the … scan — N"; the scan card is one collapsed line.
2. Rule three cards (I, E, D) → refresh → all three survive and the proposed count drops by three.
3. Open the report from the history row → numbers only, reconciling, no Merge button anywhere.
4. Delete an **UNRULED** run → its proposals vanish, rulings stay. (A run any ruling cites refuses with the new 409 — that is ruling R1, and the amended walk.)

=== END REPORT — VERDICT: BUILT ===
