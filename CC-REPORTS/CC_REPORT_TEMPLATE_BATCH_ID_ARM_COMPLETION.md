=== CC REPORT — CC_TASK_TEMPLATE_BATCH_AND_ID_ARM_v1 (+ P7 ADDENDUM) — COMPLETION — 2026-08-16 ===

**Branch:** `feat/template-batch-id-arm` · **HEAD:** `3684df1` on top of `08c7c2e`.
**Nothing pushed. Nothing tagged. No version bump.**
**Nothing was run against DEV that writes.** Every live query in this session was
read-only `SELECT` / `MATCH`; no reprocess, no remap, no merge, no re-key.

---

## 1 · SHIPPED / DROPPED — one line each

| Item | State | One line |
|---|---|---|
| **Twin-merge script** | **SHIPPED, not run** | `merge_evidence_twins` — collapses the 21 same-key Evidence pairs; a pair curated on both sides is NEVER merged, it goes to a human queue file. |
| **P2 remap script** | **SHIPPED, not run** | `remap_evidence snapshot \| propose \| apply` — `apply` refuses a proposal with no `APPROVED` line. |
| **P3 ISO paragraph + discovery quirk** | **SHIPPED (narrowed — see §3.1)** | The ISO paragraph already existed in all eight v5_3 pass-1 templates; what was missing was the discovery quirk and a property correction. Three templates bump to v5_4, not eight. |
| **P4 document dates** | **SHIPPED** | Migration + `domain::date_precision` + one endpoint serving both the intake field and the post-hoc edit path + Neo4j mirror at ingest. No consumer wiring. |
| **P5a brief templates** | **ALREADY BUILT — nothing to do (§3.2)** | `appellate_brief_pass1/pass2_v5_3.md` + profile + schema shipped at `3dda37b`, registered in the pipeline registry. |
| **P5b motion templates** | **ALREADY BUILT — nothing to do (§3.2)** | `motion_pass1/pass2_v5_3.md` + profile + schema shipped at `0bc9459`, registered. |
| **P6 party/alias tightening** | **ALREADY BUILT — nothing to do (§3.3)** | The closed `pattern_tags` vocabulary and the canonical-name/aliases rule shipped in the v5_3 pass-1 batch on 2026-07-31. Verified across all eight. |
| **P7 party-merge tool** | **SHIPPED, not run** | `merge_parties` — executes a human rulings file and nothing else; `--emit-template` writes the worksheet from the live census. |
| **P7b rulings template** | **SHIPPED** | `PARTY_MERGE_RULINGS_TEMPLATE_v1.md`, filed in **both** CC-INSTRUCTIONS locations. |

**Nothing was dropped.** The drop order (P6 → P5b → P5a) never came into play,
because those three turned out to be already built.

---

## 2 · THE NUMBERS, MEASURED LIVE ON DEV 2026-08-15 (read-only)

### Twin merge — what the first dry run must print

```
Same-key clusters seen   : 21
Nodes in those clusters  : 42
Clusters to merge        : 14
Nodes to delete          : 14
Refused, curated on 2+   : 7
Refused, edges diverge   : 0
```

Per-pair curated exposure, measured across the ten curated columns:

```
21 pairs
   7  BOTH twins carry curated rows   (112 rows between them)
   0  exactly ONE twin curated
  14  NEITHER twin curated
```

Zero one-sided pairs, exactly as Phase A found. The 14 clean pairs merge
mechanically; the 7 go to the queue file for Roman.

**Also measured, and it is why the merge is provably lossless for the 14:**
20 of the 21 pairs have **byte-identical `(edge type, other node)` sets**. The one
that differs — `doc-george-phillips-response-to-discovery` p9,
`042d8287` / `be12ddef` — is also one of the 7 already refused for curation, so it
is refused twice over.

### Party census — what `merge_parties` will see

54 `Person` + 8 `Organization`; 8 Persons carry statements. Tighe is
`person-karen-a-tighe` (39) + `person-tighe` (62) = **101 to conserve**.

**Correction to the 2026-08-13 census, on the record:** it is TIGHE ×2, not ×3.
`Judge Tighe` and `The Court` are recorded as **aliases on existing nodes**, not
as separate nodes. 39 + 62 = 101 still holds and the acceptance test is unchanged.

Party-incident relationship types, with direction — the compiled list the merge
moves and the preflight refuses to run without:
incoming `ABOUT` (1065) · `STATED_BY` (514) · `CHARACTERIZES` (44) ·
`SUFFERED_BY` (12); outgoing `CONTAINED_IN` (133). All five carry properties
(`extraction_run_id`, `source_document_id`, `created_at`), which the repoint
copies across.

---

## 3 · FINDINGS — read these before the deploy

### 3.1 · P3's premise was already satisfied. Three templates changed, not eight.

The task asks for "one shared paragraph into EVERY pass-1 template" and a
v5_3 → v5_4 bump. **The paragraph is already in all eight v5_3 pass-1 templates**
— it shipped in the 2026-07-31 batch, including the clause the task names
("do not use the document's own date as a substitute"). Verified by reading all
eight.

What was genuinely missing, and is now built:

1. **The discovery quirk** — absent everywhere. Added to
   `discovery_response_pass1_v5_4.md` with a worked case from this corpus (the
   5 November 2009 letter answered in a response verified 16 August 2010).
2. **A property the schemas never declared.** The shared paragraph names
   `` `event_date` and `statement_date` `` in all eight templates, but
   **`statement_date` is declared in exactly ONE schema** —
   `affidavit_schema_v5_1.yaml`. The complaint, court-ruling and discovery-response
   templates were instructing the model to emit a property their own schemas do not
   have. Corrected in all three.

**Deviation, stated plainly, and it is yours to overrule.** I bumped **only the
three templates whose content changed**, not all eight. Bumping five
byte-identical files to v5_4 would mark five current documents as
template-stale for no reason, and the wave gates on `template_name`. If you want
the blanket bump anyway it is five `cp` + two `sed` commands and I will do it on
one word.

**Also built, because this class needs a scan and not a reviewer:**
`backend/src/template_invariants.rs` (Standing Rule 21) asserts, over every
profile on disk: every named template and schema exists · every live pass-1
template carries the ISO date rule · **no pass-1 template names a date property
its schema does not declare**. That last one is the defect above, now a build
failure. It strips authoring comments first, the way the pipeline does.

### 3.2 · P5a and P5b are already built, and were before this batch

- Appellate brief: `3dda37b feat(templates): appellate_brief document type — profile, schema, both passes`
- Motion: `0bc9459 feat(templates): motion document type — profile, schema, both passes`

Both at v5_3, both with profile + schema + both passes, both registered in
`pipeline_registry.yaml`. This is the same shape as the correspondence-template
census error the wave-verification report caught: the task file was written from
a stale inventory. **Nothing was built for P5. Nothing needed to be.**

### 3.3 · P6 is already built, and was before this batch

The v5_3 pass-1 batch (2026-07-31) shipped, in all eight templates:
the **closed `pattern_tags` vocabulary** ("use ONLY these", omit rather than emit
an empty string) and the **canonical-name rule** — one `party_name` per party,
every other form into `aliases`, with the rule spelled out rather than assumed.
That is B2 §5's template side, complete. Verified by reading all eight.

### 3.4 · **`rekey_evidence` does not update three Evidence-referencing columns**

> **RULED AND FIXED 2026-08-16 — see §11.** The section below is left as it was
> written, because it is the finding that earned the ruling. What it describes is
> no longer true of the code.

Found by measuring `information_schema.columns` and then sampling each candidate's
contents, as the P7 addendum required for parties. There are **eleven** columns
holding Evidence graph ids, not eight:

| Column | Rows | Evidence ids | In rekey's list? |
|---|---|---|---|
| the eight from Phase A | 947 | 947 | yes |
| `evidence_summary_overrides.graph_node_id` | 0 | 0 | **no** |
| `response_item_fact_refs.graph_node_id` | 0 | 0 | **no** |
| **`extraction_items.neo4j_node_id`** | 849 | **525** | **no** |

The first two are real curated surfaces that are merely empty this Tuesday. The
third is not empty and it is **read**: `lookup_neo4j_node_ids` resolves
cross-document references from it at ingest, and pass-2 prefers it over
re-resolving. After `rekey_evidence --apply`, those 525 rows point at ids that no
longer exist.

**Not fixed, deliberately** (CLAUDE.md §13, and the launch says P1b is done).
Adding the column would change the `947` the runbook tells you to expect, and
that is your call, not this batch's. The gap is recorded in
`oneshot::refs::REKEY_OMITS` with a test that fails if it drifts, so it cannot
quietly stop being true. **The new merge tools DO walk all eleven.**

**My read on the risk:** low for Saturday. Morris is reprocessed from scratch in
the gate test, so its `extraction_items` rows are new. The exposure is the eight
non-reprocessed documents if pass-2 is ever re-run alone against them. Worth a
ruling before the wave, not a reason to hold the deploy.

### 3.5 · `default.yaml` — the "Other / Unknown" fallback — runs a v4 complaint template

Found by the new invariant scan. The pipeline registry maps `default` to
`default.yaml`, which is what any unrecognised document type lands on, and it
points at `pass1_complaint_v4.md` — two generations stale, no ISO date rule, no
closed `pattern_tags` vocabulary, no canonical-name rule. Not fixed: repointing
the fallback changes what an unknown document extracts, and that is a ruling.
Recorded in `PROFILES_WITHOUT_DATE_RULE` with a test that fails when it is fixed.

### 3.6 · The correspondence template contradicts the no-inheritance law

`correspondence_pass1_v5_3.md` line 162 reads, in the date rule:
`` `event_date` defaults to `sent_date` ``. B2 §1 says **no date inheritance,
ever** — extraction never silently assigns a document's date to a statement.
This is a deliberate, authored rule in a template a prior read-and-report
verified clean, on the corpus's only 100%-dated document. **Not touched.** It is
either a considered exception for a genre where the letter IS the event, or it
is the exact class P3 exists to kill. Your ruling.

---

## 4 · WHAT WAS BUILT

### 4.1 · `oneshot` — the shared spine of the four maintenance tools

`backend/src/oneshot/{exit,cli,refs}.rs`. The exit-code scheme is a runbook
CONTRACT, so it now has ONE definition instead of four that would drift;
`rekey::report` re-exports its constants under the names it has always used, so
`rekey_evidence`'s behaviour is byte-identical and only the address of the
constants moved. `refs` holds the measured column registries from §3.4 and the
count/repoint primitives.

Deliberately NOT shared: each tool's `plan` module. A re-key decides per node, a
merge per cluster, a remap per candidate pair; one `Plan` trait would buy nothing
but a layer between a reader and the decision.

### 4.2 · `merge_evidence_twins`

Refuses a cluster if more than one member carries curated rows (7 of 21), and
refuses if a loser holds an edge the survivor does not — so a delete is provably
lossless. Survivor is the curated member where there is one, else the
lexicographically smallest id (deterministic across runs and hosts). The survivor
takes the cluster's stable-arm key, so after the merge `rekey_evidence` has
nothing left to refuse. Per-cluster Postgres transaction, counted before and
verified after; graph touched only after that verification passes.

Two files: a count proof, and a **human queue** written always — even empty, and
saying so, because an absent file cannot distinguish "nothing was refused" from
"the tool died first".

### 4.3 · `remap_evidence`

`snapshot` (before the reprocess) → `propose` (after) → `apply`. `propose` writes
nothing to either store. The generated proposal carries its `APPROVED` line
**commented out**, so approving is deleting a `#` on a file someone opened;
deleting a `MAP` line rejects that one match. `apply` refuses an unapproved file.
Ambiguity in either direction and every unmatched node go to the queue, sorted by
the curated rows at stake rather than by node.

`propose` prints the yield the Morris gate test checks against the measured
**87.8%** floor. It counts an unchanged id as a success, because that is the best
possible outcome — and with the stable-id arm live, most ids should not move at
all.

### 4.4 · `merge_parties`

Executes a rulings file and nothing else. No fuzzy matching in the execution
path, no default survivor. `--emit-template` writes the worksheet from the live
census — every party with its label, statement count, source documents and
aliases, grouped by shared name token as a **reading aid that is labelled as
one**, every block pre-filled `SKIP`. A template handed back unedited merges
nothing; a test asserts exactly that by parsing the generated file.

Both stores are transactional here, which the re-key could not manage: the graph
side runs in one `Txn` committed only after the statement count proves
conservation. A cluster that loses a statement is rolled back whole, in both
stores.

Edge repointing uses a compiled list of the five measured types and **refuses the
whole run** if the graph holds an incident type it does not know — rather than
depending on an APOC procedure whose absence would fail differently on a
different host. Members are deleted with plain `DELETE`, never `DETACH DELETE`,
so a leftover edge is a loud Neo4j error instead of a silent deletion.

### 4.5 · P4 — document dates

- **Migration** `20260816143722_add_document_date_and_precision.sql` (pipeline DB;
  `documents` lives in `colossus_legal_v2`). Two CHECK constraints: the vocabulary,
  and the mandatory-with-override invariant — `unknown` ⟺ no date, and
  `NULL` precision ⟺ nobody has been asked. Partial index on the dated rows.
- **`domain::date_precision`** — the `actor_role` code-owned-lookup convention.
  `day | month | year | unknown`, versioned, with the validation rule as one free
  function used by every caller.
- **API** — `GET /documents/date-precisions` (the vocabulary, so the frontend
  hardcodes nothing), `GET /documents/:id/date`, `PUT /documents/:id/date`.
- **Neo4j mirror** — `create_document_node` now sets `document_date` and
  `date_precision` on both the CREATE and MATCH arms, so a re-ingest picks up a
  date entered since the last one.
- **Frontend** — `DocumentDateField` (shared control), `DocumentDateEditor` (the
  edit path), wired into `UploadDialog` and `DocumentWorkspace`.

**No consumer wiring** (P4c). Nothing reads these values yet.

---

## 5 · P4 FIELD PLACEMENT — the mockup-first flag STANDS

**Flagging it, as instructed. Render this yourself before deploy if you want it
different; it is two files and a small change.**

### Intake — `UploadDialog`, top to bottom

```
┌─ Upload Document ─────────────────────────── ✕ ┐
│                                                 │
│  ┌───────────────────────────────────────────┐  │
│  │   Drop PDF, Word, or text file here       │  │   ← unchanged
│  └───────────────────────────────────────────┘  │
│                                                 │
│  Document Type                                  │   ← unchanged
│  [ Discovery Response (v5.1)            ▾ ]     │
│                                                 │
│  Document date                                  │   ← NEW
│  [ How is this document dated?          ▾ ]     │     (starts EMPTY)
│  [ 2009-11-05                              ]    │     (appears only when a
│                                                 │      real precision is chosen)
│                     [ Cancel ]  [ Upload ]      │
└─────────────────────────────────────────────────┘
```

The precision select has **no pre-selected value** — that is what makes the
question mandatory. The date input appears only when the chosen precision needs
one. Choosing "No date on the document" hides it and enables **Upload**. A
month- or year-precision choice shows a one-line note that only the stated part
is kept. **Upload stays disabled until the question is answered**, exactly as it
does for Document Type.

**One thing worth your eye:** the date is a SECOND call, made right after the
upload succeeds, hitting the same endpoint the document page uses — so the rule
is validated in exactly one place. If that call fails, the dialog says so
explicitly ("The file uploaded, but its date was not saved… Set it on the
document page") and does not navigate away pretending otherwise.

### Post-hoc edit — `DocumentWorkspace` top bar

```
Back    CFS Interrogatory Response 08-08-16   [DISCOVERY]
        review mode
        Document date: Date not set            ← NEW, a link
```

Clicking it opens the same control inline with Save / Cancel. Three readings,
never collapsed: **"Date not set"** (nobody asked — all nine start here) ·
**"No date on the document"** (someone looked; an ANSWER) · **"2009-11-05"**, or
`2009-11 (month only)` / `2009 (year only)` when the stored day is padding the
source never stated.

---

## 6 · VERIFICATION — every command, every result

| Check | Result |
|---|---|
| `cargo build --lib --bins` | **clean** |
| `cargo check --bins` | **clean** |
| `cargo test --lib` | **2065 passed / 0 failed / 2 ignored** (was 1911 at handoff — **+154**) |
| `cargo clippy --lib --bins -- -D warnings` | **clean** |
| `cargo fmt --check` | **clean** |
| `npm run typecheck` | **clean** |
| `npm test` | **992 passed / 70 files** (+13) |
| `npm run build` | **clean** (1.52 s) |
| Module size (§8 command) | **no new module over 300 lines** |
| `.env` / `.fastembed_cache` | not present in the diff |

`cargo test --workspace` was NOT run — it is broken on `main` for unrelated
reasons (stale `AppState` fields in `backend/tests/*.rs` since ~beta.343), and
the handoff note says not to fix those here. `cargo test --lib` is the honest
baseline and it is clean. Frontend has no lint script; there is no eslint config.

**New test coverage by area:** oneshot 19 · twinmerge 23 · partymerge 53 ·
remap 33 · date_precision 10 · document_date 9 · template_invariants 6 ·
frontend 13 — post-gate figures; 25 of these were added in answer to the
test-auditor.

### Four-agent gate

Run once against the commit. **All four returned FAIL on the first pass, every
finding was real, and all are fixed** — see §9. Every check above was re-run
after the fixes; the table holds the post-fix numbers.

---

## 7 · THE RUNBOOK — exact steps, in order

Every block is copy-paste. **Read the dry-run output before the next step in
every case.** A failure at any step means STOP at that step.

### Step A — deploy, then push the changed template and profile files

The engine re-reads template files at every scan start; profiles are read at
startup. Both live under `/data/documents` on the app host.

```bash
cd ~/Projects/colossus-legal && ./scripts/push-templates.sh discovery_response_pass1_v5_4.md complaint_pass1_v5_4.md court_ruling_pass1_v5_4.md
```

The three **profile** YAMLs (`discovery_response.yaml`, `complaint_v5_1.yaml`,
`court_ruling.yaml`) changed too, and `push-templates.sh` only moves templates.
They ride the normal Ansible deploy. **If the profiles are not deployed, the
pipeline keeps using the v5_3 templates** — which is a safe failure, not a broken
one, but it means the quirk is not live. Verify after deploy:

```bash
ssh core@10.10.100.220 'grep -H template_file /data/documents/profiles/discovery_response.yaml /data/documents/profiles/complaint_v5_1.yaml /data/documents/profiles/court_ruling.yaml'
```

Expect three lines ending `_v5_4.md`.

### Step B — the migration

Runs automatically at backend startup (sqlx migrator, pipeline pool). Verify:

```bash
ssh core@10.10.100.200 "sudo podman exec -i colossus-postgres psql -U postgres -d colossus_legal_v2 -X -c \"SELECT column_name, data_type FROM information_schema.columns WHERE table_name='documents' AND column_name IN ('document_date','date_precision');\""
```

Expect two rows: `document_date | date`, `date_precision | text`.

### Step C — the re-key (unchanged from the earlier handoff)

```bash
cd ~/Projects/colossus-legal/backend && cargo run --bin rekey_evidence
```

Expect **483 to re-key · 42 refused in 21 groups · 525 seen**. Anything else:
STOP.

**Take backups. Then:**

```bash
cd ~/Projects/colossus-legal/backend && cargo run --bin rekey_evidence -- --apply
```

Expect exit 0, and **1,318 referencing rows updated across ELEVEN columns**
(§11 — the eleven-column ruling of 2026-08-16; the eight-column version would
have moved 835). The per-column figures the proof should print:

```
scenario_candidate_ordinals.graph_node_id      402
scan_run_verdicts.graph_node_id                202
scenario_ruling_anchors.graph_node_id          141
extraction_items.neo4j_node_id                 483   <- new
evidence_allegation_link_events.graph_node_id   31
scenario_fact_refs.graph_node_id                27
scenario_human_facts.anchor_graph_node_id       16
evidence_allegation_links.graph_node_id          9
scenario_human_facts.answers_graph_node_id       7
evidence_summary_overrides.graph_node_id         0   <- new
response_item_fact_refs.graph_node_id            0   <- new
                                             ─────
                                             1,318
```

**Exit 3 = a document aborted and was rolled back — STOP and read
`rekey_evidence_report.txt`.**

### Step D — the twin merge (NEW)

```bash
cd ~/Projects/colossus-legal/backend && cargo run --bin merge_evidence_twins
```

Expect exactly:

```
Same-key clusters seen   : 21
Clusters to merge        : 14
Nodes to delete          : 14
Refused, curated on 2+   : 7
Refused, edges diverge   : 0
```

Read `twin_merge_human_queue.txt` — it is the agenda for the 7 pairs, with each
side's quote and curated row count. Then, **with backups already taken**:

```bash
cd ~/Projects/colossus-legal/backend && cargo run --bin merge_evidence_twins -- --apply
```

Expect exit 0, 14 nodes deleted, 28 `extraction_items.neo4j_node_id` rows updated
and **zero rows on every curated column** — the 14 mergeable pairs carry no
curated rows at all, which is precisely why they are the mergeable ones.

### Step E — the party merge session (NEW)

```bash
cd ~/Projects/colossus-legal/backend && cargo run --bin merge_parties -- --emit-template ~/party_merge_rulings.txt
```

Rule the clusters in that file — the format and the census are in
`PARTY_MERGE_RULINGS_TEMPLATE_v1.md`, in both CC-INSTRUCTIONS locations. Then:

```bash
cd ~/Projects/colossus-legal/backend && cargo run --bin merge_parties -- --rulings ~/party_merge_rulings.txt
```

Check "Nodes merging in" equals the number of `MERGE` lines you wrote, and each
cluster's expected statement total (Tighe: 101). Then, **with backups**:

```bash
cd ~/Projects/colossus-legal/backend && cargo run --bin merge_parties -- --rulings ~/party_merge_rulings.txt --apply
```

Expect exit 0 and "Clusters conserving statements : N/N". **Exit 3 means a
cluster lost a statement and was rolled back — STOP.**

### Step F — the Morris gate test

Before the reprocess:

```bash
cd ~/Projects/colossus-legal/backend && cargo run --bin remap_evidence -- snapshot --document doc-sabrina-morris-affidavit --out ~/morris_before.json
```

Reprocess Morris through the UI. Then:

```bash
cd ~/Projects/colossus-legal/backend && cargo run --bin remap_evidence -- propose --snapshot ~/morris_before.json --out ~/morris_proposal.txt --queue ~/morris_queue.txt
```

**This is the gate's answer.** The header prints the yield. Morris carries zero
curated state, so nothing is at risk either way — what is being measured is
whether the stable-id arm works: **most ids should come back `unchanged`, not
merely `unambiguous`**. A yield at or below the 87.8% floor means the arm is not
doing its job and the wave should not run.

`apply` is not needed for Morris (no curated rows to move). Do not run it.

---

## 8 · WALK CHECKLIST — DEV, after deploy, DOM-targeted reads, ZERO writes

1. **Badge** reads the new version.
2. **Documents → Upload** — the dialog shows **Document date** under Document
   Type, with the select reading "How is this document dated?" and **no**
   pre-selection.
3. Choose **"Exact date"** → a date input appears; **Upload stays disabled**
   until it is filled.
4. Choose **"No date on the document"** → the date input disappears and
   **Upload enables**. *(Then Cancel — do not upload.)*
5. Choose **"Month and year only"** → the one-line note about the stated part
   appears.
6. **Open any document** → the top bar shows `Document date: Date not set` under
   the mode line, as a link. All nine will read this.
7. Click it → the same control opens inline with Save / Cancel. **Click Cancel.**
   *(Do not Save — that is a write, and the backfill is a separate, deliberate
   pass.)*
8. **Documents list, drill into a discovery response** → items render unchanged;
   nothing about the templates is visible until something is reprocessed, and
   nothing has been.
9. **S-5 and S-6** → render identical. Read-only to this batch, and nothing here
   has touched a curated row.

---

## 9 · FOUR-AGENT GATE RESULTS

Run once against the commit, per §11. **All four FAILED on the first pass. Every
finding was real and every one is fixed; the commit was amended.** Nothing was
overridden and nothing was deferred.

### observability-checker — FAIL → fixed

**One finding, and a good one.** `set_document_date` called the repository and
discarded the `rows_affected` it returns. A date typed against a document id that
does not exist would have matched zero rows, been logged "document date recorded"
at INFO, and returned **200 with the values echoed back** — identical to a
successful write, with nothing stored. The repository's own doc comment says the
count exists precisely so a caller can tell those apart, and the caller was not
using it.

Fixed: zero rows is now a `404` naming the document and saying the date was NOT
stored, with a WARN alongside. The Neo4j mirror moved to AFTER the check, so a
date is never mirrored onto a node whose Postgres row does not exist.

Everything else passed, including the judgement that the maintenance tools'
report-file-plus-tracing model is the right observability for a hand-run tool.

### architecture-reviewer — FAIL → fixed

The `oneshot` seam itself passed — "this is the right seam", including the
decision NOT to share the plan modules. Four findings:

1. **`twinmerge`'s defensive arm borrowed `UnsafePlan`**, whose message says two
   nodes are sharing an id. That is not what happened, and it would have sent an
   operator hunting through the corpus for a problem that is in the code.
2. **`partymerge`'s did worse** — it borrowed `UnknownEdgeTypes`, whose message
   ends "add them to `PARTY_EDGE_TYPES`". Someone would have pasted a cluster
   label into a relationship-type list.
3. and 4. Two constants carried their justification in `///` prose rather than
   the repo's `// STRUCTURAL:` / `// CONST:` format, where a reader at the
   declaration actually sees it.

Fixed: both tools gained an `InvariantViolated` variant whose message says
plainly that it is a BUG needing a code fix, not a data problem — with tests
asserting the party one does NOT mention `PARTY_EDGE_TYPES`. Both constants now
carry the formal justification comments.

### test-auditor — FAIL → fixed

Five gaps, and the fifth was the sharpest: **`failure_reason` was decision logic
stranded in an execute module**, private and therefore unreachable by any test —
in a module whose own header says every decision lives elsewhere. Its three abort
conditions are not interchangeable and none was covered.

Fixed: `failure_reason` moved to `report.rs` as a method on `ClusterProof`, with
five tests covering all three conditions plus "deleted more nodes than named".
The other four gaps produced `twinmerge/execute_tests.rs` (5),
`partymerge/execute_tests.rs` (4), `remap/execute_tests.rs` (7) and three more in
`oneshot/refs_tests.rs`.

### rules-enforcer — FAIL → fixed

Ten violations, all in this diff. Two constants needing `// STRUCTURAL:` rather
than `///` (the same pair the architecture agent named). Three `Deserialize`
structs without `deny_unknown_fields` — one an HTTP request body, two the remap
snapshot format, where an unknown field means this build and the writing build
disagree about what was captured. And **four bare relationship literals in
Cypher** — `[:STATED_BY]` and `[:CONTAINED_IN]` typed as strings instead of
interpolated from `neo4j::schema`, where every graph-schema name in this repo
lives.

Fixed: all ten. `PARTY_EDGE_TYPES` now reads from `schema::` too, and
`schema::SUFFERED_BY` was added — it was the one party-incident type the schema
module did not have.

**Pre-existing, reported for awareness, NOT fixed here:** four files were already
over the 300-line module limit before this batch, and this commit's additions
made them slightly longer — `ingest_helpers.rs` (748 → 788), `ingest.rs`
(890 → 911), `steps/ingest.rs` (571 → 586), `document_records.rs` (337 → 371).
Splitting any of them is a refactor of the ingest path on the eve of a deploy,
which is the wrong week. Every module this batch CREATED is under the limit.

---

## 10 · WHAT I OBSERVED THAT WAS NOT IN THE PLAN

- §3.1, §3.2, §3.3 — three of the eight work items were already built. The task
  file was written from a stale inventory, the same way the correspondence-template
  census error was. **Roughly half this batch's nominal scope did not exist as
  work**, which is why the drop order never came into play.
- §3.4 — the re-key's column list is incomplete by three columns, one of which
  holds 525 live Evidence ids. Reported, not fixed. **This is the one item I
  would want a ruling on before the wave.**
- §3.5 — the "Other / Unknown" fallback profile runs a v4 complaint template.
- §3.6 — the correspondence template inherits `sent_date` into `event_date`,
  which the standing law forbids.
- The party census is **TIGHE ×2**, not ×3 — the third and fourth surfaces are
  aliases on existing nodes. The 101 acceptance test is unaffected.
- `evidence_summary_overrides` and `response_item_fact_refs` are real curated
  surfaces with zero rows today. They are in the new tools' registry so that
  stops being load-bearing.

**Remaining, and stated rather than hidden:** ten functions in the new modules
sit between 51 and 75 code lines against Rule 18's 50. The rules-enforcer did not
flag them, and the two worst — `apply_cluster` in both merge tools, at 104 and 88
— were split down to 75 and 60. The rest are report renderers and CLI shells
where "extract a helper" costs more legibility than it buys. Named here so the
decision is visible rather than assumed.

---

## 11 · AMENDMENT — THE ELEVEN-COLUMN RE-KEY (2026-08-16, ruled)

**Ruling:** `rekey_evidence` must update all eleven Evidence-referencing columns,
from the same registry the merge tools already walk — one list, not two.
`REKEY_OMITS` removed; the drift test kept, now proving the list is complete.

**Commit:** on `feat/template-batch-id-arm`, on top of `3684df1` / `bb4559a`.
Not pushed, no tag, no bump. **Still nothing run that writes.**

### What changed

- **`rekey::execute` no longer owns a column list.** Its private
  `REFERENCING_COLUMNS` (eight) is gone, along with its private `count_expected`
  and `apply_updates`. It now reads `oneshot::refs::EVIDENCE_REFERENCES` and
  calls the shared `count_rows` / `repoint` / `table_proofs` — the same code
  path the twin merge, the remap and the party merge use. "The re-key's list"
  and "the registry" are now one object and cannot drift apart.
- **`rekey::report::TableProof` is now a re-export** of the shared type. It was
  the fourth identical copy of that struct; there is one.
- **`REKEY_OMITS` is gone**, replaced by `REKEY_UPDATES_EVERYTHING` — a constant
  whose only job is to give the test something to name and to give anyone
  minded to re-introduce an exception a place where the refusal is written down.
- **The drift test became a completeness test.** `SWEEP_2026_08_15` records the
  eleven columns the `information_schema` query returned, verbatim, and the
  registry is asserted equal to it. The query itself is now in the module header
  so the sweep can be re-run rather than re-derived.
- **`SWEPT_AND_EXCLUDED` added** — the five columns that matched the sweep by
  NAME and were excluded by CONTENT (`authored_entities.entity_id`,
  `authored_relationships` ×2, `scenario_human_facts.person_refs`,
  `scan_run_merges.selected_node_ids`). A test asserts no registry lists them, so
  the two records cannot disagree about what a column holds.
- Per-document count → verify → commit is **unchanged**. The three new columns
  are counted, updated and verified inside the same transaction as the other
  eight, and a mismatch on any of the eleven rolls that document back whole.
  Idempotency and exit codes unchanged.

### The expected figures — measured, and they differ from the ruling's

The ruling projected **1,472** rows to update (947 + 525 + 0 + 0). **1,472 is
correct as the total number of rows referencing an Evidence id across all eleven
columns** — I re-measured it on DEV today and it is exact.

But it is not what the re-key UPDATES. 154 of those rows sit on the **42 refused
twins**, which the re-key deliberately does not move. The number the count proof
will print is therefore:

| | Rows |
|---|---|
| All Evidence-referencing rows, eleven columns | **1,472** |
| …of which sit on the 42 refused twins | −154 |
| **Rows the re-key updates** | **1,318** |

Cross-check from the other direction: 947 curated rows − 112 on twins = 835,
plus 483 `extraction_items` rows on re-keyed nodes = 1,318. The two agree.

**Per-column, as the proof will print it:** `scenario_candidate_ordinals` 402 ·
`scan_run_verdicts` 202 · `scenario_ruling_anchors` 141 ·
**`extraction_items.neo4j_node_id` 483** · `evidence_allegation_link_events` 31 ·
`scenario_fact_refs` 27 · `scenario_human_facts.anchor` 16 ·
`evidence_allegation_links` 9 · `scenario_human_facts.answers` 7 ·
`evidence_summary_overrides` 0 · `response_item_fact_refs` 0.

Node figures are unchanged: **483 to re-key · 42 refused in 21 groups · 525
seen.** Runbook Step C in §7 now carries the full per-column table.

The measurement query is in `oneshot::refs`'s module header; the twin-id list it
excludes is the same 21 pairs §2 reports.

### Tests for the three new columns

- `the_registry_is_exactly_the_measured_information_schema_sweep` — registry vs
  the dated sweep, both directions.
- `the_rekey_walks_the_entire_registry_and_has_no_exceptions`.
- `the_three_columns_the_rekey_used_to_miss_are_in_the_registry` — named
  individually, because a regression would most likely drop exactly those three.
- `the_two_empty_curated_columns_are_treated_as_curated_not_as_provenance` — if
  a future edit reasons "empty means it does not matter" and moves them out of
  the curated set, the twin merge stops counting them when deciding whether a
  twin carries a ruling, and the first summary override Roman writes becomes
  mergeable without him.
- `the_swept_and_excluded_columns_are_in_no_registry`.
- In `rekey::execute_tests`: `the_rekey_walks_every_column_in_the_shared_registry`
  and `the_columns_added_on_the_sixteenth_are_still_there`, replacing the old
  `the_referencing_column_list_is_the_measured_eight`.

### Verification

| Check | Result |
|---|---|
| `cargo test --lib` | **2068 passed / 0 failed / 2 ignored** (+3) |
| `cargo clippy --lib --bins -- -D warnings` | **clean** |
| `cargo fmt --check` | **clean** |
| `cargo check --bins` | **clean** |
| `cargo build --lib --bins` | **clean** |
| Module size (§8 command) | no `rekey` or `oneshot` module over 300 lines |

Frontend untouched by this amendment.

### One thing worth knowing

`extraction_items.neo4j_node_id` is pipeline provenance, not curated state, and
it is deliberately NOT in `EVIDENCE_CURATED_REFERENCES`. That distinction still
does real work: the re-key now MOVES it, while the twin merge still does not
count it when deciding whether a twin carries a human ruling. A twin whose only
reference is an `extraction_items` row remains mechanically mergeable, which is
what keeps the expected 14-merge / 7-refuse split intact.

=== END REPORT — VERDICT: PASS ===
