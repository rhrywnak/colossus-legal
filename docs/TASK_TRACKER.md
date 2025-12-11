# Colossus-Legal — TASK TRACKER
A structured task index for Codex, ChatGPT, and human contributors.
This tracker defines all phases (0–9), tasks, layers (L0–L3), personae, branch patterns, statuses, and acceptance criteria.

It supports:
- Codex planning/execution prompts
- Branching workflow
- Persona-layer constraints (AGENTS.md)
- Breadth-first layered development (WORKFLOW.md)

---

# Phase 0 — Project Initialization

> **Goal:** Establish scaffolding, docs, workflow, and Codex safety rules.

### T0.1 — Repository Bootstrap
- **Status:** DONE
- **Persona:** DocsAgent
- **Criteria:** Repo created; README + docs folder.

### T0.2 — Architecture & Workflow Docs
- **Status:** DONE
- **Persona:** DocsAgent
- **Criteria:** ARCHITECTURE.md, WORKFLOW.md, API_DESIGN.md, DATA_MODEL.md present.

### T0.3 — Codex Safety Bundle
- **Status:** DONE
- **Persona:** DocsAgent
- **Criteria:** CODEx-SESSION-RULES.md, CODEx-CHECKLIST.md, CODEX-PROMPT-TEMPLATE.txt.

---

# Phase 1 — Foundations (Backend + Frontend Bootstrap)

> **Goal:** Minimal working backend + minimal working frontend.

### T1.1 — Backend Skeleton (Axum)
- **Status:** DONE
- **Layer:** L0
- **Persona:** BackendAgent
- **Criteria:** `/health` endpoint, logging, AppState.

### T1.2 — Neo4j Integration & Test Harness
- **Status:** DONE
- **Persona:** BackendAgent
- **Criteria:** Neo4j driver + integration test harness.

### T1.3 — Frontend Skeleton (React/Vite)
- **Status:** DONE
- **Persona:** FrontendAgent
- **Criteria:** SPA routing, dev server works.

### T1.5 — Backend Dev Env Configuration (Runtime Readiness & Test Isolation)
- **Status:** DONE (2025-11-26)
- **Layer:** L0
- **Persona:** BackendAgent + DocsAgent
- **Criteria:**
  - `backend/.env` loading via dotenv.
  - Test-marker system for Claims tests.
  - DEV_ONBOARDING updated with backend setup details.

---

# Phase 2 — Claims Slice (API + UI)

> **Goal:** Fully realize Claims (API L0→L3, UI L0→L1). Establish slice pattern.

### T2.1a — Claims API L0 (Skeleton)
- **Status:** DONE
- **Layer:** L0
- **Persona:** BackendAgent

### T2.1b — Claims API L1 (Real Neo4j List)
- **Status:** DONE
- **Layer:** L1
- **Persona:** BackendAgent

### T2.1c — Claims API L2 (Validation)
- **Status:** DONE
- **Layer:** L2
- **Persona:** BackendAgent

### T2.1d — Claims API L3 (Analysis)
- **Status:** DONE
- **Layer:** L3
- **Persona:** BackendAgent

---

### T2.2a — Claims UI L0 (Skeleton)
- **Status:** DONE
- **Layer:** L0
- **Persona:** FrontendAgent

### T2.2b — Claims UI L1 (Integration + Tests)
- **Status:** DONE
- **Layer:** L1
- **Persona:** FrontendAgent

### T2.3 — Claims End-to-End Integration + Docs Update
- **Status:** DONE (2025-11-27)
- **Layer:** L1
- **Persona:** DocsAgent
- **Criteria:**
  - E2E Claims flow verified.
  - TASK_TRACKER, WORKFLOW, API_DESIGN, DATA_MODEL updated.

---

# Phase 3 — Document Slice (API + UI)

> **Goal:** Implement a full Document slice (API L0–L3, UI L0–L1, docs).

### T3.1a — Document API L0 (Skeleton)
- **Status:** DONE (2025-12-03)
- **Layer:** L0
- **Persona:** BackendAgent

### T3.1b — Document API L1 (Neo4j Happy Path)
- **Status:** DONE (2025-12-03)
- **Layer:** L1
- **Persona:** BackendAgent

### T3.1c — Document API L2 (Validation + 400/404)
- **Status:** PLANNED
- **Layer:** L2
- **Persona:** BackendAgent

### T3.1d — Document API L3 (Analysis Queries)
- **Status:** PLANNED
- **Layer:** L3
- **Persona:** BackendAgent

## T3.1e — Document Insight L3 (Recent Documents)
- **Status:** DONE (2025-12-03)
- **Layer:** L3
- **Persona:** BackendAgent

---

### T3.2a — Document UI L0 (Skeleton)
- **Status:** DONE (2025-12-03)
- **Layer:** L0
- **Persona:** FrontendAgent

### T3.2b — Document UI L1 (Real API Integration + Tests)
- **Status:** DONE (2025-12-03)
- **Layer:** L1
- **Persona:** FrontendAgent

### T3.3 — Document Slice Integration + Docs
- **Status:** DONE (2025-12-02)
- **Layer:** L1
- **Persona:** DocsAgent
- **Notes:** Document slice integrated at L1. API L0–L1 + UI L0–L1 functional. Docs updated accordingly.

---

# Phase 4 — Core Graph Expansion (Evidence / Person / Hearing / Decision)

> **Goal:** Add CRUD + minimal UI (L0–L1) for remaining graph nodes.

### T4.1 — Evidence API + UI  
- **Status:** FUTURE

### T4.2 — Person API + UI  
- **Status:** FUTURE

### T4.3 — Hearing API + UI  
- **Status:** FUTURE

### T4.4 — Decision API + UI  
- **Status:** FUTURE

---

# Phase 5 — Relationship APIs (Graph Connections)

> **Goal:** Add relationship endpoints connecting graph elements.

### T5.1 — Relationship APIs L2  
- **Status:** FUTURE

### T5.2 — Relationship UI L1  
- **Status:** FUTURE

---

# Phase 6 — Analysis Layer (Graph Queries)

> **Goal:** Deep graph traversal & analysis queries.

### T6.1 — Analysis API L3  
- **Status:** FUTURE

### T6.2 — Analysis UI L3  
- **Status:** FUTURE

---

# Phase 7 — Document Upload & Extraction

### T7.1 — File Upload Endpoint  
- **Status:** FUTURE

### T7.2 — Text Extraction (PDF/DOCX/OCR)  
- **Status:** FUTURE

### T7.3 — Upload UI  
- **Status:** FUTURE

---

# Phase 8 — AI Suggestion Pipeline  
*(AI ONLY — no reporting here)*

> **Goal:** Introduce AI-driven insights, suggestions, and automated graph annotations.

### T8.1 — LLM Integration  
- **Status:** FUTURE

### T8.2 — AI Suggestion Model  
- **Status:** FUTURE

### T8.3 — Graph Suggestion / Annotation Engine  
- **Status:** FUTURE

---

# Phase 9 — Reporting & Visualization  
*(UI-only — all reporting, exports, dashboards, graph views)*

### T9.1 — Reporting & PDF/Doc Export  
- **Status:** FUTURE

### T9.2 — Graph Views & Timelines  
- **Status:** FUTURE

### T9.3 — Dashboards, UX Polish, Auto-Save  
- **Status:** FUTURE

---

# Phase Summary

| Phase | Status |
|-------|--------|
| Phase 0 | Complete |
| Phase 1 | Complete |
| Phase 2 | Complete |
| Phase 3 | In Progress |
| Phase 4 | Not Started |
| Phase 5 | Not Started |
| Phase 6 | Not Started |
| Phase 7 | Not Started |
| Phase 8 | Not Started |
| Phase 9 | Not Started |

---

# Notes
- One Task ID → one branch → one persona → one layer.
- Codex must follow AGENTS.md + WORKFLOW.md.
- L1+ tasks require tests.
- Keep `main` deployable at all times.

# End of TASK_TRACKER.md
