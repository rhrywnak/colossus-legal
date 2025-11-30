# Colossus-Legal — TASK TRACKER
A structured task index for Codex, ChatGPT, and human contributors.
This tracker defines all phases and tasks in order, with Layers (L0–L3), Personae, branch patterns, and acceptance criteria.

It supports:
- Codex planning/execution prompts  
- Branching workflow  
- AGENTS.md persona/layer constraints  
- Phase progression discipline (0 → 1 → 2 → 3)

---

# Phase 0 — Project Initialization

> **Goal:** Establish minimum scaffolding, repo structure, high-level architecture docs, and baseline developer workflow.

### T0.1 — Repository Bootstrap
- **Status:** DONE  
- **Persona:** DocsAgent  
- **Description:** Initialize repository, folders, README, docs directory.
- **Acceptance Criteria:** GitHub repo live, basic README, empty backend/frontend dirs.

### T0.2 — Architecture & Workflow Docs
- **Status:** DONE  
- **Persona:** DocsAgent  
- **Description:** Produce ARCHITECTURE.md, WORKFLOW.md, DEV_ONBOARDING.md, DATA_MODEL.md, API_DESIGN.md.
- **Acceptance Criteria:** Docs created and usable by Codex; Phase 1 can start.

### T0.3 — Codex Safety Bundle & Process
- **Status:** DONE  
- **Persona:** DocsAgent  
- **Description:** Create CODEx-SESSION-RULES.md, CODEx-CHECKLIST.md, CODEx-PROMPT-TEMPLATE, session bootstrap procedure.
- **Acceptance Criteria:** Safety bundle stored in `docs/`, referenced by agents, used by all sessions.

---

# Phase 1 — Backend & Frontend Bootstrap

> **Goal:** Operational Rust backend (Axum + Neo4j), React/Tailwind frontend skeleton, routing, initial DTOs.

### T1.1 — Backend Bootstrap (Rust + Axum)
- **Status:** DONE  
- **Persona:** BackendAgent  
- **Criteria:** `/health` works; server boots; AppState; logging.

### T1.2 — Neo4j Integration & Test Harness
- **Status:** DONE  
- **Persona:** BackendAgent  
- **Criteria:** Driver setup; AppState includes Neo4j driver; minimal test harness touches DB.

### T1.3 — Frontend Bootstrap (React + Vite)
- **Status:** DONE  
- **Persona:** FrontendAgent  
- **Criteria:** SPA routing; dev server builds; basic layout.

---

# Phase 2 — Claims Slice (API + UI)  
This slice establishes the architectural pattern for the entire system.

### T2.1a — Claims API L0 (Skeleton)
- **Status:** DONE  
- **Layer:** L0  
- **Persona:** BackendAgent  
- **Criteria:** `/claims` route stub; ClaimDto exists; compiles.

### T2.1b — Claims API L1 (Happy Path)
- **Status:** DONE  
- **Layer:** L1  
- **Persona:** BackendAgent  
- **Criteria:** List claims from DB; repo + handler; tests for empty/non-empty.

### T2.1c — Claims API L2 (Validation)
- **Status:** DONE  
- **Layer:** L2  
- **Persona:** BackendAgent  
- **Criteria:** Create/update validation; 400/404; tests.

### T2.1d — Claims API L3 (Analysis)
- **Status:** DONE  
- **Layer:** L3  
- **Persona:** BackendAgent  
- **Criteria:** Graph relationships; traversal queries; tests.

---

### T2.2a — Claims UI L0 (Skeleton)
- **Status:** DONE  
- **Layer:** L0  
- **Persona:** FrontendAgent  
- **Criteria:** `/claims` route; stub service; basic list renders.

### T2.2b — Claims UI L1 (Backend Integration)
- **Status:** DONE  
- **Layer:** L1  
- **Persona:** FrontendAgent  
- **Criteria:** Real backend fetch; error/loading states; manual testing.

---

# Phase 3 — Document Slice (API + UI)

> **Goal:** Implement a full Document data slice, mirroring the Claims slice, with API, validation, analysis, UI, and integration.

---

### T3.1a — Document API L0 (Skeleton)
- **Status:** DONE (auto-satisfied by L1 start)
- **Layer:** L0  
- **Persona:** BackendAgent  
- **Criteria:** `/documents` route compiles; DocumentDto exists; stub handler.

### T3.1b — Document API L1 (Happy Path + Neo4j List)
- **Status:** DONE (2025-11-30)  
- **Layer:** L1  
- **Persona:** BackendAgent  
- **Criteria:**  
  - Document model + DocumentDto mapping  
  - `DocumentRepository::list_documents` (Cypher)  
  - GET /documents returns list  
  - Integration tests empty/non-empty  
  - cargo fmt/check/test pass  
- **Branch:** `feature/T3.1b-document-api-l1`  

---

### T3.1c — Document API L2 (Validation)
- **Status:** PLANNED  
- **Layer:** L2  
- **Persona:** BackendAgent  
- **Branch:** `feature/T3.1c-document-api-l2`  
- **Criteria:**  
  - Implement:  
    - POST /documents  
    - GET /documents/{id}  
    - PUT/PATCH /documents/{id}  
  - Request validation  
  - Error handling (400/404)  
  - Integration tests (valid + invalid)

---

### T3.1d — Document API L3 (Analysis)
- **Status:** PLANNED  
- **Layer:** L3  
- **Persona:** BackendAgent  
- **Branch:** `feature/T3.1d-document-api-l3`  
- **Criteria:**  
  - Analysis endpoints  
  - Graph traversal queries  
  - Nontrivial integration tests

---

### T3.2a — Document UI L0 (Skeleton)
- **Status:** PLANNED  
- **Layer:** L0  
- **Persona:** FrontendAgent  
- **Branch:** `feature/T3.2a-document-ui-l0`  
- **Criteria:**  
  - `/documents` route  
  - Skeleton page  
  - Stub service

---

### T3.2b — Document UI L1 (Integration)
- **Status:** PLANNED  
- **Layer:** L1  
- **Persona:** FrontendAgent  
- **Branch:** `feature/T3.2b-document-ui-l1`  
- **Criteria:**  
  - Connect to backend  
  - Display real DocumentDto list  
  - Loading/error states  

---

### T3.3 — Document Slice Integration + Docs
- **Status:** PLANNED  
- **Layer:** L1  
- **Persona:** DocsAgent  
- **Branch:** `feature/T3.3-document-integration-l1`  
- **Criteria:**  
  - Update DEV_ONBOARDING, WORKFLOW, API_DESIGN  
  - End-to-end validation  
  - TASK_TRACKER updated  
  - Claims + Documents slice work verified together

---

# Phase 4 — Relationship APIs & Basic Analysis

### T4.1 — Relationship Endpoints (APPEARS_IN, RELIES_ON, PRESENTED_AT, ...)  
- **Status:** FUTURE  

### T4.2 — Basic Analysis (Refuted Claims, Paths, Timeline)  
- **Status:** FUTURE  

---

# Phase 5 — Document Upload & Text Extraction

All FUTURE tasks.

### T5.1 — File Upload Endpoint  
### T5.2 — PDF/DOCX/OCR Extraction Pipeline  
### T5.3 — Upload UI  

---

# Phase 6 — AI Suggestion Pipeline

### T6.1 — LLM Integration  
### T6.2 — AI Suggestion Model  

All FUTURE.

---

# Phase 7 — Reporting & Visualization

### T7.1 — PDF/Doc Export  
### T7.2 — Graph Views & Timelines  
### T7.3 — UX Polish  

---

# Notes

| Phase | Status |
|-------|--------|
| Phase 0 | ✅ Complete |
| Phase 1 | ✅ Complete |
| Phase 2 | ✅ Complete |
| Phase 3 | 🚧 In Progress |
| Phase 4 | Not Started |


- Each task corresponds to a feature branch.  
- Codex planning prompts must obey persona/layer + allowed file lists.  
- All tasks must follow AGENTS.md + WORKFLOW.md.
- All L1+ tasks require tests.

# End of TASK_TRACKER.md  
