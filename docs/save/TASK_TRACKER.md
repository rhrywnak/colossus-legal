# Colossus-Legal — Task Tracker (Post-Reset Baseline, Nov 2025)

This file defines all high-level tasks for the Colossus-Legal project.
You + ChatGPT act as **Architects / PMs**, and Codex is the **implementing engineer**.

Codex (v0.63+) must read this file *and* `AGENTS.md` before performing any task.

---

# 1. Current Baseline (Nov 2025)

### Backend
- [x] Backend compiles (`cargo check`)
- [x] Backend runs (`cargo run`)
- [x] `/health` and `/api/status` work
- [x] Env vars loaded (NEO4J_URI, etc.)
- [x] Clean `main` branch committed and pushed

### Frontend
- [x] Restored from WIP snapshot
- [x] Builds (`npm run build`)
- [x] Dev server runs (`npm run dev`)
- [x] Successfully calls `/api/status`

### Git & Repo Health
- [x] WIP snapshot preserved (`wip/codex-refactor-2025-11`)
- [x] Stable `main`
- [x] AGENTS.md updated
- [x] Docs restored
- [x] Repo safe for Codex v0.63

---

# 2. New Baseline Tasks (Forward-Looking)

## Phase A — Backend API Reconstruction
- [ ] Define Claims API v1 surface
- [ ] Pull minimal Claims logic from WIP
- [ ] Clean/refactor ClaimRepository (Neo4j queries)
- [ ] Implement `/claims` CRUD endpoints (v1)
- [ ] Add integration tests

## Phase B — Frontend Reconstruction
- [ ] Build Claims List page
- [ ] Build Claims Detail page
- [ ] Add create/update forms
- [ ] Wire frontend services to Claims API
- [ ] Validate builds (`npm run build`, `npm run dev`)

## Phase C — Core Graph Model
- [ ] Define canonical graph schema
- [ ] Reintroduce Document/Evidence/Person models (incremental)
- [ ] Add repository functions
- [ ] Add graph validation queries

## Phase D — Neo4j Relationship APIs
- [ ] Implement APPEARS_IN, RELIES_ON, PRESENTED_AT, REFUTES, IGNORES
- [ ] Create Cypher queries
- [ ] Add endpoints & minimal UI integration

## Phase E — Document + Ingestion Pipeline
- [ ] File upload endpoint
- [ ] Build extraction queue
- [ ] Reintroduce PDF/DOCX/OCR processing
- [ ] Frontend upload UI

## Phase F — AI Assistance Pipeline
- [ ] Define prompts/templates
- [ ] Implement `/documents/analyze`
- [ ] Add AISuggestion model
- [ ] Build review UI

---

# 3. Historical Tasks (Pre-Reset, Reference Only)

*These tasks reflect the pre-reset Codex-generated work. They remain as historical context only.*

## Phase 0 — Bring-Up
- `/api/status` endpoint — DONE
- Frontend status panel — DONE
- Dev CORS — DONE

## Phase 1 — Foundations
- Backend skeleton — DONE
- Models & DTOs — DONE
- Stub CRUD endpoints — DONE
- Frontend skeleton pages — DONE

## Phase 2–6
(See WIP branch for original details)

---

# End of TASK_TRACKER.md
