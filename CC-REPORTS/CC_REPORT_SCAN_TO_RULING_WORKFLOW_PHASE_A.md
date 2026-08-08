# CC_REPORT_SCAN_TO_RULING_WORKFLOW — PHASE A (pre-coding analysis)

**Task:** `CC_TASK_SCAN_TO_RULING_WORKFLOW_v1` · **Date:** 2026-08-08
**Design read:** SCAN_TO_RULING_WORKFLOW_REDESIGN_v1.md · SCAN_PAGE_MOCKUP_v1_2026-08-08.html (v1.1 in the file's own banner — see R0) · CC_REPORT_SCAN_WORKFLOW_DIAGNOSTIC.md · SCENARIO_FUNCTION_REQUIREMENTS_v2 §2b/§2c/§6/§7
**Branch:** `fix/scan-to-ruling-workflow`, created off `main` at `9f3bf1d`, working tree clean.
**Status: STOPPED at the Phase A gate.** No product code written. **Eight rulings** are needed before implementation starts — §12.

Everything marked **[measured]** was read out of this tree today at `9f3bf1d`. Line numbers are as of that SHA. The diagnostic's own measurements are cited, not repeated.

---

## 1 — TASK UNDERSTANDING

Select-twice dies. A completed scan's admitted verdicts become a **read-time projection** served by the cards route, so they appear in the queue as PROPOSED candidates with no human bridging step. The human's ruling stays the only write, and gains the proposing run's id as provenance. Merge — the endpoint, the checkboxes, the button, the confirm — is removed; the findings list dies with it and the scan card becomes a collapsible numbers-only report.

Five pieces, one branch, riding as beta.385: the projection (backend), ruling provenance (backend), the queue leading with proposals (frontend), the scan card (frontend), and the removals.

---

## 2 — BRANCH AND BASELINE VERIFICATION [measured]

- Current branch: `fix/scan-to-ruling-workflow` (off `main`)
- Working tree clean: **YES** (this report is the only new file)
- Last commit: `9f3bf1d docs(cc-report): scan-to-ruling workflow diagnostic`
- `cargo check --bins`: **clean** (39.8 s)
- `cargo test --lib`: **1701 passed, 0 failed, 2 ignored** — the honest baseline. `cargo test --workspace` remains broken on `main` (stale `tests/*.rs` since ~beta.343), unrelated to this task and not repaired by it.
- `npm run typecheck`: **clean**
- `npm run lint`: **there is no lint script in `frontend/package.json`** — recorded, not silently skipped.
- Module-size check (§8's own command): the files this task touches are all UNDER 300, with **one exception** — `repositories/pipeline_repository/scenario_store.rs` is already **551**. New reads therefore go in a NEW repository module rather than growing an existing violation.

---

## 3 — THE MEASUREMENTS THIS DESIGN TURNS ON

Eight facts the design does not state and the implementation depends on. Four of them force a ruling.

### 3.1 — Every byte-identical twin already has its OWN relevant verdict row [measured]

`process_one` fans the group's single judgment out to every member: one `scan_run_verdicts` row per node, each carrying `relevant = true`, the same role, confidence and reason ([theme_scan_persist.rs:141-155](backend/src/services/theme_scan_persist.rs#L141)), pinned by `every_member_of_a_collapsed_group_gets_its_own_verdict_row`.

So a naive projection of "the latest completed run's relevant verdicts" serves **C-45 and C-46 as two separate proposed cards** — which is also exactly what the queue does today (the pool is one card per Evidence node; the twins have always both been in the 148). The mockup instead shows ONE card badged `×2 — covers C-46`. Folding is a genuine change to the card count, not a badge. → **R2.**

The fold key, if we fold, exists and is one line: the verbatim quote text ([theme_scan_prefilter.rs:265](backend/src/services/theme_scan_prefilter.rs#L265)).

### 3.2 — Piece 2 collides head-on with the delete guard [measured]

The 409 provenance gate counts BOTH merge events and attributed facts:

```sql
(SELECT COUNT(*) FROM scan_run_merges     WHERE run_id = $1) AS merge_events,
(SELECT COUNT(*) FROM scenario_fact_refs  WHERE source_run_id = $1) AS attributed_facts
```
— [scan_run_merges.rs:168-170](backend/src/repositories/pipeline_repository/scan_run_merges.rs#L168), and `is_protected()` is `merge_events > 0 || attributed_facts > 0` ([:159](backend/src/repositories/pipeline_repository/scan_run_merges.rs#L159)).

Piece 2 makes every ruling of a proposed card write `source_run_id`. **The first ruling therefore makes that run permanently undeletable (409).** The instruction's "verify the 409 guard still permits deleting never-merged runs" is satisfied only for a run nobody has ruled from; R-d's "deleting a run deletes its unruled proposals with it" stops being reachable the moment the human rules one card. This is not a bug I can code around — it is a policy question. → **R1.**

### 3.3 — Ordinals: gather mints them, and gather is called by the panel Piece 4a collapses [measured]

- Minting happens in exactly one place: `ensure_candidate_ordinals` on the gather route ([scenario_gather.rs:395-428](backend/src/api/scenario_gather.rs#L395)).
- The cards route deliberately does NOT mint, and says why: two endpoints racing to mint the same ordinal ([scenario_cards.rs:64-72](backend/src/api/scenario_cards.rs#L64)).
- That race is real, not theoretical: the insert is `ON CONFLICT (scenario_id, graph_node_id) DO NOTHING` but the table also carries a unique `(scenario_id, ordinal)`, and a concurrent mint is documented as a LOUD failure ([scenario_candidate_ordinals.rs:63-74, :95-98](backend/src/repositories/pipeline_repository/scenario_candidate_ordinals.rs#L63)). The page already calls `/facts/cards` and `/facts/gather` **concurrently on load** — cards from `ScenarioDetailPage` ([:258](frontend/src/pages/ScenarioDetailPage.tsx#L258)) and from `CardQueue` ([:249](frontend/src/components/CardQueue.tsx#L249)); gather from `ThemeScanPanel`'s mount effect ([:238](frontend/src/components/ThemeScanPanel.tsx#L238)).
- **Every projectable node is a pool node**, and gather numbers the whole pool — so `code: null` on a proposed card can only happen if gather has not run since that node appeared.

Conclusion: R-f needs no new minting path *provided `ThemeScanPanel` stays mounted when the scan card collapses* (collapse hides the body, it does not unmount the component). If the architect wants minting moved into the cards route instead, that is a race to design for, not a line to add. → **R3.**

### 3.4 — `upsert_fact_ref` omits `source_run_id` deliberately, and the omission does two jobs [measured]

It is absent from both the column list and the `DO UPDATE SET` ([scenario_store.rs:673-729](backend/src/repositories/pipeline_repository/scenario_store.rs#L673)): on INSERT the column defaults NULL (a hand-tagged fact is human-authored); on CONFLICT it PRESERVES whatever run was already recorded.

Piece 2 is therefore a two-line change with an exact shape: **add `source_run_id` to the INSERT column list only, and leave the `DO UPDATE SET` untouched.** A proposed card has no ref row by definition, so the INSERT path is the one that fires, and "a human ruling never invents provenance and never destroys it" survives verbatim. Where the run id comes from is the open question. → **R4.**

### 3.5 — The card DTO has nowhere to put a proposal [measured]

`ScenarioCard` ([dto/scenario_card.rs:281-385](backend/src/dto/scenario_card.rs#L281)) carries no proposed role, no judge's reason, no run attribution, no duplicate coverage. Both the Rust DTO and its TS mirror are `#[serde(deny_unknown_fields)]` / structurally mirrored, so this is a paired change: a new nested `proposed` object, present exactly when the card is a projection.

Note the status arithmetic: a projected node has **no ref row**, so `status` decodes to `Undecided` ([scenario_card.rs:483](backend/src/services/scenario_card.rs#L483)) and the browser's `candidateState()` calls it `not_ruled` ([candidateFilters.ts:62-66](frontend/src/components/candidateFilters.ts#L62)). "Proposed" is therefore `not_ruled` AND `proposed != null` — a subset of not-ruled, exactly as `rulable` is, and it must be counted that way or the facet totals stop reconciling (§9).

### 3.6 — The "scan-scored" facet's two defects both die by construction [measured]

`isScanScored` is `band !== "unscored"` ([candidateFilters.ts:96](frontend/src/components/candidateFilters.ts#L96)), and a band exists only where a merge wrote a confidence. Under the projection the band comes from the verdict, and precedence R-a keeps any human-touched card out of Proposed regardless of what `include` did to its confidence ([scenario_ruling.rs:346-348](backend/src/services/scenario_ruling.rs#L346)). Both diagnostic §3 defects — "scored = merged" and "include NULLs the confidence, so an included card leaves the facet" — stop being expressible.

### 3.7 — Piece 3c is already built; the unbounded list is the one that dies [measured]

`CandidateList`'s scroll region is `maxHeight: 70vh; overflowY: auto` ([CandidateList.tsx:44-53](frontend/src/components/CandidateList.tsx#L44)). The unbounded list measured in diagnostic §7 is `RunResult`'s findings map inside `ThemeScanPanel`, which Piece 4b deletes. **Piece 3c requires no new code** — it is satisfied by what exists plus what Piece 4b removes. I will assert it, not build it.

### 3.8 — "Latest completed run" has an ordering ambiguity [measured]

`scan_runs` records `started_at` and `duration_ms`; the history orders `ORDER BY started_at DESC` ([scan_runs.rs:510-515](backend/src/repositories/pipeline_repository/scan_runs.rs#L510)) and `count_completed_scan_runs` filters `status = 'completed'` ([:483](backend/src/repositories/pipeline_repository/scan_runs.rs#L483)). There is no completion timestamp column. A run started earlier but finishing later would be "latest" by completion and not by start. Scans are serialised in practice (one active run per scenario, swept at boot), so I will use `started_at DESC` for consistency with the history the human is reading — stated here rather than silently chosen.

---

## 4 — FILES TO MODIFY

| File | Change | Est. lines |
|---|---|---|
| `backend/src/api/scenario_cards.rs` | two new reads (projecting run + its verdicts), thread the projection into `assemble`, serve `proposal_source` | ~70 |
| `backend/src/services/scenario_card_assembly.rs` | accept the projection index; pass per-node proposals to `build_card` | ~25 |
| `backend/src/services/scenario_card.rs` | build the `proposed` block; band from the verdict when there is no ref row | ~45 |
| `backend/src/dto/scenario_card.rs` | `CardProposal` + `ScenarioCardsResponse.proposal_source` | ~70 (mostly doc) |
| `backend/src/repositories/pipeline_repository/scenario_store.rs` | `upsert_fact_ref` gains `source_run_id` (INSERT list only); `MERGE_SCAN_RUN_SQL` + `merge_scan_run_into_scenario` removed | +12 / −60 |
| `backend/src/services/scenario_ruling.rs` | `RulingRequest` carries the proposing run; threaded to the upsert | ~20 |
| `backend/src/api/scenario_facts.rs` | resolve the proposing run before ruling (R4) | ~30 |
| `backend/src/repositories/pipeline_repository/mod.rs` | re-export the new module; drop the merge re-exports | ~10 |
| `backend/src/api/mod.rs` | remove the `/merge` route | −6 |
| `backend/src/api/scenario_theme_scan.rs` | remove the merge handler | −40 |
| `backend/src/services/theme_scan_run.rs` | remove `merge_scenario_scan_run`; delete-guard change per R1 | −70 |
| `backend/src/services/theme_scan.rs` | retire `EmptySelection` / `ScanRunMergeFailed` error variants | −25 |
| `backend/src/services/scan_run_enrich.rs` | the `applied` annotation dies with merge | −40 |
| `backend/src/services/theme_scan_run.rs` (annotate) | drop `list_applied_node_ids_for_run` call; add the live proposed count (Piece 1d) | ~25 |
| `backend/src/dto/theme_scan.rs` | drop `ScanRunMergeRequest`/`Response`; `ThemeScanSuggestion` loses nothing (the report keeps no list — see R5) | −40 |
| `backend/src/domain/wording_scan.rs` | new keys for the scan card's summary + report sentences | ~35 |
| `backend/src/domain/wording.rs` | new key for the queue's proposed heading | ~15 |
| `backend/src/services/theme_scan_persist.rs` | re-scope the module doc + the dead-database test's wording (Piece 5) | ~15 |
| `frontend/src/services/scenarioCards.ts` | mirror `CardProposal` + `proposal_source` | ~55 |
| `frontend/src/services/themeScan.ts` | remove `mergeScanRun` | −35 |
| `frontend/src/components/candidateFilters.ts` | the Proposed facet; the Scan dropdown per R7 | ~50 |
| `frontend/src/components/CandidateFilterBar.tsx` | the facet's option + counts | ~20 |
| `frontend/src/components/CandidateCard.tsx` | proposed-role chip, banded-confidence chip, judge's reason, ×N badge, proposed-by line | ~60 |
| `frontend/src/components/CardQueue.tsx` | default to Proposed when proposals exist | ~20 |
| `frontend/src/components/ScanSection.tsx` | the proposed heading; the collapsible scan card | ~55 |
| `frontend/src/components/queueRegion.ts` | the heading's wording seam | ~30 |
| `frontend/src/components/ThemeScanPanel.tsx` | remove merge state/handler/notice, checkboxes, findings list; numbers-only report; collapsed-by-default header | −220 / +70 |
| `frontend/src/services/__tests__/themeScan.test.ts` | drop the merge cases | −40 |

## 5 — FILES TO CREATE

| File | Purpose | Est. lines |
|---|---|---|
| `backend/src/repositories/pipeline_repository/scan_run_projection.rs` | the projecting run's header, its relevant verdicts, and the live proposed count — kept out of the already-oversized `scenario_store.rs` | ~140 |
| `backend/src/services/scenario_card_projection.rs` | PURE: precedence (R-a), latest-run gating (R-b), twin folding per R2, proposal assembly | ~150 |
| `backend/src/services/scenario_card_projection_tests.rs` | the six precedence/projection behaviour tests | ~200 |
| `backend/pipeline_migrations/<ts>_scan_to_ruling_wording.sql` | the new wording rows (Piece 6) | ~60 |
| `frontend/src/components/__tests__/candidateProposal.test.ts` | the two frontend behaviour tests | ~90 |

---

## 6 — DEPENDENCIES / EXTERNAL SURFACE

- New crates: **None.** Removed crates: **None.**
- New env vars: **None.** (Pre-existing debt, not this task: `THEME_SCAN_MODEL` / `THEME_SCAN_CONCURRENCY` are still owed to the colossus-ansible template.)
- New config: **wording rows only** — every one of them a migration seed (Piece 6).
- **API endpoints REMOVED:** `POST /cases/:slug/scenarios/:scenario_id/scan-runs/:run_id/merge` ([api/mod.rs:206-209](backend/src/api/mod.rs#L206)).
- **API payloads CHANGED (additive):** `GET …/facts/cards` gains `proposed` per card and `proposal_source` on the response; `GET …/scan-runs/:run_id` loses each suggestion's `applied` flag and gains the live proposed count.
- Merge-surface inventory, so nothing is left orphaned: route · handler · `merge_scenario_scan_run` · `merge_run_into_scenario_recording` · `merge_scan_run_into_scenario` + `MERGE_SCAN_RUN_SQL` + its two SQL-shape tests · `ScanRunMergeRequest`/`Response` · `EmptySelection`/`ScanRunMergeFailed` · `list_applied_node_ids_for_run` + `scan_run_enrich`'s `applied` annotation · `mergeScanRun` + its service tests · `SuggestionRow`'s checkbox fork · the merge notice's three tones.
- **RETAINED as history, exactly as the design says:** the `scan_run_merges` table, its record type, `insert_scan_run_merge`'s row shape, and `count_run_provenance` (whose fate is R1).

---

## 7 — RUST PATTERNS TO IMPLEMENT

| Pattern | Where | Why |
|---|---|---|
| Pure service module over borrowed indexes | `scenario_card_projection.rs` | the precedence law must be testable without a database — same seam as `scenario_card_assembly` |
| `Option<T>` as the load-bearing absence | `CardProposal` on the card | "no proposal" and "a proposal with no reason" are different states and never collapse |
| Newtype-free plain data + `deny_unknown_fields` | the DTO pair | a client typo is a 400, not a silently-ignored field |
| INSERT-list-only column in an upsert | `upsert_fact_ref` | writes provenance on creation, preserves it on conflict — the property is in the SQL, not in a comment |
| `## Rust Learning:` on the fold | `scenario_card_projection.rs` | `HashMap<&str, Vec<…>>` borrowing its keys from the pool being walked — the same lifetime lesson `collapse_exact_duplicates` teaches, at a second call site |

---

## 8 — TESTS TO WRITE

Behaviour only, no shape-pinning. All ten named in the instruction are mapped; none restates code.

| Test | Asserts | Module |
|---|---|---|
| `a_ref_row_always_wins_over_a_verdict` | included, excluded AND **deferred** (undecided + reason) nodes are never re-proposed nor altered — the diagnostic §6 defer gap | `scenario_card_projection_tests` |
| `only_the_latest_completed_run_projects` | two completed runs → only the newer one's verdicts appear | same |
| `a_failed_run_projects_nothing` | a `failed`/`running` run yields no proposals even with relevant verdicts | same |
| `a_rescan_never_alters_a_ruled_or_deferred_card` | §6 as a guard, not an absence: role/confidence/reason of a ruled card are untouched by a newer run | same |
| `proposed_count_is_admitted_minus_ruled` | the live count equals admitted verdicts minus precedence-ruled | same |
| `a_proposed_card_always_carries_a_code` | no `code: null` reaches a proposed card | `scenario_card_tests` |
| `banded_confidence_never_a_percentage_on_the_card` | the proposal carries a band + label and no raw score | `scenario_card_tests` |
| `deleting_a_run_removes_its_unruled_proposals_and_keeps_rulings` | delete → the projection yields nothing; the ref rows survive | `theme_scan_run_tests` (+ `scenario_card_projection_tests` for the read half) |
| `ruling_a_proposed_card_records_the_proposing_run` | the INSERT carries `source_run_id`; a second ruling does not overwrite it | `scenario_store` SQL-behaviour test + `scenario_ruling_tests` |
| `queue_defaults_to_proposed_when_proposals_exist_and_full_pool_otherwise` | the default-filter rule, both branches | `candidateProposal.test.ts` |
| `scan_card_collapsed_summary_reports_run_model_and_proposed_count` | the one-line summary names all three | `candidateProposal.test.ts` |

**Re-scoped, not deleted:** `scan_never_attempts_a_fact_ref_write_even_against_a_dead_database` ([theme_scan_persist_tests.rs:197](backend/src/services/theme_scan_persist_tests.rs#L197)) passes unchanged under this design — scan COMPLETION still performs no fact-ref write. Only its doc comment and the module header's "one write path … Merge selected" law move to "one write path … the human's ruling."

---

## 9 — STANDING RULE COMPLIANCE

- **No silent failures.** The two new reads propagate: a projection that cannot be read must NOT degrade to "no proposals", which would look exactly like a scenario nobody scanned — the precise class of lie task 2.15 piece 3 existed to remove. Both get `tracing::error!` with scenario and run ids. The delete-guard message names which record blocked it (R1).
- **No hardcoded values.** Every new sentence is a wording row + migration seed; a declared key with no row is a boot refusal, so the migration is ordered with the code exactly as in .384. No new numeric threshold is introduced. ThemeScanPanel's pre-existing literals: I convert ONLY the sentences this task touches (the report's tile labels and its advisory line, the collapsed summary); the rest stays filed debt.
- **Tutorial comments.** Every new non-trivial function gets a `///` doc; the fold and the precedence rule get `// Why:` and `// Domain note:` blocks; one `## Rust Learning:` header per new module.

---

## 10 — DEPLOYMENT IMPACT

- New env vars: **None** → no Ansible template change owed by this task.
- Migration: **one**, pipeline database (`colossus_legal_v2`), created with `./scripts/new-migration.sh`, additive `INSERT … ON CONFLICT (key) DO NOTHING`, forward-only. Roll-forward without it = boot refusal (declared keys, no compiled defaults); rollback to an older image is safe (extra rows ignored).
- Container rebuild: **both** (backend and frontend).
- Traefik / auth: **no change.**
- Data migration: **none.** No backfill of historical `source_run_id` (design Piece 2), and no `scan_run_merges` rows are touched.

---

## 11 — VERIFICATION PLAN

Local: `cargo check --bins` after every edit · `cargo test --lib` (baseline 1701) · `cargo clippy --workspace -- -D warnings` · `cargo fmt --check` · `npm run typecheck` · `npx vitest run` · `npm run build` · §8's module-size command · the route-link guard test · the §11 agents on the final SHA, with a targeted re-pass on any amend.

DEV, after Roman's beta.385 build: open S-4 cold → the queue leads with "Proposed by the … scan — N", the scan card is collapsed to one line; rule three cards (I, E, D) → refresh → all three survive and the proposed count drops by three; open the report from the history row → numbers only, reconciling, no Merge button anywhere; delete a test run → its unruled proposals vanish and the rulings stay (subject to R1). Full path: browser → Traefik → Authentik → backend → Postgres/Neo4j → response.

## 12 — ROLLBACK PLAN

The whole task is one branch. `git revert` the commit (or reset the branch) and rebuild both containers; the migration's seed rows are inert under the previous image. Nothing in this task rewrites or deletes a stored row, so a rollback loses no case data — the only user-visible regression is the return of the Merge button.

---

## 13 — RULINGS NEEDED BEFORE ANY CODE

**R0 — the mockup's own version.** The instruction cites `SCAN_PAGE_MOCKUP_v1_2026-08-08.html` as **v1.2**; the file on disk banners itself **MOCKUP v1.1**. I have built this analysis against the file as it stands. Confirm the file is the contract, or send v1.2.

**R1 — the 409 guard versus the provenance Piece 2 creates.** §3.2. Three options:
 (a) **Keep the guard as written** — a run becomes undeletable as soon as one of its proposals is ruled. Chain of custody wins; junk scans (nothing ruled) still delete freely, which is the case R-d actually cares about. **Cost:** your acceptance walk deletes a run you have just ruled from, and it will 409. *This is my recommendation, with the walk amended to delete an unruled run.*
 (b) **Narrow `is_protected()` to `merge_events > 0`** — historical merged runs keep their protection, ruled-from runs delete freely, and the FK nulls `source_run_id` on the surviving ruled rows. Delivers R-d literally. **Cost:** the delete silently erases exactly the provenance Piece 2 was built to record.
 (c) Preserve provenance across the delete by copying the run's identity onto the ruled rows — a new column and a write on delete. Out of this task's scope; named so it is not mistaken for unconsidered.

**R2 — do twins fold in the queue?** §3.1. The queue shows one card per node today, twins included. The mockup shows one card badged `×2 — covers C-46`.
 (a) **Fold at read** (mockup contract). Then ruling the folded card MUST also settle its twins server-side, or the twin reappears as a fresh proposal on the next read and the fold saved nothing — which drags in a ref row **and a ledger anchor per twin** on every ruling. That is real scope, and it is the honest price of the badge.
 (b) **One card per node**, with §7.9's duplicate awareness as a note naming the twin ("byte-identical to C-46"). No new write semantics, no anchor question — but it asks the human to rule the same sentence twice, which your 2026-08-08 ruling R2 rejected.
 *Recommendation: (a), because the mockup is the UI contract and (b) reinstates the duplicate work the fold exists to end — but only if you accept the per-twin ruling write it requires.*

**R3 — the minting path for R-f.** §3.3. Gather is the only minter and is race-unsafe against a second minter; the cards route already runs concurrently with it. My answer: **no new minting path** — keep gather as the sole minter and keep `ThemeScanPanel` MOUNTED when the scan card is collapsed (Piece 4a hides the body, it does not unmount). Confirm, or rule that minting moves into the cards route and gather's mint is retired in the same commit (bigger, and it touches a route this task otherwise leaves alone).

**R4 — where the proposing run id comes from at ruling time.** The card knows it, but a client-supplied provenance value is a claim the server cannot check. **Recommendation: the server re-resolves the projecting run for that node at ruling time** — the same query the projection uses, so the recorded run is by construction the run that proposed the card. Costs one indexed read per ruling. The alternative (trust the payload) is cheaper and forgeable.

**R5 — the live "proposed" number versus the frozen conservation line.** The stored template's five placeholders describe what a run DID and are composed from frozen counts; "proposed" is a live number that changes with every ruling. Splicing it into that sentence makes a frozen record appear to move. **Recommendation: the conservation sentence stays exactly as it is (five terms), and "proposed" is its own tile plus its own short stored sentence labelled as live** — the mockup's own tile row already shows it as the fifth tile. If you want the mockup's single "… · 30 proposed — every number reconciles" sentence instead, that is a sixth required placeholder and a settings-validation change, and I need that said.

**R6 — the history table's PROPOSED column.** Only the projecting run has proposals; every older completed run projects nothing (R-b). Showing each row's frozen `relevant_count` under a "PROPOSED" heading would be false for every row but one. **Recommendation: the column shows a number only on the row that is actually projecting, and an em dash elsewhere** (the house treatment for "not measured").

**R7 — chips versus the selects you ruled for in 1.7G.** The mockup's filter row is five CHIPS. The shipped bar is two labelled `<select>`s with counts, which was your explicit ruling in 1.7G after you had named the Bias Analysis pattern twice. Piece 3a says the filters are "one chip away". I will not overturn a standing ruling from a mockup detail. **Recommendation: keep the selects and add "Proposed" as the leading Status option (with its count), folding the now-meaningless Scan dropdown ("Scan-scored"/"Never scanned") away since Piece 5 retires that facet.** Say the word if you want the chips instead — it is a small change, but it is yours to make, not mine.

**R8 — the default filter when there are no proposals.** The instruction's test name says "full pool otherwise", which overturns today's computed default of "Rulable now" (falling back to "Not ruled") — the behaviour of `defaultFilters` since 1.7E. I will implement exactly what the test name says. Confirm you intend the old default to go, because on a scanned-and-fully-ruled scenario the queue will now open on all 148 rather than on the not-ruled handful.

---

**Nothing built. Nothing committed. Awaiting rulings R0–R8.**

=== END REPORT — VERDICT: STOPPED ===
