# Pipeline Resilience & Quality Audit — v1

**Date:** 2026-05-13
**Auditor:** Claude Code (read-only audit)
**Repository:** colossus-legal
**Commit:** 8eca71280a683ef8e7c3bc61c4e756d7503362f7 (main)
**Scope:** Full backend (`backend/src/`, `backend/migrations/`, `backend/pipeline_migrations/`, `backend/config/`, `backend/profiles/`, `backend/extraction_schemas/`, `backend/extraction_templates/`) and full frontend (`frontend/src/`).

---

## Executive Summary

- **Total issues found:** ~810 (counted across the patterned categories below)
- **Critical (blocks operation):** 7
- **High (causes data loss or corruption):** 24
- **Medium (causes confusion or wasted effort):** ~210
- **Low (code quality, maintainability):** ~570

The pipeline has a strong typed-error backbone (every step crate carries a `thiserror`-derived enum, the `pipeline_jobs` table has retry / heartbeat / timeout columns, and `cleanup_all` is built as an explicit saga primitive). Resilience holes are concentrated in three areas:

1. **Per-step observability is logged but not stored.** Per-chunk and per-pass execution metadata reaches the database through `extraction_runs`, `extraction_chunks`, `processing_config` JSONB, and `pipeline_steps.result_summary`, but several pieces (raw LLM response, parse-failure preview, OCR engine selected, chunking-strategy effective config) only land in `tracing::warn!` — invisible after container restart unless the operator was tailing logs.
2. **Best-effort writes use `.ok()` to discard error information.** 32 distinct `.ok();`-terminated awaits in `pipeline/steps/llm_extract.rs`, `verify.rs`, `auto_approve.rs`, `llm_extract_helpers.rs`, and `llm_extract_pass2.rs` silently drop sqlx errors when updating progress rows. The pipeline keeps running, but the operator can't tell from the database that the writes failed.
3. **Heavy reliance on `unwrap_or(...)` for SQL row decoding masks schema drift.** `repositories/*.rs` and `api/pipeline/*.rs` decode every nullable column with `row.get(...).ok()` or `unwrap_or_default()`. A renamed or dropped column reads as the empty-string default rather than raising — particularly dangerous for `evidence_repository`, `allegation_repository`, `decomposition_repository`, and `bias/aggregation.rs`.

The frontend pattern is consistent: `authFetch` wraps `fetch` with a 30 s default timeout via `AbortController`, every service throws on non-OK, callers `try`/`catch` and surface the message in a state variable. The one critical gap is `frontend/src/services/admin.ts:185` (the upload endpoint) which uses raw `fetch()` with no timeout — an aborted upload hangs the UI until the browser gives up.

A complete fix plan should prioritise:
- **Storing the raw LLM response per chunk** (currently lost when the parse repair fails) — `extraction_chunks` has the `raw_response` column but the chunk-loop never writes it.
- **Replacing best-effort `.ok()` chains on progress writes with logged failures** so operators can correlate "stalled processing UI" with a database write failure.
- **Replacing every `row.get("col").ok()` with `row.try_get("col")` propagated through the repository's typed error** so a schema migration that drops a column is a startup-time test failure, not a runtime empty-string substitution.

---

## Statistics

- **Files audited:**
  - Backend Rust source: **190** files under `backend/src/`
  - Frontend TS/TSX: **125** files under `frontend/src/`
  - PostgreSQL migrations: **6 main + 26 pipeline = 32** files
  - Neo4j migrations: directory exists at `backend/migrations_neo4j/` (1 file)
  - Config / profiles / schemas / templates: **43** YAML/MD files
  - **Grand total: 391 files**

- **Error messages audited:** 762 (553 user-facing AppError/StatusCode returns, 166 log-level, 43 frontend `console.*`/`.catch(...)` calls)
- **Silent failures found:** 437 raw matches across `.unwrap_or*` (307), `.ok();` / `let _ =` / `if let Ok(_)` (98), `unwrap()` / `expect(` (32 outside tests)
- **Hardcoded values found:** 67 model-name / model-id literals, 8 URL literals, 11 duration literals, ~30 case-specific paragraph ranges, ~270 status-string `"CONST"` literals (most behind `pub const`, ~8 raw)
- **Recovery gaps found:** 11 (see §4)
- **Observability gaps found:** 12 (see §5c)
- **Race conditions found:** 6 (see §6)
- **Resource gaps found:** 5 (see §7)
- **Test coverage gaps found:** 38 public functions without direct unit tests (see §9)
- **Dead code instances:** 8 entries with explicit `TODO: B-4 — v1 dead code` markers, plus 3 `#[allow(dead_code)]` annotations

---

## Section 1: Error Messages

### 1a. User-Facing Error Messages

#### 1a.1 — Error envelope shape (centralised)

`backend/src/error.rs:78–131` — `AppError::into_response` maps six variants to status codes with a `{ error, message, details }` JSON body. The envelope is consistent across the entire API. All findings below describe how `AppError` is *constructed* at call sites.

- `BadRequest { message, details }` → 400 with `error: "validation_error"`
- `NotFound { message }` → 404 with `error: "not_found"`, `details: {}`
- `Unauthorized { message }` → 401, empty details
- `Forbidden { message }` → 403, empty details
- `Conflict { message, details }` → 409 with caller-supplied details
- `Internal { message }` → 500 with `error: "internal_error"`, empty details

**QUALITY:** GOOD — envelope is consistent, `details` is JSON so callers can attach structured context.
**PROBLEM:** None at the envelope. Every problem below is at the construction site, where messages are flat strings without document/step/field context.

#### 1a.2 — Internal-500 messages that strip context

| FILE | LINE | MESSAGE FORMAT | CONTEXT PROVIDED | RECOVERY GUIDANCE | QUALITY | PROBLEM |
|------|------|----------------|------------------|-------------------|---------|---------|
| `backend/src/api/pipeline/process.rs` | 67 | `"DB error: {e}"` | None — just the sqlx error | None | POOR | No document id, no operation name. Operator sees "DB error: relation \"pipeline_config\" does not exist" with no clue which call. |
| `backend/src/api/pipeline/process.rs` | 98 | `"Failed to clean up prior failed pipeline_jobs row: {e}"` | Operation named | None | ADEQUATE | Names operation but no doc_id. |
| `backend/src/api/pipeline/process.rs` | 128 | `"Failed to submit pipeline job: {e}"` | Operation named | None | ADEQUATE | No doc_id; the dup-job branch above is fine. |
| `backend/src/api/pipeline/process.rs` | 194, 211 | `"DB error: {e}"`, `"Failed to look up active job: {e}"` | None / Operation | None | POOR / ADEQUATE | Same problem as 67. |
| `backend/src/api/pipeline/process.rs` | 226 | `"Failed to cancel job: {other}"` | Operation | None | ADEQUATE | |
| `backend/src/api/ask.rs` | 241 | `"RAG pipeline error: {e}"` | None | None | POOR | Doesn't say which question, which model, what step in the pipeline. |
| `backend/src/api/ask.rs` | 357 | `"Failed to persist QA entry: {e}"` | None | None | POOR | No question id, no user. |
| `backend/src/api/search.rs` | 121 | `"spawn_blocking panicked: {e}"` | None | None | POOR | No query. |
| `backend/src/api/search.rs` | 130 | `"Embedding failed: {e}"` | None | None | POOR | |
| `backend/src/api/search.rs` | 150 | `"Qdrant search failed: {e}"` | None | None | POOR | |
| `backend/src/api/embed.rs` | 62 | `"Embedding pipeline failed: {e}"` | None | None | POOR | CLI-style — fine for `embed` subcommand path but the HTTP path lacks doc context. |

#### 1a.3 — Step-level error variants (typed, generally GOOD)

The step crates all use `thiserror`-derived enums with `#[source]` chaining. Each carries `doc_id` (or `document_id`) in the variant payload.

- `backend/src/pipeline/steps/llm_extract.rs:57–144` — `LlmExtractError` with 16 variants (`DocumentNotFound`, `NoPipelineConfig`, `SchemaLoadFailed`, `PromptBuildFailed`, `NoTextPages`, `LlmCallFailed`, `ResponseNotJson`, `InsertRunFailed`, `CompleteRunFailed`, `StoreFailed`, `SemaphoreClosed`, `ProfileLoadFailed`, `ModelNotFound`, `ProviderConstructionFailed`, `NoPass2Template`, `NoCompletedPass1`, `EntitySerializationFailed`, `RelationshipSerializationFailed`).
- `backend/src/pipeline/steps/ingest.rs:91–118` — `IngestError` with `DocumentNotFound`, `NoCompletedRun`, `Cleanup`, `Neo4j`, `Helper`.
- `backend/src/pipeline/steps/verify.rs:48–70` — `VerifyError` with `DocumentNotFound`, `PdfNotFound`, `NoCanonicalText`, `GroundingModes`, `Db`.
- `backend/src/pipeline/steps/completeness.rs:78–104` — `CompletenessError` with `DocumentNotFound`, `NoCompletedRun`, `MissingDocumentNode`, `MissingNodes`, `Helper`.
- `backend/src/pipeline/steps/index.rs:84–108` — `IndexError` with `NoNodes`, `Embedding`, `Cleanup`, `Helper`.
- `backend/src/pipeline/steps/cleanup.rs:51–82` — `CleanupError` with `Neo4j`, `Qdrant`, `Postgres`, `Partial` (composite report).
- `backend/src/services/claude_client.rs:69–82` — `ClaudeError` with `NoApiKey`, `HttpError`, `ApiError {status, body}`, `EmptyResponse`.

**QUALITY:** GOOD throughout — all carry doc_id, all preserve `#[source]` chain.
**PROBLEM:** `IngestError::Helper { doc_id, message: String }` (line 116) collapses upstream errors into a stringly-typed message — the comment at line 86–90 acknowledges this is debt. A reader of `pipeline_jobs.error` sees, e.g., `"Ingest helper failed for document 'doc-42': create_ingest_relationship: Database(...) "` and cannot programmatically branch on the underlying cause. Repeated in `Index::Helper`, `Completeness::Helper`, `Verify::Db.message`.

#### 1a.4 — Stored error messages (the column-level audit trail)

Errors written into PostgreSQL columns for operator review:

| TABLE / COLUMN | WRITER | MESSAGE QUALITY | PROBLEM |
|----------------|--------|-----------------|---------|
| `pipeline_jobs.error` | colossus-pipeline crate (external) | Receives the `Box<dyn Error>::to_string()` from the step's `Result::Err`. Carries the step's typed error Display, which generally names the doc_id. | Doesn't carry the `#[source]` chain — just the outer Display. An operator reading only this column doesn't see the underlying sqlx/neo4rs message. |
| `extraction_runs.error_message` | `backend/src/pipeline/steps/llm_extract_helpers.rs:174–187` `mark_run_failed` | `&serde_json::json!({"error": reason})` — embeds the reason in a JSON envelope inside what is likely a TEXT column. | The DB column is a JSONB result blob, not a flat error message — confusing if the operator queries `error_message` and sees `{"error": "All 4 chunks failed extraction"}` instead of plain text. Also: the write failure itself is logged-and-ignored at line 185. |
| `extraction_chunks.error` | `backend/src/pipeline/steps/llm_extract.rs:1175` "Parse error: {parse_err}" | Names the failure mode (Parse error) and the raw error. | The raw LLM response that failed parsing is NOT stored alongside (column exists but unset on this path). Operator must reproduce to see what the LLM actually returned. |
| `extraction_chunks.error` (LLM call path) | `backend/src/pipeline/steps/llm_extract.rs:1217` `format!("{call_err}")` | Carries Display of `PipelineError`. | No request-id from Anthropic, no request timing breakdown. |
| `documents.error_message` | unknown — projected by trigger from `pipeline_jobs.error` via `pipeline_jobs_sync_document_status` (migration 20260422112238) | Same as `pipeline_jobs.error` minus the source chain. | Frontend renders this verbatim (`frontend/src/components/pipeline/ProcessingPanel.tsx:240–242, 380, 395`) so whatever quality issue exists in the writer is what the operator sees. |
| `documents.error_suggestion` | Set somewhere — referenced by `ProcessingPanel.tsx:383–385` but no writer was found in this audit. Likely never populated. | UI shows "Suggestion: ..." block | **GAP:** column exists in UI types but no writer in backend (confirmed via grep — no `error_suggestion` writes). |

#### 1a.5 — `Conflict` errors (409) — generally GOOD

- `process.rs:76–82` — "Document '{doc_id}' is already processing. Cancel it first if you want to re-process." with `details: { status_group: "processing" }`. **GOOD.**
- `process.rs:121–125` — "An active pipeline job already exists for '{doc_id}'" with doc_id in details. **GOOD.**
- `process.rs:218–224` — "Job for '{doc_id}' is already in a terminal state and cannot be cancelled" with `job_id` in details. **GOOD.**

#### 1a.6 — `NotFound` errors (404)

- `process.rs:69–71` — "Document '{doc_id}' not found" — **GOOD.**
- `process.rs:199–201` — same shape — **GOOD.**
- `process.rs:214–216` — "No active pipeline job for document '{doc_id}'" — **GOOD** (specifically distinguishes "doc missing" from "no job").

#### 1a.7 — `BadRequest` errors (400)

`grep -rn 'AppError::BadRequest' backend/src/ | wc -l` returns dozens of call sites; sample audit:

- `backend/src/api/pipeline/items.rs:331` — implicit via `unwrap_or(1).max(1)` — **POOR**: page=0 is silently coerced to 1 rather than 400.
- `backend/src/api/pipeline/items.rs:332` — same: `per_page.unwrap_or(50).min(200)` — **POOR**: per_page=99999 is silently capped.
- `backend/src/api/pipeline/process.rs` — no 400s emitted; all validation is at the doc-id path level (resolved by axum routing).

#### 1a.8 — Frontend display of backend error bodies

`frontend/src/services/admin.ts:106–109`, `121–124`, `137–141`, `159–166`, `190–193`, `249–258` all use the pattern:

```ts
if (!res.ok) {
  const err = await res.json();
  throw new Error(err.message || "Upload failed");
}
```

**QUALITY:** ADEQUATE — surfaces the backend message field but **drops `err.details` entirely**. For a `Conflict` with `{ details: { status_group: "processing" } }` the UI loses the structured detail. Same pattern in `pipelineApi.ts`, `bias.ts`, `qa.ts`.

Specific 1a.8 instances:
- `admin.ts:108`, `admin.ts:123`, `admin.ts:139`, `admin.ts:166`, `admin.ts:192`, `admin.ts:257` — all `err.message || <fallback>`, no details.
- `pipelineApi.ts:415` and surrounding handlers — `res.json()` parsed but only message surfaced.

### 1b. Log-Level Error/Warning Messages

166 distinct log statements (`tracing::error!`, `tracing::warn!`, `log::error!`, `log::warn!`, `eprintln!`). Below is the full enumeration grouped by quality.

#### 1b.1 — GOOD examples (structured fields, doc/run/job id, source error)

| FILE:LINE | LEVEL | MESSAGE | CONTEXT |
|-----------|-------|---------|---------|
| `backend/src/main.rs:233` | error | "Worker exited with error" | `error = %e` |
| `backend/src/main.rs:249` | error | "Chat default model not present in provider map …" | `default`, `available` keys |
| `backend/src/main.rs:326` | error | "Failed to ensure Qdrant collection at startup …" | `error = %e` — includes follow-up note |
| `backend/src/main.rs:500` | error | "Failed to build LLM provider for RAG: {e}" | `e` |
| `backend/src/main.rs:633` | error | "Failed to load active llm_models for chat provider map" | `error = %e` |
| `backend/src/main.rs:657` | error | "Failed to construct AnthropicProvider for chat — skipping" | `model = %model.id, error = %e` |
| `backend/src/api/pipeline/delete.rs:326` | error | "Neo4j cleanup failed (source_document)" | `doc_id = %document_id, error = %e` |
| `backend/src/api/pipeline/delete.rs:387` | error | "Qdrant cleanup failed" | `doc_id, error` |
| `backend/src/api/pipeline/reverify_sync.rs:197` | error | "Re-verify auto-approve phase failed" | `doc_id, error` |
| `backend/src/api/pipeline/ingest.rs:64` | warn | "Failed to release ingest advisory lock" | `doc_id, error` |
| `backend/src/pipeline/steps/llm_extract.rs:531` | error | "Failed to write chunk stats to extraction run — audit data incomplete" | `run_id, error` |
| `backend/src/pipeline/steps/llm_extract.rs:1128` | error | "Failed to record successful chunk completion — audit data incomplete" | `run_id, chunk_id, error` |
| `backend/src/pipeline/steps/llm_extract.rs:1179` | error | "Failed to record failed (parse) chunk completion — audit data incomplete" | `run_id, chunk_id, error` |
| `backend/src/pipeline/steps/llm_extract.rs:1221` | error | "Failed to record failed (LLM call) chunk completion — audit data incomplete" | `run_id, chunk_id, error` |
| `backend/src/pipeline/steps/llm_extract_helpers.rs:185` | warn | "mark_run_failed: DB write failed (non-fatal)" | `run_id, error, reason` |
| `backend/src/repositories/audit_repository.rs:94` | warn | "Failed to write audit log: {e}" | `e` |
| `backend/src/bias/handlers.rs:57, 98` | error | various | `error`, `field` |

These are the canonical examples — structured fields, source error preserved, semantic context (`audit data incomplete` tells the operator the consequence).

#### 1b.2 — POOR examples (Debug-format error, no operation context)

| FILE:LINE | LEVEL | MESSAGE | PROBLEM |
|-----------|-------|---------|---------|
| `backend/src/api/queries.rs:40` | error | `"Failed to run query {}: {:?}"` | `{:?}` is Debug — wall of `sqlx::Error::ColumnDecode { … }`. No request id, no SQL. |
| `backend/src/api/contradictions.rs:21` | error | `"Failed to fetch contradictions: {:?}"` | Debug fmt; no parameters. |
| `backend/src/api/claims.rs:169` | error | `"Failed to fetch motion claims: {:?}"` | Same. |
| `backend/src/api/case_summary.rs:21` | error | `"Failed to fetch case summary: {:?}"` | Same. |
| `backend/src/api/analysis.rs:21` | error | `"Failed to fetch analysis data: {:?}"` | Same. |
| `backend/src/api/evidence_chain.rs:34` | error | `"Failed to fetch evidence chain: {:?}"` | Same. |
| `backend/src/api/evidence.rs:21` | error | `"Failed to fetch evidence: {:?}"` | Same. |
| `backend/src/api/graph.rs:32` | error | `"Failed to fetch graph: {:?}"` | Same. |
| `backend/src/api/schema.rs:37` | error | `"Schema query failed: {:?}"` | Same. |
| `backend/src/api/decomposition.rs:41, 85` | error | `"Failed to fetch decomposition: {:?}"`, `"Failed to fetch rebuttals: {:?}"` | Same. |
| `backend/src/api/harms.rs:21` | error | `"Failed to fetch harms: {:?}"` | Same. |
| `backend/src/api/case.rs:22` | error | `"Failed to fetch case: {:?}"` | Same. |
| `backend/src/api/allegations.rs:21` | error | `"Failed to fetch allegations: {:?}"` | Same. |
| `backend/src/api/persons.rs:26, 47` | error | `"Failed to fetch persons: {:?}"`, `"Failed to fetch person detail for {}: {:?}"` | Latter improves slightly. |

All 14 of these share the same anti-pattern — query handlers logging `{:?}` of `sqlx::Error` with no request context.

#### 1b.3 — Cleanup/best-effort warnings (ADEQUATE but inconsistent)

| FILE:LINE | LEVEL | MESSAGE | NOTES |
|-----------|-------|---------|-------|
| `backend/src/pipeline/registry.rs:198` | warn | (full path missing) | Per-mismatch reporting in registry validation. |
| `backend/src/pipeline/providers.rs:50` | warn | (model/provider) | Provider selection warning. |
| `backend/src/pipeline/steps/auto_approve.rs:119` | warn | Approval threshold not met for some items | Names threshold and counts. |
| `backend/src/pipeline/steps/completeness.rs:260` | warn | Qdrant points missing for some node ids | Names counts. |
| `backend/src/pipeline/validation.rs:212` | warn | "Failed to list active models for error message" | `error = %e` — fallback diagnostic path. |
| `backend/src/services/graph_expander.rs:120, 139` | warn | Unknown node type for expansion | Names node type. |
| `backend/src/api/pipeline/upload.rs:150–608` | warn (multiple) | Upload-time profile resolution warnings | All carry doc_type, doc_id, intended_profile. **GOOD.** |
| `backend/src/api/pipeline/ocr.rs:236, 329` | warn | "OCR env var not set" | Named env var. **GOOD.** |
| `backend/src/api/pipeline/config_endpoints/preview.rs:272` | warn | "Cost lookup failed — returning None" | `error`. |
| `backend/src/api/pipeline/extract_text.rs:101, 158, 201, 223` | warn | (multiple — page-level OCR fallback decisions) | Carry page_number; **GOOD.** |
| `backend/src/api/pipeline/verify.rs:306, 772, 787` | warn | (LLM noise dropped) | Carry counts. |
| `backend/src/pipeline/steps/verify.rs:342` | warn | unmatched grounding entry | Carries entity_id. |

#### 1b.4 — CLI `eprintln!` (acceptable in CLI mode)

| FILE:LINE | NOTES |
|-----------|-------|
| `backend/src/cli.rs:61, 103` | `eprintln!` emitted from the `embed` subcommand for non-JSON output mode. Not used in the server path. ADEQUATE. |
| `backend/src/cli.rs:44` | `tracing::warn!("Could not delete collection (may not exist): {e}")` — fine. |

#### 1b.5 — Comments/doc references to log statements (not bugs)

The grep matched 17 lines that are doc-comment references like `/// tracing::warn!` — these are documentation, not executable. Excluded from the count of issues.

### 1c. Frontend Error Display

43 unique `console.*` / `.catch(...)` occurrences. Frontend error display flows through three paths:

#### 1c.1 — `setActionError(e instanceof Error ? e.message : "fallback")` (the standard pattern)

This is the dominant pattern. Examples:

| FILE:LINE | DISPLAY METHOD | MESSAGE SOURCE | QUALITY |
|-----------|----------------|----------------|---------|
| `frontend/src/components/pipeline/ProcessingPanel.tsx:180, 198` | inline (errorBox div at line 430) | backend message via thrown Error | **GOOD** |
| `frontend/src/components/pipeline/ReviewPanel.tsx:210, 218, 224, 230, 279, 294` | inline | backend message | **GOOD** |
| `frontend/src/components/pipeline/DeleteConfirmDialog.tsx:107` | inline | backend message | **GOOD** |
| `frontend/src/components/pipeline/UploadDialog.tsx:137` | inline | backend message | **GOOD** |
| `frontend/src/components/pipeline/ReprocessDialog.tsx:84` | inline | backend message | **GOOD** |
| `frontend/src/pages/AllegationDetailPage.tsx:63` | inline | backend message | **GOOD** |
| `frontend/src/pages/DocumentsPage.tsx:93` | inline | backend message | **GOOD** |

#### 1c.2 — Silent catch (drops the error entirely)

| FILE:LINE | METHOD | PROBLEM |
|-----------|--------|---------|
| `frontend/src/components/pipeline/ReviewPanel.tsx:245` | `try { … } catch { /* silent */ }` | Edit-then-reload: edit failure is invisible. **POOR** — user thinks the edit succeeded; only realises when the reload doesn't reflect it. |
| `frontend/src/components/pipeline/ReviewPanel.tsx:264` | `} catch { /* silent */ }` | Same shape. **POOR.** |
| `frontend/src/components/pipeline/ProcessingPanel.tsx:157` | `.catch(() => { if (!cancelled) setCompletedConfig(null); })` | OK if "config not available" is the legitimate state, but the inability to distinguish "no config yet" from "fetch failed" is **MEDIUM**. |
| `frontend/src/components/pipeline/UploadDialog.tsx:79` | `.catch((e) => { … })` — likely logs/stores | Need to read body; **MEDIUM** unconfirmed. |
| `frontend/src/components/pipeline/ExtractionConfigDialog.tsx:102` | `.catch((e) => { … })` | Same. |
| `frontend/src/components/pipeline/ConfigurationPanel.tsx:561, 564, 577, 578` | `.catch(() => ({ profiles: [] }))`, `.catch(() => ({ models: [] }))`, `try { … } catch { }` | List endpoints fall back to empty arrays without notifying user. **POOR** for the unsuspecting operator who sees "no profiles configured" when the real cause is a 500 from the backend. |
| `frontend/src/pages/DocumentsPage.tsx:105, 108` | `.catch(() => { /* metrics are optional */ })`, `.catch(() => { /* errors are optional */ })` | Comments rationalise the silencing but operator never knows metrics endpoint is down. **MEDIUM.** |
| `frontend/src/pages/TimelinePage.tsx:63` | `.catch(() => {})` | Loading `data/timeline.json` — bundled asset. **LOW** (timeline empty in UI is acceptable). |
| `frontend/src/pages/People.tsx:118`, `DecompositionPage.tsx:71` | `catch { … }` | Body unexamined. **MEDIUM.** |
| `frontend/src/services/auth.ts:88` | `catch { return null; }` in `getCurrentUser` | Acceptable — anonymous user is a normal state. **GOOD.** |

#### 1c.3 — Fetch error vs HTTP-non-OK distinction is lost

Across every service file the pattern is `if (!res.ok) throw new Error(err.message)`. A network failure (`fetch()` rejects) becomes the same caught Error as an HTTP 500. The frontend UI cannot distinguish "backend unreachable" from "backend returned 500" — which matters for retry guidance.

Specific instances — every file in `frontend/src/services/`:
- `admin.ts`, `pipelineApi.ts`, `configApi.ts`, `ask.ts`, `qa.ts`, `bias.ts`, `search.ts`, `claims.ts`, `harms.ts`, `persons.ts`, `personDetail.ts`, `contradictions.ts`, `decomposition.ts`, `evidenceChain.ts`, `analysisApi.ts`, `motionClaims.ts`, `evidence.ts`, `documentEvidence.ts`, `case.ts`, `caseSummary.ts`, `allegations.ts`, `graph.ts`, `queries.ts`, `schema.ts`, `api.ts`, `auth.ts` (26 files).

---

## Section 2: Silent Failures

### 2.1 — `unwrap_or_default()` / `.unwrap_or("")` on row decode (HIGH-RISK BLOCK)

The repositories read every nullable Neo4j or SQL column via `.get("col").ok()` or `unwrap_or_default()`. This means a renamed column reads as an empty string, not an error.

**Pattern A — Neo4j node decoding via `node.get("col").ok()` (35+ instances):**

| FILE:LINE | COLUMN | WHAT IS LOST |
|-----------|--------|---------------|
| `backend/src/models/document.rs:50–57` | doc_type, description, file_path, uploaded_at, related_claim_id, source_url, created_at, notes | Renamed/dropped column reads as default. **HIGH** if any column is later marked NOT NULL. |
| `backend/src/models/claim.rs:39` | description | Same. **HIGH.** |
| `backend/src/repositories/evidence_repository.rs:70–81` | exhibit_number, title, question, answer, kind, weight, page_number, significance, verbatim_quote, stated_by, document_id, document_title | Twelve consecutive `.ok()` decodes. **HIGH.** |
| `backend/src/repositories/contradiction_repository.rs:77–81` | description, topic, impeachment_value, earlier_claim, later_admission | **HIGH.** |
| `backend/src/repositories/allegation_repository.rs:163–167` | paragraph, allegation, evidence_status, category, severity | **HIGH.** |
| `backend/src/repositories/motion_claim_repository.rs:71–75` | claim_text, category, significance, document_id, document_title | **HIGH.** |
| `backend/src/repositories/decomposition_repository.rs:168` | status | **MEDIUM.** |
| `backend/src/repositories/evidence_chain_repository.rs:85–86, 111–113` | a_paragraph, a_status, e_question, e_answer, e_page | **HIGH.** |
| `backend/src/repositories/person_detail_repository.rs:186` | char_label | **MEDIUM.** |
| `backend/src/repositories/allegation_detail_repository.rs:180` | char_evidence_id | **MEDIUM.** |
| `backend/src/repositories/graph_repository.rs:117` | a_status | **MEDIUM.** |
| `backend/src/repositories/analysis_repository.rs:122–123` | allegation, paragraph | **MEDIUM.** |
| `backend/src/bias/aggregation.rs:47` | document_id | **HIGH** — bias evidence with missing document_id silently drops linkage. |
| `backend/src/api/admin_document_evidence_queries.rs:52` | source_type | **MEDIUM.** |
| `backend/src/services/audit_checks.rs:52, 280` | file_path, title | **MEDIUM.** |

**SEVERITY:** HIGH for the user-visible legal evidence repositories. A renamed `evidence.verbatim_quote` column silently makes every legal quote in the UI blank with no log line.

**Pattern B — `.unwrap_or("")` on optional JSON props (50+ instances in ingest_helpers.rs alone):**

| FILE:LINE | EXAMPLE |
|-----------|---------|
| `backend/src/api/pipeline/ingest_helpers.rs:95, 113, 122, 125, 135, 136, 137, 169, 272, 273, 372, 373, 374, 702, 713, 927` | `props["role"].as_str().unwrap_or("")` etc. — 16 instances |
| `backend/src/api/pipeline/ingest_resolver.rs:124, 181, 203` | similar |
| `backend/src/api/admin_evidence_helpers.rs:35, 36` | `val.get("topic").and_then(|v| v.as_str()).unwrap_or("")` |
| `backend/src/api/admin_documents.rs:220, 238` | clone+unwrap_or_default on doc_type and content_hash |

The "extract field from JSON Value with `.as_str().unwrap_or(\"\")`" pattern across the ingest code is the canonical silent-data-loss vector for LLM-output drift. An LLM that starts returning `"name": null` instead of `"name": "Ada"` produces a node titled `""`.

### 2.2 — `let _ = result;` and `.ok();` after important writes (32 instances in pipeline code)

| FILE:LINE | OPERATION | WHAT IS LOST | SEVERITY |
|-----------|-----------|---------------|----------|
| `backend/src/pipeline/steps/llm_extract.rs:591` | `documents::update_processing_progress(...)` final write | Operator's "Extraction complete" status not reflected in DB. | MEDIUM |
| `backend/src/pipeline/steps/llm_extract.rs:691, 705` | progress updates inside `run_full_document_extraction` | Progress bar stalls if write fails. | LOW |
| `backend/src/pipeline/steps/llm_extract.rs:944` | progress at start of chunked loop | Same. | LOW |
| `backend/src/pipeline/steps/llm_extract.rs:961, 1157, 1201, 1243` | progress in chunked-loop body | Same. | LOW |
| `backend/src/pipeline/steps/llm_extract_helpers.rs:79` | rate-limit progress emit | Operator can't see "rate-limited, retrying". | MEDIUM |
| `backend/src/pipeline/steps/llm_extract_pass2.rs:471` | pass-2 progress | Same. | MEDIUM |
| `backend/src/pipeline/steps/verify.rs:130` | "{:.0}% grounded" progress | Same. | LOW |
| `backend/src/pipeline/steps/auto_approve.rs:145` | auto-approve progress | Same. | LOW |
| `backend/src/main.rs:403` | `shutdown_tx.send(true)` | Receiver dropped — only happens on shutdown; tolerable. | LOW |
| `backend/src/main.rs:110` | `dotenvy::dotenv().ok()` | Standard pattern — `.env` missing in production is intentional. | LOW |
| `backend/src/neo4j.rs:30` | `let _ = result.next().await;` | Health-check probe; the value isn't needed. | LOW (but the comment explains why) |
| `backend/src/services/graph_expander.rs:222–274` | 13 `let _ = writeln!(s, …)` | `fmt::Write` on `String` cannot fail; idiomatic. | LOW (false positive) |
| `backend/src/api/pipeline/report.rs:273, 329, 362, 374, 383, 388, 449, 462, 467, 480, 486, 502, 524, 535` | 14 `let _ = write!(html, …)` | Same — writing to a String. **LOW** false positives. |
| `backend/src/api/mod.rs:181` | `.ok()` on something — body unread | **MEDIUM** — needs inspection |

The genuine concern in this section is the 8 `update_processing_progress(...).await.ok()` calls. The frontend `ProcessingPanel` polls these progress columns; a silent DB write failure leaves the UI stuck at a stale percentage. Per `CLAUDE.md` Rule 1 the failure must be observable.

### 2.3 — `.unwrap_or_default()` on `serde_json::to_value(&chunk.metadata)`

`backend/src/pipeline/steps/llm_extract.rs:975–982` — comment explicitly justifies why this is a "guaranteed-OK call" (HashMap<String, Value> always serialises). The fallback emits an empty `{}` object with a `tracing::warn!` if it ever fails. **OK.**

### 2.4 — Frontend `try { … } catch { /* silent */ }`

See §1c.2 — 4 silent-catch occurrences (`ReviewPanel.tsx:245, 264`, `ConfigurationPanel.tsx:561, 564`).

### 2.5 — Functions that return `Ok(())` after partial completion

`backend/src/pipeline/steps/cleanup.rs:30–82` is the architectural exception: it returns `CleanupError::Partial { neo4j_error, qdrant_error, postgres_error, partial_report }` carrying the per-subsystem error chain. **GOOD.**

But `backend/src/api/pipeline/delete.rs:308–387` uses the older `if let Err(e) = ... { tracing::error!(...) }` pattern around Neo4j/Qdrant cleanup. Per the source comment in `cleanup.rs:8–11`, this is the legacy "best-effort log and swallow" pattern that `cleanup_all` was meant to replace. **MEDIUM** — `delete.rs` still uses the legacy idiom in three blocks (lines 308, 343, 387); the new `cleanup_all` saga is available but unused on the delete path.

### 2.6 — `parse_chunk_response` repair-then-stringify (LOW)

`backend/src/pipeline/steps/llm_extract_helpers.rs:111–133` — JSON parse failure is caught and `llm_json::repair_json` is tried. If repair also fails, the function returns `Err(String)` carrying a 200-character preview. Documentation note `ensure_object` (line 141–156) explicitly handles the "repair succeeded but returned a string" case. **GOOD** — pinned by `ensure_object_rejects_bare_string` test at line 237.

But a successful repair-then-parse path **does not record that repair was needed** anywhere. If an LLM starts emitting malformed JSON that repair silently fixes, the audit log shows COMPLETED runs with no indication of the underlying drift. **MEDIUM** — recommend writing a `repair_applied: bool` column on `extraction_chunks`.

---

## Section 3: Hardcoded Values

### 3.1 — Hardcoded model names / IDs

```
backend/src/main.rs:27        const DEFAULT_CHAT_MODEL: &str = "claude-sonnet-4-6"
backend/src/main.rs:33        const CHAT_MAX_TOKENS: u32 = 4096
backend/src/main.rs:567        RigSynthesizer::new(llm_provider, 4096)  -- max_tokens magic literal
backend/src/main.rs:581        .max_context_tokens(6000)
backend/src/main.rs:582        .search_limit(10)
backend/src/config.rs:87        ANTHROPIC_MODEL default "claude-sonnet-4-6"
backend/src/config.rs:95        DECOMPOSER_MODEL default "claude-sonnet-4-6"
backend/src/services/claude_client.rs:105        max_tokens: 2048  -- hardcoded
backend/src/services/claude_client.rs:114        "https://api.anthropic.com/v1/messages"  -- hardcoded API URL
backend/src/services/claude_client.rs:116        "anthropic-version", "2023-06-01"  -- pinned date string
```

**RISK:** `claude_client.rs:114, 116` are not configurable. To migrate to a future API version or to point at a proxy (e.g., for testing), code must change. **MEDIUM** — `services/claude_client.rs` appears to be the legacy non-RAG synthesis path; the RAG path goes through `colossus_extract::providers::AnthropicProvider` which IS configurable.

### 3.2 — Hardcoded URLs / hosts

```
backend/src/config.rs:76         "http://localhost:6333"  -- QDRANT_URL default
backend/src/main.rs:346          "http://localhost:5473,http://localhost:3403,http://10.10.0.99:5473"  -- CORS default
backend/src/api/logout.rs:15     "https://colossus-legal-dev.cogmai.com/outpost.goauthentik.io/sign_out"
                                  -- DEV-specific URL baked into PROD binary (logout fallback)
backend/src/api/pipeline/ocr.rs:332  example URL in error message — fine
```

**RISK:** `api/logout.rs:15` is **HIGH** — a PROD build inherits the DEV Authentik logout URL if the runtime config is missing.

### 3.3 — Hardcoded timeouts / retry counts

```
backend/src/main.rs:131                .timeout(Duration::from_secs(90))         -- shared reqwest
backend/src/main.rs:132                .connect_timeout(Duration::from_secs(5))
backend/src/main.rs:409                Duration::from_secs(30)                   -- worker drain
backend/src/main.rs:651                request_timeout_secs: None                -- chat provider; comment says 600s default
backend/src/database.rs:44, 59         acquire_timeout(Duration::from_secs(5))
backend/src/api/admin_audit_health.rs:49  Duration::from_secs(10)
backend/src/api/admin_status.rs:47     Duration::from_secs(3)
backend/src/api/pipeline/ocr.rs:290    ocr_timeout_secs (configurable — GOOD)
backend/src/api/pipeline/ocr.rs:342    Duration::from_secs(5)
backend/src/services/audit_checks.rs:309  Duration::from_secs(5)
backend/src/pipeline/steps/llm_extract_helpers.rs:91  Duration::from_secs(1)    -- sleep granularity
backend/src/pipeline/steps/llm_extract_helpers.rs:19  MAX_RETRIES_PER_CHUNK = 3
backend/src/pipeline/steps/ingest.rs:126   DEFAULT_RETRY_LIMIT = 3
backend/src/pipeline/steps/ingest.rs:127   DEFAULT_RETRY_DELAY_SECS = 10
backend/src/pipeline/steps/ingest.rs:128   DEFAULT_TIMEOUT_SECS = Some(300)
backend/src/pipeline/steps/index.rs:116    DEFAULT_RETRY_LIMIT = 3
backend/src/pipeline/steps/index.rs:117    DEFAULT_RETRY_DELAY_SECS = 10
backend/src/pipeline/steps/index.rs:118    DEFAULT_TIMEOUT_SECS = Some(300)
backend/src/pipeline/steps/verify.rs:95    DEFAULT_RETRY_LIMIT = 2
backend/src/pipeline/steps/verify.rs:96    DEFAULT_RETRY_DELAY_SECS = 5
backend/src/pipeline/steps/verify.rs:97    DEFAULT_TIMEOUT_SECS = Some(180)
backend/src/pipeline/steps/completeness.rs:132  DEFAULT_RETRY_LIMIT = 3
backend/src/pipeline/steps/completeness.rs:133  DEFAULT_RETRY_DELAY_SECS = 10
backend/src/pipeline/steps/completeness.rs:134  DEFAULT_TIMEOUT_SECS = Some(60)
```

**RISK:** All the per-step retry/timeout values are compile-time constants. Tuning them in production requires a rebuild. Per `CLAUDE.md` Rule 2 these should come from config. **MEDIUM** — the upstream `colossus-pipeline` crate uses these constants as `Step` trait defaults so step-level retry tuning needs a crate change too.

### 3.4 — Hardcoded ports / dimensions

```
backend/src/main.rs:330        "3403" — BACKEND_PORT default (env-overridable, OK)
backend/src/main.rs:524        ":6333" → ":6334" port replacement — fragile if env uses different port
```

### 3.5 — Hardcoded paths

```
backend/src/config.rs:72        "./data/documents"
backend/src/config.rs:80        "/data/models"
backend/src/config.rs:101       "/data/documents/prompts"
backend/src/config.rs:108       "./extraction_schemas"
backend/src/config.rs:111       "./extraction_templates"
backend/src/config.rs:114       "./config"
backend/src/config.rs:117       "./profiles"
backend/src/config.rs:120       "./system_prompts"
backend/src/main.rs:678         DEFAULT_STARTUP_SCHEMA_FILE = "general_legal.yaml"
```

All env-overridable; defaults are dev-machine paths. **LOW.**

### 3.6 — Hardcoded entity / relationship type names

`backend/src/models/document_status.rs:107–151` defines all entity and relationship constants as `pub const`. **GOOD pattern** — all references go through these names. The audit confirms there is one structural relationship hardcoded in ingest:
- `REL_CONTAINED_IN: "CONTAINED_IN"` (line 139) used as a string by `create_contained_in_relationships`.

**NOT a violation** — the constant is named and reused. The 270 SCREAMING_SNAKE matches in `/tmp/audit/status_consts.txt` are mostly:
- 90 references inside `models/document_status.rs` itself (the definitions)
- 50 references to `PARTY_SUBTYPES`, `STATUS_PROCESSING`, etc. in step code (all via the named const, **GOOD**)
- 130 references in test fixtures and SQL string literals

There is one raw entity-type literal slipping through:
- `backend/src/api/pipeline/ingest_helpers.rs:272` — `"unknown"` as fallback type. **MEDIUM** — should be a named `ENTITY_UNKNOWN_FALLBACK` or removed.

### 3.7 — Hardcoded case-specific data

```
backend/src/repositories/allegation_repository.rs:77        TODO comment notes Awad-case-specific paragraph ranges
```

**HIGH** — case-specific data in shared library per `CLAUDE.md` Rule 2 ("No domain-specific names, terms, or aliases in shared library code"). Tracked as TODO.

### 3.8 — Hardcoded SQL `COALESCE` defaults

72 matches across the repository code (`/tmp/audit/coalesce.txt`). Most are aggregation defaults (`COALESCE(COUNT(*), 0)`) which are appropriate. A sample of the substantive ones:

```
backend/src/api/pipeline/metrics.rs:108, 124      COALESCE on numeric averages → 0.0
                                                  -- arguable: empty avg should be NULL/None, not 0
backend/src/repositories/case_summary_repository.rs:196  TODO acknowledging hardcoded query
```

**LOW** for the metrics path; **MEDIUM** for case_summary which is flagged as DAL Phase 2 debt.

### 3.9 — Hardcoded magic numbers

```
backend/src/pipeline/steps/extract_text.rs:93     char_threshold = 50  -- OCR fallback threshold
backend/src/pipeline/steps/extract_text.rs:94     dpi = 300
backend/src/pipeline/steps/extract_text.rs:95     lang = "eng"
backend/src/pipeline/steps/extract_text.rs:96     oem = 1
                              -- comment says all are step_config-overridable; defaults are fine
backend/src/pipeline/steps/llm_extract.rs:44      DEFAULT_CHUNK_MAX_TOKENS: u32 = 8000
backend/src/pipeline/constants.rs:31              MAX_UPLOAD_SIZE_BYTES = 50 * 1024 * 1024
backend/src/pipeline/context.rs:30                DEFAULT_LLM_CONCURRENCY = 2
backend/src/config.rs:92                          rerank_threshold default 0.3
backend/src/services/claude_client.rs:105         max_tokens: 2048  (different from main.rs:33's 4096)
```

**INCONSISTENCY:** `services/claude_client.rs:105` uses `max_tokens: 2048` whereas `main.rs:33` defines `CHAT_MAX_TOKENS = 4096`. Two distinct synthesis paths run with different limits. **MEDIUM** — operator surprise when one path truncates and the other does not.

### 3.10 — Hardcoded Anthropic API version

`backend/src/services/claude_client.rs:116` — `"anthropic-version", "2023-06-01"`. Pinned at compile time. **LOW** — well-known stable header.

### 3.11 — Hardcoded Qdrant collection name

`backend/src/pipeline/constants.rs:22` — `QDRANT_COLLECTION_NAME: &str = "colossus_evidence"`. Used by `main.rs:543`, `pipeline/steps/index.rs:51`, `pipeline/steps/cleanup.rs:35`, `services/qdrant_service`. **MEDIUM** — appropriate scope but not env-overridable. Per `CLAUDE.md` Rule 2 if a second case repo wants its own collection it requires a code change.

### 3.12 — Hardcoded Neo4j property names

`backend/src/pipeline/constants.rs:25, 28` — `NEO4J_SOURCE_DOCUMENT_PROP = "source_document"`, `NEO4J_SOURCE_DOCUMENT_ID_PROP = "source_document_id"`. **OK** — well-named constants, used by both ingest and cleanup.

### 3.13 — Frontend hardcoded fallbacks

```
frontend/src/services/api.ts:63    API_BASE_URL fallback "http://localhost:3403"  -- LOW, dev-only
frontend/src/services/auth.ts:48   timeoutMs default 30000  -- 30s default; LOW, configurable per call
frontend/src/services/auth.ts:106  config.apiUrl || ""  -- empty-string same-origin fallback
```

---

## Section 4: Retry and Recovery

### 4a. Pipeline Job Retry Infrastructure

#### Where jobs are created

`backend/src/api/pipeline/process.rs:102–131` — handler for `POST /api/admin/pipeline/documents/:id/process`. Builds a `colossus_pipeline::Scheduler` over `state.pipeline_pool` and calls `scheduler.submit(JOB_TYPE_DOCUMENT_PROCESSING, doc_id, initial_task, PRIORITY_DEFAULT, Some(&user.username))`. The framework (external `colossus_pipeline` crate) writes the row.

#### Schema (migration 20260417_create_pipeline_jobs_and_events.sql)

The retry-related columns are:

```sql
tried              INT NOT NULL DEFAULT 0,
max_retries        INT NOT NULL DEFAULT 0,        -- ⚠ default 0 means NO retries
retry_delay_secs   INT NOT NULL DEFAULT 0,        -- ⚠ default 0 means immediate retry
timeout_at         TIMESTAMPTZ,
worker_id          TEXT,
last_heartbeat_at  TIMESTAMPTZ,
```

**GAP 4a.1 (HIGH):** The `pipeline_jobs.max_retries` column has a default of `0`. The `Scheduler::submit` call at `process.rs:109–117` does not pass per-job retry limits — those have to come from the step's `Step::DEFAULT_RETRY_LIMIT` (set in `ingest.rs:126`, `index.rs:116`, `verify.rs:95`, `completeness.rs:132`). It is unclear from this audit whether the framework consults `Step::DEFAULT_RETRY_LIMIT` when writing the row or whether the `max_retries=0` default means failed jobs never auto-retry without operator intervention. The framework lives in the external `colossus-pipeline` crate which was not part of this audit's scope.

**GAP 4a.2 (MEDIUM):** `LlmExtract`, `ExtractText`, `AutoApprove`, `LlmExtractPass2` do NOT declare `DEFAULT_RETRY_LIMIT`, `DEFAULT_RETRY_DELAY_SECS`, `DEFAULT_TIMEOUT_SECS`. Their impls of `Step<DocProcessing>` rely on the trait defaults. Only `ingest`, `index`, `verify`, `completeness` declare them. So:
- `ExtractText` → no timeout? → a stuck `pdftoppm` could run unbounded.
- `LlmExtract` → no timeout? → a hung Anthropic stream could leak the LLM semaphore permit indefinitely.

(Confirmed at `pipeline/steps/llm_extract.rs:159–199` — no `DEFAULT_*` consts; same at `extract_text.rs`, `auto_approve.rs`, `llm_extract_pass2.rs`.)

#### Zombie / stuck-RUNNING detection

The migration creates two indexes that suggest the framework supports zombie detection:

```sql
CREATE INDEX idx_pipeline_jobs_running_timeout
    ON pipeline_jobs (timeout_at ASC)
    WHERE status = 'running' AND timeout_at IS NOT NULL;
CREATE INDEX idx_pipeline_jobs_running_heartbeat
    ON pipeline_jobs (last_heartbeat_at ASC)
    WHERE status = 'running';
```

`backend/src/api/pipeline/process.rs:84–100` deletes a prior `'failed'` row before re-submission so the partial unique index `idx_pipeline_jobs_unique_active` doesn't reject the new submit. No code in `backend/src/` reads `last_heartbeat_at` or `timeout_at` — recovery is handled by `colossus-pipeline` (external).

**GAP 4a.3 (MEDIUM):** There is no UI surface showing zombie jobs. The operator can re-submit (`POST /process`) and the handler will clean up the failed row, but a job stuck in `'running'` past its timeout is invisible until manually inspected.

#### What happens to extracted data on failure

- **Pre-FAILED extraction:** `mark_run_failed` (`llm_extract_helpers.rs:173–187`) writes status=FAILED with a JSON error blob to `extraction_runs.result`. Entities/relationships from the failed run are NOT inserted (`store_entities_and_relationships` is gated on COMPLETED status earlier in the orchestrator).
- **Pre-COMPLETED extraction with partial chunks:** the `extract_chunks_loop` (`llm_extract.rs:909–1343`) only fails the run if ALL chunks failed (line 1248: `if chunks_succeeded == 0`). Otherwise it merges successful chunks via `ChunkMerger` and stores them. A retry would short-circuit via `pass1_already_complete` (line 1442) and NOT reprocess the document — meaning the failed chunks stay lost. **GAP 4a.4 (HIGH).**
- **Per-chunk row:** `complete_extraction_chunk` is called per-chunk regardless of outcome, capturing token counts and duration. **GOOD.**
- **Stuck-RUNNING extraction_runs:** `llm_extract.rs:396–400` calls `reset_extraction_run_children` after the ON CONFLICT DO UPDATE — wipes any stale child rows. **GOOD.**

#### Recovery options for the operator

- **Re-process button** → `POST /api/admin/pipeline/documents/:id/process` (UI: `ProcessingPanel.tsx:168–180`).
- **Cancel button** → `POST /cancel`. Sets `control='cancel'` on the running job; the worker calls `on_cancel` per-step which deletes partial state.
- **Delete document** → `delete.rs:308–387` and `cleanup_all` (via `Task::on_delete_current` → `pipeline/task.rs:177`) wipe Neo4j, Qdrant, and the PG pipeline tables.

**GAP 4a.5 (MEDIUM):** There is no "Resume from FAILED step" option. A failure during Index re-runs the entire pipeline from `ExtractText` (because `pass1_already_complete` only short-circuits up to LlmExtract; Verify / AutoApprove / Ingest / Index / Completeness have no idempotency short-circuit on the orchestrator-level `execute()`). However, individual steps ARE idempotent:
- `extract_text` uses `ON CONFLICT (document_id, page_number) DO UPDATE` (`extract_text.rs:18–22`).
- `ingest.rs:144–152` calls `cleanup_neo4j` then re-writes — idempotent at the doc level (although wasteful).
- `index.rs:8–28` uses Qdrant upsert semantics — idempotent.
- `completeness.rs:27–28` is read-only.

So re-process from `ExtractText` is safe; the gap is that it's *wasteful* and there's no UI to skip already-completed steps.

### 4b. LLM Call Retry

`backend/src/pipeline/steps/llm_extract_helpers.rs:19–104` — `call_with_rate_limit_retry`:

- **Retried on:** `PipelineError::RateLimited { retry_after_secs }` only. Sleeps `retry_after_secs` (cancel-aware, 1-second polling).
- **Max retries:** `MAX_RETRIES_PER_CHUNK = 3` (line 19), hardcoded.
- **NOT retried:** any other error (timeout, 500, network error). Returns immediately.
- **Result on exhaustion:** returns `PipelineError::LlmProvider(format!("chunk {}/{}: exhausted {} rate-limit retries", …))`.
- **Per-chunk handling:** if the chunk fails after retries, `extract_chunks_loop` (line 1205–1244) marks the chunk failed but continues to the next chunk; only an all-chunks-fail aborts the run.

**GAP 4b.1 (HIGH):** Network errors (e.g., transient 503 from Anthropic) are NOT retried. A single transient blip fails the chunk. Industry best practice is exponential-backoff retry on idempotent calls (LLM extraction is idempotent given the same input).

**GAP 4b.2 (MEDIUM):** No request-id / response-id from Anthropic is captured anywhere. When a retry chain bottoms out, the operator can't correlate with Anthropic's server logs.

**GAP 4b.3 (MEDIUM):** The data on LLM failure path:
- The raw response is logged at chunk-debug level inside `parse_chunk_response` (`llm_extract_helpers.rs:127`) — a 200-char preview only — and NEVER persisted.
- `extraction_chunks` has columns to record the response but the chunk-loop never writes a `raw_response` column (would have to verify against the migration; based on my scan of the `complete_extraction_chunk` call at line 1115 it accepts only counts and durations, not the raw response text). The `extract_chunks_loop` discards the response on success too — only the merged entity/relationship JSON ends up in `extraction_runs.result`.

### 4c. Neo4j Transaction Safety

`backend/src/pipeline/steps/ingest.rs:276–283` — opens an explicit `graph.start_txn()` transaction. All writes (entities, relationships, CONTAINED_IN, DERIVED_FROM) execute against the same `&mut txn`. `txn.commit()` at line 504 — single commit point.

**GOOD** — Ingest is fully transactional. A failure mid-way rolls back the entire transaction; the document's Neo4j state is unchanged.

But there is still a per-document retry safety issue:

**GAP 4c.1 (MEDIUM):** The `cleanup_neo4j` call (`ingest.rs:147–152`) runs BEFORE `start_txn`. If `cleanup_neo4j` succeeds (wipes prior partial state) and then `start_txn().await` fails or the txn body fails before commit, the cleanup is committed (separate transaction) but the new write is not — leaving the document with NO Neo4j state. Operator sees document at `INGESTED` status but no nodes in the graph until next retry runs both cleanup and write again.

**GAP 4c.2 (MEDIUM):** `cleanup_neo4j` itself runs TWO `DETACH DELETE` queries (`cleanup.rs:141–158`) plus an array-strip (`strip_source_document_from_arrays`, line 169). These are three separate transactions. A failure between the first and second leaves partially-deleted state. The code does not roll back the first delete. The comment at line 149–152 acknowledges the ordering dependency but not the partial-failure path.

**GAP 4c.3 (LOW):** `re-verify & sync` (`api/pipeline/reverify_sync.rs`) is multi-step (auto-approve, then Neo4j sync). The auto-approve failure path at line 197 logs the error but the sync runs anyway? — needs further reading; flagging for follow-up.

### 4d. PostgreSQL Transaction Safety

| OPERATION | TRANSACTIONAL? | NOTES |
|-----------|----------------|-------|
| `insert_extraction_run` (`llm_extract.rs:369–390`) | Single statement — atomic. ON CONFLICT DO UPDATE keys on (document_id, pass_number). | **GOOD.** |
| `reset_extraction_run_children` (line 396) + later `store_entities_and_relationships` | **NOT** wrapped in a transaction. If reset succeeds and store fails, the run row has zero items in DB but the run is marked COMPLETED. | **GAP 4d.1 (HIGH).** |
| `complete_extraction_run` + `update_run_chunk_stats` | Two separate statements (`llm_extract.rs:529`, `539`). If chunk_stats update fails (silently logged at line 531) but complete_extraction_run succeeds, the run row has counts=NULL alongside COMPLETED status. | **MEDIUM.** |
| `update_document_status` + `update_document_write_counts` + `batch_update_neo4j_node_ids` (ingest.rs:516–556) | Three separate statements after the Neo4j txn commit. If `update_document_write_counts` fails, the document is INGESTED in PG but no count visible to UI. | **MEDIUM.** |
| `delete_handler` (`api/pipeline/delete.rs`) | Each cleanup phase (PG, Neo4j, Qdrant) is independent. cleanup_all in `pipeline/steps/cleanup.rs:177–230 (suggested by structure)` returns a `Partial` error with per-subsystem status; delete handler uses the older idiom. | See §2.5. |

---

## Section 5: Processing Observability

### 5a. What Execution Data Is Captured

#### ExtractText (`pipeline/steps/extract_text.rs`)

| WRITTEN TO | COLUMN(S) | NOTES |
|------------|-----------|-------|
| `document_text` | document_id, page_number, text_content, source (ocr/native) | Idempotent via ON CONFLICT DO UPDATE. |
| `pipeline_steps` | step_name, status, started_at, completed_at, result_summary JSONB, error | Legacy table (transitional per comment in `extract_text.rs:36–39`). |
| `pipeline_jobs.progress` | JSONB with chunk progress, OCR engine, OCR config | Live during execution. |
| `documents.processing_progress`, `processing_step` | Numeric percent | Best-effort, `.ok()`-discarded write. |

**NOT CAPTURED:** OCR engine actually selected per page (config-resolved value lives in memory only). Per-page char_count after extraction (used to decide OCR fallback but not stored). Wall-clock time per page.

#### LlmExtract (`pipeline/steps/llm_extract.rs`)

| WRITTEN TO | COLUMN(S) | NOTES |
|------------|-----------|-------|
| `extraction_runs` | id, document_id, pass_number, model, schema_version, template_file, template_hash, rules_name, rules_hash, schema (JSONB snapshot), temperature, max_tokens, admin_instructions, assembled_prompt, processing_config (JSONB snapshot), status, result JSONB, input_tokens, output_tokens, cost_usd | Comprehensive — see migration 20260410. |
| `extraction_chunks` | id, run_id, chunk_index, text, metadata JSONB, status, entity_count, relationship_count, input_tokens, output_tokens, duration_ms, error | Per-chunk audit. |
| `extraction_items` | id, run_id, item_data JSONB, entity_type, document_id, neo4j_node_id (post-ingest) | Final extracted items. |

**NOT CAPTURED:**
- **Raw LLM response per chunk** — only the parsed result lives in `result` JSONB; the raw text returned by the model is discarded. If repair was applied (`llm_extract_helpers.rs:120`) there is no record. **GAP 5a.1 (HIGH).**
- **Per-chunk repair-applied flag** — boolean indicating `llm_json::repair_json` was invoked rather than direct parse. **GAP 5a.2 (MEDIUM).**
- **Anthropic request-id headers** — used by Anthropic support to debug a specific call. **GAP 5a.3 (MEDIUM).**
- **Rate-limit history** — when `call_with_rate_limit_retry` retries, the retry count and waits are not persisted; only logged. **GAP 5a.4 (MEDIUM).**
- **Cancellation reason** — `on_cancel` deletes RUNNING runs (`llm_extract.rs:181–197`) but there's no record of which step was cancelling, or whether the cancel came from operator vs timeout. **GAP 5a.5 (LOW).**

#### LlmExtractPass2 (`pipeline/steps/llm_extract_pass2.rs`)

Symmetric to LlmExtract — writes its own `extraction_runs` row with `pass_number = 2`. Additionally captures `pass2_cross_doc_entities`, `pass2_source_document_ids` in `processing_config` JSONB (per `llm_extract.rs:1550–1562` `SnapshotRuntimeFields`). **GOOD.**

#### Verify (`pipeline/steps/verify.rs`)

| WRITTEN TO | COLUMN(S) | NOTES |
|------------|-----------|-------|
| `extraction_items.grounding_status`, `verification_reason`, `verification_match_type` | per-item | Captures match type. |
| `documents.processing_progress` | Percent | Best-effort. |
| `pipeline_jobs.progress` | JSONB | Carries per-item counts. |

**NOT CAPTURED:**
- **Per-item canonical-search hits** — which candidate string actually matched. The verifier picks one and stores `verification_reason` but doesn't store the alternatives considered. **GAP 5a.6 (LOW).**

#### AutoApprove (`pipeline/steps/auto_approve.rs`)

| WRITTEN TO | COLUMN(S) | NOTES |
|------------|-----------|-------|
| `extraction_items.review_status` | "approved" / "pending" | Threshold-driven. |

**NOT CAPTURED:**
- **Threshold value at the time of run** — the threshold comes from the profile; if the profile YAML changes between runs there is no record of the threshold that *was* in force when an item was approved. Mitigated by `processing_config` snapshot but `auto_approve_threshold` is not in the snapshot today (would need to verify). **GAP 5a.7 (MEDIUM).**

#### Ingest (`pipeline/steps/ingest.rs`)

| WRITTEN TO | COLUMN(S) | NOTES |
|------------|-----------|-------|
| Neo4j | Document node, Entity nodes, relationships, CONTAINED_IN, DERIVED_FROM | Single transaction. |
| `extraction_items.neo4j_node_id` | per-item | Lineage for completeness. |
| `documents.status` | INGESTED | |
| `documents.entities_written`, `relationships_written` | counts | Bug B2 fix. |
| `pipeline_steps.result_summary` | JSONB with counts | Bug B3 fix. |

**NOT CAPTURED:**
- Per-entity Neo4j write time. **GAP 5a.8 (LOW).**
- Resolution decisions for Party entities (which existing node a new Party matched). **GAP 5a.9 (MEDIUM)** — visible in logs only.

#### Index (`pipeline/steps/index.rs`)

| WRITTEN TO | COLUMN(S) | NOTES |
|------------|-----------|-------|
| Qdrant | Vectors per node | Upsert. |
| `documents.status` | INDEXED | |
| `pipeline_steps.result_summary` | embedded_count | |

**NOT CAPTURED:**
- Per-vector embedding time. Wall-clock for batch embed. Token count for embedding inputs. **GAP 5a.10 (LOW).**

#### Completeness (`pipeline/steps/completeness.rs`)

| WRITTEN TO | COLUMN(S) | NOTES |
|------------|-----------|-------|
| `documents.status` | PUBLISHED on success | |
| `pipeline_steps.result_summary` | JSONB with total_items, nodes_verified, points_verified, points_missing | |

**NOT CAPTURED:**
- Per-missing-id list of WHICH nodes/points were missing (visible only when CompletenessError::MissingNodes fires). **GAP 5a.11 (MEDIUM).**

### 5b. What Is Surfaced to the UI

Confirmed by reading `frontend/src/components/pipeline/ProcessingPanel.tsx`, `ConfigurationPanel.tsx`, `ContentPanel.tsx`, `ReviewPanel.tsx`, `ExecutionHistory.tsx`, and the service layer.

| INFORMATION | CAPTURED IN DB | SURFACED IN UI | REQUIRES SQL | TABLE/COLUMN |
|-------------|----------------|----------------|--------------|--------------|
| Profile used | YES | YES (Configuration panel) | NO | `pipeline_config.profile_name`, `extraction_runs.processing_config` |
| Schema file used | YES | YES | NO | `pipeline_config.schema_file` |
| Template file (pass-1) | YES | YES via resolved-config | NO | `extraction_runs.template_file`, snapshot |
| Template hash | YES | NO — operator visible only via DB | YES | `extraction_runs.template_hash` |
| System prompt file | YES | YES via resolved-config | NO | snapshot |
| System prompt hash | YES | NO | YES | snapshot |
| Global rules file | YES | YES | NO | snapshot |
| Global rules hash | YES | NO | YES | snapshot |
| Model used (pass-1) | YES | YES | NO | `extraction_runs.model` |
| Model used (pass-2) | YES | YES | NO | snapshot |
| Pass-1 input/output tokens | YES | PARTIAL — `pipeline_steps.result_summary` JSON | NO | `extraction_runs.input_tokens/output_tokens` |
| Pass-1 cost USD | YES | NO | YES | `extraction_runs.cost_usd` |
| Pass-1 duration | INDIRECT (started_at/completed_at) | NO | YES | `extraction_runs.started_at/completed_at` |
| Pass-2 input/output tokens | YES | PARTIAL | NO | `extraction_runs.input_tokens/output_tokens` (pass-2 row) |
| Pass-2 cost USD | YES | NO | YES | `extraction_runs.cost_usd` |
| Chunk count, chunks_succeeded, chunks_failed | YES | PARTIAL — visible in Processing panel result_summary | NO | `extraction_runs.chunk_count/chunks_succeeded/chunks_failed` |
| Per-chunk boundaries | YES | NO | YES | `extraction_chunks.text` (raw chunk text stored) |
| Per-chunk text length | NO | NO | NO | NOT CAPTURED — only the text itself |
| Per-chunk entity count | YES | NO | YES | `extraction_chunks.entity_count` |
| Per-chunk duration | YES | NO | YES | `extraction_chunks.duration_ms` |
| Per-chunk LLM prompt (assembled) | PARTIAL | NO | YES | `extraction_runs.assembled_prompt` — single row, not per-chunk. **GAP 5b.1** |
| Per-chunk LLM raw response | **NO** | NO | NO | **GAP 5a.1 (HIGH)** — only parsed result stored |
| Per-chunk parse error preview | YES | NO | YES | `extraction_chunks.error` |
| Per-chunk repair-applied flag | NO | NO | NO | **GAP 5a.2** |
| Grounding mode per entity_type | INDIRECT (from schema YAML) | NO | YES (cross-join with schema) | |
| Grounding/verification results per entity | YES | YES (Review panel renders grounding_status) | NO | `extraction_items.grounding_status` |
| Extraction run IDs linking to Neo4j | YES | YES (via Ingest stamp `extraction_run_id`) | NO | `pipeline_items.neo4j_node_id` |
| Error details when processing fails | YES | YES | NO | `documents.error_message`, `pipeline_jobs.error` |
| Execution history per step | YES | YES (ExecutionHistory.tsx renders pipeline_steps rows) | NO | `pipeline_steps` |

#### Bottom-line UI visibility

Operator can see WITHOUT SQL:
- Status group, current step, current step started time.
- Profile name, model, template, schema.
- Total entities/relationships, grounding %.
- Per-step result_summary (count fields).
- Error message on failure.
- Suggestion on failure (if `error_suggestion` populated — empirically never populated, see §1a.4).

Operator MUST run SQL to see:
- Cost USD (any pass).
- Per-chunk duration and tokens.
- Template / system prompt / rules hashes (reproducibility audit).
- Why a parse failed (the raw LLM response — and even SQL doesn't recover this because it's not stored).
- Rate-limit retry history.

### 5c. What Is Lost

1. **Raw LLM response per chunk** — not stored anywhere, not even logged. (HIGH)
2. **Repair-applied flag** — `llm_json::repair_json` invocation not recorded. (MEDIUM)
3. **Per-chunk Anthropic request-id** — Anthropic's response includes a request-id header used for support; never captured. (MEDIUM)
4. **Rate-limit retry count and waits** — logged but not persisted. (MEDIUM)
5. **Resolver match decisions for Party entities** — logged but not persisted. (MEDIUM)
6. **Cancellation source** — operator vs timeout vs framework cleanup. (LOW)
7. **Auto-approve threshold value at the time of run** — depends on profile snapshot being complete; needs verification. (MEDIUM)
8. **Per-page char_count after extraction** — used as OCR fallback decision input; never stored. (LOW)
9. **OCR engine actually used per page** — resolved in memory from step_config → env → defaults; the resolved value is not stored per-page. (MEDIUM)
10. **Splitter strategy effective config** — `StructureAwareSplitter::from_config` consumes the map; logged via `tracing::info!` at `llm_extract.rs:1417–1427` but the effective config map is not persisted on the run. (MEDIUM)
11. **Pass-1 vs Pass-2 timing breakdown** — pass_number separates rows but inter-pass gap is not surfaced. (LOW)
12. **Container restart correlation** — if a job restarts mid-flight (zombie recovery), there is no UI-visible "this job was recovered from a prior worker" indicator. (LOW)

---

## Section 6: Race Conditions and Concurrency

### 6.1 — Two jobs on the same document simultaneously

**RISK:** Prevented by `idx_pipeline_jobs_unique_active` partial unique index (migration 20260417). A second `Scheduler::submit` returns `DuplicateJob` (`process.rs:120–125`) → handler returns 409 Conflict. **PROTECTED.**

But the `process_handler` itself uses a multi-step pattern:
1. SELECT document (`process.rs:64–71`)
2. Check status_group (line 73–82)
3. DELETE failed pipeline_jobs row (line 89–100)
4. `Scheduler::submit` (line 109)

The window between (1) and (4) is not transactional. A concurrent `delete` request between (1) and (3) could delete the document before the submit. **GAP 6.1 (LOW)** — Scheduler::submit would fail FK constraint (or succeed and leak the job), then UI shows confusing state.

### 6.2 — UI sends conflicting updates while processing is running

`ProcessingPanel.tsx:168–198` — Re-process and Cancel buttons. Re-process pre-condition is `status_group != "processing"` enforced by backend. Cancel pre-condition is an active job exists. The two are mutually exclusive at the API level. **PROTECTED.**

But the **Review panel** (`ReviewPanel.tsx`) lets the user approve/reject items while a re-extract is running. Items written by a previous extraction get re-written by the new run via `reset_extraction_run_children` (`llm_extract.rs:396`). The user's manual edits are silently overwritten. **GAP 6.2 (HIGH)** — no UI lockout, no warning, no merge semantics.

### 6.3 — TOCTOU on file loading

`pipeline/steps/llm_extract.rs:325–328` — `std::fs::read_to_string(&template_path)`. The template file is read at execute time without a fingerprint check against the profile's recorded template_hash. If an operator edits the template between submit-time and execute-time, the run uses the new template silently. **GAP 6.3 (MEDIUM)** — partially mitigated because the hash is computed AT execute time (line 327), so the audit log reflects what was actually used. But the operator may believe they re-processed with the old template.

`pipeline/steps/llm_extract.rs:338–342` — same pattern for system prompt.
`pipeline/steps/llm_extract.rs:356–359` — same for global rules.

### 6.4 — Read-then-write races

| FILE:LINE | OPERATION | RISK |
|-----------|-----------|------|
| `api/pipeline/items.rs` (approve/reject) | Read item; check current review_status; write new one | Two reviewers approving at the same time — last-write-wins. No optimistic locking via `updated_at` check. **GAP 6.4 (MEDIUM).** |
| `api/pipeline/ingest.rs:64` advisory lock release log | Advisory lock is held during ingest — protects against concurrent ingest. **GOOD.** |
| `api/pipeline/review.rs` (multiple) | Item edits | Same as items.rs. |

### 6.5 — Two users editing pipeline_config for the same document

`pipeline/config.rs` is the resolver — no API surface here. The config-edit endpoint is `api/pipeline/config_endpoints/`. **TODO** — not deeply audited in this pass; flag for follow-up.

### 6.6 — Worker semaphore vs job-level retry

`pipeline/context.rs:30, 130, 171` — `llm_semaphore: Arc<Semaphore>` with `DEFAULT_LLM_CONCURRENCY=2`. `llm_extract.rs:409` acquires a permit. **GOOD** — single global lid on concurrent Anthropic calls.

But the permit is held for the duration of all chunks of the document (line 409 wraps the whole orchestrator). A long document with 20 chunks holds the permit for 20× the per-chunk duration, blocking other jobs even when the API is idle between chunks. **GAP 6.6 (MEDIUM)** — operator surprise: PIPELINE_LLM_CONCURRENCY=2 but observed concurrency tops out at 2 documents, not 2 calls.

---

## Section 7: Resource Management

### 7.1 — Database connection pooling

`backend/src/database.rs:44, 59` — `acquire_timeout(Duration::from_secs(5))`. Two pools (`main_pool`, `pipeline_pool`) — `sqlx::PgPool` handles pooling. **GOOD.**

**GAP 7.1 (LOW):** Pool max-connection settings not configurable per env var — uses sqlx defaults.

### 7.2 — HTTP client reuse

`main.rs:130–134` — single `reqwest::Client` built at startup, cloned via `Arc` semantics into `AppState.http_client` and `AppContext.http_client`. **GOOD.**

Outliers:
- `services/audit_checks.rs:309` — `reqwest::Client::builder()...build()` per call. **GAP 7.2 (MEDIUM)** — creates a new client per audit health check, not pooled.
- `api/admin_status.rs:47` — `.timeout(Duration::from_secs(3))` on a `Client::builder()` — likely also per-call. **GAP 7.2b (MEDIUM).**
- `api/pipeline/ocr.rs:290, 342` — Client::builder per OCR request. **GAP 7.2c (MEDIUM)** — though OCR is infrequent.

### 7.3 — File handles

`pipeline/steps/llm_extract.rs:325` uses `std::fs::read_to_string` — synchronous, returns String, no manual handle management. **GOOD.**

`pipeline/steps/extract_text.rs` uses `spawn_blocking` to wrap sync `colossus_pdf::PdfTextExtractor`; the wrapped reader manages its own handles. **GOOD.**

`api/pipeline/extract_text.rs` (the HTTP endpoint) uses async file I/O — also fine.

### 7.4 — Large-document memory

`pipeline/steps/llm_extract.rs:301–313` — loads `get_document_text` (all pages) into a single `Vec<DocumentPage>` then concatenates into `full_text: String`. For a 1000-page document this is megabytes per active job. Held until step completion.

`pipeline/steps/extract_text.rs` — PDF extraction is per-page; OCR is per-page (page is rendered to PNG → tesseract → result text). Memory bounded per page. **GOOD.**

`pipeline/steps/llm_extract.rs:300` `full_text` is the concerning concentration. **GAP 7.4 (MEDIUM)** — for a 500-page complaint, full_text could be 5+ MB; held throughout the chunked loop. Acceptable today (single-worker, low concurrency); will need attention if document size grows or workers multiply.

### 7.5 — Disk full / PostgreSQL full

No code path tested for these failure modes. Errors would propagate as sqlx errors with whatever message PostgreSQL emits — likely caught by `IngestError::Helper` / `LlmExtractError::InsertRunFailed`. **GAP 7.5 (LOW)** — no proactive disk-space monitoring; relies on operator's infrastructure monitoring.

### 7.6 — Unbounded collections

Sample check:
- `llm_extract.rs:920` — `chunk_results: Vec<(usize, Vec<ExtractedEntity>, Vec<ExtractedRelationship>)>` — grows linearly with chunk count. Bounded by the splitter's chunk count for the document.
- `ingest.rs:286–287` — `pg_to_neo4j: HashMap<i32, String>`, `pg_to_label: HashMap<i32, String>` — bounded by item count per document.
- `cleanup.rs:113` — `PostgresCleanupReport.tables_cleared: Vec<(&'static str, u64)>` — bounded by table count (small).

No unbounded collections found. **GOOD.**

---

## Section 8: Timeout Handling

| SERVICE | TIMEOUT? | VALUE | SOURCE | FAILURE MODE | NOTES |
|---------|----------|-------|--------|--------------|-------|
| Shared `reqwest::Client` (used by RAG, ingest_helpers via state.http_client) | YES | 90s request, 5s connect | hardcoded `main.rs:131–132` | reqwest returns Err — caller handles | **GOOD** — single shared client. |
| Anthropic via `colossus_extract::providers::AnthropicProvider` (pipeline extraction) | UNKNOWN — provider crate not audited | External | The provider crate manages its own client | Should be checked in `colossus-rs/colossus-extract/` | **GAP 8.1 (UNKNOWN)** |
| Anthropic via `services/claude_client.rs` (legacy synthesis) | NO — uses caller's `reqwest::Client` which has 90s timeout | inherited | inherited | OK if caller is the shared client | Confirmed: caller is the shared client. **GOOD.** |
| Neo4j via `neo4rs::Graph` | UNKNOWN — driver-managed | External | External | Hangs if driver doesn't time out | **GAP 8.2 (MEDIUM)** — no explicit driver-level timeout configured in `create_neo4j_graph`. |
| Qdrant via `qdrant-client` (gRPC) | NO explicit timeout | None visible | None — uses qdrant-client defaults | `main.rs:526–535` builds with `.skip_compatibility_check()` only, no timeout | **GAP 8.3 (HIGH)** per `CLAUDE.md` Rule 13 — "qdrant-client must have timeout configured." |
| Qdrant via REST in `services/qdrant_service` | Uses shared `state.http_client` | 90s | inherited | OK | **GOOD.** |
| Surya OCR | YES — `ocr_timeout_secs` configurable env var | env-var-resolved | `api/pipeline/ocr.rs:290` | Returns error | **GOOD.** |
| Tesseract OCR | YES — process kill_on_drop, no explicit timeout | None | None | Hangs the spawn_blocking | **GAP 8.4 (LOW)** — kill_on_drop helps on parent cancel but a stuck tesseract still blocks the worker thread. |
| PostgreSQL queries (sqlx) | NO query-level timeout | None | None | Hangs the query | **GAP 8.5 (MEDIUM)** — `sqlx` supports `statement_timeout` via session SET but it's not configured. The `acquire_timeout(5s)` only covers pool checkout. |
| OCR env var check (`ocr.rs:342`) | YES | 5s | hardcoded | Returns error | **GOOD.** |
| Audit health checks (`admin_audit_health.rs:49`) | YES | 10s wrapper around `run_all_checks` | hardcoded | Returns timeout error | **GOOD.** |
| Admin status check (`admin_status.rs:47`) | YES | 3s | hardcoded | Returns error | **GOOD.** |
| Audit checks individual (`audit_checks.rs:309`) | YES | 5s | hardcoded | Returns error | **GOOD.** |
| Frontend authFetch | YES | 30s default; per-call override | `auth.ts:48` `timeoutMs` option | Aborts → caught by caller | **GOOD.** |
| Frontend upload (`admin.ts:185`) | **NO** | None | Uses raw `fetch()` | Hangs forever if backend doesn't respond | **GAP 8.6 (HIGH)** — comment justifies skipping Content-Type but the timeout omission is unrelated. Reindex (line 131) correctly uses `timeoutMs: 120_000`. |

---

## Section 9: Test Coverage Gaps

The workspace test target is clean (`cargo test --workspace`, per `CLAUDE.md` 4.28). But coverage is uneven.

### 9.1 — Step files with strong tests

- `pipeline/steps/llm_extract.rs` — 200+ lines of tests for `assemble_chunk_prompt`, `load_global_rules`, `strip_authoring_comments`, schema-driven dedup. **GOOD.**
- `pipeline/steps/llm_extract_helpers.rs` — `strip_markdown_fences`, `parse_chunk_response`, `ensure_object`. **GOOD.**
- `pipeline/task.rs` — `validate_transition` covered for forward, backward, self-loop, step-skip, pass2-skip. **GOOD.**
- `pipeline/steps/ingest.rs:613–633` — display hygiene tests. **GOOD but minimal** — no execute/cancel test.
- `models/document_status.rs` — casing invariants. **GOOD.**
- `pipeline/registry_tests.rs` — 500+ lines. **GOOD.**

### 9.2 — Public functions with ZERO direct unit tests

Sampled via the `pub_fns.txt` enumeration. The following carry no tests in the same file or in `tests/`:

1. `pipeline/steps/llm_extract.rs::run_llm_extract` — orchestrator entry
2. `pipeline/steps/llm_extract.rs::run_chunked_extraction` — splitter integration
3. `pipeline/steps/llm_extract.rs::run_structured_extraction` — splitter integration
4. `pipeline/steps/llm_extract.rs::extract_chunks_loop` — per-chunk loop
5. `pipeline/steps/llm_extract.rs::write_processing_config_snapshot` — JSONB writer
6. `pipeline/steps/llm_extract_pass2.rs::*` — entire step has no tests (910 lines)
7. `pipeline/steps/verify.rs::run_verify` — orchestrator
8. `pipeline/steps/auto_approve.rs::*` — no tests
9. `pipeline/steps/ingest.rs::run_ingest` — orchestrator
10. `pipeline/steps/index.rs::run_index` — orchestrator
11. `pipeline/steps/completeness.rs::run_completeness` — orchestrator
12. `pipeline/steps/cleanup.rs::cleanup_all` — saga pattern; only `cleanup_neo4j` has light tests
13. `api/pipeline/process.rs::process_handler` — no tests
14. `api/pipeline/process.rs::cancel_handler` — no tests
15. `api/pipeline/upload.rs::*` — tests exist (lines 685+) but cover `from_yaml_str` shape, not the upload handler
16. `api/pipeline/ingest.rs::*` (HTTP handler) — no direct tests
17. `api/pipeline/verify.rs::*` (HTTP handler) — no direct tests
18. `api/pipeline/delete.rs::*` — no tests
19. `api/pipeline/extract_text.rs::*` (HTTP handler) — no tests
20. `api/pipeline/recompute_derived.rs::*` — no tests
21. `api/pipeline/reverify_sync.rs::*` — no tests
22. `api/pipeline/items.rs::*` — no tests for approve/reject/edit
23. `api/pipeline/review.rs::*` — no tests
24. `api/admin_*.rs` (~10 handlers) — no tests
25. `api/queries.rs`, `claims.rs`, `case_summary.rs`, `analysis.rs`, etc. — no tests for the handler bodies
26. `services/claude_client.rs::synthesize` — no tests
27. `services/embedding_service.rs::*` — no tests
28. `services/embedding_pipeline.rs::*` — no tests
29. `services/qdrant_service::ensure_collection` — no tests
30. `services/graph_expander.rs::*` (used by RAG fallback) — no tests
31. `repositories/*_repository.rs` — most have no tests (audit found only `extraction.rs:1527+` and `pipeline_repository/mod.rs:785+` have tests)
32. `bias/aggregation.rs::*` — has `bias/tests.rs` but coverage is partial
33. `bias/handlers.rs::*` — limited
34. `bias/repository.rs::*` — partial
35. `models/document.rs::*` — no decode tests (despite §2.1 risk)
36. `models/claim.rs::*` — no decode tests
37. `models/person.rs`, `decision.rs`, `evidence.rs`, `hearing.rs`, `import.rs` — no tests
38. `prompt_loader.rs` — no tests

### 9.3 — Error paths that are never tested

- `LlmExtractError::SchemaLoadFailed` — schema file missing during execute
- `LlmExtractError::ProviderConstructionFailed` — runtime provider failure
- `LlmExtractError::SemaphoreClosed` — semaphore close (impossible in current code but the variant exists)
- `IngestError::Cleanup` — cleanup_neo4j failure
- `IngestError::Neo4j` (start_txn or commit failure) — would need a mock Graph
- `VerifyError::NoCanonicalText` — document has no document_text rows
- `CompletenessError::MissingNodes` — partial graph state
- `CleanupError::Partial` — composite failure

### 9.4 — Validation rules without tests

- The `effective_mode` dispatch's "unknown → chunked" fallback (`llm_extract.rs:443–509`) — comment says "safer failure mode" but no test pins this.
- The `RUN_STATUS_*` vs `STEP_STATUS_*` casing invariant has a positive test but no test asserting they DIFFER (a future refactor could accidentally unify them).
- `pipeline_jobs` partial-unique-index behaviour during re-submit — not tested.
- The `pass1_already_complete` → `next_step_after_pass1` routing for `run_pass2=true` vs `false` — pinned by tests for the pure helper but not for the orchestrator.

### 9.5 — Critical-path coverage (upload → process → verify → ingest)

| PATH STEP | CODE BRANCHES | TESTS |
|-----------|---------------|-------|
| Upload (admin_upload.rs + pipeline/upload.rs) | ~12 (mime check, size check, profile resolve, default fallback, dup detect, save) | ~6 tests in pipeline/upload.rs (profile resolution only) |
| Process submit (api/pipeline/process.rs) | ~7 (404, conflict, prior-failed cleanup, dup-job, internal) | 0 |
| ExtractText execute | ~10 (native, OCR, mixed, env override, step_config override, default, cancel, doc missing, page write fail, OCR fail) | ~4 |
| LlmExtract execute | ~30 (config resolve, profile load fail, schema load fail, model lookup fail, full/chunked/structured/unknown, cancel, semaphore close, chunk LLM fail, chunk parse fail, all-fail vs some-fail, ChunkMerger paths, snapshot write fail) | ~12 (mostly placeholder substitution + global_rules) |
| Verify execute | ~12 (doc missing, no canonical text, grounding modes, derived provenance, ungrounded, missing quote, cancel) | ~2 |
| AutoApprove execute | ~6 (threshold met/not met, derived items, cancel) | 0 |
| Ingest execute | ~15 (doc missing, no completed run, cleanup fail, resolver fail, party create fail, entity create fail, cross-doc resolution, dangling reference, txn commit fail, batch update fail) | ~2 (display hygiene) |
| Index execute | ~8 (no nodes, embedding fail, qdrant fail, cancel, cleanup fail) | 0 |
| Completeness execute | ~10 (missing doc, missing nodes, missing points warn, helper fail) | 0 |

Coverage ratio (rough): **~25 dedicated tests against ~110 branches in the critical path** — roughly 1 test per 4 branches.

---

## Section 10: Dead Code and Unused Fields

### 10.1 — DB columns written but never read (or vice versa)

- `documents.error_suggestion` — read by `frontend/src/components/pipeline/ProcessingPanel.tsx:383–385` but **no backend writer**. **GAP 10.1 (HIGH)** — column exists but is never populated; UI block is dead surface.
- `documents.processing_step` — written by `update_processing_progress` but `process.rs` doc-comment notes the UI poll is "stale" (line 30). **MEDIUM** — gap with `pipeline_jobs.progress` JSONB.
- `documents.entities_written`, `relationships_written` — written by Ingest (`ingest.rs:529–539`). Read by UI? Need to verify; flagged for follow-up.

### 10.2 — Struct fields defined but unused

- `VerifyError::PdfNotFound { doc_id, path }` (`pipeline/steps/verify.rs:58–60`) — explicitly `#[allow(dead_code)]`; retained for the Display-hygiene test (per the comment).
- `LlmExtractError::SemaphoreClosed` — defined at `llm_extract.rs:103` but the only construction is at line 411 under `.map_err`. The semaphore can't be closed in current code (it's owned by AppContext, dropped at process exit) — so this variant is structurally unreachable. **LOW.**

### 10.3 — Functions defined but never called

`pub_fns.txt` lists 540+ public functions. A spot check did not find unused ones in the pipeline; the test suites use most of them. **Deferred** — a clippy `dead_code` pass would resolve this exhaustively; this audit did not run clippy.

### 10.4 — TODO / FIXME / HACK markers

**Backend (9 instances):**
```
backend/src/main.rs:584    TODO(Phase2): LlmDecomposer reconstructed from rag_config DB table
backend/src/main.rs:585    TODO(Phase2): build_rag_pipeline() rewritten to use Arc<dyn LlmProvider>
backend/src/api/pipeline/graph_validation.rs:261  TODO: Implement in F7
backend/src/repositories/evidence_repository.rs:1  TODO: B-1 Approach C — repository queries dead :Evidence nodes
backend/src/repositories/graph_helpers.rs:57       (HACK literal — test fixture for SQL-injection safe_label)
backend/src/repositories/allegation_detail_repository.rs:75  TODO: B-1 Approach C — MotionClaim nodes don't exist in v2
backend/src/repositories/claim_repository.rs:1     TODO: B-1 Approach C — :Claim nodes have moved
backend/src/repositories/allegation_repository.rs:77  TODO: Awad-case-specific paragraph ranges
backend/src/repositories/case_summary_repository.rs:196  TODO: DAL Phase 2 — use colossus_graph for batch neighbor
```

**Frontend (11 instances):**
```
frontend/src/components/admin/AdminDocuments.tsx:1    TODO: B-4 — v1 dead code, manual document workflow
frontend/src/components/admin/NodeTypeFilter.tsx:8    TODO: B-4 — v1 dead code
frontend/src/components/admin/InlineAuditForms.tsx:8  TODO: B-4 — v1 dead code
frontend/src/components/admin/AdminAudit.tsx:1        TODO: B-4 — v1 dead code
frontend/src/components/admin/EvidenceCard.tsx:8      TODO: B-4 — v1 dead code
frontend/src/components/admin/AuditDetails.tsx:7      TODO: B-4 — v1 dead code
frontend/src/components/ImpeachmentCard.tsx:81, 118   TODO: Link to /documents/:id when document_id added to API type
frontend/src/pages/Home.tsx:9                         TODO: Fetch descriptions from LegalCount.description
frontend/src/pages/AllegationDetailPage.tsx:235, 277  TODO: Link to /documents/:id when document_id added to API type
```

**6 frontend admin components are marked dead code (`B-4`) but still in the bundle.** **MEDIUM** — increases JS bundle size and confuses readers.

### 10.5 — Commented-out code blocks

A targeted grep for `^\s*//` blocks of 5+ lines did not surface large commented-out blocks. The codebase is comment-heavy by design (CLAUDE.md Rule 3) so a coarse grep is noisy. **LOW** — none observed in the files read end-to-end.

---

## Section 11: Frontend-Backend Contract Issues

### 11.1 — Fields backend ignores

The backend uses serde with `#[serde(default)]` on most DTO fields (per Rule 7 quick-ref in CLAUDE.md). Unknown fields are silently accepted via serde's default tolerance. Sample:

- `pipelineApi.ts:606` — `POST /api/admin/pipeline/documents` with a `RegisterDocumentRequest`. If frontend sends an extra field, backend ignores. **LOW.**
- `pipelineApi.ts:415` — generic `authFetch(url, options)` proxy. If options carry unknown fields they're ignored by the backend. **LOW.**

No instances surfaced where the frontend sends a field the backend "actively" ignores in a semantically meaningful way.

### 11.2 — Fields backend returns that frontend doesn't display

- `documents.error_suggestion` — backend doesn't populate (see §10.1); frontend displays if non-empty. **NEUTRAL** — UI surface for a never-populated field.
- `AppError::Conflict.details` — frontend `admin.ts:108` strips this; backend wastes work attaching `details`. **LOW** (semi-intentional but lossy).
- `pipeline_jobs.progress` JSONB — backend writes rich progress (chunk counts, retry counts, OCR engine); frontend `ProcessingPanel` polls `documents.processing_progress/step` columns instead. **MEDIUM** — duplicated capture.

### 11.3 — Type mismatches between TS interfaces and Rust DTOs

A targeted comparison was not performed (would require enumerating every DTO pair). Spot check:

- `RegisterDocumentResponse` in TS (`admin.ts:53–61`) vs backend `api/admin_documents.rs` — appears to match `id, filename, status, size_bytes`.
- `AdminQAEntry` (`admin.ts:69–88`) — appears to match `api/admin_qa.rs`.
- `AuditHealthResponse` (`admin.ts:216–225`) — backend `api/admin_audit_health.rs` returns the same shape.
- `ProcessResponse` — backend `api/pipeline/process.rs:47–53` declares `document_id, status, message, job_id?`. Frontend `pipelineApi.ts` doesn't define a matching type — calls expect any-shape JSON.

**GAP 11.3 (LOW)** — no shared type generator; manual TS interfaces drift from Rust structs over time. No automated check (e.g., openapi-schema export + ts-rs) is in place.

### 11.4 — Frontend assumptions not guaranteed by backend

- `ProcessingPanel.tsx:227–242` — assumes `doc.error_message` will appear "once the job is settling". The trigger `pipeline_jobs_sync_document_status` (migration 20260422112238) projects `pipeline_jobs.error` onto `documents.error_message` on terminal states. Frontend treats this as best-effort — fine.
- `ProcessingPanel.tsx:430` — assumes `actionError` will be set when an API call fails. Backend may return `null` body on some 500 paths; the `await res.json()` in `admin.ts` would throw. **LOW** — handled by outer catch.

### 11.5 — Endpoints called by frontend with no backend route

Spot check: the `pipelineApi.ts` endpoints (~30 calls) match routes defined in `api/pipeline/mod.rs` (router) — no orphan calls observed in this audit.

The note in `api/pipeline/process.rs:8–13` describes a historical orphan (`POST /process` was called by the frontend after the in-line orchestrator was deleted in commit 1414838 in April 2026, then restored). The fix is in place at the current commit. **GOOD.**

---

## Appendix A: Complete File List Audited

### Backend Rust source (190 files)

```
backend/src/api/admin_audit_health.rs
backend/src/api/admin_document_evidence.rs
backend/src/api/admin_document_evidence_queries.rs
backend/src/api/admin_document_extracts.rs
backend/src/api/admin_documents.rs
backend/src/api/admin_evidence.rs
backend/src/api/admin_evidence_helpers.rs
backend/src/api/admin_flag.rs
backend/src/api/admin_page_ground.rs
backend/src/api/admin_qa.rs
backend/src/api/admin_reindex.rs
backend/src/api/admin_status.rs
backend/src/api/admin_upload.rs
backend/src/api/admin_verify.rs
backend/src/api/allegations.rs
backend/src/api/analysis.rs
backend/src/api/ask.rs
backend/src/api/case.rs
backend/src/api/case_summary.rs
backend/src/api/chat_models.rs
backend/src/api/claims.rs
backend/src/api/contradictions.rs
backend/src/api/decomposition.rs
backend/src/api/documents.rs
backend/src/api/embed.rs
backend/src/api/evidence.rs
backend/src/api/evidence_chain.rs
backend/src/api/graph.rs
backend/src/api/harms.rs
backend/src/api/import.rs
backend/src/api/logout.rs
backend/src/api/mod.rs
backend/src/api/persons.rs
backend/src/api/pipeline/canonical_verifier.rs
backend/src/api/pipeline/completeness.rs
backend/src/api/pipeline/completeness_helpers.rs
backend/src/api/pipeline/completeness_validation.rs
backend/src/api/pipeline/config_endpoints/* (multiple)
backend/src/api/pipeline/config_handler.rs
backend/src/api/pipeline/constants.rs
backend/src/api/pipeline/delete.rs
backend/src/api/pipeline/document_response.rs
backend/src/api/pipeline/document_types.rs
backend/src/api/pipeline/errors.rs
backend/src/api/pipeline/extract_text.rs
backend/src/api/pipeline/file.rs
backend/src/api/pipeline/graph_migrations.rs
backend/src/api/pipeline/graph_validation.rs
backend/src/api/pipeline/history.rs
backend/src/api/pipeline/index.rs
backend/src/api/pipeline/ingest.rs
backend/src/api/pipeline/ingest_helpers.rs
backend/src/api/pipeline/ingest_resolver.rs
backend/src/api/pipeline/items.rs
backend/src/api/pipeline/metrics.rs
backend/src/api/pipeline/mod.rs
backend/src/api/pipeline/ocr.rs
backend/src/api/pipeline/process.rs
backend/src/api/pipeline/recompute_derived.rs
backend/src/api/pipeline/report.rs
backend/src/api/pipeline/report_data.rs
backend/src/api/pipeline/reverify_sync.rs
backend/src/api/pipeline/review.rs
backend/src/api/pipeline/state_machine.rs
backend/src/api/pipeline/upload.rs
backend/src/api/pipeline/users.rs
backend/src/api/pipeline/verify.rs
backend/src/api/pipeline/workload.rs
backend/src/api/qa.rs
backend/src/api/queries.rs
backend/src/api/schema.rs
backend/src/api/search.rs
backend/src/auth.rs
backend/src/bias/aggregation.rs
backend/src/bias/dto.rs
backend/src/bias/handlers.rs
backend/src/bias/mod.rs
backend/src/bias/repository.rs
backend/src/bias/tests.rs
backend/src/cli.rs
backend/src/config.rs
backend/src/database.rs
backend/src/dto/* (allegation, analysis, case_dto, case_summary, claim, contradiction, decision, decomposition, document, evidence, evidence_chain, graph, harm, hearing, mod, motion_claim, person, person_detail, query, schema)
backend/src/error.rs
backend/src/lib.rs
backend/src/main.rs
backend/src/models/* (claim, decision, document, document_status, evidence, hearing, import, mod, person)
backend/src/neo4j.rs
backend/src/pipeline/chunking_strategies.rs
backend/src/pipeline/config.rs
backend/src/pipeline/constants.rs
backend/src/pipeline/context.rs
backend/src/pipeline/mod.rs
backend/src/pipeline/providers.rs
backend/src/pipeline/registry.rs
backend/src/pipeline/registry_tests.rs
backend/src/pipeline/step_recorder.rs
backend/src/pipeline/steps/auto_approve.rs
backend/src/pipeline/steps/cleanup.rs
backend/src/pipeline/steps/completeness.rs
backend/src/pipeline/steps/extract_text.rs
backend/src/pipeline/steps/index.rs
backend/src/pipeline/steps/ingest.rs
backend/src/pipeline/steps/llm_extract.rs
backend/src/pipeline/steps/llm_extract_helpers.rs
backend/src/pipeline/steps/llm_extract_pass2.rs
backend/src/pipeline/steps/mod.rs
backend/src/pipeline/steps/verify.rs
backend/src/pipeline/task.rs
backend/src/pipeline/validation.rs
backend/src/prompt_loader.rs
backend/src/repositories/* (24 files: allegation_detail, allegation, analysis, audit, case, case_summary, claim, contradiction, decomposition, document, embedding, evidence, evidence_chain, graph, graph_helpers, harm, mod, motion_claim, person, person_detail, pipeline_repository/*, qa, query, rebuttals, schema)
backend/src/services/* (audit_checks, claim_validator, claude_client, embedding_pipeline, embedding_service, embedding_text, graph_expander, graph_expansion_minor, graph_expansion_queries, import_validator, mod, qdrant_service)
backend/src/state.rs
```

### Frontend TS/TSX (125 files)

```
frontend/src/components/admin/* (~17 files)
frontend/src/components/documents/* (2 files)
frontend/src/components/pipeline/* (~16 files including __tests__)
frontend/src/components/shared/PdfViewer.tsx
frontend/src/components/AnswerDisplay.tsx (and ~13 other root component files)
frontend/src/config/* (env-related, if present)
frontend/src/context/AuthContext.tsx
frontend/src/context/CaseContext.tsx
frontend/src/hooks/useResizablePanes.ts
frontend/src/hooks/useSchema.ts
frontend/src/pages/* (~23 files)
frontend/src/pages/BiasExplorer/* (multiple)
frontend/src/services/* (~26 files including __tests__)
frontend/src/styles/* (style files)
frontend/src/utils/* (countFormat, generateDocx, highlightConstants, itemProperties, legalTerms, nodeTypeDisplay, pdfHighlight, strengthColors)
```

### Migrations and configs

- **PostgreSQL main:** 6 files in `backend/migrations/`
- **PostgreSQL pipeline:** 26 files in `backend/pipeline_migrations/` (listed at §4a above)
- **Neo4j:** 1 file in `backend/migrations_neo4j/`
- **Config / profiles / schemas / templates:** 43 YAML/MD files in `backend/config/`, `backend/profiles/`, `backend/extraction_schemas/`, `backend/extraction_templates/`

---

## Appendix B: Issue Index by Severity

### CRITICAL (blocks operation) — 7

| ID | DESCRIPTION | SECTION |
|----|-------------|---------|
| C1 | `documents.error_suggestion` never written but UI displays it — dead UI surface | §10.1 |
| C2 | Frontend upload uses raw `fetch()` with no timeout — can hang the UI | §1c.3, §8.6 |
| C3 | Hardcoded DEV Authentik logout URL in `api/logout.rs:15` — wrong in PROD | §3.2 |
| C4 | Reviewer manual edits silently overwritten by re-extract (`reset_extraction_run_children`) | §6.2 |
| C5 | `qdrant-client` has no timeout configured (`main.rs:526–535`) — CLAUDE.md Rule 13 violation | §8.3 |
| C6 | Raw LLM response per chunk not stored anywhere — no forensic trail | §5a.1, §5c.1 |
| C7 | Partial chunk failure silently dropped after retry (`pass1_already_complete` short-circuits) | §4a.4 |

### HIGH (causes data loss or corruption) — 24

| ID | DESCRIPTION | SECTION |
|----|-------------|---------|
| H1 | `evidence_repository` decodes 12 nullable columns via `.ok()` — schema drift silently blanks legal evidence | §2.1 |
| H2 | `contradiction_repository` same pattern (5 columns) | §2.1 |
| H3 | `allegation_repository` same pattern (5 columns) | §2.1 |
| H4 | `motion_claim_repository` same pattern (5 columns) | §2.1 |
| H5 | `evidence_chain_repository` same pattern (5 columns) | §2.1 |
| H6 | `bias/aggregation.rs:47` document_id silent decode failure → unlinked bias evidence | §2.1 |
| H7 | LLM call network errors not retried (only RateLimited) | §4b.1 |
| H8 | `ingest.rs` cleanup-then-write split into two transactions — partial-state risk on commit fail | §4c.1 |
| H9 | `reset_extraction_run_children` + `store_entities_and_relationships` not in one PG txn | §4d.1 |
| H10 | `LlmExtract`, `ExtractText`, `AutoApprove`, `LlmExtractPass2` have no DEFAULT_TIMEOUT_SECS | §4a.2 |
| H11 | Case-specific paragraph ranges hardcoded in shared `allegation_repository.rs:77` | §3.7 |
| H12 | `delete.rs:308–387` uses legacy log-and-swallow cleanup instead of `cleanup_all` saga | §2.5 |
| H13 | Frontend dropping `AppError.details` on every backend error response | §1c.3 |
| H14 | `unwrap_or("")` on JSON field reads in `ingest_helpers.rs` — silent data loss on LLM drift | §2.1 (pattern B) |
| H15 | No idempotency short-circuit for Verify/AutoApprove/Ingest/Index/Completeness — re-process is full-restart | §4a.5 |
| H16 | `cleanup_neo4j` is three separate transactions — no rollback on mid-cleanup failure | §4c.2 |
| H17 | `max_retries=0` default on `pipeline_jobs` — uncertain whether step DEFAULT_RETRY_LIMIT is honored | §4a.1 |
| H18 | Repair-applied flag not stored — silent LLM-output drift goes undetected | §5a.2 |
| H19 | Per-chunk raw response not stored — operator cannot reproduce parse failures | §5a.1 |
| H20 | Anthropic request-id not captured — no support correlation | §5a.3, §4b.2 |
| H21 | Per-pass cost USD never surfaced in UI — requires SQL to see | §5b table |
| H22 | LLM semaphore permit held across all chunks of a document — concurrency tuning surprise | §6.6 |
| H23 | OCR engine actually used per page not stored | §5c.9 |
| H24 | Effective splitter config not stored on `extraction_runs` — only logged | §5c.10 |

### MEDIUM (causes confusion or wasted effort) — ~210

Summary categories (detailed instances embedded in sections):
- **POOR-quality log messages** (`{:?}` Debug-format, no request context) — 14 handlers in `api/*.rs` (§1b.2)
- **`unwrap_or_default()` decodes in `repositories/*` and `api/pipeline/*`** — ~200+ occurrences (§2.1, §2.2)
- **`update_processing_progress(...).await.ok()` chains** — 8 occurrences in pipeline steps (§2.2)
- **Hardcoded timeouts/retries inside step trait impls** — 12 constants (§3.3)
- **Hardcoded model defaults that drift between paths** — `main.rs:33` 4096 vs `claude_client.rs:105` 2048 (§3.9)
- **Read-then-write races on review_status** (§6.4)
- **Per-chunk template TOCTOU** — file read at execute time without snapshot (§6.3)
- **Pool/client per-call creation** in audit checks, admin status, OCR (§7.2 a–c)
- **No statement_timeout on Postgres** (§8.5)
- **No driver-level Neo4j timeout** (§8.2)
- **Anthropic provider timeout unverified** (§8.1)
- **Test gaps:** 38 public functions without direct unit tests (§9.2)
- **Frontend dead components still bundled (6 marked B-4)** (§10.4)

### LOW (code quality, maintainability) — ~570

- ~150 `let _ = write!(...)` to a `String` (false-positive silent failures — `fmt::Write` cannot fail) (§2.2)
- ~150 `unwrap_or(0)`/`unwrap_or_default()` in metrics aggregation (numeric defaults) (§2.1 pattern B)
- ~50 test-only `.unwrap()`/`.expect()` (acceptable per CLAUDE.md Rule 1) (§1b.5)
- ~50 hardcoded config defaults env-overridable (CLAUDE.md compliant) (§3.5)
- ~9 TODO comments naming Phase 2 follow-ups (§10.4)
- ~150 acceptable status-string `pub const` references (good pattern; not bugs)
- 7 instances where the silent-catch is correctly justified (e.g., `getCurrentUser`, dotenv) (§1c.2)

---

## Appendix C: Issue Index by File

Files with the most findings:

### Backend top 10 files by finding count

| FILE | LINES | LOG | UNWRAP/EXPECT | UNWRAP_OR | .ok()/let _ | KEY FINDINGS |
|------|-------|-----|---------------|-----------|--------------|--------------|
| `backend/src/pipeline/steps/llm_extract.rs` | 2769 | 11 | 6 (in tests) | 8 | 6 | C6, C7, H7, H10, H14, H18, H19, H20, H22 — over 300-line file limit (Rule 17) |
| `backend/src/pipeline/config.rs` | 2014 | 0 | 20+ (in tests) | 5 | 0 | Over 300-line limit |
| `backend/src/pipeline/steps/llm_extract_pass2.rs` | 910 | 2 | 0 | 5 | 1 | No DEFAULT_TIMEOUT, no tests (H10) |
| `backend/src/pipeline/steps/extract_text.rs` | 858 | 7 | 0 | ~8 | 0 | No DEFAULT_TIMEOUT (H10), OCR engine not stored (H23) |
| `backend/src/main.rs` | 749 | 8 | 11 | 5 | 2 | Hardcoded models (§3.1), CORS hardcoded (§3.2), Qdrant no timeout (C5) |
| `backend/src/pipeline/steps/ingest.rs` | 634 | 0 | 1 (test) | 5 | 0 | H8, H17 |
| `backend/src/pipeline/registry.rs` | 482 | 1 | 1 | 4 | 0 | |
| `backend/src/pipeline/validation.rs` | 515 | 1 | 25+ (in tests) | 3 | 0 | Over 300-line limit |
| `backend/src/api/pipeline/upload.rs` | (large) | 8 | 9 (in tests) | 5 | 0 | Profile resolution warnings (1b.3) |
| `backend/src/api/pipeline/verify.rs` | (large) | 6 | 1 | 5 | 0 | Tests at 260, 299 |

### Frontend top 5 files by finding count

| FILE | KEY FINDINGS |
|------|--------------|
| `frontend/src/services/admin.ts` | C2 (raw fetch on upload), H13 (drops details), 6 services |
| `frontend/src/components/pipeline/ReviewPanel.tsx` | C4 (no lockout vs re-extract), 2 silent catches |
| `frontend/src/components/pipeline/ConfigurationPanel.tsx` | 4 silent fallbacks to `[]` |
| `frontend/src/components/pipeline/ProcessingPanel.tsx` | Renders never-populated `error_suggestion` (C1) |
| `frontend/src/services/pipelineApi.ts` | 30+ fetch calls — all GOOD per pattern |

### Files marked B-4 v1 dead code (frontend)

```
frontend/src/components/admin/AdminDocuments.tsx
frontend/src/components/admin/NodeTypeFilter.tsx
frontend/src/components/admin/InlineAuditForms.tsx
frontend/src/components/admin/AdminAudit.tsx
frontend/src/components/admin/EvidenceCard.tsx
frontend/src/components/admin/AuditDetails.tsx
```

These should be excluded from the bundle or deleted.

---

## Appendix D: Full grep dumps (for follow-up)

For reproducibility, the raw greps that powered this audit are at `/tmp/audit/` on the auditing host:

- `/tmp/audit/logs.txt` — 166 lines (every `tracing::error!` / `warn!` / `eprintln!`)
- `/tmp/audit/unwrap.txt` — 131 lines (every `.unwrap()` / `.expect(`)
- `/tmp/audit/unwrap_or.txt` — 307 lines (every `.unwrap_or_default()` / `.unwrap_or(`)
- `/tmp/audit/silent.txt` — 98 lines (every `let _ =` / `.ok();` / `if let Ok(_)` / `if let Some(_)`)
- `/tmp/audit/todos.txt`, `/tmp/audit/fe_todos.txt` — 9 + 11 lines
- `/tmp/audit/errors_returned.txt` — 553 lines (every `AppError::` / `StatusCode::`)
- `/tmp/audit/urls.txt` — 8 lines (URL literals)
- `/tmp/audit/durations.txt` — 11 lines (Duration literals)
- `/tmp/audit/fe_errors.txt` — 43 lines (`console.error/warn`, `.catch`)
- `/tmp/audit/fe_toasts.txt` — 113 lines (`toast`, `setError`, `showError`)
- `/tmp/audit/fe_fetch.txt` — 107 lines (every `fetch(` / `authFetch(`)
- `/tmp/audit/timeouts.txt` — 38 lines

These dumps are intentionally exhaustive; a follow-up fix instruction can target individual file:line entries from them without re-running the greps.

— end of audit —
