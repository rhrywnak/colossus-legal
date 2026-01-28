# Colossus-Legal — TASK TRACKER

A structured task index for development.

---

# Phase 0 — Project Initialization ✅ COMPLETE

### T0.1 — Repository Bootstrap — DONE
### T0.2 — Architecture & Workflow Docs — DONE
### T0.3 — Codex Safety Bundle — DONE

---

# Phase 1 — Foundations ✅ COMPLETE

### T1.1 — Backend Skeleton (Axum) — DONE
### T1.2 — Neo4j Integration & Test Harness — DONE
### T1.3 — Frontend Skeleton (React/Vite) — DONE
### T1.5 — Backend Dev Env Configuration — DONE (2025-11-26)

---

# Phase 2 — Claims Slice ✅ COMPLETE

### T2.1a — Claims API L0 (Skeleton) — DONE
### T2.1b — Claims API L1 (Neo4j) — DONE
### T2.1c — Claims API L2 (Validation) — DONE
### T2.1d — Claims API L3 (Analysis) — DONE
### T2.2a — Claims UI L0 (Skeleton) — DONE
### T2.2b — Claims UI L1 (Integration) — DONE
### T2.3 — Claims E2E Integration — DONE (2025-11-27)

---

# Phase 3 — Document Slice (In Progress)

### T3.1a — Document API L0 (Skeleton) — DONE (2025-12-03)
### T3.1b — Document API L1 (Neo4j) — DONE (2025-12-03)
### T3.1c — Document API L2 (Validation) — PLANNED
### T3.1d — Document API L3 (Analysis) — PLANNED
### T3.2a — Document UI L0 (Skeleton) — DONE (2025-12-03)
### T3.2b — Document UI L1 (Integration) — DONE (2025-12-03)
### T3.3 — Document Slice Integration — DONE (2025-12-02)

---

# Phase 4 — Core Graph Expansion — FUTURE

### T4.1 — Evidence API + UI — FUTURE
### T4.2 — Person API + UI — FUTURE
### T4.3 — Hearing API + UI — FUTURE
### T4.4 — Decision API + UI — FUTURE

---

# Phase 5 — Schema v2 + Claims Import (Active)

> **Goal:** Update models to v2 schema and implement claims import pipeline.

## Feature F5.1 — Schema v2 Migration ✅ COMPLETE

**Branch:** `feature/P5-F5.1-schema-v2-migration` (merged to main 2025-12-20)

| Task | Description | Status | Date |
|------|-------------|--------|------|
| T5.1.1 | Clear Neo4j test data | DONE | 2025-12-20 |
| T5.1.2 | Update Claim model to v2 schema | DONE | 2025-12-20 |
| T5.1.3 | Update Document model to v2 schema | DONE | 2025-12-20 |
| T5.1.4 | Add Person model with v2 fields | DONE | 2025-12-20 |
| T5.1.5 | Add Evidence model with v2 fields | DONE | 2025-12-20 |
| T5.1.6 | Create Neo4j constraints and indexes | DONE | 2025-12-20 |

**Deliverables:**
- ClaimCategory enum (19 variants)
- DocumentType enum (34 variants)
- PersonRole enum (14 variants)
- EvidenceKind enum (9 variants)
- 5 unique constraints, 10 indexes in Neo4j
- 20 unit tests passing

---

## Feature F5.2 — Import Validation Endpoint 🔄 IN PROGRESS

**Branch:** `feature/P5-F5.2-import-validation` (to be created)
**Dependency:** F5.1 ✅

| Task | Description | Status | Layer |
|------|-------------|--------|-------|
| T5.2.1 | Create import DTOs | DONE | 2025-12-23 | L0 |
| T5.2.2 | Implement JSON schema validation | DONE | 2025-12-23 | L1 |
| T5.2.3 | Implement claim field validation | DONE | 2025-12-23 | L1 |
| T5.2.4 | Implement duplicate detection | DONE | 2025-12-23 | L1 |
| T5.2.5 | Create POST /api/import/validate endpoint | DONE | 2025-12-23 | L1 |
| T5.2.6 | Integration tests | DONE | 2025-12-23 |

**Exit Criteria:**
- [x] Endpoint accepts JSON file upload
- [x] Returns validation errors for invalid JSON
- [x] Returns validation errors for missing required fields
- [x] Returns validation errors for invalid enum values
- [x] Detects duplicate claim IDs within file
- [x] All tests pass
- [x] Manual test with Awad claims JSON succeeds

---

## Feature F5.3 — Import Execution Endpoint — PLANNED

**Branch:** `feature/P5-F5.3-import-execution`
**Dependency:** F5.2

| Task | Description | Status |
|------|-------------|--------|
| T5.3.1 | Implement Case node creation | TODO |
| T5.3.2 | Implement Document node creation | TODO |
| T5.3.3 | Implement Person node creation (MERGE) | TODO |
| T5.3.4 | Implement Claim node creation | TODO |
| T5.3.5 | Implement Evidence node creation | TODO |
| T5.3.6 | Implement relationship creation | TODO |
| T5.3.7 | Create POST /api/import/execute endpoint | TODO |
| T5.3.8 | Implement transaction rollback on error | TODO |
| T5.3.9 | Write unit tests for node creation | TODO |
| T5.3.10 | Write integration tests for import | TODO |

---

## Feature F5.4 — Update Existing API Endpoints — PLANNED

**Dependency:** F5.3

| Task | Description | Status |
|------|-------------|--------|
| T5.4.1 | Update ClaimRepository to v2 schema | TODO |
| T5.4.2 | Update DocumentRepository to v2 schema | TODO |
| T5.4.3 | Add PersonRepository | TODO |
| T5.4.4 | Add EvidenceRepository | TODO |
| T5.4.5 | Fix ignored integration tests | TODO |

---

## Feature F5.5 — Frontend Import UI — PLANNED

**Dependency:** F5.3

| Task | Description | Status |
|------|-------------|--------|
| T5.5.1 | File upload component | TODO |
| T5.5.2 | Validation results display | TODO |
| T5.5.3 | Import preview page | TODO |
| T5.5.4 | Import progress display | TODO |
| T5.5.5 | Import report display | TODO |

---

# Phase 6-9 — FUTURE

See original TASK_TRACKER for details on:
- Phase 6: Analysis Layer
- Phase 7: Document Upload & Extraction
- Phase 8: AI Suggestion Pipeline
- Phase 9: Reporting & Visualization

---

# Technical Debt

### From F5.1 (ignored tests, need F5.4 to fix)
- [ ] `tests/claims_list.rs` — 2 tests ignored (v1 test data)
- [ ] `tests/claims_validation.rs` — 1 test ignored (ClaimRepository v1)
- [ ] `tests/documents_list.rs` — 1 test ignored (invalid doc_type)

---

# Phase Summary

| Phase | Status |
|-------|--------|
| Phase 0 | ✅ Complete |
| Phase 1 | ✅ Complete |
| Phase 2 | ✅ Complete |
| Phase 3 | 🔄 In Progress (L2+ remaining) |
| Phase 4 | ⏳ Future |
| Phase 5 | 🔄 In Progress (F5.2 starting) |
| Phase 6-9 | ⏳ Future |

---

# Notes

- One Task ID → one branch → one persona → one layer
- F5.x uses feature branches per Feature set
- L1+ tasks require tests
- Keep `main` deployable at all times

# End of TASK_TRACKER.md
