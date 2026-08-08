# CC_REPORT_SCAN_WORKFLOW_DIAGNOSTIC

**Task:** `CC_TASK_SCAN_TO_RULING_WORKFLOW_DIAGNOSTIC_v1` · **Date:** 2026-08-08
**Read-only.** No code changed, no branch created. The only file written is this
report.
**Measured against:** `main` @ `11ef8a6` (v2.0.0-beta.384) — i.e. WITH the Tier-2
scan work merged (`daef777`). Where beta.383 behaved differently, it is noted.

Every claim carries `file:line`. Line numbers are as of `11ef8a6`.

---

## 0 — THE WORKFLOW IN ONE PICTURE [measured]

```
   Run scan
      │
      ▼
  scan_runs row  (status running → completed)          ← the ONLY tables a scan writes
  scan_run_verdicts rows (one per node, incl. twins)
  summary_json (suggestions + conservation)
      │
      │   ✗ NOTHING here touches scenario_fact_refs — the queue cannot see any of it
      ▼
  RELEVANT FINDINGS panel  ── human ticks checkboxes ──▶  "Merge selected (N)"
      │
      ▼
  scenario_fact_refs rows, status='undecided', confidence=<model>, source_run_id=<run>
  scan_run_merges audit row
      │
      │   ✗ NO ruling anchor is written here
      ▼
  Candidates queue  ── human presses I / E / D ──▶  POST …/facts/:id/action
      │
      ▼
  scenario_fact_refs UPDATE (status) + scenario_ruling_anchors APPEND  ← the ruling
```

**The defect on record, located:** a candidate must be selected twice — once as a
checkbox in FINDINGS, once as a card in the QUEUE — because the two surfaces read
different stores and only the human bridges them. §6 says a scan "proposes
candidates as **pending rulings**"; today a scan proposes nothing to the queue at
all.

---

## 1 — SCAN COMPLETE: WHAT A FINISHED RUN PERSISTS [measured]

Three writes, in this order, all inside `run_scan_job`
([theme_scan_start.rs:240-289](backend/src/services/theme_scan_start.rs#L240)):

| # | Write | Where | Content |
|---|---|---|---|
| 1 | `scan_run_verdicts` — one row **per graph node** | `insert_scan_run_verdicts`, called by `write_verdicts` ([theme_scan_persist.rs:269](backend/src/services/theme_scan_persist.rs#L269)) | `relevant`, `proposed_role`, `confidence`, `reason`, `raw_reply`, `error`. PK `(run_id, graph_node_id)` |
| 2 | `scan_runs` finalize | `finalize_scan_run_completed` ([theme_scan_start.rs:281](backend/src/services/theme_scan_start.rs#L281)) | counts, tokens, cost, duration, and `summary_json` |
| 3 | — | — | *(nothing else)* |

`summary_json` holds the `ThemeScanSummary`
([dto/theme_scan.rs:265](backend/src/dto/theme_scan.rs#L265)): `suggestions` (one
per RELEVANT verdict, each carrying its `BiasInstance` content, `proposed_role`,
`reason`, `confidence`, `covers_node_ids`, `duplicate_count`), `rejected_sample`,
the counts, and the `conservation` block.

### Does completion write anything the queue can see? **NO.** [measured]

The queue reads `scenario_fact_refs`. A scan writes `scan_runs` and
`scan_run_verdicts` and nothing else. This is deliberate and documented as a law:

> "This module deliberately does NOT write `scenario_fact_refs`. Under the unified
> merge model there is exactly ONE write path from scan-land into a scenario's
> candidate facts — an explicit, human-driven **Merge selected** — and it is
> pick-keyed, not run-keyed."
> — [theme_scan_persist.rs:9-20](backend/src/services/theme_scan_persist.rs#L9)

There is a test that PINS the absence: `persist_and_summarize` is run against a
dead database and must still report `failed: 0`, proving no per-candidate write is
even attempted
([theme_scan_persist_tests.rs](backend/src/services/theme_scan_persist_tests.rs),
`scan_never_attempts_a_fact_ref_write_even_against_a_dead_database`).

**Consequence for the architect:** the entire scan result is invisible to the queue
until a human clicks Merge. There is no partial or degraded visibility — it is
total.

---

## 2 — MERGE: EVERY WRITE, ROW BY ROW [measured]

Entry point `merge_scenario_scan_run`
([theme_scan_run.rs:238](backend/src/services/theme_scan_run.rs#L238)) → two writes
in ONE transaction, owned by
`merge_run_into_scenario_recording`
([scan_run_merges.rs:263-290](backend/src/repositories/pipeline_repository/scan_run_merges.rs#L263)).

### Write A — the candidate facts

`MERGE_SCAN_RUN_SQL`
([scenario_store.rs:731-742](backend/src/repositories/pipeline_repository/scenario_store.rs#L731)):

```sql
INSERT INTO scenario_fact_refs
    (scenario_id, graph_node_id, role_in_this_scenario, status, confidence, source_run_id)
SELECT $1, v.graph_node_id, v.proposed_role, $2, v.confidence, $3
FROM scan_run_verdicts v
JOIN scan_runs r ON r.run_id = v.run_id
WHERE v.run_id = $3 AND r.scenario_id = $1 AND v.relevant = true
  AND v.graph_node_id = ANY($4)
ON CONFLICT (scenario_id, graph_node_id) DO UPDATE SET
    role_in_this_scenario = EXCLUDED.role_in_this_scenario,
    confidence            = EXCLUDED.confidence,
    source_run_id         = EXCLUDED.source_run_id,
    tagged_at             = NOW()
  WHERE scenario_fact_refs.status = $2          -- $2 = 'undecided'
```

Per merged pick it sets, on `scenario_fact_refs`: `role_in_this_scenario` (the
model's proposed role), `status = 'undecided'`, `confidence` (the model's score),
`source_run_id`, `tagged_at`. It does NOT set `note`, `defer_reason`, `tier`, or
`sort_ordinal`.

### Write B — the audit event

`insert_scan_run_merge` → one `scan_run_merges` row: `merge_id`, `run_id`,
`scenario_id`, `merged_at`, `rows_affected`, `selected_node_ids` (the human's
selection **as given**, not filtered to what landed — the difference against
`rows_affected` is the status-guard's footprint,
[scan_run_merges.rs:280-284](backend/src/repositories/pipeline_repository/scan_run_merges.rs#L280)).

### What merge does NOT write [measured]

- **No ruling anchor.** `scenario_ruling_anchors` is written only by
  `record_ruling` / `record_removal`
  ([scenario_ruling.rs:354, :420](backend/src/services/scenario_ruling.rs#L354)).
  A merged pending ruling therefore has **no anchor** until a human rules it.
- **No candidate ordinal.** Ordinals are minted only by the gather route
  ([scenario_gather.rs:402](backend/src/api/scenario_gather.rs#L402)); the cards
  route deliberately does not
  ([scenario_cards.rs:64-72](backend/src/api/scenario_cards.rs#L64)). A merged pick
  whose node gather has never numbered serves `code: null`.
- **No tier / sort_ordinal / note.**

### `covers_node_ids` [measured]

Server-side it is the pre-filter group's whole membership
([theme_scan_prefilter.rs](backend/src/services/theme_scan_prefilter.rs),
`CandidateGroup.members`), carried onto the suggestion
([theme_scan_persist.rs](backend/src/services/theme_scan_persist.rs),
`to_suggestion`). The browser sends **every** id a checked pick covers
([ThemeScanPanel.tsx](frontend/src/components/ThemeScanPanel.tsx), `mergeNodeIds`),
so one ruling covers a byte-identical twin set. The backend merge itself is
agnostic — it just receives more ids in `$4`.

### Is merge reversible? **NO — not as merge.** [measured]

There is no un-merge endpoint. The only way back is a human ruling: `remove`
deletes the fact-ref row and appends a `Remove` anchor
([scenario_ruling.rs:392-425](backend/src/services/scenario_ruling.rs#L392)). Note
this is a RULING, not an undo — it lands in the ledger as a human act.

### Double-merge [measured]

Safe and idempotent-ish, by the `ON CONFLICT … WHERE status = 'undecided'` tail:

| Target row state | Second merge does |
|---|---|
| absent | inserts, `status='undecided'` |
| `undecided` | refreshes role/confidence/source_run_id/tagged_at |
| `included` | **skipped** — human curation preserved, not counted in `rows_affected` |
| `dropped` | **skipped** — same |

A second merge of the same picks reports a `merged` count that may be lower than
the selection; the UI's `applied` badge suppresses re-selection in the first place
(`list_applied_node_ids_for_run`,
[scan_run_merges.rs:209](backend/src/repositories/pipeline_repository/scan_run_merges.rs#L209)).

---

## 3 — THE QUEUE: WHAT IT READS [measured]

`CardQueue` fetches `GET …/facts/cards`
([scenario_cards.rs:74](backend/src/api/scenario_cards.rs#L74)), which composes:

1. the **graph pool** — `all_evidence_about_subject(subject_id)`
   ([scenario_cards.rs:106](backend/src/api/scenario_cards.rs#L106)); this is the
   card list, one card per Evidence node ABOUT the subject, ungated;
2. `scenario_fact_refs` for the scenario (`list_fact_refs_for_scenario`, :129);
3. card extras from the graph, ordinals, summary overrides, page text, human links.

### What "pending ruling" IS in the data — **an ABSENCE, or a row. Both.** [measured]

This is the finding most relevant to the redesign. A node with **no fact-ref row at
all** renders exactly like a merged one:

```rust
let status = ref_state.status.unwrap_or(FactStatus::Undecided);
```
— [scenario_card.rs:483](backend/src/services/scenario_card.rs#L483), where
`ref_state` is `CardRefState::default()` for any node with no row
([scenario_card_assembly.rs:67](backend/src/services/scenario_card_assembly.rs#L67)).

So today the queue shows **all 148 pool nodes as `undecided`** whether or not
anything ever proposed them. "Pending ruling" is not a state anything writes; it is
the default rendering of everything unruled. The only visible difference a merge
makes to a card is that it gains a `confidence` (and thus a band) and a
`role_in_this_scenario`.

### What makes a card "scan-scored" [measured]

`isScanScored(card) === card.confidence.band !== "unscored"`
([candidateFilters.ts:96](frontend/src/components/candidateFilters.ts#L96)), and
the band is `Unscored` exactly when `confidence` is `None`
([confidence_band.rs:117-119](backend/src/domain/confidence_band.rs#L117)). Since
`confidence` reaches a fact ref only through the MERGE SQL, **"scan-scored" means
"a scan verdict was merged onto it"** — not "a scan looked at it". A scenario
scanned three times with nothing merged reports every card as never-scanned.

### Filters and counts [measured]

All client-side, over the payload, in one pass:
`candidateCounts` ([candidateFilters.ts:143](frontend/src/components/candidateFilters.ts#L143)).
State facets come from `candidateState`
([candidateFilters.ts:62-66](frontend/src/components/candidateFilters.ts#L62)):

```ts
if (card.status === "included") return "included";
if (card.status === "dropped")  return "excluded";
return card.defer_reason != null ? "deferred" : "not_ruled";
```

Note `deferred` is `undecided` + a non-null `defer_reason` — deferral is not a
status, it is a status plus a reason column.

### The ruling write [measured]

`POST …/facts/:graphNodeId/action` → `apply_fact_action`
([scenario_facts.rs:391](backend/src/api/scenario_facts.rs#L391)) → `record_ruling`
([scenario_ruling.rs:325](backend/src/services/scenario_ruling.rs#L325)), which in
ONE transaction: `upsert_fact_ref` (status, and **`confidence: None`** — a human
ruling deliberately erases the model score,
[scenario_ruling.rs:346-348](backend/src/services/scenario_ruling.rs#L346)) plus
`insert_ruling_anchor`. On `include` it then best-effort assigns an end ordinal
([scenario_facts.rs:456](backend/src/api/scenario_facts.rs#L456)).

**Side effect worth naming for the redesign:** ruling a card IN sets its
`confidence` to NULL, so an included card stops being "scan-scored" and moves into
the `never_scanned` facet.

---

## 4 — THE CARD CONTRACT (§7): QUEUE CARD vs FINDINGS ENTRY [measured]

QUEUE card = `ScenarioCard`
([dto/scenario_card.rs:281](backend/src/dto/scenario_card.rs#L281)).
FINDINGS entry = `ThemeScanSuggestion`
([dto/theme_scan.rs](backend/src/dto/theme_scan.rs)) rendering a `BiasInstance`
through `EvidenceCard`.

| § | Element | QUEUE card | FINDINGS entry |
|---|---|---|---|
| 1 | Quote **in context** | **Yes** — `quote: CardQuote` with `context_before/after`, completeness flags and page-edge notices | Quote only (`BiasInstance.verbatim_quote`); **no surrounding source text** |
| 2 | Pinpoint (doc + page, click to PDF) | **Yes** — `pinpoint: CardPinpoint` | Document title + page on the instance; **no viewer link** |
| 3 | Who said it + statement kind | **Yes** — `speaker: CardSpeaker`, `statement_kind` | Speaker yes; `statement_type` now on `BiasInstance` (Tier-2) but **not rendered** |
| 4 | Hazard or ammunition | **NO** — no such field exists on the card | **NO** |
| 5 | Stance **with its object** | **Yes** — `stance: Option<CardStance>`, plus `defer_required` + `defer_required_reason` when the object cannot exist | `proposed_role` only — a bare role token (`supports`), the C-222 shape |
| 6 | What it bears on | **Yes** — `bears_on: Vec<CardBearsOn>` in complaint language | **NO** |
| 7 | Grounding status | **Yes** — `grounding: Option<CardGrounding>` | **NO** |
| 8 | Banded confidence | **Yes** — `confidence: CardConfidence` (banded, from settings cutoffs) | **Naked percentage** — `Math.round(confidence * 100)` in `SuggestionRow` |
| 9 | Duplicate awareness | **NO** — no duplicate field on the card | **Partial (Tier-2)** — `duplicate_count`/`×N`, but only for byte-identical twins within one run |
| 10 | The three rulings | **Yes** — I/E/D with one-key triage in `CardQueue` | **NO** — only a merge checkbox |

**The delta any merged surface must close** — the FINDINGS entry is missing §1
context, §2 pinpoint link, §5 stance object, §6 bears-on, §7 grounding, §8 banding,
§10 rulings; and BOTH surfaces are missing §4 hazard/ammunition entirely, with §9
only partly served. Note the queue card is the richer of the two by a wide margin:
the cheaper direction is bringing rulings to the card, not context to the finding.

---

## 5 — AUTO-PROPOSAL DELTA: OPTIONS AND COSTS

*(Options and costs only — no recommendation, per the instruction.)*

**What "auto-proposal" requires, minimally:** after a run completes, every admitted
verdict must be visible in the queue as a pending ruling, with no human step. Three
distinct mechanisms exist in the code as it stands.

### Option A — server-side merge-on-complete (call the existing merge at persist time)

`run_scan_job` calls `merge_run_into_scenario_recording` with all relevant node ids
after `finalize_scan_run_completed`.

- **Writes touched:** none new. Same two writes as a human merge (`scenario_fact_refs`
  + `scan_run_merges`), same transaction.
- **One-guarded-write-path:** preserved in letter — merge stays the single writer —
  but its stated meaning changes: the law says the write is "explicit, human-driven"
  ([theme_scan_persist.rs:9-20](backend/src/services/theme_scan_persist.rs#L9)), and
  the machine would now drive it. The dead-pool test that pins "a scan attempts no
  fact-ref write" would have to be retired or re-scoped.
- **Durability (anchor):** unchanged — merge writes no anchor, and a proposal is not
  a ruling, so no anchor is owed until the human rules. Nothing in §12 is weakened.
- **Conservation:** unaffected; the counts already describe the judged set. A new
  number would be worth adding ("N proposed"), since admitted ≠ proposed once the
  status guard skips already-curated rows.
- **Merge button:** loses its purpose for the current run. It would still be needed
  for **historical** runs (re-proposing an older run's picks) unless that is also
  dropped.
- **Cost/risk:** a scan now writes into the scenario's working state. A junk scan
  pollutes the queue with ~24 rows that must each be ruled or removed, and removal
  is a ledger event.

### Option B — the queue reads verdicts directly (no fact-ref write at all)

`GET …/facts/cards` left-joins `scan_run_verdicts` (relevant, latest run) alongside
`scenario_fact_refs`, and composes a card for a verdict that has no ref row.

- **Writes touched:** **none.** Proposal becomes a read-time projection.
- **One-guarded-write-path:** strengthened — nothing new writes; merge could be
  deleted outright, and the human's ruling remains the only write.
- **Durability:** unchanged, and arguably cleaner: nothing is persisted until a
  human rules, and the ruling writes its anchor as today.
- **Conservation:** unaffected.
- **Merge button:** removable entirely.
- **Cost/risk:** the biggest read change. The cards route currently derives status
  from `scenario_fact_refs` alone; a second source means deciding precedence per
  node (ref row wins over verdict), and `confidence`/`role` would come from the
  verdict when no row exists. "Which run" must be chosen — latest completed, or all
  runs unioned. Also: `source_run_id` provenance is only recorded on a real row, so
  a purely projected proposal has no stored provenance until it is ruled.

### Option C — a new proposal write at persist time (a distinct status)

Scan completion writes `scenario_fact_refs` rows with a NEW status (e.g. `proposed`)
distinct from `undecided`.

- **Writes touched:** a third writer into `scenario_fact_refs`, plus a migration for
  the status vocabulary and `FactStatus`
  ([fact_status.rs:111-117](backend/src/domain/fact_status.rs#L111)).
- **One-guarded-write-path:** broken as stated — three writers instead of two, and
  the new one is machine-driven.
- **Durability:** unchanged (still no anchor until a ruling).
- **Conservation:** unaffected.
- **Merge button:** removable.
- **Cost/risk:** highest. Every status consumer must learn the new token — the
  status decode refuses unknown tokens loudly
  ([scenario_cards.rs:390-400](backend/src/api/scenario_cards.rs#L390)), so an
  un-updated reader is a 500, not a degradation. It does, however, make "pending
  ruling" a real state in the data rather than an absence, which is the only option
  that closes the §3 finding above.

### Cutting across all three

- Merged/proposed picks carry **no candidate ordinal** unless gather has numbered
  the node ([scenario_gather.rs:402](backend/src/api/scenario_gather.rs#L402)) — so
  auto-proposed cards may show `code: null` where the human expects `C-14`.
- A human `include` NULLs `confidence`
  ([scenario_ruling.rs:346](backend/src/services/scenario_ruling.rs#L346)), so any
  design that treats "has a confidence" as "was proposed" will lose that fact the
  moment it is ruled.

---

## 6 — RE-SCAN BEHAVIOUR: WHERE THE §6 GUARANTEE LIVES [measured]

**§6: scans NEVER touch existing rulings.** Today that holds trivially — a scan
writes nothing to `scenario_fact_refs` at all (§1). The guarantee is therefore
enforced by ABSENCE, not by a guard.

Under any auto-proposal option, the guarantee would rest on the one clause that
already exists:

```sql
ON CONFLICT (scenario_id, graph_node_id) DO UPDATE SET … WHERE scenario_fact_refs.status = $2
```
— [scenario_store.rs:737-742](backend/src/repositories/pipeline_repository/scenario_store.rs#L737),
`$2 = 'undecided'`.

That single `WHERE` is the whole of the status-preserving law: a re-scan re-proposing
a node the human has already `included` or `dropped` updates **nothing**. It is
asserted by a SQL-shape test
([scenario_store.rs:1156](backend/src/repositories/pipeline_repository/scenario_store.rs#L1156),
`merge_sql_is_relevant_only_and_fenced_to_run_and_scenario`).

Two gaps a redesign should know:

1. **A DEFERRED card is `undecided`** ([candidateFilters.ts:65](frontend/src/components/candidateFilters.ts#L65)),
   so the guard does NOT protect it: a re-scan's merge would overwrite the deferred
   card's role/confidence/source_run_id. The `defer_reason` column survives (the
   merge SQL does not touch it), so the card stays deferred — but its model
   judgment is silently replaced. This is measurable today via a re-merge.
2. **Option B (read-time projection) has no such guard at all** and would need the
   precedence rule written explicitly: a ref row must win over a verdict, or a
   re-scan would re-propose already-ruled nodes.

---

## 7 — THE SCROLL DEFECT [measured]

**Component:** `RunResult` inside
[ThemeScanPanel.tsx](frontend/src/components/ThemeScanPanel.tsx) — the
`suggestions.map(...)` list under the "Relevant findings" head.

**What bounds it: nothing.** Grepping the whole panel for `overflow`, `overflowY`
and `maxHeight` returns **no matches**. The containment chain is:

| element | style | bound |
|---|---|---|
| `S.results` | `marginTop:18px; display:flex; flexDirection:column; gap:16px` ([:1177](frontend/src/components/ThemeScanPanel.tsx#L1177)) | none |
| `S.runResult` | `boxShadow; borderRadius:12px; padding:16px` ([:1216](frontend/src/components/ThemeScanPanel.tsx#L1216)) | none |
| `S.finding` (per pick) | `marginBottom:8px` ([:1259](frontend/src/components/ThemeScanPanel.tsx#L1259)) | none |

So every admitted finding renders inline and the PAGE grows — on a 24-pick run the
panel is longer than the queue beneath it. `S.tileRow` + `S.sticky`
([:1178](frontend/src/components/ThemeScanPanel.tsx#L1178)) is `position:sticky;
top:0`, which pins the counts to the VIEWPORT (its nearest scrolling ancestor is
the page) — evidence the current design assumes page-scroll, not an inner window.

---

## 8 — WHAT ELSE A REDESIGN WILL TRIP OVER [measured]

### Wording rows involved (all `app_settings`, all boot-required)

| key | surface | module |
|---|---|---|
| `queue_empty_pool_summary`, `queue_all_ruled_summary`, `queue_counting_summary` | queue head, three distinct zero-states | [wording.rs](backend/src/domain/wording.rs) |
| `queue_raw_pool_toggle_template` `{count}` | the never-scanned opt-in | same |
| `scan_conservation_line_template` `{pool}{collapsed}{excluded}{judged}{relevant}` | findings reconciliation | [wording_scan.rs](backend/src/domain/wording_scan.rs) |
| `scan_history_view_label`, `scan_history_delete_confirm_template` `{run}` | history row controls | same |
| `scenario_no_target_notice`, `scenario_never_scanned_notice` | the two in-place-of-queue notices | [wording_scenario_authoring.rs](backend/src/domain/wording_scenario_authoring.rs) |
| the `fact_*` / `link_*` family (~49 keys) | card, link panel, remove control | [wording.rs](backend/src/domain/wording.rs) |

A new merged surface that speaks new sentences needs new rows + a migration; a
declared key with no row is a **boot refusal**, not a blank label.

**`ThemeScanPanel.tsx` is the exception and it is large:** its own strings
("Relevant findings", "Merge selected", the merge confirm, the tile labels) are
still literals — it predates the configuration law. A redesign that touches those
inherits converting them.

### Settings rows involved

`confidence_band_high` / `confidence_band_medium` (banding, §7 item 8),
`quote_context_window_chars` (§7 item 1), `card_test_ratio`,
`theme_scan_prompt_file`, `theme_scan_prefilter_min_chars`,
`theme_scan_prefilter_statement_types`, `link_short_list_max`.

### States the UI knows that the server does not [measured]

Every one of these is lost on reload, and each is a candidate for the redesign to
promote or delete:

| state | where | consequence |
|---|---|---|
| which picks are **checked** for merge | `RunResult`'s `checked: Set<string>` | a reload loses the selection |
| which run's results are **open** | `selectedRunIds` in `ThemeScanPanel` | now auto-restored to the latest completed run (Tier-2), not persisted |
| the **auto-open-once** latch | `autoOpenedFor` ref | resets per mount |
| queue region **collapsed/expanded** | `openOverride` in `ScanSection` | deliberately not persisted (ruling R7) |
| panel collapse preference | `localStorage` `colossus.themeScan.collapsed.<id>` | written but nothing reads it for visibility today |
| the triage cursor / undo | `cardTriage` reducer in `CardQueue` | single-step undo is in-memory only; a reload loses it |

### Two more that will bite

- **`source_run_id` does not survive a human edit** — measured 2026-08-06 in the
  2.15 report: 82 of S-2's 83 refs carry NULL despite four recorded merges, because
  `upsert_fact_ref` sets every column from `EXCLUDED` and the ruling path passes no
  run id. Any design that reads provenance from that column will read mostly NULL;
  the scorecard joins on `graph_node_id` for exactly this reason
  ([scripts/scan-scorecard.sql](scripts/scan-scorecard.sql)).
- **Deleting a merged run is refused** (409) once it has provenance
  ([theme_scan_run.rs:175-212](backend/src/services/theme_scan_run.rs#L175)). Under
  auto-proposal, EVERY completed run becomes merged, so no run would ever be
  deletable again — the history's ✕ would 409 permanently.

---

**No design, no fixes, no recommendation — measured only.**

=== END REPORT — VERDICT: MEASURED ===
