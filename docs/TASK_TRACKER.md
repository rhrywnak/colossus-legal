# Colossus-Legal — Task Tracker

This file defines all high-level tasks for the Colossus-Legal project.  
You + ChatGPT act as **Architects / PMs**, and Codex is the **implementing engineer**.

Codex tasks should reference this file **and** `CODEX.md` when generating code.

---

# Task Tracker – Post-Reset Baseline (Nov 2025)

> This document was partially written before the major reset and stabilization.
> The original tasks remain as history. This new baseline reflects the current
> stable state of the repository.

## New Baseline Tasks

- [x] Create WIP snapshot (`wip/codex-refactor-2025-11`)
- [x] Reset `main` to clean foundation
- [x] Stabilize backend to minimal compiling baseline (/health ok)
- [ ] Define initial Backend API surface (Claims v1)
- [ ] Reintroduce Mongo/Neo4j/Repo logic in structured feature branches
- [ ] Bootstrap frontend into stable, compiling shape
- [ ] Rebuild end-to-end workflow in small steps

## Historical Tasks (Pre-Reset)

# Phase 0 — Wiring & Bring-Up (Smoke Test)

These tasks ensure the project runs end-to-end before implementing features.

## T0.1 — Add `/api/status` Endpoint (Backend) (DONE – 2025-11-22)
- Add `GET /api/status` returning JSON:
  ```json
  { "app": "colossus-legal-backend", "version": "0.1.0", "status": "ok" }
  ```
- Place code in `backend/src/main.rs` (later will move into an API module).
- Must return HTTP 200 with correct JSON.

## T0.2 — Frontend Status Panel (DONE – 2025-11-22)
- Add `src/services/api.ts` with `getStatus()` calling `/api/status`.
- Update `App.tsx` to show:
  - Loading
  - Success (`Backend OK — name + version`)
  - Failure (`Backend unreachable`)
- No external libs; just fetch + useState.

T0.3 – Dev CORS configured between 5473 and 3403 (DONE – 2025-11-22)

---

# Phase 1 — Foundations & Manual Workflow

Backend + Frontend minimal foundations, but **no Neo4j** yet.

## T1.1 — Backend Skeleton
- Ensure Axum 0.7 server (with `axum::serve`) starts at `BACKEND_PORT`.
- `/health` route works.
- Logging is enabled via `tracing`.

## T1.2 — Core Models & DTOs (Backend) (DONE – 2025-11-22)
Create basic structs (hard-coded, no DB yet):
- Claim
- Document
- Evidence
- Person
- Hearing
- Decision
- DTOs for create/update operations

## T1.3 — Basic CRUD Endpoints (Stubbed) (DONE – 2025-11-22)
Implement minimal endpoints:
- `/claims`
- `/documents`
- `/evidence`
- `/people`
- `/hearings`
- `/decisions`

Using:
- in-memory storage, OR
- static placeholder responses

Purpose = allow frontend integration before Neo4j exists.

T1.3b – Claim model/DTO normalization (DONE – 2025-11-22)

## T1.4 — Frontend Skeleton Pages (DONE – 2025-11-22)
Create stub pages (no data yet):
- Dashboard
- Claims
- Documents
- Evidence
- People
- Hearings
- Decisions

Add navigation.

---

# Phase 2 — Neo4j Integration & Basic Queries

## T2.1 — Neo4j Connection Layer
- Use `NEO4J_URI`, `NEO4J_USER`, `NEO4J_PASSWORD`
- Test connection on startup, log success/failure

T2.1 – Neo4j connection established and pinged at startup using env config (DONE – 2025-11-22)

## T2.2 — Neo4j Repositories
Implement read/write functions for:
- Claim
- Document
- Evidence
- Person
- Hearing
- Decision

T2.2a – Added AppState with Neo4j Graph and wired it into Axum (DONE – 2025-11-23).

## T2.3 — Relationship APIs
Implement actions to create:
- APPEARS_IN
- REFUTES
- RELIES_ON
- PRESENTED_AT
- IGNORES

## T2.4 — Analysis Endpoints
Add endpoints:
- `/analysis/refuted-claims`
- `/analysis/paths/{claim_id}`
- `/analysis/timeline`

Frontend still uses simple tables/lists for now.

---

# Phase 3 — Document Upload & Text Extraction

## T3.1 — File Upload Endpoint
- Upload to `uploads/`
- Track in Document node

## T3.2 — Text Extraction Pipeline
- PDF → extract
- DOCX → extract
- Image → OCR with Tesseract
- Store extracted text in `extracted_text/`

## T3.3 — Frontend Upload UI
- Drag & drop file upload
- List of documents with extraction status
- Simple text viewer

---

# Phase 4 — AI Suggestion Pipeline

## T4.1 — LLM Service
- Claude API + Local Ollama support
- Select provider per request
- Use prompt templates in `/prompts`

## T4.2 — AI Suggestion Model
Nodes in Neo4j:
- AISuggestion (pending, approved, rejected)

## T4.3 — Document Analysis Endpoint
- `/documents/analyze`
- Extract → LLM → Store suggestions → Await review

## T4.4 — Review Queue UI
Frontend:
- List suggestions
- Approve / Edit / Reject
- Merge accepted suggestions into graph

---

# Phase 5 — Batch Analysis & Advanced Patterns

## T5.1 — Batch Document Processing
- Upload 10–50 files
- Show progress UI

## T5.2 — Contradiction Detection
- `/analysis/contradictions`
- LLM-based document-to-document checking

## T5.3 — Statistics Dashboard
- Number of claims, refuted claims, ignored evidence, etc.
- AI vs Manual entry ratios

---

# Phase 6 — PDF Exports & UX Polish

## T6.1 — PDF Export Service
- `/export/pdf/claim/{id}`
- `/export/pdf/report`
- Court-ready documents

## T6.2 — Visualizations
Graph + Timeline:
- Cytoscape.js
- Recharts (or simple D3)
- Highlight AI vs Manual nodes

## T6.3 — QA, Error Handling, Graceful Failures
- Better errors
- Autosave forms
- Undo operations where possible

---

# Notes

- Codex must follow `docs/ARCHITECTURE.md` and `CODEX.md`
- Tasks must be executed in **order** (phases build on each other)
- You + ChatGPT provide architecture; Codex must not improvise

---

# End of TASK_TRACKER.md
