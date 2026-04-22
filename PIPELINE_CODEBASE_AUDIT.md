# PIPELINE_CODEBASE_AUDIT.md

**Date:** 2026-04-21
**Branch:** `feature/config-system` (HEAD `160ae22` + Cargo.toml at `v2.0.0-beta.93`)
**Scope:** read-only review of the document-processing pipeline against data-pipeline, knowledge-graph, and entity-resolution engineering principles.

Evidence is cited by `file:line` against the working tree at audit time. This is a structural review — no fixes are proposed as code diffs. Section 8 enumerates recommendations.

---

## Executive summary

The pipeline is correct on the happy path for a single-document extract-then-ingest flow. Under closer examination against idempotency, immutability, and lineage principles, it has five structural weaknesses, in rough order of severity:

1. **No persisted extraction-item → Neo4j-id lineage.** The only mapping exists in the `pg_to_neo4j: HashMap<i32, String>` local variable inside `run_ingest` (ingest.rs:265 / ingest.rs:325 HTTP variant). It is discarded when the function returns. Every downstream verifier (completeness, cross-document queries) must recompute the id — and for Party items that went through cross-document resolution, recomputation is mathematically impossible without re-running the resolver. This is the root cause of the completeness false-positive "missing node" seen on `doc-jeffrey-humphrey-affidavit`.
2. **Entity resolution is inline with entity creation.** `resolve_parties` runs inside `run_ingest` (ingest.rs:239–252), the resulting map is consumed by `create_party_nodes` in the same transaction (ingest.rs:279), and the resolver's decisions are never persisted to a table. Neo4j's own GraphRAG pipeline documentation separates extraction, loading, and resolution into three distinct steps — the current code couples load+resolve.
3. **`extraction_items.entity_type` is mutated after initial write.** `update_item_entity_type` (repositories/pipeline_repository/extraction.rs:241) rewrites `Party` → `Person` or `Organization` inside `run_ingest` (ingest.rs step:407 / ingest.rs http:281). The LLM output — the source of truth — is altered to match the shape of the writer. Downstream code that assumes "`entity_type` is what the LLM said" breaks subtly depending on whether Ingest has run.
4. **LlmExtract is not idempotent on re-run.** `insert_extraction_run` (extraction.rs:85) and `insert_extraction_item` (extraction.rs:150) are plain INSERTs with no ON CONFLICT. Re-running LlmExtract without prior cleanup creates a second `extraction_runs` row and duplicate `extraction_items`. Re-processing is currently gated on `cleanup.rs` running DELETE statements before re-insertion (cleanup.rs:222–228) — the idempotency comes from the cleanup, not from the step itself.
5. **Six of seven pipeline steps have `pipeline_steps` recording gaps on error paths.** Framework confirmed not to touch `pipeline_steps` anywhere in `colossus-pipeline/src/`. Each step owns its own recording. `extract_text`, `llm_extract`, `verify`, `auto_approve`, `ingest`, and `index` all use the pattern `record_step_start` → `let result = run_inner().await?` → `record_step_complete`. The `?` short-circuits on any inner error, so `record_step_complete` is skipped and the row stays at `status='running'` forever. Only `completeness` (rewritten in commit `160ae22`) has the match-wrap that calls `record_step_failure` on the Err path.

Sections 1–7 below give per-principle detail. Section 8 is the prioritized recommendation list.

---

## Section 1 — Source Data Immutability

**Principle:** Source data (LLM output) should be written once and treated as the immutable source of truth.

**Answer to audit question:** Yes, `extraction_items` rows are mutated after initial write. There are at least three post-write mutation paths.

### 1.1 Mutations observed

| Table.column | Mutating function | Caller | Why it's done |
|---|---|---|---|
| `extraction_items.entity_type` | `update_item_entity_type` (repositories/pipeline_repository/extraction.rs:241) | `run_ingest` step (pipeline/steps/ingest.rs:407) and HTTP `run_ingest` (api/pipeline/ingest.rs:281) | Rewrites `"Party"` to `"Person"` or `"Organization"` to match the Neo4j label. |
| `extraction_items.grounding_status`, `grounded_page` | `update_item_grounding` (extraction.rs:220) | `run_verify` step (pipeline/steps/verify.rs:250, 260, 270, 287) and HTTP verify handler | Writes the Verify step's result back into the source row. |
| `extraction_items.review_status`, `reviewed_by`, `reviewed_at`, `review_notes` | `bulk_approve` (repositories/pipeline_repository/review.rs) + human-review endpoints | `run_auto_approve` step + admin review UI | Records whether the extraction item was approved for ingestion. |

### 1.2 Assessment

- **`entity_type` mutation is a true violation.** The LLM wrote `"Party"`; the Ingest step is overwriting that verbatim LLM output with a label derived from a property (`party_type`). The mutation is destructive: the original `"Party"` value cannot be recovered after Ingest runs. Any later code that wants to know "what type did the LLM report?" reads a post-facto derivative.

- **`grounding_status` and `grounded_page` are not violations** in the strict principle sense, because they describe the *relationship* between the extracted item and the source text, which is new information produced downstream. But they *are* co-mingled into the same table as the LLM output, which conflates "what was extracted" with "how it was verified." A clean design would put grounding results in a separate `extraction_item_grounding` table keyed on `item_id`.

- **`review_status` mutation is acceptable** as a workflow field — it's application state, not source data.

### 1.3 The immutable alternative

- Keep `extraction_items.entity_type` as the LLM output (`"Party"`). Add an `extraction_items.resolved_entity_type` column populated by Ingest. Downstream readers choose: `COALESCE(resolved_entity_type, entity_type)`.
- Move grounding fields to a separate `extraction_item_grounding` table with FK to `extraction_items.id`.

Neither alternative requires a code rewrite — additive columns + parallel-writing behavior suffices.

---

## Section 2 — Idempotent Re-processing

**Principle:** Running the pipeline twice must produce the same output.

**Answer to audit question:** The pipeline IS idempotent on full re-process, but only because `cleanup.rs` wipes state first. Individual steps are not self-idempotent; idempotency is a property of the wrapping cleanup.

### 2.1 Re-processing flow (from the code)

1. User clicks "Re-process" in the UI.
2. `ReprocessDialog` POSTs to the process endpoint.
3. `cleanup.rs` runs 6 DELETE statements (cleanup.rs:222–228) against Postgres, `cleanup_neo4j` (cleanup.rs:128) DETACH-DELETEs Neo4j nodes for the doc_id, `cleanup_qdrant` (cleanup.rs:184) filters on `document_id` and deletes Qdrant points.
4. Pipeline re-runs from `ExtractText` onward.

### 2.2 Per-step idempotency

| Step | Self-idempotent? | Evidence |
|---|---|---|
| `upload` | N/A (user action) | — |
| `extract_text` | ✅ yes | `insert_document_text` uses `ON CONFLICT (document_id, page_number) DO UPDATE` (mod.rs:224). Re-run overwrites per page. |
| `llm_extract` | ❌ no | `insert_extraction_run` is a plain INSERT (extraction.rs:85); `insert_extraction_item` is a plain INSERT (extraction.rs:150). Re-run without cleanup creates duplicates. |
| `verify` | ⚠️ partial | `update_item_grounding` is idempotent (just overwrites) but mutates in place, so a concurrent reader during re-run sees mixed old/new state. Not a strict violation. |
| `auto_approve` | ⚠️ partial | `bulk_approve` is an UPDATE on rows matching a filter — idempotent, but applies to whatever items exist at the moment of the query. Depends on upstream state. |
| `ingest` | ✅ yes (by cleanup) | Module docstring at ingest.rs:23–27 is explicit: "uses a cleanup-then-write idempotency model: call `cleanup_neo4j` first, then write fresh." `create_entity_node` itself uses `MERGE` (ingest_helpers.rs:297), so strictly speaking it would be idempotent without cleanup, but the step unconditionally calls `cleanup_neo4j` at ingest.rs:138. Party nodes are also MERGE'd (ingest_helpers.rs:222). |
| `index` | ✅ yes | Qdrant `upsert_points` is idempotent by design; `cleanup_qdrant` also deletes prior points for the doc. |
| `completeness` | ✅ yes (read-only) | No writes besides the PUBLISHED status update. |

### 2.3 What state persists across re-runs that affects the new run

- **Cross-document Neo4j nodes** (Person/Organization created by *other* documents). `cleanup_neo4j` only deletes nodes whose `source_document = doc_id`. Shared Persons created by the first doc remain; the second doc's re-run will MERGE into them via `resolve_parties`. This is correct behavior, but it means **the second run's outcome depends on the first run's outcome**. Not "idempotent on the same input" in the strict sense.
- **`llm_models` row changes.** If a model is deactivated between runs, the second run's profile lookup may produce a different model id and re-extract with different results. Also correct behavior, but noting it.
- **`pipeline_config.step_config` JSONB edits.** If admin edits the config between runs, behavior changes. Correct, but not idempotent.

### 2.4 Second-run-produces-different-results cases

- **New cross-document parties.** If doc A is re-processed after doc B has been added, resolution now sees B's parties and may match to them. A's Neo4j node ids will differ from the first run. This is the **cross-document resolution blind spot** for completeness verification (see Section 5).
- **LLM non-determinism.** Without `LLM_TEMPERATURE=0`, extraction output varies per run. With it, output is deterministic. Ansible currently sets it. ✓

---

## Section 3 — Entity Identity

**Principle:** Every entity has one stable, reproducible id derivable from intrinsic properties.

**Answer to audit question:** There are **three distinct code paths** that produce Neo4j ids for entities, and they do not always produce the same id for the same entity.

### 3.1 ID-producing code paths

| Path | Location | Applies to | Formula |
|---|---|---|---|
| A | `stable_entity_id` (ingest_helpers.rs:66) | Non-Party entities written via `create_entity_node` | Per-type: ComplaintAllegation → `{doc_slug}:para:{paragraph_number}`, LegalCount → `{doc_slug}:count:{count_number}`, Harm → `{doc_slug}:harm:{hash}`, other → `{doc_slug}:{slug(type)}:{hash}` |
| B | `create_party_nodes` fallback branch (ingest_helpers.rs:211) | Party entities, **no** resolution match | `{prefix}-{slug(name)}` where prefix is `person` or `org` |
| C | `resolve_parties` resolver branch (ingest_resolver.rs:187) via `resolution_map.get(name)` (ingest_helpers.rs:208) | Party entities, **resolution match** | Whatever id the matched existing Neo4j node already has (opaque to the current doc's pipeline) |
| D (verification) | `compute_expected_neo4j_ids` (completeness_helpers.rs:79) | All entities, at completeness time | Same as A for non-Party; same as B for Party (always — ignores resolver) |

### 3.2 The mismatch

- Paths A and D agree for non-Party entities.
- Paths B and D agree for Party entities *only when the resolver found no match*.
- Path C is opaque to D. A Party that was resolved to an existing node has an id that D cannot reproduce — verification reports the node as missing.

Evidence: the 2026-04-21 Humphrey Affidavit failure reported "Missing ids: `person-mr-dalek`" while 39 of 40 nodes verified. Completeness computed the naive `person-mr-dalek`; Ingest wrote the node under whatever id `resolution_map` provided (path C).

### 3.3 Per-entity-type ID scheme

| entity_type (post-ingest) | Write path | ID scheme | Stable across runs? |
|---|---|---|---|
| `Document` | `create_document_node` (ingest_helpers.rs:134) | `slug(doc_id)` (after B6 fix in commit `879b3c4`) | ✅ |
| `ComplaintAllegation` | `create_entity_node` → A | `{doc_slug}:para:{n}` (or hash fallback if paragraph_number missing) | ✅ modulo fallback-hash path |
| `LegalCount` | `create_entity_node` → A | `{doc_slug}:count:{n}` (or hash fallback) | ✅ |
| `Harm` | `create_entity_node` → A | `{doc_slug}:harm:{sha256(harm_type+description)[0..8]}` | ✅ |
| unknown entity | `create_entity_node` → A | `{doc_slug}:{slug(type)}:{sha256(item_data)[0..8]}` | ✅ |
| `Person` (resolved) | C via resolver | existing node's id | ❌ ties to existing state |
| `Person` (new) | B fallback | `person-{slug(name)}` | ✅ |
| `Organization` (resolved) | C | existing node's id | ❌ |
| `Organization` (new) | B fallback | `org-{slug(name)}` | ✅ |

---

## Section 4 — Entity Resolution Architecture

**Principle:** Extract → Write → Resolve are three distinct pipeline steps.

**Answer:** Resolution is **inline with writing**, not separated.

### 4.1 Current architecture

In `run_ingest`:

1. `fetch_existing_parties` queries Neo4j for all Person/Organization nodes (ingest_resolver.rs:60).
2. `resolve_parties` runs the resolver against the current doc's Party items (ingest_resolver.rs:147).
3. The `resolution_map` returned from step 2 is passed into `create_party_nodes` (ingest.rs:279).
4. `create_party_nodes` consults the map per item — MERGE on the resolved id if present, fallback to slug-based id otherwise.

All four steps happen in a single function call within a single Neo4j transaction. Resolution decisions are exposed in the HTTP `IngestResponse.resolution_summary` but **not persisted** anywhere (neither to `extraction_items` nor to a dedicated resolution-audit table).

### 4.2 Problems this causes

- **Re-process loses all resolution decisions.** Because `cleanup.rs:223` deletes `extraction_items` and related tables, and because resolution decisions weren't persisted there anyway, a re-process re-runs resolution from scratch against whatever Neo4j state exists at that moment.
- **Second-doc resolution is dependent on first-doc state.** "Marie Awad" from doc B may be resolved to a Person node created by doc A. That decision is implicit — there is no record saying "Marie in doc B was determined to be the same entity as Marie in doc A, via NormalizedMatch."
- **Verification is blocked** (Section 5).

### 4.3 Neo4j's recommended pattern (for reference, not a prescriptive)

The Neo4j Knowledge-Graph-Builder project separates:

1. **Extract**: LLM emits entities/relationships into a staging structure.
2. **Load**: Entities are written with naive, document-scoped ids (no dedup).
3. **Resolve**: A separate resolution step creates `SAME_AS` relationships between duplicate entities and optionally collapses them.

This pattern makes each step idempotent, independently verifiable, and re-runnable without rebuilding the whole graph. The current code conflates load+resolve.

### 4.4 What happens to resolution on re-process

`cleanup_neo4j` (cleanup.rs:128) runs `MATCH (n) WHERE n.source_document = $doc_id OR n.source_document_id = $doc_id DETACH DELETE n`. Shared Person nodes owned by *another* document are NOT deleted. On re-process:

- If the doc being re-processed was the *first* to create "Marie Awad", her node is deleted. When doc A is re-processed, she is re-created with naive id `person-marie-awad`.
- If doc B is being re-processed and "Marie Awad" was originally created by doc A, her node survives. Re-process resolves to her existing id again. Same outcome as the first run.
- If doc A is re-processed *after* doc B already resolved to A's original Marie, A's Marie is deleted. Doc A's re-run creates a *new* Marie. Doc B's `source_documents` array now points to a dangling id. **This is a latent correctness bug** for the sequence A-ingest, B-ingest, A-reprocess.

Evidence: `create_party_nodes` ON MATCH clause (ingest_helpers.rs:227–229) appends to `source_documents` but never removes. `cleanup_neo4j` deletes by `source_document` (singular, scalar), so the shared-node scenario leaves stale array entries.

---

## Section 5 — Entity Lineage

**Principle:** Each extraction item → Neo4j node and each Neo4j node → Qdrant point mapping must be persistently recorded.

**Answer:** Not stored anywhere.

### 5.1 The mapping exists in memory only

`run_ingest` maintains two `HashMap<i32, String>` structures:
- `pg_to_neo4j: HashMap<i32, String>` (ingest.rs step:265; ingest.rs http:181) — extraction_item.id → neo4j_id.
- `pg_to_label: HashMap<i32, String>` — extraction_item.id → Neo4j label.

Both are local variables inside `run_ingest`. They are used to resolve relationship endpoints during the same transaction (ingest.rs step:329–347) and then dropped when the function returns. Nothing is written to a persistent mapping table.

### 5.2 Consequence for completeness

`compute_expected_neo4j_ids` (completeness_helpers.rs:79) has to **recompute** expected ids from `extraction_items` + `doc_id`. It cannot reproduce path C (resolver-assigned id), so resolver-matched entities always surface as missing.

Today's Humphrey Affidavit failure is the direct symptom. The doc has a Person with `full_name = "Mr. Dalek"` and `party_name = NULL`. Completeness computes `person-mr-dalek`. Ingest's resolver matched Mr. Dalek to a pre-existing Person created by another document, so the node in Neo4j has a *different* id. Completeness reports 1 of 40 missing. The node IS present — just not under the expected id.

### 5.3 Qdrant → Neo4j mapping

Each Qdrant point's payload has `node_id` (verified in api/pipeline/index.rs:165 and pipeline/steps/index.rs:305) — the Neo4j node id. This is the reverse mapping. It is stored in the Qdrant payload, not in Postgres. `verify_qdrant_points` (completeness_helpers.rs:169) uses it.

So Qdrant → Neo4j lineage is recorded; Neo4j → extraction_item lineage is NOT.

### 5.4 Gap summary

| Mapping | Where stored | Reachable at completeness time |
|---|---|---|
| extraction_item → Neo4j | `pg_to_neo4j` HashMap (in-memory, discarded) | ❌ |
| Neo4j → Qdrant | `qdrant_point.payload.node_id` | ✅ (via `verify_qdrant_points`) |
| Neo4j → extraction_item (inverse) | not stored | ❌ |

---

## Section 6 — Step Lifecycle Recording

**Principle:** The framework owns step lifecycle, not individual steps.

**Answer:** The framework does NOT own `pipeline_steps` lifecycle. Each step manages its own recording. Six of seven steps have error-path gaps.

### 6.1 Framework scope

Confirmed via `grep -r "pipeline_steps" /home/roman/Projects/colossus-rs/colossus-pipeline/src` — zero matches. The framework updates `pipeline_jobs` via `ExecutionResult` returned from `executor::execute_step` (worker/executor.rs:47). `pipeline_steps` is a colossus-legal-backend-owned table. Each step body must explicitly call `record_step_start` → `record_step_complete` / `record_step_failure`.

### 6.2 Per-step matrix

| Step | `record_step_start` | `record_step_complete` on Ok | `record_step_failure` on Err | Err-path coverage |
|---|---|---|---|---|
| `extract_text` (pipeline/steps/extract_text.rs) | ✅ line 287 | ✅ line 523 (inside `if let Err(e)` log) | ❌ missing | **gap**: any Err before line 523 (NoUsableText at line 471, DB writes, cancel returns at 300/319/347/422) leaves pipeline_steps row at `running` |
| `llm_extract` (pipeline/steps/llm_extract.rs) | ✅ line 189 | ✅ line 409 | ❌ missing | **gap**: any `?`-propagated error between 189 and 409 leaves row at `running` |
| `verify` (pipeline/steps/verify.rs) | ✅ line 102 | ✅ line 164 | ❌ missing | **gap**: `let result = self.run_verify(...).await?` at verify.rs:~130 propagates Err, skips record_step_complete |
| `auto_approve` (pipeline/steps/auto_approve.rs) | ✅ line 90 | ✅ line 180 | ❌ missing | **gap**: `bulk_approve` / `count_pending` return via `?`, skip record_step_complete |
| `ingest` (pipeline/steps/ingest.rs) | ✅ line 146 | ✅ line 194 | ❌ missing | **gap**: `let stats = self.run_ingest(...).await?` at line 172 |
| `index` (pipeline/steps/index.rs) | ✅ line 136 | ✅ line 173 | ❌ missing | **gap**: `let stats = self.run_index(...).await?` at line 152 |
| `completeness` (pipeline/steps/completeness.rs) | ✅ line 152 | ✅ line 193 (inside Ok arm) | ✅ line 216 (inside Err arm) | **no gap** — match-wrap pattern |

### 6.3 Why this keeps producing "running forever" rows

Every step that errors via `?` propagation skips its own completion-record call. The framework catches the error and records the failure to `pipeline_jobs.error`, but `pipeline_steps.status` stays at `'running'`. The Execution History UI reads `pipeline_steps`, so the user sees a step stuck in-flight even though the pipeline job is failed.

This is a systemic class of bug. It has appeared three times this session (Ingest, Completeness, and — before my rewrites — Index). Each time the fix was local to the one step that happened to fail. The underlying pattern (6 of 7 steps vulnerable) remains.

---

## Section 7 — Schema Consistency

**Principle:** Entity type names should be consistent across extraction schemas and pipeline stages.

### 7.1 Entity-type names in the schemas

| Schema | Entity types |
|---|---|
| `complaint_v2.yaml` (default) | Party, LegalCount, ComplaintAllegation, Harm |
| `complaint.yaml` (older, deprecated) | Party, ComplaintAllegation, LegalCount, Harm |
| `affidavit.yaml` | Party, SwornStatement, DocumentReference |
| `general_legal.yaml` | Party, Statement, LegalCitation, CourtOrder, DocumentReference |

`Party` is the universal type. All schemas use it; none declare `Person` or `Organization` directly.

### 7.2 Where Party vs Person/Organization appears in code

| Stage | What it sees |
|---|---|
| `LlmExtract` | LLM emits whatever the schema declares. Rows written with `entity_type='Party'`. |
| `Verify` | Reads `entity_type` — treats all values opaquely. |
| `AutoApprove` | Reads `entity_type` — treats all values opaquely. |
| `Ingest` | Splits Party into Person/Organization based on the `party_type` property (ingest_helpers.rs:204). Calls `create_party_nodes` for `entity_type == "Party"` (ingest_helpers.rs:187 filter). Calls `create_entity_node` for everything else (ingest.rs step:307 filter `entity_type != "Party"`). |
| `Ingest` (post-write sync) | `update_item_entity_type` rewrites the row to `"Person"` or `"Organization"` (ingest.rs step:407). |
| `Index` | `fetch_nodes_for_document` is label-agnostic post-Batch-5 rewrite — uses `(n)-[:CONTAINED_IN]->(d:Document)` pattern. Does not depend on entity_type string. |
| `Completeness` | `compute_expected_neo4j_ids` filters on `entity_type IN ("Person", "Organization")` for Party handling (completeness_helpers.rs:67). Assumes Ingest has already rewritten Party → Person/Organization. |

### 7.3 The ordering trap

`Completeness` assumes `entity_type == "Person"` or `"Organization"` for party items. That is true *only after* `update_item_entity_type` runs inside Ingest. If Ingest fails partway through the `for item in &items` loop at ingest.rs step:400–415, some items will have been updated to Person/Organization while others remain Party. Completeness computes different expected ids for each subset — some via path D (Person branch), some via path A (other branch, which produces `{doc_slug}:party:{hash}`).

This is a latent inconsistency bug that only manifests on partial Ingest failure. The current codebase has no test that exercises it.

---

## Section 8 — Recommendations

Prioritized structural changes, ordered by severity × dependency-depth. Each item states what it fixes, approximate scope, and dependencies on other items.

### R1 — Persist extraction-item → Neo4j-id lineage (HIGHEST)

**What it fixes:** Section 5 (lineage gap), Section 3 (resolver blind spot), Section 4 (re-process consistency), root cause of the Humphrey Affidavit completeness false-positive.

**Scope:** Small-to-medium.
- Add `extraction_items.neo4j_id VARCHAR NULL` column.
- `create_party_nodes` and `create_entity_node` already compute the final id; store it into `pg_to_neo4j`. At transaction commit, `UPDATE extraction_items SET neo4j_id = $1 WHERE id = $2` in a batch.
- `compute_expected_neo4j_ids` reads `neo4j_id` directly instead of recomputing — path D disappears, falls back to "compute" only when the column is NULL (legacy rows).

**Dependencies:** none. Can be rolled out before any of R2–R5.

### R2 — Framework-level `pipeline_steps` recording

**What it fixes:** Section 6. Closes the 6-of-7-steps recording-gap class systematically rather than per-step.

**Scope:** Medium. Requires changes to `colossus-pipeline` (upstream repo). The executor already knows when a step starts and finishes (`execute_step` at `colossus-pipeline/src/worker/executor.rs:47`). Add a DB-trait hook so the executor itself writes `pipeline_steps.status` based on the `ExecutionResult` variant. Each backend step can then shed its own `record_step_*` calls — the framework does it uniformly. Alternatively, the framework could expose a `StepRecorder` trait and call it automatically between each `execute_current` invocation.

**Dependencies:** colossus-rs release, then backend dep bump and removal of per-step recording. Moderate coordination.

### R3 — Separate resolve from write

**What it fixes:** Section 4 (coupled load+resolve), makes Section 2 partial cases idempotent.

**Scope:** Medium-to-large.
- Add `Resolve` as a distinct pipeline step, between Ingest and Index.
- Ingest writes all Party items with naive ids (`{prefix}-{slug(name)}` unconditionally — ignores resolution map).
- Resolve then creates `SAME_AS` relationships between nodes that the resolver considers the same entity, or collapses them.
- Persist resolver decisions to a `party_resolution` table so decisions are auditable and re-runnable.

**Dependencies:** R1 (need persisted id lineage first to know what Ingest wrote). Medium effort — 1–2 weeks.

### R4 — Stop mutating `extraction_items.entity_type`

**What it fixes:** Section 1 (immutability), Section 7.3 (ordering trap).

**Scope:** Small.
- Add `extraction_items.resolved_entity_type VARCHAR NULL`. `update_item_entity_type` writes to the new column, leaves `entity_type` alone.
- Downstream readers use `COALESCE(resolved_entity_type, entity_type)`.

**Dependencies:** R1 conceptually similar; can land independently.

### R5 — Make LlmExtract self-idempotent

**What it fixes:** Section 2.2 (`llm_extract` is the only post-upload step that isn't self-idempotent).

**Scope:** Small-to-medium.
- Add a unique constraint on `extraction_items (run_id, content_hash)` (or some deterministic key).
- `insert_extraction_item` uses `INSERT … ON CONFLICT DO NOTHING`.
- `insert_extraction_run` keyed on `(document_id, pass_number)` with ON CONFLICT returning the existing row id.

**Dependencies:** none.

### R6 — Separate grounding fields from source data

**What it fixes:** Section 1.2 (co-mingling of LLM output with verification result).

**Scope:** Small.
- Add `extraction_item_grounding` table with FK to `extraction_items.id`, columns `grounding_status`, `grounded_page`, `verified_at`, `verified_by`.
- `update_item_grounding` writes to the new table.
- Verify queries remain the same shape (join via FK).

**Dependencies:** none. Low-priority relative to R1–R3.

### R7 — Safer `source_document` semantics on shared nodes

**What it fixes:** Section 4.4 latent bug (A-ingest, B-ingest, A-reprocess leaves dangling array entry in B's `source_documents`).

**Scope:** Small.
- On `cleanup_neo4j`, also iterate `n.source_documents` arrays and remove the current doc_id from nodes owned by other docs.
- Or: switch to a dedicated `OWNS` relationship from Document → entity instead of storing the association on the entity node.

**Dependencies:** ideally R3 (resolve separation) first, so shared-entity lifecycle becomes an explicit concern rather than an implicit MERGE side effect.

### Short-term mitigations vs long-term fixes

**Short-term (this week):**
- R1 is the single highest-leverage fix. Addresses the immediate completeness false-positive on Humphrey and similar, and unlocks R3/R4.
- Add a match-wrap + `record_step_failure` to the 6 remaining step files as a tactical patch. Per-step duplication, but closes the visible symptom until R2 is ready.

**Long-term (this quarter):**
- R2 — framework-level lifecycle recording (proper fix for the recording-gap class).
- R3 — separate Resolve step (Neo4j-recommended architecture).

### What is well-designed and should NOT be changed

To be explicit about what's working:

- The ExtractText step's OCR fallback path is correctly structured (one HTTP call per doc for batch Surya OCR, per-page loop in Rust).
- Qdrant upsert + payload indexes for `document_id`/`node_id` are correctly configured.
- The `stable_entity_id` function (for non-Party entities) is exactly the pattern LlamaIndex and neo4j-graphrag-python recommend — content-derived, order-independent, deterministic.
- The `cleanup.rs` DELETE ordering is FK-safe and transactional.
- `completeness` (post-Batch-5 rewrite) has the right error-path discipline that every other step should adopt.
- The `CONTAINED_IN` generic query in `fetch_nodes_for_document` (Batch 5) is label-agnostic and future-proof.

---

*Audit produced by read-only review on 2026-04-21. No code was modified. This document is not committed.*
