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

# Phase 2 — Query Layer ✅ COMPLETE (2026-01-29)

> **Goal:** Expose Neo4j database to end users (Marie Awad, Charles Penzien) through REST API and React UI.

## Feature F2.1 — Schema Discovery ✅ COMPLETE

| Task | Description | Status | Date |
|------|-------------|--------|------|
| T2.1.1 | GET /schema endpoint | DONE | 2026-01-29 |
| T2.1.2 | Dashboard UI with entity counts | DONE | 2026-01-29 |
| T2.1.3 | Clickable dashboard cards | DONE | 2026-01-29 |

## Feature F2.2 — Persons API + UI ✅ COMPLETE

| Task | Description | Status | Date |
|------|-------------|--------|------|
| T2.2.1 | GET /persons endpoint | DONE | 2026-01-29 |
| T2.2.2 | People page with role badges | DONE | 2026-01-29 |

## Feature F2.3 — Allegations API + UI ✅ COMPLETE

| Task | Description | Status | Date |
|------|-------------|--------|------|
| T2.3.1 | GET /allegations endpoint | DONE | 2026-01-29 |
| T2.3.2 | Allegations page with status badges | DONE | 2026-01-29 |
| T2.3.3 | Fix duplicate rows (legal_counts array) | DONE | 2026-01-29 |

## Feature F2.4 — Evidence API + UI ✅ COMPLETE

| Task | Description | Status | Date |
|------|-------------|--------|------|
| T2.4.1 | GET /evidence endpoint | DONE | 2026-01-29 |
| T2.4.2 | Evidence page with CRITICAL highlighting | DONE | 2026-01-29 |
| T2.4.3 | Source document links | DONE | 2026-01-29 |

## Feature F2.5 — Harms/Damages API + UI ✅ COMPLETE

| Task | Description | Status | Date |
|------|-------------|--------|------|
| T2.5.1 | GET /harms endpoint | DONE | 2026-01-29 |
| T2.5.2 | Damages page with totals | DONE | 2026-01-29 |
| T2.5.3 | Route /damages (user-friendly URL) | DONE | 2026-01-29 |

**Phase 2 Deliverables:**
- 5 backend endpoints
- 5 frontend pages (Dashboard, People, Allegations, Evidence, Damages)
- All 18 allegations PROVEN
- $40,258.61 in quantifiable damages displayed

---

# Phase 2.5 — Extended Query Layer ✅ COMPLETE (2026-01-29)

> **Goal:** Add MotionClaims and Contradictions to complete evidence chain visibility.

## Feature F2.6 — MotionClaims API + UI ✅ COMPLETE

| Task | Description | Status | Date |
|------|-------------|--------|------|
| T2.6.1 | GET /motion-claims endpoint | DONE | 2026-01-29 |
| T2.6.2 | Claims page with category badges | DONE | 2026-01-29 |
| T2.6.3 | Linked allegations and evidence | DONE | 2026-01-29 |

## Feature F2.7 — Contradictions API + UI ✅ COMPLETE

| Task | Description | Status | Date |
|------|-------------|--------|------|
| T2.7.1 | GET /contradictions endpoint | DONE | 2026-01-29 |
| T2.7.2 | Side-by-side comparison page | DONE | 2026-01-29 |

**Phase 2.5 Deliverables:**
- 2 backend endpoints
- 2 frontend pages (Claims, Contradictions)
- 26 motion claims displayed
- Contradiction comparison view

---

# Phase 2.6 — Graph Visualization 🔄 IN PROGRESS

> **Goal:** Visual evidence chains using React + dagre for hierarchical display.

## Feature F2.8 — Graph Visualization — PLANNED

**Branch:** `feature/graph-visualization` (created)

| Task | Description | Status |
|------|-------------|--------|
| T2.8.1 | Design graph data endpoint | TODO |
| T2.8.2 | GET /graph/legal-proof endpoint | TODO |
| T2.8.3 | GET /graph/damages endpoint | TODO |
| T2.8.4 | Install dagre in frontend | TODO |
| T2.8.5 | Create GraphViewer component | TODO |
| T2.8.6 | Integrate with hierarchy selector | TODO |
| T2.8.7 | Add "View Graph" buttons to pages | TODO |

**Supported Hierarchies:**
| Type | Pattern |
|------|---------|
| Legal Proof | Count → Allegation → Claim → Evidence |
| Damages | Count → Harm → Allegation → Evidence |
| Document | Document → Evidence → Claims |
| Party | Person → Evidence → Documents |

---

# Phase 3 — Document Slice (Partial)

### T3.1a — Document API L0 (Skeleton) — DONE (2025-12-03)
### T3.1b — Document API L1 (Neo4j) — DONE (2025-12-03)
### T3.1c — Document API L2 (Validation) — DEFERRED
### T3.1d — Document API L3 (Analysis) — DEFERRED
### T3.2a — Document UI L0 (Skeleton) — DONE (2025-12-03)
### T3.2b — Document UI L1 (Integration) — DONE (2025-12-03)
### T3.3 — Document Slice Integration — DONE (2025-12-02)

---

# Phase 5 — Schema v2 + Claims Import (Historical)

> Note: This phase was completed earlier. Import validation endpoint exists.

## Feature F5.1 — Schema v2 Migration ✅ COMPLETE (2025-12-20)
## Feature F5.2 — Import Validation Endpoint ✅ COMPLETE (2025-12-23)
## Feature F5.3 — Import Execution Endpoint — DEFERRED
## Feature F5.4 — Update Existing API Endpoints — SUPERSEDED by Phase 2
## Feature F5.5 — Frontend Import UI — DEFERRED

---

# Future Phases

### Phase 6 — Analysis Layer — FUTURE
### Phase 7 — Document Upload & Extraction — FUTURE
### Phase 8 — AI Suggestion Pipeline — FUTURE
### Phase 9 — Reporting & Visualization — FUTURE (partially in Phase 2.6)

---

# API Endpoints Summary

| Endpoint | Method | Purpose | Status |
|----------|--------|---------|--------|
| `/health` | GET | Health check | ✅ |
| `/api/status` | GET | Backend status | ✅ |
| `/schema` | GET | Database discovery | ✅ |
| `/persons` | GET | List persons | ✅ |
| `/allegations` | GET | List allegations | ✅ |
| `/evidence` | GET | List evidence | ✅ |
| `/harms` | GET | List harms/damages | ✅ |
| `/motion-claims` | GET | List motion claims | ✅ |
| `/contradictions` | GET | List contradictions | ✅ |
| `/documents` | GET | List documents | ✅ |
| `/documents/:id` | GET | Document detail | ✅ |
| `/documents` | POST | Create document | ✅ |
| `/documents/:id` | PUT | Update document | ✅ |
| `/claims` | GET | List claims (old) | ✅ |
| `/claims/:id` | GET | Claim detail | ✅ |
| `/claims` | POST | Create claim | ✅ |
| `/claims/:id` | PUT | Update claim | ✅ |
| `/import/validate` | POST | Validate import JSON | ✅ |
| `/graph/:type` | GET | Graph data for visualization | TODO |

---

# Frontend Pages Summary

| Route | Page | Status |
|-------|------|--------|
| `/` | Dashboard | ✅ |
| `/allegations` | Allegations | ✅ |
| `/claims` | Motion Claims | ✅ |
| `/documents` | Documents List | ✅ |
| `/documents/:id` | Document Detail | ✅ |
| `/evidence` | Evidence | ✅ |
| `/damages` | Harms/Damages | ✅ |
| `/people` | People | ✅ |
| `/contradictions` | Contradictions | ✅ |
| `/hearings` | Hearings | Placeholder |
| `/decisions` | Decisions | Placeholder |
| `/graph` | Graph Visualization | TODO |

---

# Technical Debt

### From F5.1 (ignored tests, need fixing)
- [ ] `tests/claims_list.rs` — 2 tests ignored (v1 test data)
- [ ] `tests/claims_validation.rs` — 1 test ignored (ClaimRepository v1)
- [ ] `tests/documents_list.rs` — 1 test ignored (invalid doc_type)

---

# Data Quality Issues

### DATA-001: Review CONTRADICTED_BY Relationships
**Priority:** High
**Type:** Data Quality
**Description:** Current CONTRADICTED_BY relationships don't represent actual logical contradictions. Example: "Camille claims Marie withdrew $140K" linked to "Emil demanded Nadia return $50K" - these are not direct contradictions.
**Action:** Review all 2 existing CONTRADICTED_BY relationships in Neo4j and correct or remove invalid ones.

### DATA-002: Identify Missing Contradictions
**Priority:** High
**Type:** Data Extraction
**Description:** Many actual contradictions exist in the case documents but haven't been captured. Need systematic review of:
- Sisters' affidavits vs caregiver affidavits
- Phillips/CFS interrogatory answers vs complaint allegations
- Sisters' claims vs CFS admissions

**Action:** Re-analyze documents specifically for contradictions.

### DATA-003: Refine Document Ingestion Process
**Priority:** Medium
**Type:** Process Improvement
**Description:** Current document ingestion doesn't reliably identify contradictions. Need improved extraction logic that:
- Compares claims across documents
- Identifies logical contradictions (not just related topics)
- Validates contradiction relationships before creating them

**Action:** Update extraction prompts/process for better contradiction detection.

---

# Database Statistics (2026-01-29)

| Entity | Count |
|--------|-------|
| Documents | 13 |
| Evidence | 60 |
| Persons | 10 |
| Organizations | 3 |
| ComplaintAllegations | 18 (all PROVEN) |
| MotionClaims | 26 |
| Harms | 9 ($40,258.61) |
| LegalCounts | 4 |
| Total Nodes | 146 |
| Total Relationships | 340 |

---

# Phase Summary

| Phase | Status |
|-------|--------|
| Phase 0 - Initialization | ✅ Complete |
| Phase 1 - Foundations | ✅ Complete |
| Phase 2 - Query Layer | ✅ Complete |
| Phase 2.5 - Extended Query | ✅ Complete |
| Phase 2.6 - Graph Visualization | 🔄 Next |
| Phase 3 - Document Slice | ⏸️ Partial |
| Phase 5 - Import Pipeline | ⏸️ Partial |
| Phase 6-9 | ⏳ Future |

---

# Notes

- One Task ID → one branch → one persona → one layer
- Phase 2.x uses feature branches per Feature set
- L1+ tasks require tests
- Keep `main` deployable at all times
- Graph visualization is next priority

# End of TASK_TRACKER.md
