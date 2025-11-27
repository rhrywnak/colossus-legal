# Colossus-Legal – Phase Plan

This document describes the phased implementation plan for Colossus-Legal.
It is a roadmap, not a rigid Gantt chart. Phases may overlap, but **must not be skipped**.

See also: `TASK_TRACKER.md` for concrete tasks and status.

---

## Phase 0 – Recovery & Baseline (DONE)

Goals:
- Recover repository from Codex refactor.
- Establish clean `main` with:
  - Minimal backend (health + status)
  - Minimal frontend (status panel) 
- Preserve all prior work in WIP branch.

Status:
- ✅ Completed (Nov 2025).

---

## Phase 1 – Foundations (IN PROGRESS)

Focus: Stable backend + frontend skeletons.

### Backend
- Minimal `AppState` + Neo4j wiring.
- `/health` and `/api/status` stable.
- Logging via `tracing`.
- Claims domain types defined (models + DTOs).

### Frontend
- App shell and navigation.
- Status panel calling `/api/status`.
- Placeholder pages for key domains.

Exit Criteria:
- Backend and frontend both:
  - Compile
  - Run
  - Provide basic status and navigation

---

## Phase 2 – Claims API v1 (NEXT)

Focus: Implement real Claims API and UI.

**Current status (slice integrated):** Claims v1 slice is usable end-to-end (backend Claims API L1–L2 + frontend Claims UI L1). Remaining Phase 2 items (e.g., T2.1d analysis endpoints, richer relationships/UX) are still FUTURE.

### Backend
- Implement Claims repository with Neo4j queries.
- Add:
  - `GET /claims`
  - `GET /claims/{id}`
  - `POST /claims`
  - `PUT /claims/{id}`
  - `DELETE /claims/{id}` (soft or hard delete).
- Add tests (unit + integration where practical).

### Frontend
- Claims list page:
  - Table or simple list.
- Claim detail view:
  - Basic fields.
- Create/edit forms:
  - Minimal validation.

Exit Criteria:
- Claims can be created, read, updated, and (soft) deleted via UI.
- Basic tests in place.

---

### 📘 Tutorial: How to Work a Phase (Example: Claims API v1)

---

## Layered Execution Per Phase

Each phase is built in **Layer 0–3** increments across the stack:

- L0: Skeleton (routes/pages/docs, stubs allowed)
- L1: Real data (happy path)
- L2: Validation, errors, relationships
- L3: Analysis, AI, and polish

Example for Phase 2 (Claims API v1):

- T2.1a – Claims API L0 (routes + stubs)
- T2.1b – Claims API L1 (real Neo4j list)
- T2.1c – Claims API L2 (validation + errors)
- T2.1d – Claims API L3 (analysis endpoints)

Each layer:
- Must compile (backend + frontend).
- Must be manually verified.
- Can be tagged and deployed as its own version slice.


1. **Create a feature branch**  
   ```bash
   git switch main
   git switch -c feature/claims-api-v1
   ```

2. **Backend tasks**  
   - Follow the tutorial in `ARCHITECTURE.md` and `API_DESIGN.md` for `GET /claims`.
   - Add repository methods, handlers, DTOs.
   - Ensure `cargo check` and basic tests pass.

3. **Frontend tasks**  
   - Add `ClaimsPage` and `claims` service.
   - Wire `/claims` route and confirm list renders.

4. **Update docs**  
   - Mark relevant items in `TASK_TRACKER.md` as completed.
   - Add notes to any session/Dev logs if in use.

5. **Merge into main**  
   ```bash
   git switch main
   git merge --no-ff feature/claims-api-v1
   git push origin main
   ```

This is the pattern for every subsequent phase: create a branch, implement a thin vertical slice, update docs, merge when stable.

---

## Phase 3 – Graph Core: Documents, Evidence, People, Hearings, Decisions

Focus: Flesh out main domain entities.

### Backend
- Add models/DTOs for:
  - Document
  - Evidence
  - Person
  - Hearing
  - Decision
- Add CRUD endpoints for each (mirroring Claims).
- Add repositories with Neo4j queries.

### Frontend
- Simple screens for each domain:
  - Table views
  - Detail views
  - Create/edit forms

Exit Criteria:
- All primary node types can be managed via UI.
- Neo4j graph visually reflects case structure.

---

## Phase 4 – Relationship APIs & Basic Analysis

Focus: Connect the entities and enable simple analysis.

### Backend
- Relationship endpoints:
  - APPEARS_IN, RELIES_ON, PRESENTED_AT, MADE_BY, DECIDES, REFUTES, IGNORES.
- Basic analysis endpoints:
  - `/analysis/paths/{claim_id}`
  - `/analysis/refuted-claims`
  - `/analysis/timeline`

### Frontend
- Controls to link:
  - Claims ↔ Documents
  - Claims ↔ Evidence
  - Evidence ↔ Hearings
  - Claims ↔ Decisions
- Views:
  - Simple timeline
  - Relationship lists

Exit Criteria:
- User can “walk” the graph through UI.
- Key analysis questions answerable with existing endpoints.

---

## Phase 5 – Document Ingestion & Text Extraction

Focus: Getting real data in.

### Backend
- File upload endpoint(s).
- Storage of raw files (local path or object storage).
- Extraction pipeline:
  - PDF text
  - DOCX text
  - OCR for images (Tesseract).

### Frontend
- Upload UI with:
  - File drop zone
  - Upload progress
  - Extraction status

Exit Criteria:
- Users can upload documents and see extracted text attached to Document nodes.

---

## Phase 6 – AI Suggestion Pipeline

Focus: Add smart assistance on top of the graph.

### Backend
- LLM integration (Claude / OpenAI / local models).
- AISuggestion nodes and relationships.
- Endpoints like `/documents/analyze`.

### Frontend
- Review queue:
  - Show suggested claims/evidence/relations.
  - Approve/reject suggestions.
- UI to preview AI suggestions in context.

Exit Criteria:
- System can propose new graph nodes/edges and user can control what is accepted.

---

## Phase 7 – Reporting, Visualization, and Polish

Focus: Make the system pleasant and effective to use.

### Features
- PDF/Doc exports:
  - Claim-level reports.
  - Case summaries.
- Visual graph views (Cytoscape or similar).
- Timelines and dashboards.
- UX refinements:
  - Faster navigation
  - Better error handling
  - Helpful inline explanations/tutorial hints.

Exit Criteria:
- System feels cohesive and usable for real case work.
- Core flows (from ingestion to reporting) are robust and documented.

---

# End of PHASE_PLAN.md
