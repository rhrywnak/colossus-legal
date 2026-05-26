# Postgres Schema Audit — 2026-05-26

> Read-only research. Every claim below is grounded in a `file:line` grep result or a migration
> definition. No code, migrations, or files were changed. Categorisation is factual (driven by the
> presence/absence of read and write paths); architectural decisions are deferred to Roman.

## Executive Summary

The system runs **two PostgreSQL databases** through two independent `sqlx` pools (`backend/src/database.rs:40-80`, wired in `backend/src/main.rs:153-154`):

- **`colossus_legal`** (`AppState.pg_pool`) — migrations embedded at compile time from `backend/migrations/` (5 files). Holds QA/feedback (`qa_ratings`, `qa_entries`), the v2-phase1 audit tables (`document_extractions`, `admin_audit_log`, `audit_findings`, `audit_verifications`), and the newly-added case-metadata tables (`cases`, `parties`, `counsel`).
- **`colossus_legal_v2`** (`AppState.pipeline_pool`) — migrations loaded at runtime from `backend/pipeline_migrations/` (28 files). Holds the entire extraction/review pipeline. This is where the **16 audited tables** live.

The 16 audited tables = 15 application tables in `colossus_legal_v2` + `_sqlx_migrations` (sqlx bookkeeping, one per database).

**Headline findings:**

1. **4 of 16 tables have no live read path.** `rag_config` and `pipeline_events` have **zero DML anywhere** in `backend/src` (referenced only in a TODO and doc-comments). `extraction_chunks` is **write-only** — rows are inserted and updated but never SELECTed by any code path. `pipeline_jobs` is a **legacy worker-era table** that current code never INSERTs into; it survives only through a DB trigger and delete-time cleanup.

2. **11 of 15 audited `extraction_runs` columns are dead.** The F3 "reproducibility" columns (`assembled_prompt`, `prior_context`, `temperature`, `max_tokens_requested`, `admin_instructions`, `template_hash`, `rules_hash`, `schema_hash`, `schema_content`) are written at INSERT but **never read by any SELECT** — the quality report reads its fingerprints from the `processing_config` JSONB blob instead (`report_queries.rs:144-150`). `chunks_pruned_nodes` / `chunks_pruned_relationships` are only ever `= NULL` (reset on conflict, never populated, never read).

3. **The two databases are never joined.** No function takes both pools; no query crosses the boundary. The only inter-DB link, `cases.complaint_document_id` → `documents.id`, is a soft reference: SELECTed and passed to the DTO (`case_header_repository.rs:95`, `case_header_builder.rs:68`) but never resolved against the pipeline DB, and not rendered in the frontend.

4. The remaining **12 tables and all `pipeline_config` / `documents` audited columns are essential** — each has both a live writer and a live reader, and (for most) a user-visible frontend consumer.

---

## Table-Level Findings

Reader/writer counts are distinct SQL-DML sites in `backend/src` (paths relative to `backend/src/`). FK dependents are derived from the migrations (`pipeline_migrations/20260327_create_pipeline_tables.sql`, `…20260411`, `…20260412`, `…20260417`).

### FK dependency graph (within `colossus_legal_v2`)

```
documents (id) ◄── document_text, extraction_runs, extraction_items,
                   extraction_relationships, pipeline_config, pipeline_steps   (6 dependents)
extraction_runs (id) ◄── extraction_items, extraction_relationships,
                         extraction_chunks (ON DELETE CASCADE)                  (3 dependents)
extraction_items (id) ◄── extraction_relationships (from_item_id, to_item_id),
                         review_edit_history (item_id)                          (2 dependents)
pipeline_jobs (id) ◄── pipeline_events (ON DELETE CASCADE)                       (1 dependent)
```

| # | Table | Category | Readers | Writers | FK dependents | Finding |
|---|-------|----------|---------|---------|---------------|---------|
| 1 | `_sqlx_migrations` | **A** | sqlx runtime | sqlx runtime | none | sqlx's own ledger (one per DB). No app code touches it (grep: 0 matches). Load-bearing — dropping it forces every migration to re-run. |
| 2 | `document_audit_log` | **A** | `delete_restate_purge.rs:192` (reads `snapshot->'restate'`) | `delete.rs:97` (INSERT) | none | Append-only deletion snapshot. Intentionally excluded from cleanup (`steps/cleanup.rs:273`). Read by the Restate purge path. |
| 3 | `document_text` | **A** | `document_records.rs:406`, `verify.rs:249`, `delete.rs:185` | `document_records.rs:287` (upsert), `extract_text.rs:333`, `documents_delete.rs:203`, `cleanup.rs:282` | none | Canonical per-page text. Read by Verify grounding + text viewer. |
| 4 | `documents` | **A** | `document_records.rs:300/352`, `metrics.rs:89/195/223/242`, `workload.rs:72/111`, `documents_state.rs:25/34/46`, `errors.rs:49/68` | `document_records.rs:173/202/225/247/267`, `documents_progress.rs:27/81/129/171`, `upload.rs:518`, `extract_text.rs:351/782`, `review.rs:694`, `users.rs:80`, `documents_delete.rs:228` | 6 | Central entity. Drives the entire Documents/Processing UI. |
| 5 | `extraction_chunks` | **C** | **none** (no SELECT anywhere) | `extraction_runs.rs:326` (INSERT), `:358` (UPDATE), `:188`/`documents_delete.rs:106,189`/`review.rs:665`/`cleanup.rs:280` (DELETE) | none | **Write-only.** Per-chunk observability (status, counts, tokens, duration, error, `chunk_metadata`) is written but never read back — chunk stats shown in the UI are aggregated onto `extraction_runs` by `update_run_chunk_stats`, not by reading this table. |
| 6 | `extraction_items` | **A** | `extraction_items.rs:110/277/301/322/347/373/395/418`, `review_items.rs:79/106/128/146`, `review_grounding.rs:62/83/101`, `report_queries.rs:324`, `workload.rs:60/64/80`, `extraction_relationships.rs:101…`, `extraction_context.rs:259`, `delete.rs:194/230` | `extraction_items.rs:89/147/184/263`, `review_actions.rs:22…133`, `extraction_runs.rs:281/292` (graph_status), DELETEs | 2 | Core review surface. |
| 7 | `extraction_relationships` | **A** | `extraction_relationships.rs:74/99/127/147/161`, `review_items.rs:156`, `report_queries.rs:248/286`, `delete.rs:204/244` | `extraction_relationships.rs:52` (INSERT), DELETEs (`review.rs:648`, `documents_delete.rs:90/178`, `extraction_runs.rs:178`, `cleanup.rs:278`) | none | Read by graph build + report. |
| 8 | `extraction_runs` | **A** (table) | `extraction_runs.rs:245/262`, `report_queries.rs:148/287/325`, `document_records.rs:322/334/373/385` (LATERAL), `metrics.rs:100/222`, `delete.rs:215` | `extraction_runs.rs:85` (INSERT), `:211/389`, `complete_extraction_run:211`, `llm_extract.rs:821/1816`, DELETEs | 3 | Table is essential, but **11/15 audited columns are dead** — see Column-Level Findings. |
| 9 | `known_users` | **A** | `users.rs:64`, `workload.rs:73` (JOIN) | `users.rs:43` (upsert via `/api/me`, `mod.rs:257`) | none | Reviewer registry; powers workload + assignment dropdowns. |
| 10 | `llm_models` | **A** | `models.rs:122/138/152/166`, `main.rs:619` (chat providers), `chat_models.rs`, `validation.rs:195` | `models.rs:183/215/250/267` (CRUD/toggle) | none | Runtime model registry. Admin CRUD + chat/extraction model resolution. |
| 11 | `pipeline_config` | **A** | `config.rs:104`, `config_overrides.rs:77/217`, `extract_text.rs:137` | `config.rs:85` (INSERT), `config_overrides.rs:267` (PATCH), `extract_text.rs:368/792` | none | Per-document config + overrides. All 12 audited columns live (see below). |
| 12 | `pipeline_events` | **C** | **none** | **none** | none | **Zero DML.** Table created by `pipeline_migrations/20260417`. Referenced only in doc-comments (`extraction_engine.rs:38/49/75/446`, `rig_llm_bridge.rs:38/68`) describing it as an *intended* latency-observability sink. Never wired up. |
| 13 | `pipeline_jobs` | **D** | DB trigger only (migration `20260422112238` projects `status`→`documents.status`); legacy cancel reads in `cancel.rs` | **No INSERT in current code.** Only DELETE: `process.rs:132` (cleanup failed legacy rows), `documents_delete.rs:222` | 1 (`pipeline_events`) | **Legacy worker-era table.** Restate replaced the worker (`process.rs:17`: "Restate-driven processing does NOT create a `pipeline_jobs` row"). Survives via trigger + cleanup. No new rows. |
| 14 | `pipeline_steps` | **A** | `steps.rs:95`, `state_machine.rs:62/212`, `metrics.rs:114/147/241`, `document_records.rs:328/379` (LATERAL has_failed), `errors.rs:46/50/53`, `delete.rs:260` | `steps.rs:34/53/74`, `workflow_steps/mod.rs:161…`, DELETEs (`review.rs:684`, `documents_delete.rs:126/208`) | none | Per-step audit/history. Powers Execution History UI + metrics. |
| 15 | `rag_config` | **C** | **none** | **none** | none | **Zero DML.** Created by `pipeline_migrations/20260418`. Sole reference is a TODO (`main.rs:570`: "TODO(Phase2): LlmDecomposer reconstructed from `rag_config` DB table"). Reserved for an unbuilt Phase-2 feature. |
| 16 | `review_edit_history` | **A** | `review_edit_history.rs:54` (get_edit_history) | `review_edit_history.rs:34` (INSERT), DELETEs (`review.rs:638`, `extraction_runs.rs:171`) | none | Append-only field-change log per item. Read by the review edit-history view. |

**Category totals:** A (ESSENTIAL) = 12 · C (DEAD) = 3 (`extraction_chunks`, `pipeline_events`, `rag_config`) · D (LEGACY) = 1 (`pipeline_jobs`) · B (REDUNDANT, table-level) = 0.

> **Near-redundancy note (not Category B):** `documents.processing_step` (the *current* step, denormalised onto `documents` for the live progress bar, read by `ProcessingPanel.tsx`) overlaps conceptually with `pipeline_steps` (full step *history*, read by `ExecutionHistory.tsx`). Both have distinct live readers serving distinct UI surfaces, so neither is redundant under the audit definition.

---

## Column-Level Findings

### `documents` — 19 audited columns

Every audited column is **SELECTed into `DocumentRecord`** (`document_records.rs:302-318` in `list_all_documents`, mirrored at `:353-369` in `get_document`) and has a writer. The differentiator is **frontend rendering** (from the consumer trace): a column can be read into the DTO and serialised yet never displayed.

| Column | Read (.rs) | Write (.rs) | Frontend render | Category |
|--------|-----------|-------------|-----------------|----------|
| `processing_step` | `document_records.rs:307` | `documents_progress.rs:28` | `ProcessingPanel.tsx:258` | **A** |
| `processing_step_label` | `:307` | `documents_progress.rs:29` | `ProcessingPanel.tsx:258`, `DocumentCard.tsx:175` | **A** |
| `chunks_total` | `:307` | `documents_progress.rs:30` | `ProcessingPanel.tsx:265` | **A** |
| `chunks_processed` | `:307` | `documents_progress.rs:31` | `ProcessingPanel.tsx:267` | **A** |
| `entities_found` | `:307` | `documents_progress.rs:32` | `ProcessingPanel.tsx:269` | **A** |
| `percent_complete` | `:307` | `documents_progress.rs:33` | `PipelineProgressBar.tsx:25`, `DocumentCard.tsx:187` | **A** |
| `entities_written` | `:310` | `document_records.rs:202/248` | `ProcessingPanel.tsx:298` | **A** |
| `entities_flagged` | `:310` | `document_records.rs:225` | `ProcessingPanel.tsx:312` | **A** |
| `relationships_written` | `:310` | `document_records.rs:202/249` | `ProcessingPanel.tsx:299`, `DocumentCard.tsx:204` | **A** |
| `is_cancelled` | `:309` + `documents_state.rs:25` (poller) | `documents_progress.rs:81` | not rendered | **A** (backend cancel-poll reader) |
| `error_suggestion` | `:308` | `documents_progress.rs:173` | `ProcessingPanel.tsx:381`, `DocumentCard.tsx:219` | **A** |
| `content_type` | `:315` | `upload.rs:518` | `DocumentCard.tsx:87` | **A** |
| `page_count` | `:315` | `upload.rs:518` | `DocumentCard.tsx:89/94`, `ProcessingPanel.tsx:435` | **A** |
| `text_pages` | `:315` | `upload.rs:519` | `DocumentCard.tsx:106`, `ProcessingPanel.tsx:436` | **A** |
| `scanned_pages` | `:315` | `upload.rs:519` | `DocumentCard.tsx:106`, `ProcessingPanel.tsx:437` | **A** |
| `pages_needing_ocr` | `:316` | `upload.rs:519` | **not rendered** | **A\*** (read into DTO only; no UI/functional reader) |
| `total_chars` | `:316` | `upload.rs:520` | **not rendered** | **A\*** (read into DTO only) |
| `mime_type` | `:317` | `document_records.rs:173` (insert) | **not rendered** | **A\*** (read into DTO only) |
| `original_format` | `:317` | `document_records.rs:173` (insert) | **not rendered** (ExtractText routing uses it server-side per `document_records.rs:108`) | **A** |

**A\*** = column is SELECTed into `DocumentRecord` and serialised to the frontend, but **no component renders it** and (except `original_format`) no backend logic consumes it beyond the projection. Candidates for trimming from the SELECT projection if payload size matters; not removable from the schema without confirming no external/SQL consumer. **Net: 0 dead columns; 3 carried-but-unrendered (`pages_needing_ocr`, `total_chars`, `mime_type`).**

### `extraction_runs` — 15 audited columns

The `ExtractionRunRecord` SELECT (`extraction_runs.rs:262`) reads only 10 base columns. The report SELECT (`report_queries.rs:144-150`) adds `processing_config` and reads fingerprints **out of that JSONB blob** (`report_queries.rs:200-219`), not out of the dedicated F3 columns. Result: the F3 columns are orphaned.

| Column | Written at | Read by any SELECT? | Category |
|--------|-----------|---------------------|----------|
| `chunk_count` | `extraction_runs.rs:390` (`update_run_chunk_stats`) | **Yes** — `document_records.rs:333` LATERAL → `run_chunk_count` → `DocumentCard` | **A** |
| `chunks_succeeded` | `:390` | **Yes** — `document_records.rs:333` → `run_chunks_succeeded` | **A** |
| `chunks_failed` | `:390` | **Yes** — `document_records.rs:333` → `run_chunks_failed` | **A** |
| `processing_config` | `llm_extract.rs:1816` (reset NULL on conflict `:126`) | **Yes** — `report_queries.rs:147` (fingerprint source for the quality report) | **A** |
| `assembled_prompt` | `extraction_runs.rs:88` (INSERT) + `llm_extract.rs:821` (UPDATE) | **No** — only writes; `preview.rs` `assembled_prompt` is a *different* struct computed fresh | **C** |
| `prior_context` | `extraction_runs.rs:91` + `llm_extract_pass2.rs:610` | **No** | **C** |
| `temperature` | `extraction_runs.rs:90` | **No** (the `temperature` read in `config_overrides.rs` is `pipeline_config.temperature`, a different table) | **C** |
| `max_tokens_requested` | `extraction_runs.rs:90` | **No** | **C** |
| `admin_instructions` | `extraction_runs.rs:91` | **No** (the live value is read from `pipeline_config`; this is an unread copy) | **C** |
| `template_hash` | `extraction_runs.rs:88` | **No** (report reads `template_hash` from `processing_config` JSONB, `report_queries.rs:209`) | **C** |
| `rules_hash` | `extraction_runs.rs:89` | **No** | **C** |
| `schema_hash` | `extraction_runs.rs:89` | **No** | **C** |
| `schema_content` | `extraction_runs.rs:89` | **No** | **C** |
| `chunks_pruned_nodes` | only `= NULL` reset (`extraction_runs.rs:124`) | **No** — never populated with a real value | **C** |
| `chunks_pruned_relationships` | only `= NULL` reset (`extraction_runs.rs:125`) | **No** — never populated | **C** |

**Net: 4 essential, 11 dead.** The dead columns are a textbook Category B→C transition: the JSONB `processing_config` snapshot **superseded** the dedicated F3 columns as the report's read path, and the columns were left behind write-only.

### `pipeline_config` — 12 audited columns

All 12 are read by `get_pipeline_config_overrides` (`config_overrides.rs:78-84`) and written by `patch_pipeline_config_overrides` (`config_overrides.rs:267-284`). The resolved overrides feed `resolve_config` (pipeline-functional) and the Configuration Panel (user-visible).

| Column | Read (.rs) | Write (.rs) | Frontend | Category |
|--------|-----------|-------------|----------|----------|
| `chunking_mode` | `config_overrides.rs:80` | `:274` | `ConfigurationPanel.tsx` | **A** |
| `chunk_size` | `:80` | `:275` | `ConfigurationPanel.tsx` | **A** |
| `chunk_overlap` | `:80` | `:276` | `ConfigurationPanel.tsx` | **A** |
| `max_tokens` | `:80` | `:277` | `ConfigurationPanel.tsx` | **A** |
| `temperature` | `:81` | `:278` | `ConfigurationPanel.tsx` | **A** |
| `run_pass2` | `:81` | `:279` | `ConfigurationPanel.tsx` | **A** |
| `auto_approve_grounded` | `:82` | `:280` | type only (no active control); read by `resolve_config` | **A** (backend functional) |
| `global_rules_file` | `:82` | `:281` | `AdminProfiles.tsx` | **A** |
| `pass2_template_file` | `:79` | `:271` | `ConfigurationPanel.tsx` | **A** |
| `pass2_extraction_model` | `:78` | `:270` | `ConfigurationPanel.tsx` | **A** |
| `chunking_config` (JSONB) | `:83` | `:282` | `ConfigurationPanel.tsx:192`, `configurationPanelHelpers.ts` | **A** |
| `context_config` (JSONB) | `:83` | `:283` | `ConfigurationPanel.tsx`, `configurationPanelHelpers.ts` | **A** |

**Net: 12 essential, 0 dead.** (Historical note: the legacy `pass1_model`/`pass2_model`/`pass1_max_tokens`/`pass2_max_tokens` columns described in `config.rs:19-25` were already dropped by migration `20260513_consolidate_model_columns_and_add_overrides.sql`.)

---

## Cross-Database Analysis

| | `colossus_legal` (main) | `colossus_legal_v2` (pipeline) |
|---|---|---|
| Pool | `AppState.pg_pool` (`state.rs:41`) ← `db.main_pool` (`main.rs:153`) | `AppState.pipeline_pool` (`state.rs:45`) ← `db.pipeline_pool` (`main.rs:154`) |
| Migrations | `backend/migrations/` (compile-time `sqlx::migrate!`) | `backend/pipeline_migrations/` (runtime `Migrator`) |
| Tables | `qa_ratings`, `qa_entries`, `document_extractions`, `admin_audit_log`, `audit_findings`, `audit_verifications`, `cases`, `parties`, `counsel` | the 15 pipeline tables audited above |

**Findings:**

1. **No cross-DB query exists.** Grep for any function holding both pools (`main_pool.*pipeline_pool` / `pipeline_pool.*main_pool`) returns **zero matches**. Each handler uses exactly one pool. The databases are physically and logically independent within the app.

2. **`cases` / `parties` / `counsel` (Step 5):**
   - **Created by:** `migrations/20260524095049_case_metadata_tables.sql` (main DB). Forward-only; schema only — seed data is loaded manually (`AWAD_CASE_DATA_SQL.md`, per the migration header).
   - **Read by:** `case_header_repository.rs:96` (`FROM cases WHERE case_slug=$1`), `:102` (`FROM parties … ORDER BY sort_order`), `:106` (`FROM counsel …`), all on `state.pg_pool` (`case_header.rs:89`). Shaped by `case_header_builder.rs:44`, served at `GET /api/cases/:slug`.
   - **Written by application code:** **none** — only SELECTs in `backend/src`. These tables are operator-seeded via SQL, not via the app.
   - **FKs:** `parties.case_id` → `cases.case_id` (CASCADE), `counsel.case_id` → `cases.case_id` (CASCADE). No FK to or from the pipeline DB (cross-DB FKs are impossible in Postgres).
   - **Frontend:** `services/caseHeader.ts` → `components/CaseHeader.tsx` → `pages/Home.tsx`. Renders `display_title`, `court_name`, `case_number`, parties (plaintiffs/active/dropped defendants), counsel.

3. **The only inter-DB reference is soft and unresolved.** `cases.complaint_document_id` (intended to point at a `colossus_legal_v2.documents.id`) is SELECTed (`case_header_repository.rs:95`), carried into the DTO (`case_header_builder.rs:68`, `dto/case_header.rs:29`), and **never JOINed or dereferenced** against the pipeline DB. The frontend does not render it. It is documentation-grade linkage, not an enforced relationship.

---

## Recommended Removals

Ordered by mechanical safety (consequence of the trace; not an architectural judgment). Each item lists the dependencies a removal would have to clear.

1. **`rag_config` table** — zero DML, only a TODO reference. No FK dependents, no dependents. Drop is mechanically free *iff* the Phase-2 `LlmDecomposer` work (`main.rs:570`) is abandoned or will re-create it. Migration sketch: `DROP TABLE rag_config;`.

2. **`pipeline_events` table** — zero DML. FK *to* `pipeline_jobs` (no dependents *on* it). Drop sketch: `DROP TABLE pipeline_events;` (the FK is on `pipeline_events` side, so no cascade concerns).

3. **`extraction_runs` dead columns (11)** — `assembled_prompt`, `prior_context`, `temperature`, `max_tokens_requested`, `admin_instructions`, `template_hash`, `rules_hash`, `schema_hash`, `schema_content`, `chunks_pruned_nodes`, `chunks_pruned_relationships`. All write-only or never-populated; the report reads `processing_config` instead. Removal requires editing `insert_extraction_run` (`extraction_runs.rs:65-148`) to stop binding them and the `llm_extract.rs:821` UPDATE of `assembled_prompt`. Migration sketch: `ALTER TABLE extraction_runs DROP COLUMN …` per column. **Caveat:** if "show me the exact prompt that produced this run" is a planned feature, `assembled_prompt` is the seed for it — confirm before dropping.

4. **`pipeline_jobs` + its trigger (legacy)** — no INSERT path in current code. Removal is **not** mechanically free: the trigger from `migrations …20260422112238` projects `pipeline_jobs.status` → `documents.status`, and `cancel.rs` / `process.rs:132` still reference the table for legacy in-flight rows. Safe to drop only after confirming no legacy worker rows remain and the trigger is retired. Sequence: retire trigger → drop `pipeline_events` (FK child) → drop `pipeline_jobs`.

5. **`extraction_chunks` (write-only)** — no SELECT reader. Before removal, decide whether the per-chunk forensic record (manual SQL inspection of failures) is worth keeping. If not: remove the INSERT/UPDATE in `extraction_runs.rs:317-375` and `DROP TABLE extraction_chunks` (FK child of `extraction_runs`, `ON DELETE CASCADE`, so no orphan risk).

6. **`documents` carried-but-unrendered columns (3)** — `pages_needing_ocr`, `total_chars`, `mime_type`. Not removable as cheaply (they're populated at upload and may have external/analytic value), but they can be dropped from the `DocumentRecord` SELECT projection (`document_records.rs:316-317`) to shrink the documents-list payload with zero UI impact.

---

## What Must Stay

Load-bearing — do not touch:

- **`_sqlx_migrations`** (both DBs) — sqlx's migration ledger.
- **`documents`** — 6 FK dependents; the spine of the pipeline and the entire Documents/Processing UI.
- **`extraction_runs`** (the table, and columns `chunk_count`, `chunks_succeeded`, `chunks_failed`, `processing_config`) — 3 FK dependents; `processing_config` is the sole read-path for the quality report.
- **`extraction_items`** — 2 FK dependents; the review surface.
- **`extraction_relationships`**, **`document_text`** — read by graph build, report, and grounding.
- **`pipeline_config`** (all 12 audited columns) — drives `resolve_config` + the Configuration Panel.
- **`pipeline_steps`** — Execution History UI + metrics + `has_failed_steps`.
- **`llm_models`** — runtime model registry for chat + extraction.
- **`known_users`** — reviewer workload + assignment.
- **`document_audit_log`** — deletion audit + Restate purge path.
- **`review_edit_history`** — review edit-history view.
- **`cases` / `parties` / `counsel`** (main DB) — Home-page case header.

---

## Impact on Three-Tier Architecture

Facts relevant to where planned `authored_entities` / `authored_relationships` tables should live (decision deferred to Roman):

1. **Authored case metadata already lives in the main DB.** `cases`/`parties`/`counsel` are operator-authored (manually seeded, app reads only) and sit in `colossus_legal` alongside the QA/audit tables. The precedent for "authored, human-curated data" is therefore **the main DB**, separate from machine-extracted data.

2. **Extraction data lives in the pipeline DB.** All machine-extracted entities/relationships (`extraction_items`, `extraction_relationships`) and their review state are in `colossus_legal_v2`. If authored entities are meant to be *merged with / compared against* extracted entities in a single query, co-locating them in `colossus_legal_v2` would avoid the cross-DB problem below.

3. **The current boundary blocks joins.** Because no code path holds both pools and Postgres cannot FK across databases, an authored-vs-extracted reconciliation query (e.g., "which authored parties were also extracted?") is **impossible as a single SQL statement** under today's split — it would require two queries and in-application joining, exactly as `complaint_document_id` is left unresolved today.

4. **The soft-link pattern is already in use.** `cases.complaint_document_id` shows the established (if unenforced) way the codebase links a main-DB row to a pipeline-DB row: a plain TEXT id, SELECTed and carried, never JOINed. A three-tier design that puts authored entities in the main DB would inherit this same limitation; putting them in `colossus_legal_v2` would let them participate in real FKs and JOINs with `extraction_items`.

5. **`document_extractions` (main DB, migration `20260324000000`) is out of this audit's scope** but exists in the main DB and may already represent an authored/curated extraction surface — worth tracing before adding new authored tables, to avoid a third overlapping home for the same concept.
