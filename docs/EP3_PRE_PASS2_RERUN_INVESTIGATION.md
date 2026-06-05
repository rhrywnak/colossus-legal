# EP3-PRE — Pass-2 Re-run Capability Investigation (READ-ONLY)

> **Status:** Investigation report. No code, config, templates, or migrations were
> modified to produce this. Nothing was re-run, built, or deployed.
> **Branch at time of writing:** `main` (clean working tree).
> **Date:** 2026-06-04.

## Terminology correction up front

The "141 CORROBORATES/CONTRADICTS edges" live in **Neo4j** and originate from
**Pass-2's Postgres `extraction_relationships` rows**, pushed to the graph by the
**Ingest** step. Pass-2 itself never writes Neo4j. So "re-running Pass-2" only
rewrites Postgres; the graph only changes when **Ingest** runs afterward. Every
destructive claim below hinges on that two-stage split.

---

## Q1 — Does a Pass-2-only re-run path exist?

**NO standalone path. Pass-2 is only reachable as step 3 of the full document
workflow, and it hard-short-circuits once a COMPLETED Pass-2 run exists.**

The orchestrator `run_pass2_extraction`
(`backend/src/pipeline/steps/llm_extract_pass2.rs:266`) has exactly two callers:

- `backend/src/pipeline/workflow_steps/llm_extract.rs:292` — the Restate workflow
  body (`step_llm_extract_pass2_body`), invoked from `workflow.rs:420` as step 3
  of an 8-step pipeline.
- `backend/src/pipeline/steps/llm_extract_pass2.rs:113` — the legacy FSM `Step`
  adapter.

There is **no** `/documents/:id/pass2` route, no CLI binary (only
`backend/src/bin/load_canonical_elements.rs` exists), and no admin trigger. The
only ways to *reach* the pass-2 step are the full-pipeline triggers in
`backend/src/api/pipeline/mod.rs`:

- `POST /documents/:id/process` → `process::process_handler` (`mod.rs:96`)
- `POST /documents/:id/reprocess` → `review::reprocess_handler` (`mod.rs:131`)

**The blocking short-circuit** (`llm_extract_pass2.rs:275-285`):

```rust
// 1. Idempotency: short-circuit on an existing COMPLETED pass-2 row.
if pass2_already_complete(db, document_id).await? {
    return Ok(Pass2ExtractionResult { skipped_already_complete: true, ..Default::default() });
}
```

`pass2_already_complete` (`:941`) returns true whenever a `pass_number = 2`,
`status = 'completed'` row exists for the doc. So `POST /process` on a finished
doc runs Pass-1 (which *also* short-circuits via its own
`extraction_already_complete`) and then Pass-2 **does nothing** — no
regeneration. Harmless but useless.

The **only** in-repo way to defeat the short-circuit is `reprocess_handler`
(`backend/src/api/pipeline/review.rs:596`), which deletes **all** of the
document's runs — both passes:

```rust
sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1")  // review.rs:664
```

…then resets status to `TEXT_EXTRACTED` (`:683`) and re-runs from extraction.
That **forces Pass-1 re-extraction and a fresh Pass-1 LLM call** — precisely the
API spend the EP3 plan wants to avoid. It also pre-emptively wipes Neo4j and
Qdrant (`review.rs:626-627`).

Confidence: **High.**

---

## Q2 — What does a re-run do to existing relationships?

Two layers. Both matter.

### Postgres layer — UPSERT run row, then DELETE-and-recreate children (scoped to the Pass-2 run)

`run_pass2_extraction` step 11 (`:614`) calls `insert_extraction_run(.., 2, ..)`,
which **upserts on `(document_id, pass_number)`** (`extraction_runs.rs:89`,
constraint `extraction_runs_doc_pass_unique`), reusing the *same* Pass-2 row id
and resetting it to `RUNNING`. It then calls `reset_extraction_run_children`
(`extraction_runs.rs:136`):

```rust
sqlx::query("DELETE FROM extraction_relationships WHERE run_id = $1")  // :150
sqlx::query("DELETE FROM extraction_items WHERE run_id = $1")          // :155
```

Blast radius is **scoped to the Pass-2 run_id only**. Pass-1's items/relationships
live under a different run_id and are untouched. So at the Postgres level a re-run
cleanly *replaces* the Pass-2 `extraction_relationships` set (the
CORROBORATES/CONTRADICTS rows). **But you never reach step 11 unless the
COMPLETED Pass-2 row was already cleared** (Q1).

### Neo4j layer — cleanup-then-write, provenance-blind DETACH DELETE of all the doc's nodes

Neo4j edges only change when **Ingest** runs
(`backend/src/pipeline/steps/ingest.rs:246`, `run_ingest`). Its idempotency model
is **cleanup-then-write, not MERGE-only** (header comment `:22-27`). Step 0
(`ingest.rs:261`) calls `cleanup_neo4j` before writing:

```rust
// cleanup.rs:201-208 — runs TWICE, for source_document and source_document_id
"MATCH (n) WHERE n.{property} = $doc_id DETACH DELETE n RETURN count(n) AS removed"
```

`DETACH DELETE` removes each of the document's nodes **and every relationship
attached to them**, regardless of edge type or provenance. The
CORROBORATES/CONTRADICTS edges originate at this document's own entity nodes, so
they are destroyed here — *then* re-MERGEd from the freshly-fetched Postgres rows
at `ingest.rs:527` (`create_ingest_relationship`, which uses `MERGE … ON
CREATE/ON MATCH`, `ingest_helpers.rs:522`).

Net effect of a true re-run (reprocess → extract → pass-2 → ingest): the doc's
old graph edges are wiped and rebuilt from the new (stricter-bar) Pass-2 output.
Edges the new bar drops **do** disappear — but via Ingest's DETACH DELETE, **not**
via Pass-2.

> ⚠️ Asymmetry: if Pass-2 (Postgres) reruns but Ingest does **not**, Neo4j still
> holds the *old* 141 edges. If Pass-2 produces *more* edges, Ingest's MERGE adds
> them; if *fewer*, only Ingest's cleanup removes the stale ones. Postgres and
> Neo4j can silently diverge between the two stages.

### `provenance` edges (the human/durable mappings)

1. **Postgres reconciliation is correctly scoped.** Pass-2's
   `persist_cross_tier_relationships` (`llm_extract_pass2.rs:863`) and Ingest's
   `write_proves_element_edges` (`ingest.rs:727`) both reconcile only
   `provenance = 'extracted'`:
   - `delete_extracted_authored_relationships_for_document` →
     `DELETE FROM authored_relationships WHERE document_id = $1 AND provenance = $2`
     with `$2 = 'extracted'` (`authored_entities.rs:373, 376`).
   - This preserves `provenance = 'canonical'` (loader, `document_id` NULL) and any
     `provenance = 'authored'` rows in Postgres. ✓

2. **`provenance = 'authored'` is not written by any production code path.** The
   literal `'authored'` provenance value appears **only in
   `backend/tests/authored_entities_integration.rs`**. Production writers pass
   `PROVENANCE_CANONICAL = "canonical"` (canonical loader,
   `canonical_elements/authored.rs:298,317`) or `PROVENANCE_EXTRACTED =
   "extracted"` (Pass-2, `authored_entities.rs:416`). The
   `upsert_authored_relationship` doc comment names the intended `'authored'`
   writer as *"a future authoring UI handler"* (`authored_entities.rs:257`) — not
   yet implemented. So as of this codebase, "durable human-corrected mappings" as
   `provenance='authored'` rows **do not exist in production**; the durable
   canonical mappings are `provenance='canonical'`, owned by the YAML loader.

   In **Neo4j**, the targeted cross-tier delete
   `delete_cross_tier_relationships_for_document` keys on `asserted_by_document`
   (`ingest_helpers.rs:896`:
   `MATCH ()-[r {asserted_by_document: $document_id}]->() DELETE r`), a property
   set **only** by `write_cross_tier_relationship` (extracted edges) — so it never
   targets canonical/authored edges. **However**, `cleanup_neo4j`'s DETACH DELETE
   is provenance-blind: any `'authored'`/`'canonical'` edge whose endpoint node is
   owned by the re-ingested document (`source_document_id = doc_id`) is
   collateral-deleted and is **not** re-created by Ingest (which only re-writes
   `'extracted'` cross-tier edges). Whether that bites depends on node ownership —
   see OPEN RISKS.

Confidence: **High** on Postgres mechanics and the cleanup-then-write model;
**Medium** on which graph nodes the canonical/authored edges attach to
(data-dependent, not determinable from code alone).

---

## Q3 — Is there rollback / safety?

**Essentially none. No backup, no dry-run, no graph-level transaction spanning the
destructive cleanup and the rewrite.**

- **No dry-run / preview mode** anywhere in the Pass-2 or Ingest paths.
- **Transaction scope is too narrow to roll back the destruction.** Ingest's Neo4j
  *write* runs in one txn (`ingest.rs:341` `start_txn` … `:591` commit), but
  `cleanup_neo4j` (the DETACH DELETE) runs **before and outside** that
  transaction (`ingest.rs:261`; its own auto-commit queries in `cleanup.rs`). If
  the rewrite fails after cleanup, the deletes are already committed — the edges
  are gone with nothing to restore them.
- **Postgres `reset_extraction_run_children` is transactional**
  (`extraction_runs.rs:140` `begin` … `:160` commit) but only protects against
  partial *child* deletes; it is a delete, not a snapshot.
- **`extraction_run_id` provenance enables identification but not removal.** Every
  graph edge is stamped `extraction_run_id = "run-{rel.run_id}"` (`ingest.rs:526`,
  `ingest_helpers.rs:526-528`). So edges from a specific Pass-2 run *can* be
  identified by `r.extraction_run_id` in Cypher. **But**: (a) the
  `insert_extraction_run` upsert **reuses the same Pass-2 row id** across re-runs,
  so a re-run does not mint a new distinguishable id — old and new edges can share
  `run-{id}`; and (b) there is no code that deletes edges by `extraction_run_id`.
  Removal of a bad run is manual Cypher, not a supported operation.
- The only "way back" is forward: another `reprocess` (full re-extraction) or
  hand-written Cypher. Neither is a rollback.

Confidence: **High.**

---

## Q4 — What inputs does Pass-2 read?

**Both confirmed — a prose-only template edit takes effect on the next run with no
rebuild, and the `ctx:` complaint Allegations are re-injected identically on a
re-run.**

- **Template is read from disk at runtime.** `llm_extract_pass2.rs:415-417`:
  ```rust
  let template_path = context.registry.template_path(&pass2_template_file);
  let template_text = std::fs::read_to_string(&template_path)...
  ```
  The filename comes from the resolved profile (`resolved.pass2_template_file`,
  `:362`), and the file is read fresh on every invocation. A prose-only edit to
  e.g. `prompts/.../pass2_*_v4.md` is picked up on the next run — **no recompile**.
  The content is SHA-256'd into the `processing_config` snapshot (`:418`), so the
  bar change is auditable.

- **`ctx:`-prefixed complaint Allegations are injected every run** via
  `extraction::load_cross_document_context(db, document_id)` (`:451`), rendered
  into `entities_json` and added to `id_map` with their `ctx:`-prefixed ids
  (`:559-580`). This is a live DB query each run — re-running re-pulls the current
  published complaint Allegations the same way as the first run. The cross-doc set
  is also recorded in the snapshot (`:536-549`). Authored Tier-1 canonical Elements
  are loaded prompt-only when `CASE_SLUG` is set (`:497-520`).

Confidence: **High.**

---

## VERDICT — Does a safe Pass-2-only re-run path exist? **NO.**

- No standalone Pass-2 invocation exists; Pass-2 is wired only as step 3 of the
  full workflow and **hard short-circuits on the existing COMPLETED Pass-2 run**.
- The only in-repo way to clear that short-circuit is `reprocess_handler`, which
  deletes **both** passes' runs and forces **Pass-1 re-extraction** (new extraction
  LLM spend) — violating the stated "no re-run of Pass-1" constraint.
- Reaching Neo4j requires Ingest, whose `cleanup_neo4j` does a **provenance-blind
  DETACH DELETE** of the doc's nodes and all attached edges before rewriting.

A clean Pass-2-only regeneration that (a) skips Pass-1, (b) replaces only the
CORROBORATES/CONTRADICTS edges, and (c) provably preserves the durable mappings
**does not exist as a supported operation today.** It would require new code (a
Pass-2-scoped re-run endpoint that clears only the Pass-2 run + a graph
reconciliation that deletes only that run's edges).

## DESTRUCTIVE-OP SUMMARY — what a re-run deletes and its blast radius

Via the only working trigger (`reprocess`):

| Stage | Statement | Deletes | Blast radius |
|---|---|---|---|
| `reprocess_handler` (`review.rs:626-670`) | `cleanup_neo4j` + `cleanup_qdrant` + `DELETE FROM extraction_relationships/items/runs WHERE document_id=$1` | **All** Postgres runs (Pass-1 **and** Pass-2), all items, all relationships; all Qdrant points; all Neo4j nodes for the doc | Whole document, both passes |
| Pass-2 `reset_extraction_run_children` (`extraction_runs.rs:150-158`) | `DELETE … WHERE run_id = <pass2 run>` | Pass-2 `extraction_relationships` + items | Pass-2 run only |
| Ingest `cleanup_neo4j` (`cleanup.rs:207`) | `MATCH (n) WHERE n.source_document(_id)=$doc DETACH DELETE n` | Every node owned by the doc **and every edge attached to it** — provenance-blind | The doc's entire subgraph: CORROBORATES, CONTRADICTS, DERIVED_FROM, CONTAINED_IN, and any cross-tier/authored/canonical edge touching one of this doc's nodes |
| Ingest `delete_cross_tier_relationships_for_document` (`ingest_helpers.rs:896`) | `MATCH ()-[r {asserted_by_document:$doc}]->() DELETE r` | Only `extracted` cross-tier edges this doc asserted | PROVES_ELEMENT (extracted) for this doc |

**Preserved:** Postgres `authored_relationships` rows with
`provenance IN ('canonical','authored')` (reconciliation scoped to `'extracted'`).
**At risk and NOT re-created:** any canonical/authored Neo4j edge whose endpoint
node is owned by the re-ingested document (collateral of the provenance-blind
DETACH DELETE; Ingest re-writes only `'extracted'` cross-tier edges).

## OPEN RISKS

1. **No Pass-1-sparing path exists.** Every working trigger re-runs (and re-bills)
   Pass-1. The EP3 premise ("re-run Pass-2 without re-running Pass-1") is **not
   currently achievable** without new code.
2. **Cleanup is outside the write transaction.** A failure between `cleanup_neo4j`
   and the Ingest commit leaves the doc's subgraph **deleted with no restore** —
   irreversible, no backup.
3. **Postgres/Neo4j divergence window.** Because Pass-2 (PG) and Ingest (Neo4j) are
   separate stages, a partial sequence leaves the 141 graph edges and the PG rows
   inconsistent, with no reconciliation that removes stale graph edges except
   Ingest's full cleanup.
4. **Re-run does not mint a distinguishable run id.** The Pass-2 upsert reuses the
   row id, so "remove the bad run's edges by `extraction_run_id`" is not reliably
   possible — old and new edges can collide on `run-{id}`.
5. **Provenance-blind graph cleanup vs. durable mappings.** If the case ever
   introduces real `provenance='authored'` human mappings (the doc comment's
   "future authoring UI"), or if canonical/authored edges attach to a re-ingested
   doc's nodes, `cleanup_neo4j` will silently delete them without re-creation.
   **Needs a data check** (Cypher: are there edges with `asserted_by_document`
   absent / `provenance IN ['authored','canonical']` whose endpoint node has
   `source_document_id` = the Phillips/CFS docs being re-run?) before any re-run.
   Determinable only from live graph data, not code.
6. **The short-circuit can mask intent.** A naive `POST /process` (the "safe"
   trigger) silently does nothing for Pass-2, so an operator could believe edges
   were regenerated under the new bar when they were not.

---

*End of report. Read-only investigation; no repository code/config/templates were
modified, and no re-run, build, or deploy was performed.*
