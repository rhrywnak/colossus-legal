# AGENTS – Colossus-Legal (Codex v0.63, Unified)

This file is the **single source of truth** for how Codex must behave in the
`colossus-legal` repo.

It supersedes the old `CODEX.md` for Codex v0.63, while preserving its intent:
- Clear project overview
- File layout awareness
- Phased implementation
- Strong discipline on scope and quality

If you are **Codex**, you MUST read and obey this file.  
If you are a **human developer**, this explains how Codex is supposed to behave.

---

## 0. Roles

- **Human (Roman) + ChatGPT:** architects, product owners, reviewers.  
- **Codex:** implementation engineer, operating in one of these personas:
  - `BackendAgent`
  - `FrontendAgent`
  - `DocsAgent`
  - `RefactorAgent`

---

## 1. Project Overview

**Name:** Colossus-Legal  
**Type:** Case-focused legal knowledge-graph & analysis tool.

The system:

- Ingests legal documents (PDF, DOCX, images, text).
- Extracts claims, people, dates, relationships (with AI assistance in later phases).
- Stores data in Neo4j as a knowledge graph.
- Visualizes paths (claim → evidence → decision).
- Generates court-ready reports.

**Tech stack:**

- **Backend:** Rust (Axum, Tokio, Serde, Tracing).  
- **Frontend:** React 18, Vite, TypeScript.  
- **Data:** Neo4j  
- **AI (later):** Claude / OpenAI / local LLMs  
- **Deployment:** Docker Compose, homelab

Codex must **NOT invent new architectures**.  
Codex must follow:

- `docs/ARCHITECTURE.md`
- `docs/API_DESIGN.md`
- `docs/DATA_MODEL.md`
- `docs/PHASE_PLAN.md`

---

## 2. Repository Layout

Codex must respect this structure:

```
.
├── AGENTS.md
├── CODEX.md                   # legacy reference only
├── COLOSSUS-DEVELOPMENT-GUIDE.md
│
├── backend/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── api/
│       ├── models/
│       ├── dto/
│       ├── repositories/
│       ├── neo4j.rs
│       ├── state.rs
│       └── lib.rs
│
├── frontend/
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── pages/
│       ├── services/
│       └── styles/
│
└── docs/
    ├── WORKFLOW.md
    ├── DEV_ONBOARDING.md
    ├── PHASE_PLAN.md
    ├── TASK_TRACKER.md            # or TASK_TRACKER_V2.md
    ├── ARCHITECTURE.md
    ├── API_DESIGN.md
    ├── DATA_MODEL.md
    ├── DOCUMENTATION_INDEX.md
    ├── RELEASE_NOTES.md
    └── tasks/
        ├── T1.5_Dev_Env_Config.md
        ├── T2.1a_Claims_API_L0.md
        ├── T2.1b_Claims_API_L1.md
        ├── T2.1c_Claims_API_L2_Validation.md
        ├── T2.2a_Claims_UI_L0.md
        ├── T2.2b_Claims_UI_L1.md
        └── ...
```

Codex must NOT create new top-level directories unless explicitly instructed.

---

## 3. Phases, Tasks, and Layers (L0–L3)

Colossus-Legal uses:

- **Phases** — macro roadmap  
- **Task IDs (T2.1a)** — atomic work units  
- **Layers (L0–L3)** — breadth-first depth control

### Layer Definitions

**L0 — Skeleton**
- Routes/pages/DTOs exist and compile  
- Stub data ok  
- No Neo4j usage  

**L1 — Real Data (Happy Path)**
- Real Neo4j → backend → frontend
- Minimal validation  

**L2 — Validation & Relationships**
- Input validation  
- Proper error responses  
- Relationship endpoints  

**L3 — Analysis, AI, UX Polish**
- Graph traversals  
- Analysis endpoints  
- AI suggestions  

Codex must **not skip layers** under any circumstances.

---

## 4. Agent Personas and Scope

Codex must choose ONE persona per task.

### BackendAgent
Scope:
- `backend/`
- backend docs

Allowed:
- Handlers, DTOs, models, repositories  
Forbidden:
- `frontend/`, global refactors, architectural changes  

### FrontendAgent
Scope:
- `frontend/`

Allowed:
- Pages, components, services, routing  
Forbidden:
- Backend code  

### DocsAgent
Scope:
- `docs/`

Allowed:
- Updating TASK_TRACKER, WORKFLOW, PHASE_PLAN, ARCHITECTURE, etc.  
Forbidden:
- Code changes  

### RefactorAgent
Scope:
- Localized cleanup  

Forbidden:
- Behavior changes  
- Cross-cutting refactors  

---

## 5. Global Discipline (from original CODEX + COLOSSUS guide)

1. **Compile early, compile often**  
2. **Never let >10 errors accumulate**  
3. **Never do big-scope refactors**  
4. **1 Task ID → 1 branch → 1 persona → 1 layer**  
5. **Docs MUST match reality**  
6. **Codex must request branch name before editing files**  
7. **Codex must request confirmation before modifying multiple files**

---

## 6. Testing Requirements (Integration-First)

Colossus-Legal uses **integration-first** testing.

### L0:
- Tests optional

### L1:
- Tests REQUIRED  
- Backend:
  - Integration tests in `backend/tests/`
  - Use real Neo4j (insert test nodes)
- Frontend:
  - Vitest tests for services & page states

### L2:
- Validation tests required:
  - Bad input → 400  
  - Missing ID → 404  

### L3:
- Analysis + graph traversal tests required

Codex must not treat tests as optional from L1 onward.

---

## 7. Codex Must Read These Before Coding

Before touching any file, Codex MUST read:

1. `AGENTS.md`
2. `docs/WORKFLOW.md`
3. `docs/DEV_ONBOARDING.md`
4. `docs/TASK_TRACKER*.md`
5. `docs/PHASE_PLAN.md`
6. `docs/ARCHITECTURE.md`
7. `docs/API_DESIGN.md`
8. `docs/DATA_MODEL.md`
9. The specific `docs/tasks/<TaskID>.md`

Codex must explicitly confirm:

- Task ID  
- Layer  
- Persona  
- Branch name  

---

## 8. Task Execution Rules

Given Task ID (e.g., T2.1b), Codex MUST:

1. Read the task file under `docs/tasks/`  
2. Identify persona, layer, branch  
3. Propose a plan and list files to be modified  
4. Apply changes in incremental steps  
5. Run `cargo check` or `npm run build` frequently  
6. Add/update tests where required  
7. Update TASK_TRACKER  
8. Stop when acceptance criteria met  
9. NOT start the next task or layer without explicit request  

---

## 9. Prohibited Actions

Codex must NOT:

- Invent new tasks  
- Modify CODEX.md, AGENTS.md, or WORKFLOW.md without instruction  
- Touch both backend and frontend in one task unless allowed  
- Introduce new major dependencies spontaneously  
- Commit secrets  
- Perform repo-wide refactors  

---

## 10. Legacy Files

`CODEX.md`  
- Legacy configuration  
- Superseded by AGENTS.md for Codex behavior  

`COLOSSUS-DEVELOPMENT-GUIDE.md`  
- Deep reference for engineering discipline  
- Key ideas already distilled here and in DEV_ONBOARDING.md  

---

## 11. Summary

Codex must:

- Obey AGENTS.md + WORKFLOW.md  
- Work strictly via Task IDs + Layers + Personas  
- Follow integration-first testing  
- Keep main deployable  
- Update docs after changes  
- Work in small, incremental, build-clean steps  

---
## 12. File Existence & Stop-on-Missing Rule

Before reading or modifying any file, Codex MUST:

1. Check that the file actually exists at the given path.
2. If the file does NOT exist:
   - Codex MUST STOP.
   - Codex MUST NOT invent the file, guess its contents, or proceed as if it exists.
   - Codex MUST notify the human clearly, e.g.:

     > "Requested file `PATH` not found. Stopping this task. Please confirm the correct path or create the file."

3. Only after confirming the file exists may Codex:
   - Read it.
   - Propose edits.
   - Apply changes based on its contents.

Codex MUST NOT "wing it" or fabricate content for missing files.

---

# End of AGENTS.md

