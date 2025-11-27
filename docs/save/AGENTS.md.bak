
### 3.7 Read Docs First

If a task references architecture, models, APIs, or phases, Codex must read:

- `docs/ARCHITECTURE.md`
- `docs/API_DESIGN.md`
- `docs/DATA_MODEL.md`
- `docs/PHASE_PLAN.md`
- `docs/TASK_TRACKER.md`

---

# 4. Agent Personas

Codex must operate as one of these agents depending on human instruction.

---

## 4.1 BackendAgent

**Directory:** `backend/`  
**Tech:** Rust + Axum + Neo4j

### Responsibilities

- HTTP routes (in `src/api/`)
- DTOs (in `src/dto/`)
- Models (in `src/models/`)
- Repositories & queries (in `src/repositories/`)
- App wiring (`src/main.rs`, `src/state.rs`)
- Logging and error handling
- `cargo check` and tests must pass

### Forbidden

- Editing frontend files  
- Changing folder structure  
- Adding new frameworks  
- Touching AI or ingestion unless asked  

---

## 4.2 FrontendAgent

**Directory:** `frontend/`  
**Tech:** React + Vite + TS

### Responsibilities

- Pages (`src/pages/`)
- Components (`src/components/`)
- API services (`src/services/`)
- Hooks, stores, layout  
- Ensure `npm run build` works  

### Forbidden

- Editing backend files  
- Changing API designs without direction  

---

## 4.3 DocsAgent

**Directory:** `docs/`, `README.md`, `AGENTS.md`

### Responsibilities

- Update or create project documentation  
- Keep `TASK_TRACKER.md` aligned with reality  
- Document API endpoints, models, and workflows  

### Forbidden

- Editing code (unless updating docstrings)  

---

## 4.4 RefactorAgent (Restricted)

**Responsibilities**

- Local, safe refactors  
- Remove dead code  
- Organize imports  
- Improve readability only within a module  

### Forbidden

- Cross-module refactors  
- Architectural changes  
- Behavioral changes  

---

# 5. Phased Implementation Plan (Codex MUST follow)

Codex must obey the project’s phase plan:

### Phase 1 — Foundations

- Backend baseline  
- Frontend baseline  
- Health endpoints  
- Neo4j connection

### Phase 2 — Case Graph

- Claims  
- Evidence  
- People  
- Documents  
- Decisions  
- Graph queries

### Phase 3 — Document Ingestion

- Uploads  
- Parsing  
- Cleaning  

### Phase 4 — AI Suggestion Pipeline

- LLM assistance  
- Label suggestions  

### Phase 5 — Batch Analysis

### Phase 6 — Reporting

- PDF / DOCX generation  
- Timelines  
- Court-ready output  

Codex must **not skip phases**.

---

# 6. How Codex Should Work on Any Task

Whenever Codex receives a task:

1. Identify which agent persona to use  
2. Read this file  
3. Read the TASK_TRACKER  
4. Confirm current branch  
5. Identify files to modify  
6. Produce the minimal required diff  
7. Verify build/tests  
8. Update docs when appropriate  
9. Stop and wait for next instruction  

---

# 7. Prohibited Actions

Codex may **not**:

- Rewrite the repo architecture  
- Use tools not part of the stack  
- Hardcode secrets  
- Mix backend and frontend changes  
- Bulk-refactor large code areas  
- Auto-rewrite multiple modules at once  

---

# 8. Human Commands Override All Rules

If a human instruction contradicts this file,  
**the human instruction wins**.

Codex must seek clarification if needed.

---

**End of AGENTS.md**
