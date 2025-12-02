# Colossus-Legal — Roadmap (v1.0)

This is the standalone, long-term, human-readable roadmap for the Colossus‑Legal system.
It is *not* a session transition document. It describes the project vision, phases, 
major capabilities, MVP goals, and long-term development trajectory.

---

# 1. Vision

Colossus‑Legal models a legal case as a **knowledge graph**, enabling:

- Clear structure of claims, documents, evidence, hearings, decisions, people  
- Relationship analysis (supports, refutes, mentions, appears-in, decides, etc.)  
- Cross‑linked insights between nodes  
- AI-assisted discovery of inconsistencies, missing evidence, and patterns  
- Timeline, reporting, and document analysis pipelines  

The system grows through **phases** (0–9) and **layers** (L0–L3) in a breadth‑first manner.

---

# 2. Current Functionality (as of 2025‑11‑30)

## Functional End‑to‑End Today (v0.1)

### Backend:
- Axum server running (Rust)
- Neo4j driver connected
- Claims API (L0–L3) fully implemented  
- Documents API:
  - L0: skeleton endpoint
  - L1: real Neo4j list endpoint (`GET /documents`)
  - Integration tests working

### Frontend:
- React/Vite SPA running
- Claims page (full integration)
- Documents page (full integration)
- Vitest tests pass
- Able to load and view real data from Neo4j

### End‑to‑End:
- Browser → frontend → backend → Neo4j → UI  
- Claims list works  
- Documents list works  
- Fully functional case‑browser baseline

This is **Colossus‑Legal v0.1**:  
*A working, read-only graph browser for Claims and Documents.*

---

# 3. Minimum Viable Product (v0.2) — “Is this useful?”

This MVP focuses on minimal editing and minimal insights to decide whether Colossus‑Legal
is worth deeper investment.

## v0.2 Must Include:

### M1 — Minimal Document CRUD (Backend)
- `POST /documents`
- `GET /documents/{id}`
- `PUT/PATCH /documents/{id}`
- Light validation: `title` + `doc_type` required

### M2 — Document Detail View (Frontend)
- Route: `/documents/:id`
- Render:
  - title
  - docType
  - createdAt
  - description (optional)

### M3 — One Insight Endpoint (Backend)
Example options:
- `/documents/recent`
- `/claims/unlinked`
- `/documents/orphans`

### M4 — (Optional) First Relationship Endpoint
- `POST /claims/{claim_id}/mentions/{document_id}`

This allows loading a small case (3–5 claims, 5–10 documents) and testing whether the system
adds value.

---

# 4. Full Future Roadmap (Phases 0–9)

## Phase 0 — Initialization (DONE)
- Repo scaffolding  
- Architecture docs  
- Codex safety bundle  

## Phase 1 — Foundations (DONE)
- Backend skeleton  
- Neo4j integration  
- Frontend skeleton  

## Phase 2 — Claims Slice (DONE)
- API L0–L3  
- UI L0–L1  

## Phase 3 — Document Slice (IN PROGRESS)
- API L0–L1 DONE  
- UI L0–L1 DONE  
- API L2 (Validation)  
- API L3 (Analysis)  
- Slice integration docs  

## Phase 4 — Core Graph Expansion
- Evidence  
- People  
- Hearings  
- Decisions  
(API + UI L0–L1)

## Phase 5 — Relationship APIs
- APPEARS_IN  
- RELIES_ON  
- PRESENTED_AT  
- MADE_BY  
- DECIDES  
- REFUTES  
- IGNORES  

## Phase 6 — Graph Analysis Layer
- Paths  
- Refuted claims  
- Timelines  
- Multi-hop insights  

## Phase 7 — Document Upload & Extraction
- File upload  
- PDF/DOCX extraction  
- OCR pipeline  

## Phase 8 — AI Suggestion Pipeline (AI ONLY)
- LLM integration  
- Graph suggestion engine  
- AI relationship inference  

## Phase 9 — Reporting & Visualization (UI ONLY)
- Graph views  
- Timelines  
- Dashboards  
- PDF/Doc exports  

---

# 5. Big‑Picture Summary

You have:
- A working end‑to‑end system for Claims and Documents  
- A stable foundation (backend + frontend + Neo4j)  
- A structured tasks and phases roadmap  
- Codex safety rails in place  
- A clear MVP path  

What you need next (v0.2):
- Document CRUD (minimal)
- Document detail view
- One small insight query

This will allow you to **experience the product**, not just build parts of it.

---

# End of ROADMAP (v1.0)
