# Colossus-Legal Task Tracker

**Version:** 2.0  
**Updated:** 2025-12-20  
**Status:** Active Development

---

## Task Status Legend

| Status | Meaning |
|--------|---------|
| `TODO` | Not started |
| `IN_PROGRESS` | Currently being worked on |
| `REVIEW` | Code complete, awaiting review |
| `DONE` | Completed and merged to main |
| `BLOCKED` | Waiting on dependency |

---

## Feature Branch Naming Convention

```
feature/<phase>-<task>-<short-description>
```

Examples:
- `feature/P5-T5.1-claim-model-v2`
- `feature/P5-T5.3-import-endpoint`

---

## Phase Summary

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 0 | Project Initialization | DONE |
| Phase 1 | Backend + Frontend Foundations | DONE |
| Phase 2 | Claims API + UI | DONE |
| Phase 3 | Document API + UI | DONE (L1) |
| Phase 4 | Evidence, Person, Hearing, Decision | TODO |
| **Phase 5** | **Schema v2 + Claims Import** | **IN_PROGRESS** |
| Phase 6 | Relationship APIs | TODO |
| Phase 7 | Analysis & Visualization | TODO |

---

## Phase 5 — Schema v2 + Claims Import (CURRENT)

**Goal:** Update to v2 schema and implement claims import workflow.

**Branch Strategy:** 
- Feature branch per task group
- Merge to `develop` after task completion
- Merge `develop` to `main` after phase completion

---

### Feature: F5.1 — Schema Migration & Neo4j Cleanup

**Branch:** `feature/P5-F5.1-schema-v2-migration`

| Task | Description | Status | Assignee |
|------|-------------|--------|----------|
| T5.1.1 | Clear Neo4j test data | DONE | Roman | 2025-12-20 |
| T5.1.2 | Update Claim model to v2 schema | DONE | Claude Code | 2025-12-20 |
| T5.1.3 | Update Document model to v2 schema | DONE | Claude Code | 2025-12-20 |
| T5.1.4 | Add Person model with v2 fields | DONE | Claude Code | 2025-12-20 |
| T5.1.5 | Add Evidence model with v2 fields | DONE | Claude Code | 2025-12-20 |
| T5.1.6 | Create Neo4j constraints and indexes | DONE | Claude Code | 2025-12-20 |

**Exit Criteria for F5.1:**
- [ ] Neo4j is empty (no test data)
- [ ] All models compile with v2 fields
- [ ] Neo4j constraints created for unique IDs
- [ ] `cargo build` passes
- [ ] `cargo test` passes (existing tests updated)

---
### Technical Debt from T5.1.2
- [ ] `tests/claims_list.rs` — 2 tests ignored, need v1 claims migrated
- [ ] `tests/claims_validation.rs` — 1 test ignored, needs ClaimRepository v2
- [ ] `ClaimRepository::create_claim()` — creates v1 claims, needs update in T5.4.x
---

### Feature: F5.2 — Import Validation Endpoint

**Branch:** `feature/P5-F5.2-import-validation`

**Dependency:** F5.1 must be DONE

| Task | Description | Status | Assignee |
|------|-------------|--------|----------|
| T5.2.1 | Create import DTOs (request/response structs) | TODO | Sonnet |
| T5.2.2 | Implement JSON schema validation | TODO | Sonnet |
| T5.2.3 | Implement claim field validation | TODO | Sonnet |
| T5.2.4 | Implement duplicate detection (by ID) | TODO | Sonnet |
| T5.2.5 | Create POST /api/import/validate endpoint | TODO | Sonnet |
| T5.2.6 | Write unit tests for validation | TODO | Sonnet |
| T5.2.7 | Write integration tests for endpoint | TODO | Sonnet |

**Exit Criteria for F5.2:**
- [ ] Endpoint accepts JSON file upload
- [ ] Returns validation errors for invalid JSON
- [ ] Returns validation errors for missing required fields
- [ ] Returns validation errors for invalid enum values
- [ ] Detects duplicate claim IDs within file
- [ ] All tests pass
- [ ] Manual test with Awad claims JSON succeeds

---

### Feature: F5.3 — Import Execution Endpoint

**Branch:** `feature/P5-F5.3-import-execution`

**Dependency:** F5.2 must be DONE

| Task | Description | Status | Assignee |
|------|-------------|--------|----------|
| T5.3.1 | Implement Case node creation | TODO | Sonnet |
| T5.3.2 | Implement Document node creation | TODO | Sonnet |
| T5.3.3 | Implement Person node creation (MERGE) | TODO | Sonnet |
| T5.3.4 | Implement Claim node creation | TODO | Sonnet |
| T5.3.5 | Implement Evidence node creation | TODO | Sonnet |
| T5.3.6 | Implement relationship creation | TODO | Sonnet |
| T5.3.7 | Create POST /api/import/execute endpoint | TODO | Sonnet |
| T5.3.8 | Implement transaction rollback on error | TODO | Sonnet |
| T5.3.9 | Write unit tests for node creation | TODO | Sonnet |
| T5.3.10 | Write integration tests for import | TODO | Sonnet |

**Exit Criteria for F5.3:**
- [ ] Endpoint accepts validated import request
- [ ] Creates all nodes (Case, Document, Person, Claim, Evidence)
- [ ] Creates all relationships (APPEARS_IN, MADE_BY, AGAINST, etc.)
- [ ] Returns import report with counts
- [ ] Rolls back on any error (no partial imports)
- [ ] All tests pass
- [ ] Manual test: Import Awad claims, verify in Neo4j Browser

---

### Feature: F5.4 — Update Existing API Endpoints

**Branch:** `feature/P5-F5.4-api-updates`

**Dependency:** F5.3 must be DONE

| Task | Description | Status | Assignee |
|------|-------------|--------|----------|
| T5.4.1 | Update GET /api/claims to return v2 fields | TODO | Sonnet |
| T5.4.2 | Update GET /api/claims/:id to return v2 fields | TODO | Sonnet |
| T5.4.3 | Update GET /api/documents to return v2 fields | TODO | Sonnet |
| T5.4.4 | Add GET /api/persons endpoint | TODO | Sonnet |
| T5.4.5 | Add GET /api/evidence endpoint | TODO | Sonnet |
| T5.4.6 | Update API documentation | TODO | Sonnet |
| T5.4.7 | Write/update integration tests | TODO | Sonnet |

**Exit Criteria for F5.4:**
- [ ] All endpoints return v2 schema fields
- [ ] New endpoints for persons and evidence work
- [ ] API documentation is current
- [ ] All tests pass
- [ ] Manual test: Verify Awad data appears correctly via API

---

### Feature: F5.5 — Frontend Updates

**Branch:** `feature/P5-F5.5-frontend-v2`

**Dependency:** F5.4 must be DONE

| Task | Description | Status | Assignee |
|------|-------------|--------|----------|
| T5.5.1 | Update Claims list to show v2 fields | TODO | Sonnet |
| T5.5.2 | Create Claim detail view (show quote, source) | TODO | Sonnet |
| T5.5.3 | Update Documents list to show v2 fields | TODO | Sonnet |
| T5.5.4 | Create Document detail view | TODO | Sonnet |
| T5.5.5 | Create People page | TODO | Sonnet |
| T5.5.6 | Create Evidence page | TODO | Sonnet |
| T5.5.7 | Create Import page (file upload + results) | TODO | Sonnet |

**Exit Criteria for F5.5:**
- [ ] Claims page shows: quote (truncated), category, severity, status
- [ ] Claim detail shows: full quote, source document link, parties
- [ ] Documents page shows: title, type, filed date
- [ ] People page lists all persons with roles
- [ ] Evidence page lists exhibits
- [ ] Import page allows file upload and shows results
- [ ] Manual test: Full walkthrough of Awad case data

---

## Task Detail Template

Each task, when started, should have a detail section:

```markdown
### T5.X.X — [Task Name]

**Status:** IN_PROGRESS  
**Branch:** feature/P5-FX.X-description  
**Assignee:** Sonnet (Claude Code)

**Description:**
[What needs to be done]

**Files to Modify:**
- path/to/file1.rs
- path/to/file2.rs

**Files to Create:**
- path/to/new_file.rs

**Dependencies:**
- T5.X.X must be complete

**Pre-Coding Checklist:**
- [ ] Referenced modules exist
- [ ] Required dependencies in Cargo.toml
- [ ] Understand existing code structure

**Implementation Steps:**
1. Step one
2. Step two
3. Step three

**Tests Required:**
- [ ] Unit test: test_name_1
- [ ] Unit test: test_name_2
- [ ] Integration test: test_name_3

**Exit Criteria:**
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] All tests pass
- [ ] No compiler warnings
- [ ] Code reviewed by Opus

**Completion Checklist:**
- [ ] Code complete
- [ ] Tests written and passing
- [ ] Manual verification done
- [ ] PR created
- [ ] Reviewed and approved
- [ ] Merged to develop
```

---

## Completed Phases (Reference)

### Phase 0 — Project Initialization ✓
- T0.1: Repository setup — DONE
- T0.2: Documentation structure — DONE

### Phase 1 — Backend + Frontend Foundations ✓
- T1.1: Rust backend scaffold (Axum) — DONE
- T1.2: Neo4j connection — DONE
- T1.3: React frontend scaffold — DONE
- T1.4: API proxy setup — DONE

### Phase 2 — Claims API + UI ✓
- T2.1: Claim model (v1) — DONE
- T2.2: Claims CRUD endpoints — DONE
- T2.3: Claims list UI — DONE
- T2.4: Claim detail UI — DONE

### Phase 3 — Document API + UI (L1) ✓
- T3.1: Document model (v1) — DONE
- T3.2: Documents list endpoint — DONE
- T3.3: Documents list UI — DONE

---

## Future Phases (Not Yet Planned in Detail)

### Phase 4 — Core Graph Expansion
- Evidence API
- Person API
- Hearing API
- Decision API

### Phase 6 — Relationship APIs
- Link claims to documents
- Link claims to evidence
- Link claims to persons
- Graph traversal queries

### Phase 7 — Analysis & Visualization
- Timeline view
- Relationship graph visualization
- Claim status dashboard
- Search and filter

---

## Notes

- Phase 10 (document-processor) has been removed — extraction is now done by Claude Opus via chat
- All extracted claims JSON files are stored in `~/Documents/colossus-legal-data/extracted/`
- Schema v2 documentation is in `docs/DATA_MODEL_v2.md`
- Import workflow specification is in `docs/CLAIMS_IMPORT_WORKFLOW.md`
