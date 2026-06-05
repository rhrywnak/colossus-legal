# EP3-PRE-2 — Document Deletion Completeness Investigation (READ-ONLY)

> **Status:** Investigation report. No code, config, templates, or migrations were
> modified to produce this. Nothing was deleted, reprocessed, built, or deployed.
> **Branch at time of writing:** `main` (clean tree apart from the untracked
> EP3-PRE doc).
> **Date:** 2026-06-04.
> **Companion:** [EP3_PRE_PASS2_RERUN_INVESTIGATION.md](./EP3_PRE_PASS2_RERUN_INVESTIGATION.md)

**Scope note:** "Deletion" has three distinct code paths in this repo. All three
were audited; the task's premise (reprocess = delete-then-rebuild) and the literal
"delete a document" both center on:

- **Full delete** — `DELETE /api/admin/pipeline/documents/:id` → `delete_document`
  (`delete.rs:44`) → `delete_all_document_data` (`documents_delete.rs:153`).
- **Reprocess** — `POST /documents/:id/reprocess` → `reprocess_handler`
  (`review.rs:596`, inline statements).
- **Pipeline cancel/retry cleanup** — `cleanup_postgres` / `cleanup_neo4j` /
  `cleanup_qdrant` (`pipeline/steps/cleanup.rs`).

---

## Q1 — Full artifact inventory a document creates

**Postgres** (tables keyed to a document, from `backend/pipeline_migrations` +
`backend/migrations`):

| Table | How it keys to the document |
|---|---|
| `documents` | `id` (PK) |
| `document_text` | `document_id` → `documents(id)` (RESTRICT) |
| `pipeline_config` | `document_id` PK → `documents(id)` (RESTRICT) |
| `pipeline_steps` | `document_id` → `documents(id)` (RESTRICT) |
| `extraction_runs` | `document_id` → `documents(id)` (RESTRICT) |
| `extraction_items` | `run_id` → `extraction_runs(id)`; `document_id` → `documents(id)` |
| `extraction_relationships` | `run_id`, `from/to_item_id` → `extraction_items(id)`; `document_id` |
| `extraction_chunks` | `extraction_run_id` → `extraction_runs(id)` **ON DELETE CASCADE** |
| `review_edit_history` | `item_id` → `extraction_items(id)` **RESTRICT** |
| `authored_relationships` | `document_id` (no FK) + `provenance` (`extracted`/`canonical`/`authored`) |
| `pipeline_jobs` | `job_key = document_id` (no FK) |
| `pipeline_events` | `job_id` → `pipeline_jobs(id)` **ON DELETE CASCADE** |
| `document_audit_log` | `document_id` (no FK) — append-only audit |
| `document_extractions` | `document_id` (no FK) — **no writer found in code (dormant)** |

Not document-scoped (case-scoped, excluded): `authored_entities` (keyed
`case_slug`/`entity_id`, no `document_id` column), `parties`/`cases`/`counsel`
(case metadata), `llm_models`, global `pipeline_config` rows, `rag_config`,
`qa_*`, `known_users`.

**Neo4j** — nodes carry `source_document` (Person, Organization, Allegation, Harm,
LegalCount, Evidence, etc.) or `source_document_id` (Document node). Edges
created/touched by a document's processing: `CONTAINED_IN`, `DERIVED_FROM`,
`MENTIONS`/`SUPPORTS`/etc. (Pass-1), `CORROBORATES`/`CONTRADICTS` (Pass-2),
`PROVES_ELEMENT` (cross-tier, stamped `asserted_by_document`). Case-level nodes a
document only *references*: `:Element`, `:LegalCount`, theory/declaration nodes
(`provenance='canonical'`, loader-owned, **no** `source_document`). Shared Party
nodes carry a `source_documents` **array**.

**Qdrant** — one collection, `colossus_evidence` (`qdrant_service.rs:20`,
`constants.rs:22`); points carry a `document_id` payload field (indexed).

**Filesystem** — the source PDF at `{document_storage_path}/{file_path}`
(`delete.rs:146`). OCR/extracted text is stored in Postgres `document_text`, not
on disk. No other per-document on-disk artifact found.

---

## Q2 — What the full-delete path actually deletes (coverage table)

`delete_document` (`delete.rs:44`) sequence: build audit snapshot → Restate purge
(best-effort) → INSERT `document_audit_log` → `cleanup_neo4j` (best-effort, if
ingested) → `cleanup_qdrant` (best-effort, if indexed) → `delete_all_document_data`
(one PG txn) → `remove_file` PDF (best-effort).

### Postgres (`delete_all_document_data`, `documents_delete.rs:153-208`)

| Artifact | Deleted? | Statement |
|---|---|---|
| `extraction_relationships` | ✅ | `DELETE … WHERE document_id=$1` (`:160`) |
| `extraction_items` | ✅ | `:165` |
| `extraction_runs` | ✅ | `:170` |
| `extraction_chunks` | ✅ (cascade) | via `extraction_runs` ON DELETE CASCADE |
| `document_text` | ✅ | `:176` |
| `pipeline_steps` | ✅ | `:181` |
| `pipeline_config` | ✅ | `:186` |
| `pipeline_jobs` | ✅ | `:195` (`WHERE job_key=$1`) |
| `pipeline_events` | ✅ (cascade) | via `pipeline_jobs` ON DELETE CASCADE |
| `documents` | ✅ | `:201` (last) |
| `document_audit_log` | ⚪ preserved by design | a new `DELETE` row is INSERTed (`delete.rs:96`) |
| **`review_edit_history`** | ❌ **NOT deleted** | — (and its FK **blocks** the `extraction_items` delete) |
| **`authored_relationships`** (`provenance='extracted'`, this doc) | ❌ **NOT deleted** | — |
| `document_extractions` | ❌ not deleted | dormant (no writer) — not populated in practice |

### Neo4j (`delete.rs:cleanup_neo4j`, `:311`)

```cypher
MATCH (n) WHERE n.source_document    = $doc_id DETACH DELETE n   -- :316
MATCH (n) WHERE n.source_document_id = $doc_id DETACH DELETE n   -- :351
```

Covers all doc-owned nodes and (via `DETACH`) every edge attached to them.
Preserves canonical Element/count nodes (no `source_document`).

### Qdrant (`delete.rs:cleanup_qdrant`, `:391`)

`delete_points_by_filter(colossus_evidence, "document_id", doc_id)` →
`…/collections/colossus_evidence/points/delete` (`qdrant_service.rs:13`). Single
collection; complete.

### Filesystem

`tokio::fs::remove_file({document_storage_path}/{file_path})` (`delete.rs:151`),
best-effort.

---

## Q3 — What is left behind

1. **`review_edit_history` — never deleted, and it actively breaks the delete.**
   FK `item_id → extraction_items(id)` is RESTRICT (no `ON DELETE`); rows are
   written by `insert_edit_history` (`review_edit_history.rs:34`) on every
   reviewer edit. `delete_all_document_data` deletes `extraction_items` (`:165`)
   without first clearing `review_edit_history`. For any document that was
   reviewed/edited, `DELETE FROM extraction_items` raises a foreign-key violation
   → the **entire PG transaction rolls back → the delete fails (500)**. Because
   Neo4j/Qdrant cleanup and the audit-log INSERT all run *before* the PG txn, the
   graph nodes and vectors are **already destroyed** while all PG rows remain and
   the audit log claims `DELETE`. This is the worst orphan class: cross-store
   inconsistency + a "deleted" document that still exists in Postgres.
   *(Note: `reset_extraction_run_children` (`extraction_runs.rs:142`) and
   `reprocess_handler` (`review.rs:637`) both delete `review_edit_history` first —
   so only `delete_all_document_data` and `cleanup_postgres` carry this gap.)*

2. **`authored_relationships` with `provenance='extracted'` — orphaned in Postgres
   on full delete.** These are the Pass-2 cross-tier `PROVES_ELEMENT` rows, keyed
   by `document_id` (`authored_entities.rs:391`). The only deleter,
   `delete_extracted_authored_relationships_for_document` (`authored_entities.rs:368`),
   is called **only** from Pass-2 (`llm_extract_pass2.rs:863`) and Ingest
   (`ingest.rs:727`) — never from `delete_all_document_data`, `delete.rs`, or
   `reprocess_handler`. After a full document delete, these rows survive with a
   `document_id` that no longer exists. (The matching Neo4j edges *are* removed
   when their Allegation endpoint node is DETACH-DELETEd — so PG and graph
   diverge.)

3. **Shared Party `source_documents` array residue (delete-endpoint only).**
   `delete.rs:cleanup_neo4j` does **not** strip the deleted `doc_id` from
   surviving multi-document Party nodes' `source_documents` arrays. The
   pipeline-side `cleanup.rs:cleanup_neo4j` *does*
   (`strip_source_document_from_arrays`, `cleanup.rs:153`, `:173`), but the DELETE
   endpoint calls its own `delete.rs` variant, which omits it. A Person/Org shared
   with another document keeps the deleted doc's id lingering in its
   `source_documents` array.

4. **Cross-tier edges with both endpoints surviving (low risk).**
   `delete.rs:cleanup_neo4j` relies solely on node DETACH DELETE; it never calls
   `delete_cross_tier_relationships_for_document` (the `asserted_by_document`-keyed
   delete). For current shapes (`PROVES_ELEMENT`: local extracted node → canonical
   Element) the local endpoint is deleted, so the edge goes with it. If a future
   cross-tier edge ever connected two surviving (e.g. both canonical/cross-doc)
   nodes, it would orphan.

5. **`document_extractions`** — table exists, keyed by `document_id`, deleted by no
   path; but no INSERT/UPDATE writer exists in the codebase, so it is not
   populated. Informational, not an active orphan.

---

## Q4 — Provenance safety

- **Postgres:** `delete_all_document_data` touches no `authored_*` table at all. So
  `provenance IN ('canonical','authored')` rows are preserved ✅ — but
  `provenance='extracted'` rows are **also** preserved (that's orphan #2). The
  provenance-scoped deleter exists
  (`… WHERE document_id=$1 AND provenance='extracted'`, `authored_entities.rs:373`)
  but is simply not wired into any delete/reprocess path.
- **Neo4j:** `DETACH DELETE` is provenance-blind but scoped to nodes carrying
  `source_document`/`source_document_id = doc`. Canonical nodes
  (`provenance='canonical'`, no `source_document`) are not matched → preserved ✅.
  Canonical/authored **edges** survive unless collateral to a deleted node (same
  provenance-blind caveat flagged in EP3-PRE): a `provenance='canonical'`/`'authored'`
  edge whose endpoint *is* a deleted doc-owned node is removed and not re-created.

Net: durable canonical/authored data is preserved in the common case; the residual
risk is an authored/canonical edge that happens to attach to a node owned by the
deleted document.

---

## Q5 — Transactional / recoverable?

- **Within Postgres:** ✅ atomic. `delete_all_document_data` wraps all DELETEs in
  one `pool.begin()…commit()` (`documents_delete.rs:157,206`); rollback-on-drop
  guarantees all-or-nothing.
- **Across stores:** ❌ not atomic, and ordered worst-first. Sequence: audit
  snapshot (read) → Restate purge (best-effort) → audit-log INSERT → **Neo4j DETACH
  DELETE (best-effort, errors only logged)** → **Qdrant delete (best-effort)** → PG
  txn → PDF unlink (best-effort). Failure matrix:

| Failure point | Result |
|---|---|
| Neo4j unreachable | logged, continues → PG+Qdrant gone, **graph nodes orphaned** |
| Qdrant unreachable | logged, continues → PG+graph gone, **vectors orphaned** |
| **PG txn fails** (e.g. `review_edit_history` FK, orphan #1) | PG rolls back intact, but **graph + vectors already destroyed**, audit log already says DELETE → document half-exists, unusable |
| PDF unlink fails | logged; PG already committed → **orphaned file on disk** |

- **Recoverability:** the only safety net is the `document_audit_log` JSONB
  snapshot (`delete.rs:96`), which captures `documents`, `extraction_items`,
  `extraction_relationships`, and `pipeline_steps` — but **not** `document_text`,
  `authored_relationships`, Neo4j, or Qdrant. There is no restore routine; it is a
  forensic record, not a backup.

---

## VERDICT — Does deletion leave the databases fully clean? **ORPHANS.**

Deletion does **not** leave all stores clean. It (a) can fail outright for any
reviewed document via the `review_edit_history` FK while having already destroyed
that document's graph and vectors, and (b) even on the happy path leaves
`authored_relationships(provenance='extracted')` rows in Postgres and stale
`source_documents` array entries on shared Neo4j Party nodes.

## ORPHAN INVENTORY

| # | Store | Orphan | Proof |
|---|---|---|---|
| 1 | PG (+ cross-store) | `review_edit_history` never deleted; its RESTRICT FK aborts the whole delete after Neo4j/Qdrant are already wiped | `documents_delete.rs:153-208` (no `review_edit_history` delete) vs FK `review_edit_history.item_id → extraction_items(id)` RESTRICT; writer `review_edit_history.rs:34`; cleanup order `delete.rs:118-143` |
| 2 | PG | `authored_relationships` `provenance='extracted'` rows for the deleted doc | deleter `authored_entities.rs:368` called only at `llm_extract_pass2.rs:863` + `ingest.rs:727`; absent from all delete paths |
| 3 | Neo4j | deleted `doc_id` left inside surviving shared Party `source_documents` arrays | `delete.rs:cleanup_neo4j:311-383` omits the strip that `cleanup.rs:153/173` performs |
| 4 | Neo4j | (latent) cross-tier edges with two surviving endpoints | `delete.rs` never calls `delete_cross_tier_relationships_for_document` (`ingest_helpers.rs:891`) |
| 5 | PG | `document_extractions` deleted by nothing (dormant table, no writer) | no INSERT/UPDATE writer found |

## OPEN RISKS

1. **Reprocess inherits orphan #2.** `reprocess_handler` also never clears
   `authored_relationships(extracted)` — but Pass-2/Ingest reconcile them on the
   rebuild, so reprocess masks the orphan. **Full delete does not rebuild**, so it
   leaves them permanently. Any tooling that later re-creates a document with the
   same `document_id` would inherit stale extracted PROVES_ELEMENT rows.
2. **The delete-failure window is exactly the documents you'll delete.**
   `review_edit_history` only exists after human review/edit — i.e. the
   COMPLETED/PUBLISHED documents most likely to be deleted. The failure is silent
   at the store level (graph/vectors gone) and surfaces only as a 500.
3. **Best-effort cross-store ordering is irreversible.** Neo4j/Qdrant cleanup
   precede the PG commit, so any PG failure leaves the external stores destroyed
   with no rollback and only a partial audit snapshot.
4. **Two divergent "delete extraction data" implementations**
   (`delete_document_extraction_data` in `documents_delete.rs:72`, which is exported
   but **uncalled**, and the inline block in `reprocess_handler`) both omit
   `review_edit_history`; the dead one is a latent trap if a future caller wires it
   in.
5. **`build_audit_snapshot` undercounts.** It records items/relationships/steps but
   not `authored_relationships`, `document_text`, or external-store state — so the
   audit log cannot prove what was actually removed.

---

*Read-only investigation. No code/config/templates/migrations modified; no delete,
reprocess, build, or deploy performed. Bugs are reported, not fixed.*
