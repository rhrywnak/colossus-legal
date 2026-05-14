# PostgreSQL Schema Reference — `colossus_legal_v2`

| Field | Value |
|-------|-------|
| Date generated | 2026-05-14 |
| Database name | `colossus_legal_v2` (pipeline / "clean room" DB — distinct from `colossus_legal` analytical DB) |
| Server (DEV) | `10.10.100.200:5432` |
| Server (PROD) | `10.10.100.110:5432` |
| Server version | PostgreSQL 17.7 (Debian 17.7-3.pgdg13+1) |
| Auth | password (Postgres role: `postgres`) |
| Tool used | `psql 16.10` (client) → `psql -h 10.10.100.200 -U postgres -d colossus_legal_v2` |
| Last migration applied (live DEV) | `20260509162937` — `add_verification_reason_column` |
| Local repo migrations beyond DEV | `20260513_consolidate_model_columns_and_add_overrides.sql` (not yet applied; see notes §C-1) |
| Extensions | `plpgsql 1.0` only (no custom extensions) |
| Tables | **16** (`information_schema.tables WHERE table_schema='public'`) |
| Columns | **214** (`information_schema.columns WHERE table_schema='public'`) |
| Foreign keys | **13** (`pg_constraint WHERE contype='f'` in public) |
| Indexes | **39** (`pg_indexes WHERE schemaname='public'`) |
| Unique constraints (user-defined, non-PK) | **2** |
| Triggers | **3** (all on `pipeline_jobs`) |

This document is the **source of truth for the live DEV schema as of the generation date**. It is generated from `information_schema` and `pg_*` system catalogs against the live database, so it reflects any drift from the migration files in `backend/pipeline_migrations/`. The migration files remain authoritative for re-applying schema to a fresh DB; this document is authoritative for answering "what columns does table X actually have right now?"

---

## Table of Contents

1. [`_sqlx_migrations`](#table-_sqlx_migrations) — sqlx migration metadata
2. [`document_audit_log`](#table-document_audit_log) — admin action audit trail
3. [`document_text`](#table-document_text) — per-page extracted text
4. [`documents`](#table-documents) — document registry & lifecycle state
5. [`extraction_chunks`](#table-extraction_chunks) — per-chunk extraction audit
6. [`extraction_items`](#table-extraction_items) — extracted entities (per-run)
7. [`extraction_relationships`](#table-extraction_relationships) — extracted relationships (per-run)
8. [`extraction_runs`](#table-extraction_runs) — LLM extraction-run header
9. [`known_users`](#table-known_users) — first-seen user registry
10. [`llm_models`](#table-llm_models) — model catalog (id, cost, limits)
11. [`pipeline_config`](#table-pipeline_config) — per-document processing config
12. [`pipeline_events`](#table-pipeline_events) — pipeline-job event log
13. [`pipeline_jobs`](#table-pipeline_jobs) — pipeline-job state machine
14. [`pipeline_steps`](#table-pipeline_steps) — per-step execution log (transitional)
15. [`rag_config`](#table-rag_config) — RAG configuration key-value
16. [`review_edit_history`](#table-review_edit_history) — per-item review edits

Plus:
- [Relationship Diagram (Foreign Keys)](#relationship-diagram-foreign-keys)
- [All Indexes (Full List)](#all-indexes-full-list)
- [Triggers](#triggers)
- [Notes on Unused / Redundant / Drifted Columns](#notes-on-unused--redundant--drifted-columns)

---

## Table: `_sqlx_migrations`

Migration metadata maintained by the `sqlx` migrator. Do **not** edit by hand; sqlx writes this on every `cargo run` (the pipeline pool runs `MIGRATOR.run(...)` per `backend/src/database.rs:67–72`).

**Primary key:** `version`
**Foreign keys:** none
**Row count strategy:** one row per applied migration file in `backend/pipeline_migrations/`.

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `version` | bigint | NO | — | Migration version; the YYYYMMDDhhmmss-ish prefix of the migration filename. Primary key. |
| 2 | `description` | text | NO | — | Migration description (the part of the filename after the version). |
| 3 | `installed_on` | timestamp with time zone | NO | `now()` | When the migration was applied to this database. |
| 4 | `success` | boolean | NO | — | Did the migration apply cleanly? sqlx aborts the migrator on `false`. |
| 5 | `checksum` | bytea | NO | — | SHA-384 of the migration SQL. sqlx refuses to start if a migration's checksum changed since it was applied. |
| 6 | `execution_time` | bigint | NO | — | Duration in microseconds. |

**Indexes:** `_sqlx_migrations_pkey` (UNIQUE on `version`).

---

## Table: `document_audit_log`

Admin-action audit trail per document — captures who did what (e.g., delete, reset, reassign) with a JSONB snapshot for replay/forensics. Emitted by `backend/src/repositories/audit_repository.rs::log_admin_action`.

**Primary key:** `id`
**Foreign keys:** none (text `document_id` is unconstrained — historical action survives the deletion of the underlying document row).

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | integer | NO | `nextval('document_audit_log_id_seq')` | Primary key (auto-incrementing sequence). |
| 2 | `document_id` | text | NO | — | The document the action was performed on. **No FK** — see Notes §C-2. |
| 3 | `document_title` | text | NO | — | Snapshot of the title at action time. |
| 4 | `action` | text | NO | — | Action name (e.g., `pipeline.document.process_submitted`, `pipeline.document.delete`). |
| 5 | `reason` | text | YES | — | Free-text reason from the operator (UI-supplied for destructive actions). |
| 6 | `performed_by` | text | NO | — | Authentik username. |
| 7 | `performed_at` | timestamp with time zone | NO | `now()` | When the action was logged. |
| 8 | `previous_status` | text | NO | — | The `documents.status` value at the moment the action was logged (for replay context). |
| 9 | `snapshot` | jsonb | NO | — | JSONB snapshot of additional context (typed per action; e.g., `{"job_id": "..."}`). |

**Indexes:**
- `document_audit_log_pkey` (UNIQUE on `id`).
- `idx_audit_log_document` (`document_id`).
- `idx_audit_log_action` (`action`).

---

## Table: `document_text`

Per-page canonical text. Written by `ExtractText` (`backend/src/pipeline/steps/extract_text.rs`) using `ON CONFLICT (document_id, page_number) DO UPDATE` — idempotent.

**Primary key:** composite `(document_id, page_number)`
**Foreign keys:** `document_id` → `documents(id)` (no cascade)

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `document_id` | text | NO | — | FK → `documents(id)`. Part of PK. |
| 2 | `page_number` | integer | NO | — | 1-indexed page number. Part of PK. |
| 3 | `text_content` | text | NO | — | Page text (native PDF extraction or OCR result — distinction is **not** stored here; see Notes §C-3). |

**Indexes:** `document_text_pkey` (UNIQUE composite `(document_id, page_number)`).

---

## Table: `documents`

Document registry. Holds lifecycle state, processing-progress projection, error fields, and content-classification fields. The status column drives most of the UI's status-group routing (`backend/src/api/pipeline/document_response.rs`).

**Primary key:** `id` (text)
**Foreign keys:** none (referenced by many other tables)

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | text | NO | — | Application-generated text id (UUID-like). |
| 2 | `title` | text | NO | — | Display title. |
| 3 | `file_path` | text | NO | — | Filesystem path under `DOCUMENT_STORAGE_PATH`. |
| 4 | `file_hash` | text | NO | — | SHA-256 of the uploaded bytes; dup-detection key. |
| 5 | `document_type` | text | NO | — | Profile-keyed type (e.g., `complaint`, `affidavit`). Looked up against `PipelineRegistry`. |
| 6 | `status` | text | NO | `'UPLOADED'::text` | Lifecycle state; values in `backend/src/models/document_status.rs` (e.g., `UPLOADED`, `PROCESSING`, `VERIFIED`, `INGESTED`, `INDEXED`, `PUBLISHED`, `FAILED`, `CANCELLED`). |
| 7 | `created_at` | timestamp with time zone | NO | `now()` | |
| 8 | `updated_at` | timestamp with time zone | NO | `now()` | Updated by triggers/handlers; not auto-bumped. |
| 9 | `assigned_reviewer` | text | YES | — | Username of the assigned reviewer (or NULL). |
| 10 | `assigned_at` | timestamp with time zone | YES | — | Assignment time. |
| 11 | `processing_step` | text | YES | — | Stale snapshot of current pipeline step (UI legacy; canonical now `pipeline_jobs.current_step`). |
| 12 | `processing_step_label` | text | YES | — | Human-readable step label (UI). |
| 13 | `chunks_total` | integer | YES | `0` | Total chunks for the running extraction (best-effort live update). |
| 14 | `chunks_processed` | integer | YES | `0` | Chunks completed so far. |
| 15 | `entities_found` | integer | YES | `0` | Running entity count. |
| 16 | `percent_complete` | integer | YES | `0` | 0–100 percent. |
| 17 | `failed_step` | text | YES | — | Name of the step that failed (set at FAILED status). |
| 18 | `failed_chunk` | integer | YES | — | Chunk index that failed (if applicable). |
| 19 | `error_message` | text | YES | — | Latest error string. Trigger-projected from `pipeline_jobs.error` on terminal transitions (migration `20260422112238`). |
| 20 | `error_suggestion` | text | YES | — | Operator hint — **never populated by current backend** (see Notes §C-4). UI renders if non-empty. |
| 21 | `is_cancelled` | boolean | NO | `false` | True when an operator cancel has been acknowledged. |
| 22 | `entities_written` | integer | YES | `0` | Neo4j nodes written at Ingest (set by `Ingest::run_ingest`). |
| 23 | `entities_flagged` | integer | YES | `0` | Items flagged in review. |
| 24 | `relationships_written` | integer | YES | `0` | Neo4j relationships written at Ingest. |
| 25 | `content_type` | text | YES | `'unknown'::text` | Pipeline content-classification (text / scanned / mixed). |
| 26 | `page_count` | integer | YES | — | Total pages. |
| 27 | `text_pages` | integer | YES | — | Pages classified as native-text. |
| 28 | `scanned_pages` | integer | YES | — | Pages classified as scanned (route through OCR). |
| 29 | `pages_needing_ocr` | text[] (ARRAY) | YES | — | Page numbers needing OCR — stored as text array. |
| 30 | `total_chars` | integer | YES | — | Total characters in the document_text rows. |
| 31 | `mime_type` | text | YES | — | Sniffed MIME (e.g., `application/pdf`). |
| 32 | `original_format` | text | YES | — | Original file extension/format (e.g., `pdf`, `docx`). |

**Indexes:**
- `documents_pkey` (UNIQUE on `id`).
- `idx_documents_status` (`status`).
- `idx_documents_type` (`document_type`).

---

## Table: `extraction_chunks`

Per-chunk audit row for LLM extraction. One row per chunk per `extraction_run`. Written by the chunked / structured paths in `LlmExtract::extract_chunks_loop`. Inserted in `pending` status, then `complete_extraction_chunk` (`repositories/pipeline_repository/extraction.rs`) updates to `success` or `failed`.

**Primary key:** `id` (UUID v4)
**Foreign keys:** `extraction_run_id` → `extraction_runs(id)` ON DELETE CASCADE

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | uuid | NO | `gen_random_uuid()` | PK. |
| 2 | `extraction_run_id` | integer | NO | — | FK → `extraction_runs(id)`. CASCADE delete. |
| 3 | `chunk_index` | integer | NO | — | Zero-based chunk index within the run. |
| 4 | `chunk_text` | text | NO | — | The raw chunk body sent to the LLM. |
| 5 | `status` | text | NO | `'pending'::text` | One of `pending`, `success`, `failed`. |
| 6 | `node_count` | integer | NO | `0` | Entity count returned by the LLM for this chunk. |
| 7 | `relationship_count` | integer | NO | `0` | Relationship count returned by the LLM for this chunk. |
| 8 | `error_message` | text | YES | — | Parse/LLM-call error message on failure. Raw LLM response is NOT stored here (audit gap — see audit report v1 §5a.1). |
| 9 | `input_tokens` | integer | YES | — | Tokens reported by the LLM provider. |
| 10 | `output_tokens` | integer | YES | — | |
| 11 | `duration_ms` | integer | YES | — | Per-chunk wall-clock duration. |
| 12 | `created_at` | timestamp with time zone | NO | `now()` | |
| 13 | `chunk_metadata` | jsonb | YES | — | Free-form metadata from the splitter (page anchors, boundary type, etc.). |

**Indexes:**
- `extraction_chunks_pkey` (UNIQUE on `id`).
- `idx_extraction_chunks_run` (`extraction_run_id`).

---

## Table: `extraction_items`

Each row is one extracted entity. Written by `store_entities_and_relationships` (`repositories/pipeline_repository/extraction.rs`). The `review_status` column drives the human-in-the-loop review UI.

**Primary key:** `id` (serial)
**Foreign keys:**
- `run_id` → `extraction_runs(id)`
- `document_id` → `documents(id)`

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | integer | NO | `nextval('extraction_items_id_seq')` | PK. |
| 2 | `run_id` | integer | NO | — | FK → `extraction_runs(id)`. |
| 3 | `document_id` | text | NO | — | FK → `documents(id)`. |
| 4 | `entity_type` | text | NO | — | Schema-defined entity type (e.g., `Person`, `LegalCount`). |
| 5 | `item_data` | jsonb | NO | — | The LLM's structured extraction (properties, ids, refs). |
| 6 | `verbatim_quote` | text | YES | — | LLM-provided verbatim quote for grounding. |
| 7 | `grounding_status` | text | YES | — | Set by `Verify` step: `exact`, `normalized`, `not_found`, `derived`, `derived_invalid`, `unverified`. |
| 8 | `grounded_page` | integer | YES | — | Page where the quote was found in canonical text. |
| 9 | `review_status` | text | NO | `'PENDING'::text` | Values: `PENDING`, `APPROVED`, `REJECTED`, `EDITED`. |
| 10 | `reviewed_by` | text | YES | — | Reviewer username. |
| 11 | `reviewed_at` | timestamp with time zone | YES | — | |
| 12 | `review_notes` | text | YES | — | Reviewer free-text notes. |
| 13 | `graph_status` | text | YES | `'pending'::text` | Lifecycle: pending → ingested → indexed → flagged. |
| 14 | `neo4j_node_id` | character varying(255) | YES | — | Set by `Ingest` step (R1 lineage). Used by Completeness verifier. |
| 15 | `resolved_entity_type` | character varying(100) | YES | — | Post-resolver label (`Person` / `Organization` for a `Party`). |
| 16 | `verification_reason` | text | YES | — | Diagnostic reason for grounding outcomes (added in migration `20260509162937`). |

**Indexes:**
- `extraction_items_pkey` (UNIQUE on `id`).
- `idx_extraction_items_run` (`run_id`).
- `idx_extraction_items_document` (`document_id`).
- `idx_extraction_items_review` (`review_status`).

---

## Table: `extraction_relationships`

Each row is one extracted relationship (edge) between two `extraction_items`. Written alongside items by `store_entities_and_relationships`.

**Primary key:** `id` (serial)
**Foreign keys:**
- `run_id` → `extraction_runs(id)`
- `document_id` → `documents(id)`
- `from_item_id` → `extraction_items(id)`
- `to_item_id` → `extraction_items(id)`

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | integer | NO | `nextval('extraction_relationships_id_seq')` | PK. |
| 2 | `run_id` | integer | NO | — | FK → `extraction_runs(id)`. |
| 3 | `document_id` | text | NO | — | FK → `documents(id)`. |
| 4 | `from_item_id` | integer | NO | — | FK → `extraction_items(id)`. |
| 5 | `to_item_id` | integer | NO | — | FK → `extraction_items(id)`. |
| 6 | `relationship_type` | text | NO | — | Schema-defined type (e.g., `STATED_BY`, `CORROBORATES`). |
| 7 | `properties` | jsonb | YES | — | Optional edge properties (LLM-provided). |
| 8 | `review_status` | text | NO | `'PENDING'::text` | Same enum as `extraction_items.review_status`. |
| 9 | `reviewed_by` | text | YES | — | |
| 10 | `reviewed_at` | timestamp with time zone | YES | — | |
| 11 | `tier` | integer | NO | `1` | Extraction pass (1 = pass-1, 2 = pass-2). |

**Indexes:**
- `extraction_relationships_pkey` (UNIQUE on `id`).
- `idx_extraction_relationships_run` (`run_id`).
- `idx_extraction_relationships_document` (`document_id`).

---

## Table: `extraction_runs`

Header row per extraction pass per document. The reproducibility audit core (`processing_config` JSONB snapshot, template/rules/schema hashes). Written by `LlmExtract::run_llm_extract`; finalised by `complete_extraction_run`.

**Primary key:** `id` (serial)
**Foreign keys:** `document_id` → `documents(id)`
**Unique constraint:** `(document_id, pass_number)` — at most one row per (doc, pass).

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | integer | NO | `nextval('extraction_runs_id_seq')` | PK. |
| 2 | `document_id` | text | NO | — | FK → `documents(id)`. |
| 3 | `pass_number` | integer | NO | — | 1 or 2. Part of `(document_id, pass_number)` unique constraint. |
| 4 | `model_name` | text | NO | — | LLM model id used. |
| 5 | `input_tokens` | integer | YES | — | Sum across chunks for this run. |
| 6 | `output_tokens` | integer | YES | — | |
| 7 | `cost_usd` | numeric(10,4) | YES | — | Computed from model rates × tokens. NULL if rates missing. |
| 8 | `raw_output` | jsonb | NO | — | Final merged entities + relationships JSONB blob (post-`ChunkMerger`). |
| 9 | `schema_version` | text | NO | — | Schema version string (e.g., `complaint/v4`). |
| 10 | `started_at` | timestamp with time zone | NO | — | |
| 11 | `completed_at` | timestamp with time zone | YES | — | NULL while RUNNING; set on COMPLETED/FAILED. |
| 12 | `status` | text | NO | `'RUNNING'::text` | UPPERCASE: `RUNNING`, `COMPLETED`, `FAILED`. |
| 13 | `assembled_prompt` | text | YES | — | The assembled prompt as sent to the LLM. Set after dispatch on the full-doc path; chunked paths leave NULL (Stored per-run, not per-chunk — audit gap §5b.1). |
| 14 | `template_name` | text | YES | — | Template filename used (e.g., `pass1_complaint_v4.md`). |
| 15 | `template_hash` | text | YES | — | SHA-256 of the template content for reproducibility (F3). |
| 16 | `rules_name` | text | YES | — | Global rules filename (e.g., `global_rules_v4.md`). NULL when profile has no rules file. |
| 17 | `rules_hash` | text | YES | — | SHA-256 of the rules content. Distinguishes "no file" (NULL) from "empty file" (hash of ""). |
| 18 | `schema_hash` | text | YES | — | SHA-256 of the schema JSON. |
| 19 | `schema_content` | jsonb | YES | — | Snapshot of the schema YAML as JSON. |
| 20 | `temperature` | double precision | YES | — | Sampling temperature actually used. |
| 21 | `max_tokens_requested` | integer | YES | — | max_tokens passed to the LLM call. |
| 22 | `admin_instructions` | text | YES | — | Per-document admin instructions injected at `{{admin_instructions}}`. |
| 23 | `prior_context` | text | YES | — | Reserved for future cross-document context renderer (currently NULL). |
| 24 | `chunk_count` | integer | YES | — | Total chunks if chunked/structured mode. NULL for full-document mode. |
| 25 | `chunks_succeeded` | integer | YES | — | |
| 26 | `chunks_failed` | integer | YES | — | |
| 27 | `chunks_pruned_nodes` | integer | YES | — | Count of nodes dropped by ChunkMerger dedup. |
| 28 | `chunks_pruned_relationships` | integer | YES | — | Count of relationship-endpoint remaps from dedup. |
| 29 | `processing_config` | jsonb | YES | — | Full resolved config snapshot (model, template, hashes, pass2 cross-doc entities, etc.). |

**Indexes:**
- `extraction_runs_pkey` (UNIQUE on `id`).
- `extraction_runs_doc_pass_unique` (UNIQUE on `(document_id, pass_number)`).
- `idx_extraction_runs_document` (`document_id`).

**Unique constraint:** `extraction_runs_doc_pass_unique` enforces one row per `(document_id, pass_number)`. The orchestrator's ON CONFLICT DO UPDATE in `insert_extraction_run` relies on this.

---

## Table: `known_users`

First-seen registry for users observed via Authentik headers. Written by `backend/src/api/pipeline/users.rs` on first request from a new user.

**Primary key:** `username`
**Foreign keys:** none

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `username` | text | NO | — | PK; Authentik username. |
| 2 | `display_name` | text | NO | `''::text` | Display name (defaults to empty). |
| 3 | `email` | text | NO | `''::text` | Email (defaults to empty). |
| 4 | `first_seen_at` | timestamp with time zone | NO | `now()` | |
| 5 | `last_seen_at` | timestamp with time zone | NO | `now()` | Updated on every request. |

**Indexes:** `known_users_pkey` (UNIQUE on `username`).

---

## Table: `llm_models`

Model catalog. Looked up by `models::list_active_models` and `get_active_model_by_id`. Drives both the pipeline extraction provider construction (`pipeline/providers.rs::provider_for_model`) and the chat provider map (`main.rs::build_chat_providers`).

**Primary key:** `id` (text — the model id, e.g., `claude-sonnet-4-6`)
**Foreign keys:** none

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | text | NO | — | PK; the canonical model id used as a foreign reference from profiles. |
| 2 | `display_name` | text | NO | — | UI label (e.g., "Claude Sonnet 4.6"). |
| 3 | `provider` | text | NO | — | `anthropic`, `vllm`, future others. |
| 4 | `api_endpoint` | text | YES | — | Optional override (for vLLM and self-hosted providers). |
| 5 | `max_context_tokens` | integer | YES | — | Provider-reported context window. |
| 6 | `max_output_tokens` | integer | YES | — | Provider-reported output cap. |
| 7 | `cost_per_input_token` | numeric(12,8) | YES | — | USD per token (input). Used by `compute_cost`. |
| 8 | `cost_per_output_token` | numeric(12,8) | YES | — | USD per token (output). |
| 9 | `is_active` | boolean | NO | `true` | Soft delete — inactive models won't appear in dropdowns. |
| 10 | `created_at` | timestamp with time zone | NO | `now()` | |
| 11 | `notes` | text | YES | — | Free-text operator notes. |

**Indexes:** `llm_models_pkey` (UNIQUE on `id`).

---

## Table: `pipeline_config`

Per-document processing configuration overrides on top of the profile YAML. The `PipelineRegistry`-resolved profile + this row are merged by `resolve_config` (`pipeline/config.rs`).

> **⚠ Schema drift warning:** the live DEV table is missing positions 2–5. The migrations dropped four columns (originally something like `pass1_model`, `pass1_template`, `pass1_schema`, `pass1_system_prompt` from the pre-consolidation era — see Notes §C-5). The remaining columns use ordinal positions 1, 6–27.

**Primary key:** `document_id` (1:1 with `documents`)
**Foreign keys:** `document_id` → `documents(id)`

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `document_id` | text | NO | — | PK and FK → `documents(id)`. 1:1 row per document. |
| 6 | `schema_file` | text | NO | — | Schema YAML filename (e.g., `complaint_v4.yaml`). |
| 7 | `admin_instructions` | text | YES | — | Per-document free-text instructions injected at `{{admin_instructions}}`. |
| 8 | `prior_context_doc_ids` | text[] (ARRAY) | YES | — | Array of doc ids whose extractions form Pass-2 cross-doc context. |
| 9 | `created_by` | text | NO | — | Username that created the config. |
| 10 | `created_at` | timestamp with time zone | NO | `now()` | |
| 11 | `step_config` | jsonb | NO | `'{}'::jsonb` | Per-step config overrides (e.g., OCR config under key `"ExtractText"`). |
| 12 | `profile_name` | text | YES | — | Profile to load from `PROCESSING_PROFILE_DIR`. NULL falls back to derived-from-schema name (`default_profile_name_from_schema`). |
| 13 | `template_file` | text | YES | — | Override for pass-1 template filename. NULL = use profile default. |
| 14 | `system_prompt_file` | text | YES | — | Override for system prompt filename. NULL = use profile default. |
| 15 | `chunking_mode` | text | YES | — | Legacy: `full` / `chunked`. New profiles use `chunking_config.mode`. Both shapes coexist; see `resolve_effective_mode`. |
| 16 | `chunk_size` | integer | YES | — | FixedSizeSplitter chunk size override. |
| 17 | `chunk_overlap` | integer | YES | — | FixedSizeSplitter overlap override. |
| 18 | `temperature` | numeric(3,2) | YES | — | LLM temperature override. |
| 19 | `run_pass2` | boolean | YES | — | Enable Pass-2 (synthesis) extraction. |
| 20 | `extraction_model` | text | YES | — | Pass-1 model override (FK-like to `llm_models.id`). |
| 21 | `max_tokens` | integer | YES | — | LLM `max_tokens` override. |
| 22 | `pass2_extraction_model` | text | YES | — | Pass-2 model override. |
| 23 | `chunking_config` | jsonb | YES | — | New chunking-strategy config (key/values consumed by `StructureAwareSplitter`). |
| 24 | `context_config` | jsonb | YES | — | Cross-doc context renderer config (reserved). |
| 25 | `pass2_template_file` | text | YES | — | Per-document override for the Pass 2 (synthesis) template filename. NULL means use the profile default. Mirrors the `pass2_extraction_model` override pattern. **(column comment is in the live DB)** |
| 26 | `auto_approve_grounded` | boolean | YES | — | Whether grounded items auto-approve (AutoApprove threshold gate). |
| 27 | `global_rules_file` | text | YES | — | Filename of the global-rules fragment injected at `{{global_rules}}`. |

**Indexes:** `pipeline_config_pkey` (UNIQUE on `document_id`).

---

## Table: `pipeline_events`

Append-only audit log for the colossus-pipeline framework. ON DELETE CASCADE when the parent job is deleted.

**Primary key:** `id` (bigserial)
**Foreign keys:** `job_id` → `pipeline_jobs(id)` ON DELETE CASCADE

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | bigint | NO | `nextval('pipeline_events_id_seq')` | PK. |
| 2 | `job_id` | uuid | NO | — | FK → `pipeline_jobs(id)`. CASCADE delete. |
| 3 | `step` | text | NO | — | Step name at the time of the event (e.g., `LlmExtract`). |
| 4 | `event_type` | text | NO | — | Event category (e.g., `transition`, `progress`, `error`). |
| 5 | `message` | text | NO | — | Human-readable message. |
| 6 | `details` | jsonb | YES | — | Structured payload (optional). |
| 7 | `created_at` | timestamp with time zone | NO | `now()` | |

**Indexes:**
- `pipeline_events_pkey` (UNIQUE on `id`).
- `idx_pipeline_events_job_timeline` (`job_id`, `created_at`) — timeline scan.

---

## Table: `pipeline_jobs`

The state machine for the colossus-pipeline framework. Each job tracks one document-processing FSM execution. The `colossus-pipeline` crate (external workspace) is the canonical writer; `process.rs` (this repo) submits and deletes failed rows.

**Primary key:** `id` (UUID)
**Foreign keys:** none (it's the parent; events FK in)

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | uuid | NO | — | PK. UUID v7 (time-ordered) generated by the framework. |
| 2 | `job_type` | text | NO | — | Job-type discriminator. For document processing: `JOB_TYPE_DOCUMENT_PROCESSING = "document_processing"` (`pipeline/constants.rs`). |
| 3 | `job_key` | text | NO | — | Application-level key (for document processing: the `document_id`). |
| 4 | `pipeline_version` | integer | NO | `1` | Pipeline-version discriminator (forward-compat). |
| 5 | `status` | text | NO | `'ready'::text` | `ready`, `running`, `completed`, `failed`, `cancelled`. (Lowercase — distinct from `extraction_runs.status` which is uppercase.) |
| 6 | `control` | text | NO | `'none'::text` | `none`, `cancel`, `delete`. The signal the worker reads. |
| 7 | `current_step` | text | NO | — | Step name FSM is at. |
| 8 | `step_data` | jsonb | NO | `'{}'::jsonb` | Serialised `DocProcessing` enum body for the current step. |
| 9 | `result` | jsonb | NO | `'{}'::jsonb` | Last step's result payload. |
| 10 | `tried` | integer | NO | `0` | Attempts so far. |
| 11 | `max_retries` | integer | NO | `0` | **Default 0 means no auto-retry** unless the step trait's DEFAULT_RETRY_LIMIT overrides it. See audit v1 §4a.1. |
| 12 | `retry_delay_secs` | integer | NO | `0` | Backoff between retries. |
| 13 | `priority` | integer | NO | `0` | Higher = pulled first. Complaints use `PRIORITY_COMPLAINT = 10`. |
| 14 | `wakeup_at` | timestamp with time zone | NO | `now()` | Earliest time the framework will poll this row. |
| 15 | `step_started_at` | timestamp with time zone | YES | — | When the current step started. |
| 16 | `step_completed_at` | timestamp with time zone | YES | — | When the current step completed (NULL while running). |
| 17 | `timeout_at` | timestamp with time zone | YES | — | When the step's timeout fires (NULL → no timeout). |
| 18 | `worker_id` | text | YES | — | ID of the worker that leased this row. |
| 19 | `last_heartbeat_at` | timestamp with time zone | YES | — | Heartbeat for zombie detection. |
| 20 | `progress` | jsonb | YES | — | Live progress payload (chunks done, percent, rate-limit status). |
| 21 | `error` | text | YES | — | Last error message (Display of the step's typed error). |
| 22 | `created_by` | text | YES | — | Username that submitted the job. |
| 23 | `created_at` | timestamp with time zone | NO | `now()` | |
| 24 | `updated_at` | timestamp with time zone | NO | `now()` | |
| 25 | `completed_at` | timestamp with time zone | YES | — | Set on terminal status. |

**Indexes:**
- `pipeline_jobs_pkey` (UNIQUE on `id`).
- `idx_pipeline_jobs_ready` partial: `(priority DESC, wakeup_at)` WHERE `status='ready' AND control='none'`. Drives the worker poll query.
- `idx_pipeline_jobs_key`: `(job_type, job_key)`.
- `idx_pipeline_jobs_unique_active` partial UNIQUE: `(job_type, job_key)` WHERE `status NOT IN ('completed','cancelled')`. Prevents two active jobs on the same document.
- `idx_pipeline_jobs_running_timeout` partial: `(timeout_at)` WHERE `status='running' AND timeout_at IS NOT NULL`. Zombie/timeout sweep.
- `idx_pipeline_jobs_running_heartbeat` partial: `(last_heartbeat_at)` WHERE `status='running'`. Zombie/heartbeat sweep.

**Triggers (on `pipeline_jobs`):** see [Triggers](#triggers) section.

---

## Table: `pipeline_steps`

Per-step execution log (legacy / transitional). Pre-Phase-4 surface for the Execution History panel. The framework's `pipeline_events` is the new canonical record; `pipeline_steps` is preserved for the UI's `ExecutionHistory.tsx`.

**Primary key:** `id` (serial)
**Foreign keys:** `document_id` → `documents(id)`

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | integer | NO | `nextval('pipeline_steps_id_seq')` | PK. |
| 2 | `document_id` | text | NO | — | FK → `documents(id)`. |
| 3 | `step_name` | text | NO | — | E.g., `ExtractText`, `LlmExtract`, `Verify`, `AutoApprove`, `Ingest`, `Index`, `Completeness`. |
| 4 | `status` | text | NO | `'running'::text` | Lowercase: `running`, `completed`, `failed`. |
| 5 | `started_at` | timestamp with time zone | NO | `now()` | |
| 6 | `completed_at` | timestamp with time zone | YES | — | |
| 7 | `duration_secs` | double precision | YES | — | |
| 8 | `triggered_by` | text | YES | — | Username (or `worker` for FSM-driven steps). |
| 9 | `input_params` | jsonb | YES | `'{}'::jsonb` | Step input snapshot. |
| 10 | `result_summary` | jsonb | YES | `'{}'::jsonb` | Step output summary (counts, status). Surfaced by the UI. |
| 11 | `error_message` | text | YES | — | Step-level error. |
| 12 | `created_at` | timestamp with time zone | YES | `now()` | |

**Indexes:**
- `pipeline_steps_pkey` (UNIQUE on `id`).
- `idx_pipeline_steps_document` (`document_id`).
- `idx_pipeline_steps_status` (`status`, `started_at`).
- `idx_pipeline_steps_step` (`step_name`).

---

## Table: `rag_config`

Generic key/value JSONB store for RAG-pipeline configuration (`/ask`, decomposer, synthesizer settings). Single-row-per-key.

**Primary key:** `id` (serial)
**Foreign keys:** none
**Unique constraint:** `config_key`

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | integer | NO | `nextval('rag_config_id_seq')` | PK. |
| 2 | `config_key` | text | NO | — | UNIQUE. Logical config name (e.g., `rag.decomposer.model`). |
| 3 | `config_value` | jsonb | NO | — | Arbitrary JSONB value. |
| 4 | `updated_at` | timestamp with time zone | NO | `now()` | |
| 5 | `updated_by` | text | YES | — | Username (NULL on automated updates). |

**Indexes:**
- `rag_config_pkey` (UNIQUE on `id`).
- `rag_config_config_key_key` (UNIQUE on `config_key`).

---

## Table: `review_edit_history`

Audit trail of every edit a reviewer makes to an `extraction_items` row. Backed by the Review panel "Save" action.

**Primary key:** `id` (serial)
**Foreign keys:** `item_id` → `extraction_items(id)`

| Pos | Column | Type | Nullable | Default | Notes |
|-----|--------|------|----------|---------|-------|
| 1 | `id` | integer | NO | `nextval('review_edit_history_id_seq')` | PK. |
| 2 | `item_id` | integer | NO | — | FK → `extraction_items(id)`. |
| 3 | `field_changed` | text | NO | — | Field name (e.g., `item_data.label`, `verbatim_quote`). |
| 4 | `old_value` | text | YES | — | Previous value (NULL if new field). |
| 5 | `new_value` | text | YES | — | New value (NULL on delete-of-field semantics). |
| 6 | `changed_by` | text | NO | — | Username. |
| 7 | `changed_at` | timestamp with time zone | NO | `now()` | |

**Indexes:**
- `review_edit_history_pkey` (UNIQUE on `id`).
- `idx_review_edit_history_item` (`item_id`).

---

## Relationship Diagram (Foreign Keys)

13 foreign keys total. `documents` is the central hub; `extraction_runs` is a secondary hub.

```
                                       ┌─────────────────┐
                                       │   documents     │
                                       │ PK: id (text)   │
                                       └────────┬────────┘
                                                │
        ┌──────────────────┬──────────────┬─────┴──────┬──────────────────┬─────────────────────┐
        │                  │              │            │                  │                     │
        ▼                  ▼              ▼            ▼                  ▼                     ▼
┌──────────────────┐ ┌────────────┐ ┌──────────┐ ┌──────────────┐ ┌────────────────┐ ┌──────────────────────────┐
│ extraction_runs  │ │ pipeline_  │ │ document │ │ extraction_  │ │ pipeline_steps │ │ extraction_relationships │
│ FK: document_id  │ │ config     │ │ _text    │ │ items        │ │ FK: document_id│ │ FK: document_id          │
│ UNIQ:            │ │ PK & FK:   │ │ FK:      │ │ FK:          │ │                │ │ FK: run_id               │
│  (doc, pass)     │ │  doc_id    │ │  doc_id  │ │  doc_id      │ │                │ │ FK: from_item_id         │
└────────┬─────────┘ └────────────┘ └──────────┘ │ FK: run_id   │ └────────────────┘ │ FK: to_item_id           │
         │                                       └──────┬───────┘                    └──────────────────────────┘
         │                                              │
         │ ON DELETE CASCADE                            │
         ▼                                              │
┌──────────────────┐                                    │
│ extraction_      │                                    ▼
│   chunks         │                          ┌──────────────────────┐
│ FK:              │                          │ review_edit_history  │
│  extraction_     │                          │ FK: item_id          │
│  run_id          │                          └──────────────────────┘
└──────────────────┘

                            ┌───────────────────┐
                            │   pipeline_jobs   │
                            │ PK: id (uuid)     │
                            └─────────┬─────────┘
                                      │ ON DELETE CASCADE
                                      ▼
                            ┌───────────────────┐
                            │  pipeline_events  │
                            │ FK: job_id        │
                            └───────────────────┘

  No FK (orphaned by design):
  - known_users       (PK: username)
  - llm_models        (PK: id; profile YAML keys reference llm_models.id but no DB-level FK)
  - rag_config        (PK: id; UNIQUE config_key)
  - document_audit_log (text document_id is unconstrained — survives doc deletion)
  - _sqlx_migrations  (framework table)
```

### Full FK list (psql output)

```
document_text.document_id              → documents.id                (NO ACTION / NO ACTION)
extraction_chunks.extraction_run_id    → extraction_runs.id          (NO ACTION / CASCADE)
extraction_items.document_id           → documents.id                (NO ACTION / NO ACTION)
extraction_items.run_id                → extraction_runs.id          (NO ACTION / NO ACTION)
extraction_relationships.document_id   → documents.id                (NO ACTION / NO ACTION)
extraction_relationships.from_item_id  → extraction_items.id         (NO ACTION / NO ACTION)
extraction_relationships.run_id        → extraction_runs.id          (NO ACTION / NO ACTION)
extraction_relationships.to_item_id    → extraction_items.id         (NO ACTION / NO ACTION)
extraction_runs.document_id            → documents.id                (NO ACTION / NO ACTION)
pipeline_config.document_id            → documents.id                (NO ACTION / NO ACTION)
pipeline_events.job_id                 → pipeline_jobs.id            (NO ACTION / CASCADE)
pipeline_steps.document_id             → documents.id                (NO ACTION / NO ACTION)
review_edit_history.item_id            → extraction_items.id         (NO ACTION / NO ACTION)
```

Two of 13 FKs cascade on delete:
- `extraction_chunks` cascades from `extraction_runs`.
- `pipeline_events` cascades from `pipeline_jobs`.

The other 11 use `ON DELETE NO ACTION`, meaning a `documents` row deletion will be refused if any of its child rows still exist. The application's `cleanup_all` (`pipeline/steps/cleanup.rs`) issues explicit DELETEs in FK-safe order rather than relying on cascade.

---

## All Indexes (Full List)

| Table | Index | Definition |
|-------|-------|------------|
| `_sqlx_migrations` | `_sqlx_migrations_pkey` | UNIQUE btree(`version`) |
| `document_audit_log` | `document_audit_log_pkey` | UNIQUE btree(`id`) |
| `document_audit_log` | `idx_audit_log_action` | btree(`action`) |
| `document_audit_log` | `idx_audit_log_document` | btree(`document_id`) |
| `document_text` | `document_text_pkey` | UNIQUE btree(`document_id`, `page_number`) |
| `documents` | `documents_pkey` | UNIQUE btree(`id`) |
| `documents` | `idx_documents_status` | btree(`status`) |
| `documents` | `idx_documents_type` | btree(`document_type`) |
| `extraction_chunks` | `extraction_chunks_pkey` | UNIQUE btree(`id`) |
| `extraction_chunks` | `idx_extraction_chunks_run` | btree(`extraction_run_id`) |
| `extraction_items` | `extraction_items_pkey` | UNIQUE btree(`id`) |
| `extraction_items` | `idx_extraction_items_document` | btree(`document_id`) |
| `extraction_items` | `idx_extraction_items_review` | btree(`review_status`) |
| `extraction_items` | `idx_extraction_items_run` | btree(`run_id`) |
| `extraction_relationships` | `extraction_relationships_pkey` | UNIQUE btree(`id`) |
| `extraction_relationships` | `idx_extraction_relationships_document` | btree(`document_id`) |
| `extraction_relationships` | `idx_extraction_relationships_run` | btree(`run_id`) |
| `extraction_runs` | `extraction_runs_doc_pass_unique` | UNIQUE btree(`document_id`, `pass_number`) |
| `extraction_runs` | `extraction_runs_pkey` | UNIQUE btree(`id`) |
| `extraction_runs` | `idx_extraction_runs_document` | btree(`document_id`) |
| `known_users` | `known_users_pkey` | UNIQUE btree(`username`) |
| `llm_models` | `llm_models_pkey` | UNIQUE btree(`id`) |
| `pipeline_config` | `pipeline_config_pkey` | UNIQUE btree(`document_id`) |
| `pipeline_events` | `idx_pipeline_events_job_timeline` | btree(`job_id`, `created_at`) |
| `pipeline_events` | `pipeline_events_pkey` | UNIQUE btree(`id`) |
| `pipeline_jobs` | `idx_pipeline_jobs_key` | btree(`job_type`, `job_key`) |
| `pipeline_jobs` | `idx_pipeline_jobs_ready` | btree(`priority DESC`, `wakeup_at`) WHERE `status='ready' AND control='none'` |
| `pipeline_jobs` | `idx_pipeline_jobs_running_heartbeat` | btree(`last_heartbeat_at`) WHERE `status='running'` |
| `pipeline_jobs` | `idx_pipeline_jobs_running_timeout` | btree(`timeout_at`) WHERE `status='running' AND timeout_at IS NOT NULL` |
| `pipeline_jobs` | `idx_pipeline_jobs_unique_active` | UNIQUE btree(`job_type`, `job_key`) WHERE `status <> ALL (ARRAY['completed','cancelled'])` |
| `pipeline_jobs` | `pipeline_jobs_pkey` | UNIQUE btree(`id`) |
| `pipeline_steps` | `idx_pipeline_steps_document` | btree(`document_id`) |
| `pipeline_steps` | `idx_pipeline_steps_status` | btree(`status`, `started_at`) |
| `pipeline_steps` | `idx_pipeline_steps_step` | btree(`step_name`) |
| `pipeline_steps` | `pipeline_steps_pkey` | UNIQUE btree(`id`) |
| `rag_config` | `rag_config_config_key_key` | UNIQUE btree(`config_key`) |
| `rag_config` | `rag_config_pkey` | UNIQUE btree(`id`) |
| `review_edit_history` | `idx_review_edit_history_item` | btree(`item_id`) |
| `review_edit_history` | `review_edit_history_pkey` | UNIQUE btree(`id`) |

---

## Triggers

All three triggers are defined on `pipeline_jobs`:

| Trigger | Event | Function | Purpose |
|---------|-------|----------|---------|
| `pipeline_jobs_changed` | AFTER INSERT | `pipeline_jobs_notify()` | `pg_notify('pipeline_jobs_changed', NEW.id::text)` so workers using `LISTEN` wake immediately. Channel matches `DEFAULT_NOTIFY_CHANNEL` in the colossus-pipeline crate's `worker/config.rs`. |
| `pipeline_jobs_changed` | AFTER UPDATE | `pipeline_jobs_notify()` | Same. |
| `pipeline_jobs_sync_document_status` | AFTER UPDATE | `sync_document_status_from_pipeline_job()` | Projects terminal `pipeline_jobs.status` onto `documents.status` (migration `20260422112238`). This is how the UI's `documents.status` reflects pipeline-job completion without explicit handler writes. |

---

## Notes on Unused / Redundant / Drifted Columns

### §C-1 — Migration `20260513` not yet applied to DEV

The local repo's `backend/pipeline_migrations/` contains `20260513_consolidate_model_columns_and_add_overrides.sql`, but the latest applied version on DEV is `20260509162937`. When that migration runs on DEV next, it may rename or drop columns referenced in this document. **Re-generate the doc after the next deploy.**

### §C-2 — `document_audit_log.document_id` has no FK

This is intentional. The audit log must survive deletion of the underlying document so that delete actions remain traceable. There is therefore no FK on `document_id`; queries that need to display the document's current state must outer-join.

### §C-3 — `document_text` does not record OCR vs native source per page

Per page, the table stores only `text_content`. Whether that came from native PDF text extraction or from OCR is not stored — visible only via tracing logs at extraction time. The audit report `PIPELINE_RESILIENCE_AUDIT_v1.md` §5c.9 flags this as observability gap H23.

### §C-4 — `documents.error_suggestion` never written

The column exists and the UI (`frontend/src/components/pipeline/ProcessingPanel.tsx:383–385`) renders it as a "Suggestion:" block when non-empty. **No backend code writes to this column** (audit v1 §10.1, finding C1). It is dead schema surface — a candidate for either a real writer or a drop migration.

### §C-5 — `pipeline_config` ordinal-position gap (2–5)

Live DEV shows columns at ordinal positions 1 and 6–27 — positions 2–5 are absent because four columns were dropped at some point in the migration history. The most likely candidates (based on the audit report's earlier note about `pass1_model vs extraction_model` consolidation) are pre-consolidation pass-1 columns. The pending migration `20260513_consolidate_model_columns_and_add_overrides.sql` is part of this consolidation track. The current live shape is intentional but the ordinal gaps are visible if anyone runs `\d+ pipeline_config`.

### §C-6 — `extraction_runs.assembled_prompt` only set for full-document mode

`backend/src/pipeline/steps/llm_extract.rs:732–743` writes the assembled prompt to this column **only on the `chunking_mode = "full"` path**. The chunked and structured paths assemble a different prompt per chunk and never set this run-level column. Operators querying the column expecting "the prompt used" will see NULL for any chunked-mode run.

### §C-7 — `documents.processing_step` is stale projection

The backend writes this column from each step's progress updates, but the canonical current step lives in `pipeline_jobs.current_step`. UI code that polls `documents.processing_step` (audit v1 §11.2) reads stale data — flagged as MEDIUM gap.

### §C-8 — `extraction_chunks` has no `raw_response` column

The chunk-loop in `LlmExtract` discards the raw LLM response after parsing — it is not persisted anywhere. Audit v1 finding C6 (HIGH-severity): adding a `raw_response text` column to this table (and writing to it) would close the "no forensic trail when parse fails" gap.

### §C-9 — `pipeline_jobs.max_retries` defaults to 0

Per the migration's `INT NOT NULL DEFAULT 0`, a freshly-inserted job has zero auto-retries. The colossus-pipeline framework consults each Step's `DEFAULT_RETRY_LIMIT` const at submit time — but only four of the eight DocProcessing steps declare one (`Ingest`, `Index`, `Verify`, `Completeness`). `ExtractText`, `LlmExtract`, `AutoApprove`, `LlmExtractPass2` rely on trait defaults — possibly zero. Audit v1 §4a.1, §4a.2 (HIGH-severity).

### §C-10 — `extraction_relationships.tier` vs `extraction_runs.pass_number`

Two columns encode the same pass discriminator at different granularities:
- `extraction_runs.pass_number` → 1 or 2 at run-header level.
- `extraction_relationships.tier` → 1 or 2 per relationship row.

Both are correct as designed (relationships from a pass-1 run inherit `tier=1`, etc.), but joins must use `extraction_relationships.run_id → extraction_runs.id → pass_number` rather than `tier` if any future migration could decouple them.

### §C-11 — `extraction_runs.raw_output` and `extraction_runs.processing_config` are both JSONB on the same row

`raw_output` is the final merged entities+relationships JSON (post-`ChunkMerger`). `processing_config` is the resolved-config snapshot (model, template, hashes, etc.). They serve different purposes but live on the same row and are both NOT NULL / NOT NULL respectively (raw_output is NOT NULL; processing_config is nullable best-effort). A future audit that wants "the exact result + the exact config used" can pull both from one SELECT.

### §C-12 — `llm_models.id` is text-keyed but other tables don't FK it

Profiles' `extraction_model` / `pass2_extraction_model` values name a `llm_models.id`, and so does `extraction_runs.model_name`. None of those columns has a foreign key, so deleting a row in `llm_models` (or even setting `is_active = false`) leaves prior runs referencing the now-orphan id. The application uses `is_active` as a soft-delete signal — code paths that look up a model by id should accept stale references in historical rows.

### §C-13 — `known_users` carries no FK to anything

By design (Authentik is the source of truth for users; this table is just a first-seen registry). Columns that store usernames (`documents.assigned_reviewer`, `extraction_items.reviewed_by`, etc.) do not FK into `known_users` because a user may act before being recorded here.

### §C-14 — `rag_config` is a generic kv store

Stored as `(config_key text UNIQUE, config_value jsonb)` — no per-key schema enforcement. Callers must validate the shape of `config_value` at read time. This is a deliberate trade-off (low-friction add of new config keys) but means a malformed value can only be caught at the application layer.

— end of schema reference —
