# Colossus-Legal — Documentation Index

This directory contains all project-level documentation for **Colossus-Legal**,  
including architecture, API design, data models, implementation phases, and task tracking.

Codex (v0.63+) must read this folder — especially `AGENTS.md` (in the repo root)  
and `TASK_TRACKER.md` — before performing engineering tasks.

---

# 📚 Documentation Structure

docs/
│
├── README.md ← You are here (documentation index)
├── TASK_TRACKER.md ← Active and historical task roadmap
│
├── ARCHITECTURE.md ← System architecture (backend + frontend + Neo4j)
├── API_DESIGN.md ← REST API specifications
├── DATA_MODEL.md ← Neo4j graph schema (nodes + relationships)
├── PHASE_PLAN.md ← Long-term implementation phases
│
└── … future docs


---

# 🧭 How To Use This Directory

## 1. Start With These
If you or Codex enters this repository for the first time, read:

- `AGENTS.md` (root-level, **defines Codex behavior and guardrails**)  
- `docs/TASK_TRACKER.md` (project plan and active tasks)

This ensures tasks are done **in order** and **within project constraints**.

---

## 2. Architecture Docs (Backend + Frontend + Graph)

### **ARCHITECTURE.md**
Describes:
- Backend layout (Axum modules, API routing, Neo4j state)
- Frontend layout (pages, components, services)
- How data moves through the system
- How backend / frontend / database interact

### **DATA_MODEL.md**
Defines:
- Neo4j nodes: Claim, Document, Evidence, Person, Hearing, Decision  
- Relationships: RELIES_ON, PRESENTED_AT, APPEARS_IN, REFUTES, IGNORES  
- Field definitions + constraints  
- Planned schema evolution

### **API_DESIGN.md**
Describes:
- REST endpoints  
- Request/response DTOs  
- CRUD patterns  
- Status codes  
- Relationship actions  

This is the “contract” between backend and frontend.

---

## 3. Process Docs

### **TASK_TRACKER.md**
Your **master roadmap**, including:
- Current baseline state  
- New post-reset tasks  
- Phased plan  
- Historical tasks (pre-reset)  
- What Codex is allowed to work on next  

### **PHASE_PLAN.md**
Long-term development plan:
- Phase 1: Foundations  
- Phase 2: Graph basics  
- Phase 3: Document ingestion  
- Phase 4: AI suggestion pipeline  
- Phase 5: Batch analysis  
- Phase 6: Reporting  

Codex must follow this and must not skip ahead.

---

# 🔧 How Codex Should Use This Directory

When Codex performs a task:

1. Read `AGENTS.md` to understand behavior constraints  
2. Read `TASK_TRACKER.md` for the current phase and task  
3. If architecture or API changes are involved, read:
   - `ARCHITECTURE.md`
   - `API_DESIGN.md`
   - `DATA_MODEL.md`
   - `PHASE_PLAN.md`
4. Only modify files in the **active agent’s scope** (backend/frontend/docs)
5. Always keep changes small and focused  
6. Update `TASK_TRACKER.md` when major tasks change state  

---

# 📄 Additional Notes

- Documentation grows with features — add new docs here as needed  
- Avoid duplication — this folder should be the single source of truth for project structure  
- Keep examples, diagrams, and explanatory notes in this folder  
- Avoid storing credentials or sensitive data  

---

# End of docs/README.md

